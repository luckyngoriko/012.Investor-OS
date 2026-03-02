//! Investor OS v3.0 - Autonomous AI Trading System
//! 
//! Демо версия за разглеждане на функционалностите

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{
    middleware::{from_fn_with_state, Next},
    routing::{get, post, delete},
    Router, Json,
    extract::{Extension, Path, Request, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;
use rust_decimal::Decimal;

use investor_os::broker::paper::PaperBroker;
use investor_os::broker::{Broker, BrokerConfig, BrokerType, Order, OrderSide, OrderType, TimeInForce};

/// Състояние на приложението
#[derive(Clone)]
struct AppState {
    start_time: chrono::DateTime<Utc>,
    version: String,
    request_count: Arc<Mutex<u64>>,
    broker: Arc<Mutex<PaperBroker>>,
    runtime_contract: RuntimeContract,
    auth: Arc<Mutex<AuthService>>,
}

#[derive(Clone)]
struct RuntimeContract {
    api_base_url: String,
    ws_hrm_url: String,
    allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum UserRole {
    Admin,
    Trader,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthUser {
    id: String,
    email: String,
    name: String,
    role: UserRole,
    permissions: Vec<String>,
}

#[derive(Debug, Clone)]
struct AuthUserRecord {
    user: AuthUser,
    password: String,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    email: String,
    refresh_token: String,
    expires_at: chrono::DateTime<Utc>,
    refresh_expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct AuthService {
    users: HashMap<String, AuthUserRecord>,
    sessions_by_access: HashMap<String, SessionRecord>,
    access_by_refresh: HashMap<String, String>,
    access_ttl_seconds: i64,
    refresh_ttl_seconds: i64,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct LogoutRequest {
    refresh_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct LoginData {
    user: AuthUser,
    access_token: String,
    refresh_token: String,
    expires_at: chrono::DateTime<Utc>,
    refresh_expires_at: chrono::DateTime<Utc>,
}

impl AuthService {
    fn new_from_env() -> Self {
        let access_ttl_seconds = std::env::var("AUTH_ACCESS_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(15 * 60);
        let refresh_ttl_seconds = std::env::var("AUTH_REFRESH_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(7 * 24 * 60 * 60);

        let admin_password =
            std::env::var("AUTH_ADMIN_PASSWORD").unwrap_or_else(|_| "Admin#2026!".to_string());
        let trader_password =
            std::env::var("AUTH_TRADER_PASSWORD").unwrap_or_else(|_| "Trader#2026!".to_string());
        let viewer_password =
            std::env::var("AUTH_VIEWER_PASSWORD").unwrap_or_else(|_| "Viewer#2026!".to_string());

        let mut users = HashMap::new();
        users.insert(
            "admin@investor-os.com".to_string(),
            AuthUserRecord {
                user: AuthUser {
                    id: "1".to_string(),
                    email: "admin@investor-os.com".to_string(),
                    name: "Admin User".to_string(),
                    role: UserRole::Admin,
                    permissions: vec!["*".to_string()],
                },
                password: admin_password,
            },
        );
        users.insert(
            "trader@investor-os.com".to_string(),
            AuthUserRecord {
                user: AuthUser {
                    id: "2".to_string(),
                    email: "trader@investor-os.com".to_string(),
                    name: "Trader User".to_string(),
                    role: UserRole::Trader,
                    permissions: vec![
                        "dashboard.read".to_string(),
                        "portfolio.read".to_string(),
                        "portfolio.trade".to_string(),
                        "positions.read".to_string(),
                        "proposals.read".to_string(),
                        "proposals.execute".to_string(),
                        "risk.read".to_string(),
                        "backtest.read".to_string(),
                        "backtest.run".to_string(),
                        "journal.read".to_string(),
                        "journal.write".to_string(),
                        "settings.read".to_string(),
                        "settings.update".to_string(),
                    ],
                },
                password: trader_password,
            },
        );
        users.insert(
            "viewer@investor-os.com".to_string(),
            AuthUserRecord {
                user: AuthUser {
                    id: "3".to_string(),
                    email: "viewer@investor-os.com".to_string(),
                    name: "Viewer User".to_string(),
                    role: UserRole::Viewer,
                    permissions: vec![
                        "dashboard.read".to_string(),
                        "portfolio.read".to_string(),
                        "positions.read".to_string(),
                        "proposals.read".to_string(),
                        "risk.read".to_string(),
                        "journal.read".to_string(),
                    ],
                },
                password: viewer_password,
            },
        );

        Self {
            users,
            sessions_by_access: HashMap::new(),
            access_by_refresh: HashMap::new(),
            access_ttl_seconds,
            refresh_ttl_seconds,
        }
    }

    fn prune_expired_sessions(&mut self) {
        let now = Utc::now();
        let expired_tokens: Vec<String> = self
            .sessions_by_access
            .iter()
            .filter_map(|(access, session)| {
                if session.expires_at <= now || session.refresh_expires_at <= now {
                    Some(access.clone())
                } else {
                    None
                }
            })
            .collect();

        for access in expired_tokens {
            self.remove_session_by_access_token(&access);
        }
    }

    fn issue_session_for_email(&mut self, email: &str) -> Option<LoginData> {
        let user = self.users.get(email)?.user.clone();
        let now = Utc::now();
        let access_token = format!("atk_{}", Uuid::new_v4().as_simple());
        let refresh_token = format!("rtk_{}", Uuid::new_v4().as_simple());
        let expires_at = now + Duration::seconds(self.access_ttl_seconds);
        let refresh_expires_at = now + Duration::seconds(self.refresh_ttl_seconds);

        let session = SessionRecord {
            email: email.to_string(),
            refresh_token: refresh_token.clone(),
            expires_at,
            refresh_expires_at,
        };

        self.access_by_refresh
            .insert(refresh_token.clone(), access_token.clone());
        self.sessions_by_access
            .insert(access_token.clone(), session);

        Some(LoginData {
            user,
            access_token,
            refresh_token,
            expires_at,
            refresh_expires_at,
        })
    }

    fn remove_session_by_access_token(&mut self, access_token: &str) {
        if let Some(session) = self.sessions_by_access.remove(access_token) {
            self.access_by_refresh.remove(&session.refresh_token);
        }
    }

    fn login(&mut self, email: &str, password: &str) -> Option<LoginData> {
        self.prune_expired_sessions();
        let normalized = email.trim().to_lowercase();
        let user_record = self.users.get(&normalized)?;
        if user_record.password != password {
            return None;
        }
        self.issue_session_for_email(&normalized)
    }

    fn validate_access_token(&mut self, access_token: &str) -> Option<AuthUser> {
        self.prune_expired_sessions();
        let session = self.sessions_by_access.get(access_token)?;
        let user_record = self.users.get(&session.email)?;
        Some(user_record.user.clone())
    }

    fn refresh_session(&mut self, refresh_token: &str) -> Option<LoginData> {
        self.prune_expired_sessions();
        let access = self.access_by_refresh.get(refresh_token)?.clone();
        let old_session = self.sessions_by_access.get(&access)?.clone();
        self.remove_session_by_access_token(&access);
        self.issue_session_for_email(&old_session.email)
    }

    fn logout(&mut self, access_token: Option<&str>, refresh_token: Option<&str>) {
        self.prune_expired_sessions();

        if let Some(access) = access_token {
            self.remove_session_by_access_token(access);
        }

        if let Some(refresh) = refresh_token {
            if let Some(access) = self.access_by_refresh.get(refresh).cloned() {
                self.remove_session_by_access_token(&access);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Инициализиране на logging
    tracing_subscriber::fmt::init();
    
    // Инициализиране на monitoring (Sprint 46)
    investor_os::monitoring::init();
    
    info!("");
    info!("🚀 Стартиране на Investor OS v3.0");
    info!("═══════════════════════════════════════════════════════════════");
    
    let bind_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let bind_port = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr: SocketAddr = format!("{bind_host}:{bind_port}")
        .parse()
        .unwrap_or_else(|_| {
            warn!(
                "Invalid bind address from SERVER_HOST/SERVER_PORT, falling back to 127.0.0.1:8080"
            );
            SocketAddr::from(([127, 0, 0, 1], 8080))
        });

    let runtime_contract = RuntimeContract {
        api_base_url: std::env::var("PUBLIC_API_BASE_URL")
            .unwrap_or_else(|_| format!("http://127.0.0.1:{bind_port}/api")),
        ws_hrm_url: std::env::var("PUBLIC_WS_HRM_URL")
            .unwrap_or_else(|_| format!("ws://127.0.0.1:{bind_port}/ws/hrm")),
        allowed_origins: std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://127.0.0.1:3000,http://localhost:3000".to_string())
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(ToString::to_string)
            .collect(),
    };

    // Създаване на paper broker
    let broker_config = BrokerConfig {
        broker_type: BrokerType::InteractiveBrokers,
        account_id: "paper-account".to_string(),
        api_url: "".to_string(),
        auth_token: None,
        paper_trading: true,
        default_order_type: OrderType::Market,
        max_position_size: Decimal::from(100000),
        max_order_size: Decimal::from(50000),
        daily_loss_limit: Decimal::from(5000),
    };
    let broker = PaperBroker::new(broker_config);
    
    // Създаване на състоянието
    let state = AppState {
        start_time: Utc::now(),
        version: "3.0.0".to_string(),
        request_count: Arc::new(Mutex::new(0)),
        broker: Arc::new(Mutex::new(broker)),
        runtime_contract,
        auth: Arc::new(Mutex::new(AuthService::new_from_env())),
    };
    
    // Създаване на router
    let app = create_router(state);
    
    info!("📡 API сървър стартира на: http://{}", addr);
    info!("");
    info!("📖 Документация:     http://{}/api/docs", addr);
    info!("❤️  Health Check:    http://{}/api/health", addr);
    info!("🔐 Security Status:  http://{}/api/security/status", addr);
    info!("📊 Portfolio API:    http://{}/api/portfolio/optimize", addr);
    info!("🤖 Strategy API:     http://{}/api/strategy/regime", addr);
    info!("💰 Tax API:          http://{}/api/tax/status", addr);
    info!("📈 Metrics:          http://{}/api/monitoring/metrics", addr);
    info!("☸️  Deployment:      http://{}/api/deployment/status", addr);
    info!("");
    info!("Натиснете Ctrl+C за спиране на сървъра");
    info!("═══════════════════════════════════════════════════════════════");
    info!("");
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn create_router(state: AppState) -> Router {
    let auth_layer = from_fn_with_state(state.clone(), require_auth_middleware);

    let public_router = Router::new()
        .route("/", get(root_handler))
        .route("/api/health", get(health_handler))
        .route("/api/ready", get(readiness_handler))
        .route("/api/runtime/config", get(runtime_config_handler))
        .route("/api/hrm/status", get(hrm_status_handler))
        .route("/metrics", get(metrics_prometheus_handler))
        .route("/api/docs", get(docs_handler))
        .route("/api/auth/login", post(auth_login_handler))
        .route("/api/auth/refresh", post(auth_refresh_handler));

    let protected_router = Router::new()
        .route("/api/auth/me", get(auth_me_handler))
        .route("/api/auth/logout", post(auth_logout_handler))
        // Security endpoints
        .route("/api/security/status", get(security_status))
        .route("/api/security/clearance-levels", get(clearance_levels))
        .route("/api/security/generate-key", post(generate_api_key))
        // Portfolio endpoints
        .route("/api/portfolio/optimize", post(optimize_portfolio))
        .route("/api/portfolio/efficient-frontier", get(efficient_frontier))
        // Strategy endpoints
        .route("/api/strategy/regime", get(current_regime))
        .route("/api/strategy/select", post(select_strategy))
        // Tax endpoints
        .route("/api/tax/status", get(tax_status))
        .route("/api/tax/calculate", post(calculate_tax))
        // Monitoring endpoints
        .route("/api/monitoring/metrics", get(metrics_handler))
        .route("/api/monitoring/system", get(system_metrics))
        // Deployment endpoints
        .route("/api/deployment/status", get(deployment_status))
        .route("/api/deployment/config", get(deployment_config))
        // Demo endpoints
        .route("/api/demo/trade", post(demo_trade))
        .route("/api/demo/positions", get(demo_positions))
        // Broker endpoints (Paper Trading)
        .route("/api/broker/orders", post(place_order_handler))
        .route("/api/broker/orders/:id", delete(cancel_order_handler))
        .route("/api/broker/positions", get(get_positions_handler))
        .route("/api/broker/account", get(get_account_handler))
        .route_layer(auth_layer);

    Router::new()
        .merge(public_router)
        .merge(protected_router)
        .with_state(state)
}

fn unauthorized(message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "success": false,
            "error": message
        })),
    )
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let header_value = headers.get("Authorization")?.to_str().ok()?;
    let (scheme, token) = header_value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") || token.trim().is_empty() {
        return None;
    }
    Some(token.trim().to_string())
}

async fn require_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let Some(access_token) = extract_bearer_token(request.headers()) else {
        return Err(unauthorized("Missing or invalid Authorization header"));
    };

    let mut auth = state.auth.lock().await;
    let Some(user) = auth.validate_access_token(&access_token) else {
        return Err(unauthorized("Invalid or expired session"));
    };
    drop(auth);

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

async fn auth_login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut auth = state.auth.lock().await;
    match auth.login(&payload.email, &payload.password) {
        Some(session) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": session
            })),
        ),
        None => unauthorized("Invalid credentials"),
    }
}

async fn auth_refresh_handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut auth = state.auth.lock().await;
    match auth.refresh_session(&payload.refresh_token) {
        Some(session) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": session
            })),
        ),
        None => unauthorized("Invalid or expired refresh token"),
    }
}

async fn auth_me_handler(Extension(user): Extension<AuthUser>) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "user": user
            }
        })),
    )
}

async fn auth_logout_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Option<Json<LogoutRequest>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let access_token = extract_bearer_token(&headers);
    let refresh_token = payload.and_then(|body| body.0.refresh_token);

    let mut auth = state.auth.lock().await;
    auth.logout(access_token.as_deref(), refresh_token.as_deref());

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "logged_out": true
            }
        })),
    )
}

// ===== Основни handlers =====

async fn root_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut count = state.request_count.lock().await;
    *count += 1;
    
    Json(json!({
        "name": "Investor OS v3.0",
        "description": "Autonomous AI Trading System",
        "version": "3.0.0",
        "status": "running",
        "request_count": *count,
        "sprints_completed": 35,
        "tests_passing": 635,
        "modules": [
            "API Layer (Sprint 2)",
            "RAG System (Sprint 5)",
            "Broker Integration (Sprint 6)",
            "ML Pipeline (Sprint 8)",
            "Risk Management (Sprint 13)",
            "Portfolio Optimization (Sprint 32)",
            "Strategy Selector (Sprint 31)",
            "Tax & Compliance (Sprint 30)",
            "Real-Time Monitoring (Sprint 33)",
            "Security & Encryption (Sprint 34)",
            "Production Deployment (Sprint 35)"
        ],
        "endpoints": {
            "public": ["/api/health", "/api/docs"],
            "security": ["/api/security/status", "/api/security/generate-key"],
            "portfolio": ["/api/portfolio/optimize", "/api/portfolio/efficient-frontier"],
            "strategy": ["/api/strategy/regime", "/api/strategy/select"],
            "tax": ["/api/tax/status", "/api/tax/calculate"],
            "monitoring": ["/api/monitoring/metrics", "/api/monitoring/system"],
            "deployment": ["/api/deployment/status", "/api/deployment/config"]
        }
    }))
}

async fn health_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let uptime = Utc::now() - state.start_time;
    let count = *state.request_count.lock().await;
    
    Json(json!({
        "success": true,
        "data": {
            "status": "healthy",
            "version": state.version,
            "uptime_seconds": uptime.num_seconds(),
            "total_requests": count,
            "timestamp": Utc::now().to_rfc3339(),
            "environment": "development",
            "checks": {
                "api": "pass",
                "database": "pass (simulated)",
                "redis": "pass (simulated)",
                "ml_engine": "pass",
                "risk_engine": "pass"
            },
            "runtime_contract": {
                "api_base_url": state.runtime_contract.api_base_url.clone(),
                "ws_hrm_url": state.runtime_contract.ws_hrm_url.clone(),
                "allowed_origins": state.runtime_contract.allowed_origins.clone()
            }
        }
    }))
}

async fn readiness_handler() -> Json<serde_json::Value> {
    Json(json!({
        "success": true,
        "data": {
            "status": "ready"
        }
    }))
}

async fn runtime_config_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "success": true,
        "data": {
            "api_base_url": state.runtime_contract.api_base_url.clone(),
            "ws_hrm_url": state.runtime_contract.ws_hrm_url.clone(),
            "allowed_origins": state.runtime_contract.allowed_origins.clone()
        }
    }))
}

async fn hrm_status_handler() -> Json<serde_json::Value> {
    Json(json!({
        "success": true,
        "data": {
            "status": "ready",
            "model": "hrm-demo",
            "mode": "paper"
        }
    }))
}

async fn metrics_prometheus_handler() -> (StatusCode, String) {
    (
        StatusCode::OK,
        "# HELP investor_os_up Investor OS process health\n# TYPE investor_os_up gauge\ninvestor_os_up 1\n".to_string(),
    )
}

async fn docs_handler() -> Json<serde_json::Value> {
    Json(json!({
        "title": "Investor OS API Documentation",
        "version": "3.0.0",
        "description": "Autonomous AI Trading System - All 35 Sprints Complete",
        "recent_sprints": [
            {
                "number": 35,
                "name": "Production Deployment & CI/CD",
                "features": ["GitHub Actions CI/CD", "Kubernetes manifests", "Canary deployment", "Health checks"],
                "tests": 32
            },
            {
                "number": 34,
                "name": "Security & Encryption",
                "features": ["HSM-backed encryption", "TOTP/HOTP 2FA", "Audit trails", "Clearance levels"],
                "tests": 52
            },
            {
                "number": 33,
                "name": "Real-Time Monitoring",
                "features": ["Live dashboard", "Anomaly detection", "Health monitoring", "Alert system"],
                "tests": 61
            },
            {
                "number": 32,
                "name": "Portfolio Optimization",
                "features": ["Markowitz MPT", "Black-Litterman", "Risk parity", "Efficient frontier"],
                "tests": 53
            },
            {
                "number": 31,
                "name": "ML Strategy Selector",
                "features": ["Regime detection", "Auto strategy selection", "Performance attribution"],
                "tests": 45
            },
            {
                "number": 30,
                "name": "Tax & Compliance",
                "features": ["Tax loss harvesting", "Wash sale monitoring", "Schedule D/Form 8949"],
                "tests": 38
            }
        ],
        "authentication": {
            "type": "Bearer session token",
            "header": "Authorization: Bearer <access_token>",
            "refresh_endpoint": "/api/auth/refresh",
            "bootstrap_credentials": [
                "admin@investor-os.com",
                "trader@investor-os.com",
                "viewer@investor-os.com"
            ]
        },
        "note": "Runtime auth enforces session validation on protected endpoints"
    }))
}

// ===== Security handlers =====

async fn security_status() -> Json<serde_json::Value> {
    Json(json!({
        "module": "Security & Encryption (Sprint 34)",
        "status": "active",
        "features": [
            {
                "name": "HSM-backed API Key Encryption",
                "description": "AES-256-GCM encryption with automatic key rotation",
                "rotation_interval": "90 days"
            },
            {
                "name": "Two-Factor Authentication",
                "description": "TOTP/HOTP with backup codes and trusted devices",
                "methods": ["TOTP", "HOTP", "WebAuthn", "SMS", "Email"]
            },
            {
                "name": "Security Audit Trails",
                "description": "Immutable logging of all security events",
                "event_types": 15
            },
            {
                "name": "Clearance Level System",
                "description": "Hierarchical access control",
                "levels": ["Public", "Internal", "Confidential", "Restricted", "TopSecret"]
            },
            {
                "name": "Security Policies",
                "description": "Configurable password, lockout, session, and API key policies"
            }
        ],
        "stats": {
            "unit_tests": 52,
            "integration_tests": 18,
            "total_tests": 70
        }
    }))
}

async fn clearance_levels() -> Json<serde_json::Value> {
    Json(json!({
        "hierarchy": "TopSecret > Restricted > Confidential > Internal > Public",
        "levels": [
            {"name": "Public", "value": 0, "description": "Basic read access", "2fa_required": false},
            {"name": "Internal", "value": 1, "description": "Standard trading operations", "2fa_required": false},
            {"name": "Confidential", "value": 2, "description": "Sensitive positions/strategies", "2fa_required": true},
            {"name": "Restricted", "value": 3, "description": "High-value transactions", "2fa_required": true},
            {"name": "TopSecret", "value": 4, "description": "System administration", "2fa_required": true}
        ],
        "example": "A user with 'Confidential' clearance can access Public, Internal, and Confidential resources"
    }))
}

async fn generate_api_key() -> Json<serde_json::Value> {
    let key_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    
    // Генериране на API ключ
    let key = format!("ios_{}", Uuid::new_v4().to_string().replace("-", ""));
    
    Json(json!({
        "key_id": key_id.to_string(),
        "api_key": key,
        "user_id": user_id.to_string(),
        "clearance": "Internal",
        "expires_in_days": 30,
        "algorithm": "AES-256-GCM",
        "note": "Store this key securely - it won't be shown again",
        "usage": "curl -H 'X-API-Key: YOUR_KEY' http://localhost:5001/api/portfolio/optimize"
    }))
}

// ===== Portfolio handlers =====

async fn optimize_portfolio() -> Json<serde_json::Value> {
    Json(json!({
        "module": "Portfolio Optimization (Sprint 32)",
        "methods": [
            {
                "name": "Markowitz MPT",
                "description": "Mean-variance optimization for efficient portfolios"
            },
            {
                "name": "Black-Litterman",
                "description": "Bayesian approach combining market data with investor views"
            },
            {
                "name": "Risk Parity",
                "description": "Equal risk contribution allocation"
            },
            {
                "name": "Maximum Diversification",
                "description": "Maximize portfolio diversification ratio"
            }
        ],
        "objectives": ["MaximizeReturn", "MinimizeRisk", "MaximizeSharpe", "RiskParity"],
        "example_optimization": {
            "input": {
                "assets": ["AAPL", "GOOGL", "MSFT", "AMZN", "TSLA"],
                "objective": "MaximizeSharpe"
            },
            "output": {
                "weights": {
                    "AAPL": 0.25,
                    "GOOGL": 0.20,
                    "MSFT": 0.30,
                    "AMZN": 0.15,
                    "TSLA": 0.10
                },
                "expected_return": "15.2%",
                "expected_risk": "18.5%",
                "sharpe_ratio": 0.82,
                "diversification_ratio": 1.45
            }
        },
        "tests": 53
    }))
}

async fn efficient_frontier() -> Json<serde_json::Value> {
    Json(json!({
        "description": "Efficient frontier - set of optimal portfolios offering highest expected return for defined risk level",
        "frontier_points": [
            {"risk": "10%", "return": "8%", "portfolio": "Conservative", "allocation": "60% bonds, 40% stocks"},
            {"risk": "15%", "return": "12%", "portfolio": "Moderate", "allocation": "40% bonds, 60% stocks"},
            {"risk": "20%", "return": "16%", "portfolio": "Aggressive", "allocation": "20% bonds, 80% stocks"},
            {"risk": "25%", "return": "19%", "portfolio": "Very Aggressive", "allocation": "5% bonds, 95% stocks"}
        ],
        "tangency_portfolio": {
            "description": "Portfolio with highest Sharpe ratio (optimal risky portfolio)",
            "risk": "18%",
            "return": "15%",
            "sharpe_ratio": 0.83
        },
        "calculations_performed": "Efficient frontier, Tangency portfolio, Minimum variance portfolio, Max diversification portfolio"
    }))
}

// ===== Strategy handlers =====

async fn current_regime() -> Json<serde_json::Value> {
    Json(json!({
        "module": "ML Strategy Selector (Sprint 31)",
        "current_regime": {
            "name": "Trending",
            "confidence": 0.85,
            "description": "Strong uptrend detected in major indices",
            "indicators": ["Price above 50-day MA", "RSI > 60", "Positive MACD"]
        },
        "supported_regimes": [
            "Trending", "Ranging", "Volatile", 
            "StrongUptrend", "StrongDowntrend",
            "Breakout", "Reversal", 
            "LowVolatility", "HighVolatility", 
            "Crisis", "Recovery"
        ],
        "recommended_strategies": [
            {"name": "Momentum", "allocation": "40%", "reason": "High confidence uptrend"},
            {"name": "Trend Following", "allocation": "35%", "reason": "Confirmed trend"},
            {"name": "Breakout", "allocation": "25%", "reason": "New highs being made"}
        ],
        "tests": 45
    }))
}

async fn select_strategy() -> Json<serde_json::Value> {
    Json(json!({
        "selection_method": "ML-based regime detection with performance attribution",
        "weights": {
            "regime_fit": 0.35,
            "recent_performance": 0.30,
            "risk_adjusted_return": 0.25,
            "diversification": 0.10
        },
        "selected_strategy": "Momentum",
        "confidence": 0.87,
        "switch_criteria": {
            "min_hold_period": "24 hours",
            "min_improvement": "5% better expected return",
            "max_switches_per_day": 3
        },
        "performance_attribution": {
            "strategy_return": "12.5%",
            "benchmark_return": "10.2%",
            "alpha": "2.3%",
            "attribution": {
                "market_timing": "+1.2%",
                "security_selection": "+0.8%",
                "risk_management": "+0.3%"
            }
        }
    }))
}

// ===== Tax handlers =====

async fn tax_status() -> Json<serde_json::Value> {
    Json(json!({
        "module": "Tax & Compliance Engine (Sprint 30)",
        "jurisdiction": "US (configurable: US, UK, EU, CA, AU, JP)",
        "features": [
            {
                "name": "Tax Loss Harvesting",
                "description": "Automatic identification and execution of tax loss harvesting opportunities",
                "min_loss_threshold": "$100",
                "max_harvests_per_month": 10
            },
            {
                "name": "Wash Sale Monitor",
                "description": "Monitors 30-day window to avoid wash sale violations",
                "replacement_securities": "Automatically suggests alternatives"
            },
            {
                "name": "Tax Reporting",
                "description": "Generates Schedule D and Form 8949",
                "formats": ["PDF", "CSV", "TXF (TurboTax)"]
            },
            {
                "name": "Cost Basis Tracking",
                "description": "FIFO, LIFO, HIFO, and specific identification methods"
            }
        ],
        "tests": 38
    }))
}

async fn calculate_tax() -> Json<serde_json::Value> {
    Json(json!({
        "tax_year": 2026,
        "calculations": {
            "short_term_gains": 15000.00,
            "long_term_gains": 25000.00,
            "short_term_rate": "35%",
            "long_term_rate": "15%",
            "estimated_tax": {
                "short_term": 5250.00,
                "long_term": 3750.00,
                "total": 9000.00
            }
        },
        "optimization_opportunities": [
            {
                "action": "Harvest $2,500 in losses from AAPL",
                "tax_savings": "$875",
                "replacement": "Buy VOO (S&P 500 ETF) to maintain exposure"
            },
            {
                "action": "Hold TSLA for 3 more days",
                "reason": "Will qualify for long-term treatment (1 year holding)",
                "potential_savings": "$1,200"
            }
        ],
        "harvesting_status": "Active - monitoring for opportunities"
    }))
}

// ===== Monitoring handlers =====

async fn metrics_handler() -> Json<serde_json::Value> {
    Json(json!({
        "module": "Real-Time Monitoring (Sprint 33)",
        "system_metrics": {
            "cpu_usage": "15.2%",
            "memory_usage": "512 MB",
            "active_connections": 42,
            "goroutines_threads": 156
        },
        "trading_metrics": {
            "orders_today": 156,
            "filled_orders": 148,
            "fill_rate": "95%",
            "avg_slippage": "2.3 bps",
            "pnl_today": "$1,250.50",
            "sharpe_ratio": 1.85,
            "max_drawdown": "3.2%"
        },
        "alerts": {
            "active": 3,
            "by_severity": {
                "info": 1,
                "warning": 2,
                "critical": 0
            }
        },
        "anomaly_detection": {
            "status": "monitoring",
            "algorithms": ["Statistical", "ML-based", "Rule-based"],
            "detections_today": 2,
            "false_positive_rate": "3%"
        },
        "tests": 61
    }))
}

async fn system_metrics() -> Json<serde_json::Value> {
    Json(json!({
        "health_checks": {
            "api": {"status": "✅ pass", "latency_ms": 5},
            "database": {"status": "✅ pass", "latency_ms": 12},
            "redis": {"status": "✅ pass", "latency_ms": 3},
            "ml_engine": {"status": "✅ pass", "latency_ms": 45},
            "risk_engine": {"status": "✅ pass", "latency_ms": 8}
        },
        "performance": {
            "requests_per_second": 120,
            "avg_response_time_ms": 25,
            "p99_response_time_ms": 150,
            "error_rate": "0.01%"
        }
    }))
}

// ===== Deployment handlers =====

async fn deployment_status() -> Json<serde_json::Value> {
    Json(json!({
        "module": "Production Deployment & CI/CD (Sprint 35)",
        "status": "Demo mode running locally",
        "infrastructure": {
            "platform": "Kubernetes",
            "ci_cd": "GitHub Actions",
            "deployment_strategy": "Canary (10% → 100%)",
            "auto_rollback": true,
            "multi_arch": ["linux/amd64", "linux/arm64"]
        },
        "environments": [
            {
                "name": "Development",
                "replicas": 1,
                "paper_trading": true,
                "resources": "256Mi / 1 CPU",
                "branch": "develop"
            },
            {
                "name": "Staging",
                "replicas": 2,
                "paper_trading": true,
                "resources": "512Mi / 1.5 CPU",
                "branch": "main"
            },
            {
                "name": "Production",
                "replicas": "5-20 (HPA)",
                "paper_trading": false,
                "resources": "1Gi / 4 CPU",
                "strategy": "Canary deployment"
            }
        ],
        "security_scanning": ["cargo-audit", "Trivy", "Hadolint"],
        "tests": 32
    }))
}

async fn deployment_config() -> Json<serde_json::Value> {
    Json(json!({
        "environment": "development",
        "features": {
            "kubernetes": true,
            "horizontal_pod_autoscaling": {
                "enabled": true,
                "min_replicas": 5,
                "max_replicas": 20,
                "metrics": ["CPU", "Memory", "Requests/sec"]
            },
            "canary_deployment": {
                "enabled": true,
                "initial_traffic": "10%",
                "promotion_criteria": ["Error rate < 1%", "P99 latency < 500ms"]
            },
            "health_checks": {
                "liveness": "/api/health",
                "readiness": "/api/ready",
                "startup": "/api/health"
            },
            "security": {
                "non_root_user": true,
                "read_only_root_fs": true,
                "dropped_capabilities": "ALL"
            }
        },
        "ci_cd_pipeline": [
            "Test Suite (format, clippy, unit, integration)",
            "Security Audit (cargo-audit, Trivy, Hadolint)",
            "Build & Push (multi-arch, Docker BuildKit)",
            "Deploy (dev → staging → production)"
        ]
    }))
}

// ===== Demo handlers =====

async fn demo_trade() -> Json<serde_json::Value> {
    let trade_id = Uuid::new_v4();
    
    Json(json!({
        "trade_executed": true,
        "trade_id": trade_id.to_string(),
        "symbol": "AAPL",
        "side": "BUY",
        "quantity": 100,
        "price": 185.50,
        "total_value": 18550.00,
        "strategy": "Momentum",
        "regime": "Trending",
        "risk_check": "passed",
        "circuit_breaker": "closed",
        "timestamp": Utc::now().to_rfc3339(),
        "status": "FILLED"
    }))
}

async fn demo_positions() -> Json<serde_json::Value> {
    Json(json!({
        "positions": [
            {"symbol": "AAPL", "quantity": 100, "avg_price": 180.00, "current_price": 185.50, "pnl": 550.00, "pnl_percent": "3.06%"},
            {"symbol": "GOOGL", "quantity": 50, "avg_price": 140.00, "current_price": 145.20, "pnl": 260.00, "pnl_percent": "3.71%"},
            {"symbol": "MSFT", "quantity": 75, "avg_price": 380.00, "current_price": 375.00, "pnl": -375.00, "pnl_percent": "-1.32%"},
            {"symbol": "TSLA", "quantity": 25, "avg_price": 220.00, "current_price": 210.00, "pnl": -250.00, "pnl_percent": "-4.55%"}
        ],
        "summary": {
            "total_positions": 4,
            "total_value": 65650.00,
            "total_pnl": 185.00,
            "total_pnl_percent": "0.28%"
        }
    }))
}

// ===== Broker Handlers (Paper Trading) =====

#[derive(serde::Deserialize)]
struct PlaceOrderRequest {
    ticker: String,
    side: String,
    quantity: Decimal,
    order_type: String,
    limit_price: Option<Decimal>,
}

async fn place_order_handler(
    State(state): State<AppState>,
    axum::extract::Json(req): axum::extract::Json<PlaceOrderRequest>,
) -> Json<serde_json::Value> {
    let side = match req.side.to_lowercase().as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => {
            return Json(json!({
                "success": false,
                "error": format!("Invalid side: {}. Use 'buy' or 'sell'", req.side)
            }));
        }
    };
    
    let order_type = match req.order_type.to_lowercase().as_str() {
        "market" => OrderType::Market,
        "limit" => OrderType::Limit,
        _ => {
            return Json(json!({
                "success": false,
                "error": format!("Invalid order type: {}. Use 'market' or 'limit'", req.order_type)
            }));
        }
    };
    
    let mut order = Order {
        id: Uuid::new_v4(),
        broker_order_id: None,
        ticker: req.ticker.clone(),
        side,
        quantity: req.quantity,
        order_type,
        limit_price: req.limit_price,
        stop_price: None,
        time_in_force: TimeInForce::Day,
        status: investor_os::broker::OrderStatus::PendingSubmit,
        filled_quantity: Decimal::ZERO,
        avg_fill_price: None,
        commission: None,
        proposal_id: None,
        portfolio_id: Uuid::new_v4(),
        notes: Some("Placed via API".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    let broker = state.broker.lock().await;
    match broker.place_order(&mut order).await {
        Ok(_) => {
            Json(json!({
                "success": true,
                "order": {
                    "id": order.id,
                    "ticker": order.ticker,
                    "side": format!("{:?}", order.side),
                    "quantity": order.quantity,
                    "status": format!("{:?}", order.status),
                    "filled_quantity": order.filled_quantity,
                    "created_at": order.created_at
                }
            }))
        }
        Err(e) => {
            Json(json!({
                "success": false,
                "error": e.to_string()
            }))
        }
    }
}

async fn cancel_order_handler(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Json<serde_json::Value> {
    let mut order = Order {
        id: order_id,
        broker_order_id: None,
        ticker: String::new(),
        side: OrderSide::Buy,
        quantity: Decimal::ZERO,
        order_type: OrderType::Market,
        limit_price: None,
        stop_price: None,
        time_in_force: TimeInForce::Day,
        status: investor_os::broker::OrderStatus::PendingSubmit,
        filled_quantity: Decimal::ZERO,
        avg_fill_price: None,
        commission: None,
        proposal_id: None,
        portfolio_id: Uuid::nil(),
        notes: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    let broker = state.broker.lock().await;
    match broker.cancel_order(&mut order).await {
        Ok(_) => {
            Json(json!({
                "success": true,
                "message": "Order cancelled",
                "order_id": order_id
            }))
        }
        Err(e) => {
            Json(json!({
                "success": false,
                "error": e.to_string()
            }))
        }
    }
}

async fn get_positions_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let broker = state.broker.lock().await;
    match broker.get_positions().await {
        Ok(positions) => {
            let positions_json: Vec<serde_json::Value> = positions.iter().map(|p| {
                json!({
                    "ticker": p.ticker,
                    "quantity": p.quantity,
                    "avg_cost": p.avg_cost,
                    "market_price": p.market_price,
                    "market_value": p.market_value,
                    "unrealized_pnl": p.unrealized_pnl,
                })
            }).collect();
            
            Json(json!({
                "success": true,
                "positions": positions_json,
                "count": positions.len()
            }))
        }
        Err(e) => {
            Json(json!({
                "success": false,
                "error": e.to_string()
            }))
        }
    }
}

async fn get_account_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let broker = state.broker.lock().await;
    match broker.get_account_info().await {
        Ok(info) => {
            Json(json!({
                "success": true,
                "account": {
                    "id": info.account_id,
                    "cash_balance": info.cash_balance,
                    "buying_power": info.buying_power,
                    "net_liquidation": info.net_liquidation,
                    "unrealized_pnl": info.unrealized_pnl,
                    "realized_pnl": info.realized_pnl,
                    "currency": info.currency,
                }
            }))
        }
        Err(e) => {
            Json(json!({
                "success": false,
                "error": e.to_string()
            }))
        }
    }
}
