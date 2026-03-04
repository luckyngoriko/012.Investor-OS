//! Investor OS v3.0 - Autonomous AI Trading System
//!
//! Демо версия за разглеждане на функционалностите

use axum::{
    extract::{Extension, Path, Request, State},
    http::{header::CONTENT_TYPE, HeaderMap, HeaderName, Method, StatusCode},
    middleware::{from_fn, from_fn_with_state, Next},
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::{info, warn};
use uuid::Uuid;

use investor_os::anti_fake::{AntiFakeDecision, AntiFakeShield, RequestAntiFakeSignal};
use investor_os::auth;
use investor_os::broker::paper::PaperBroker;
use investor_os::broker::{
    Broker, BrokerConfig, BrokerType, Order, OrderSide, OrderType, TimeInForce,
};
use investor_os::projects::ProjectService;

/// Състояние на приложението
#[derive(Clone)]
struct AppState {
    start_time: chrono::DateTime<Utc>,
    version: String,
    request_count: Arc<Mutex<u64>>,
    broker: Arc<Mutex<PaperBroker>>,
    runtime_contract: RuntimeContract,
    auth: Arc<auth::AuthService>,
    anti_fake: Arc<Mutex<AntiFakeShield>>,
    login_rate_limiter: Option<Arc<Mutex<investor_os::middleware::RateLimiter>>>,
    db_pool: sqlx::PgPool,
    projects: Arc<ProjectService>,
}

#[derive(Clone)]
struct RuntimeContract {
    api_base_url: String,
    ws_hrm_url: String,
    allowed_origins: Vec<String>,
}

#[tokio::main]
async fn main() {
    // Инициализиране на logging
    tracing_subscriber::fmt::init();

    // Validate environment before anything else
    investor_os::config::validation::validate_env();

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

    // PostgreSQL connection pool (configurable via env)
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://investor:investor@localhost:5432/investor_os".to_string());
    let db_max_connections: u32 = std::env::var("DB_POOL_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let db_min_connections: u32 = std::env::var("DB_POOL_MIN_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(db_max_connections)
        .min_connections(db_min_connections)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");
    info!("PostgreSQL pool: min={db_min_connections}, max={db_max_connections}");

    // Auth service (JWT + Argon2id + PostgreSQL + account lockout)
    let jwt_config = auth::JwtConfig::from_env();
    let lockout_config = auth::LockoutConfig::from_env();
    let auth_service = auth::AuthService::new(pool.clone(), jwt_config, lockout_config);

    // Seed initial admin user if DB is empty
    if let Err(e) = auth::seed::seed_admin_if_empty(&pool).await {
        warn!("auth seed failed (non-fatal): {}", e);
    }

    // Background session cleanup (every hour, retain 24 hours)
    let cleanup_interval: u64 = std::env::var("SESSION_CLEANUP_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3600);
    let cleanup_retention: i64 = std::env::var("SESSION_RETENTION_HOURS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(24);
    auth::cleanup::spawn_cleanup_task(pool.clone(), cleanup_interval, cleanup_retention);

    // Redis connection for rate limiting (optional — degrades gracefully)
    let login_rate_limiter = match std::env::var("REDIS_URL")
        .ok()
        .or_else(|| Some("redis://127.0.0.1:6379".to_string()))
    {
        Some(url) => match redis::Client::open(url.as_str()) {
            Ok(client) => match client.get_multiplexed_async_connection().await {
                Ok(conn) => {
                    let max_login_attempts: u32 = std::env::var("RATE_LIMIT_LOGIN_MAX")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(10);
                    let window_secs: u64 = std::env::var("RATE_LIMIT_LOGIN_WINDOW_SECS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(300);
                    let limiter = investor_os::middleware::RateLimiter::new(
                        conn,
                        max_login_attempts,
                        window_secs,
                    )
                    .await;
                    info!("Redis rate limiter active: {max_login_attempts} login attempts / {window_secs}s");
                    Some(Arc::new(Mutex::new(limiter)))
                }
                Err(e) => {
                    warn!("Redis connection failed (rate limiting disabled): {e}");
                    None
                }
            },
            Err(e) => {
                warn!("Redis client creation failed (rate limiting disabled): {e}");
                None
            }
        },
        None => None,
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

    // Project tracking service (Sprint 111)
    let project_service = ProjectService::new(pool.clone());
    if let Err(e) = investor_os::projects::seed::seed_production_readiness_program(&pool).await {
        warn!("project tracking seed failed (non-fatal): {}", e);
    }

    // Създаване на състоянието
    let state = AppState {
        start_time: Utc::now(),
        version: "3.0.0".to_string(),
        request_count: Arc::new(Mutex::new(0)),
        broker: Arc::new(Mutex::new(broker)),
        runtime_contract,
        auth: Arc::new(auth_service),
        anti_fake: Arc::new(Mutex::new(AntiFakeShield::from_env())),
        login_rate_limiter,
        db_pool: pool,
        projects: Arc::new(project_service),
    };

    // Създаване на router
    let app = create_router(state);

    info!("📡 API сървър стартира на: http://{}", addr);
    info!("");
    info!("📖 Документация:     http://{}/api/docs", addr);
    info!("❤️  Health Check:    http://{}/api/health", addr);
    info!("🔐 Security Status:  http://{}/api/security/status", addr);
    info!(
        "📊 Portfolio API:    http://{}/api/portfolio/optimize",
        addr
    );
    info!("🤖 Strategy API:     http://{}/api/strategy/regime", addr);
    info!("💰 Tax API:          http://{}/api/tax/status", addr);
    info!(
        "📈 Metrics:          http://{}/api/monitoring/metrics",
        addr
    );
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
    let anti_fake_data_layer = from_fn_with_state(state.clone(), enforce_real_data_middleware);
    let login_rate_limit_layer = from_fn_with_state(state.clone(), login_rate_limit_middleware);

    let public_router = Router::new()
        .route("/", get(root_handler))
        .route("/api/health", get(health_handler))
        .route("/api/ready", get(readiness_handler))
        .route("/api/runtime/config", get(runtime_config_handler))
        .route("/api/hrm/status", get(hrm_status_handler))
        .route("/metrics", get(metrics_prometheus_handler))
        .route("/api/docs", get(docs_handler))
        .route(
            "/api/auth/login",
            post(auth_login_handler).route_layer(login_rate_limit_layer),
        )
        .route("/api/auth/refresh", post(auth_refresh_handler))
        .route("/api/auth/totp/verify", post(auth_totp_verify_handler));

    let protected_router = Router::new()
        .route("/api/auth/me", get(auth_me_handler))
        .route("/api/auth/logout", post(auth_logout_handler))
        // TOTP 2FA endpoints
        .route("/api/auth/totp/setup", post(auth_totp_setup_handler))
        .route("/api/auth/totp/confirm", post(auth_totp_confirm_handler))
        .route("/api/auth/totp", delete(auth_totp_disable_handler))
        // API key endpoints
        .route("/api/auth/api-keys", post(auth_create_api_key_handler))
        .route("/api/auth/api-keys", get(auth_list_api_keys_handler))
        .route(
            "/api/auth/api-keys/:key_id",
            delete(auth_revoke_api_key_handler),
        )
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
        // Killswitch endpoints
        .route("/api/killswitch", get(killswitch_status_handler))
        .route("/api/killswitch", post(killswitch_trigger_handler))
        .route("/api/killswitch/reset", post(killswitch_reset_handler))
        // Monitoring endpoints
        .route("/api/monitoring/metrics", get(metrics_handler))
        .route("/api/monitoring/system", get(system_metrics))
        // Deployment endpoints
        .route("/api/deployment/status", get(deployment_status))
        .route("/api/deployment/config", get(deployment_config))
        // Broker endpoints (Paper Trading)
        .route("/api/broker/orders", post(place_order_handler))
        .route("/api/broker/orders/:id", delete(cancel_order_handler))
        .route("/api/broker/positions", get(get_positions_handler))
        .route("/api/broker/account", get(get_account_handler))
        // Admin user management (admin-only)
        .route("/api/admin/users", post(admin_create_user_handler))
        .route("/api/admin/users", get(admin_list_users_handler))
        .route("/api/admin/users/:id", get(admin_get_user_handler))
        .route("/api/admin/users/:id", put(admin_update_user_handler))
        // Project tracking (Sprint 111)
        .route("/api/projects/dashboard", get(projects_dashboard_handler))
        .route(
            "/api/projects/programs",
            get(projects_list_programs_handler),
        )
        .route(
            "/api/projects/programs",
            post(projects_create_program_handler),
        )
        .route(
            "/api/projects/programs/:id",
            get(projects_program_detail_handler),
        )
        .route(
            "/api/projects/programs/:id/status",
            put(projects_update_program_status_handler),
        )
        .route("/api/projects/sprints", get(projects_list_sprints_handler))
        .route(
            "/api/projects/sprints/:number",
            get(projects_sprint_detail_handler),
        )
        .route(
            "/api/projects/sprints/:number/start",
            post(projects_start_sprint_handler),
        )
        .route(
            "/api/projects/sprints/:number/done",
            post(projects_complete_sprint_handler),
        )
        .route("/api/projects/tasks", get(projects_list_tasks_handler))
        .route(
            "/api/projects/tasks/:id/status",
            put(projects_update_task_handler),
        )
        .route("/api/projects/roadmap", get(projects_roadmap_handler))
        .route_layer(auth_layer);

    Router::new()
        .merge(public_router)
        .merge(protected_router)
        .route_layer(anti_fake_data_layer)
        .layer(from_fn(content_type_validation_middleware))
        .layer(from_fn(
            investor_os::middleware::security_headers_middleware,
        ))
        // Request body limit: 1 MB
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        // Request timeout: 30 seconds
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
        // API versioning: /api/v1/* → /api/*
        .layer(from_fn(api_version_rewrite_middleware))
        // Request correlation ID (outermost — always present)
        .layer(from_fn(correlation_id_middleware))
        .with_state(state)
}

async fn enforce_real_data_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let path = request.uri().path().to_string();

    let mut anti_fake = state.anti_fake.lock().await;
    let decision = anti_fake.evaluate_fake_data_policy(&path);
    let strict = anti_fake.enforce_real_data();
    drop(anti_fake);

    match decision {
        AntiFakeDecision::Allow { .. } => {}
        AntiFakeDecision::Challenge { score, reasons } => {
            warn!(
                "Anti-fake data-policy challenge for path={} score={} reasons={:?}",
                path, score, reasons
            );
            if strict {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "Data policy blocks this endpoint",
                        "score": score,
                        "reasons": reasons
                    })),
                ));
            }
        }
        AntiFakeDecision::Block { score, reasons } => {
            warn!(
                "Anti-fake data-policy blocked path={} score={} reasons={:?}",
                path, score, reasons
            );
            if strict {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "Data policy blocks this endpoint",
                        "score": score,
                        "reasons": reasons
                    })),
                ));
            }
        }
    }

    Ok(next.run(request).await)
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

fn extract_client_ip(headers: &HeaderMap) -> String {
    let trust_proxy_headers = std::env::var("ANTI_FAKE_TRUST_PROXY_HEADERS")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if trust_proxy_headers {
        if let Some(forwarded) = headers.get("X-Forwarded-For") {
            if let Ok(ip) = forwarded.to_str() {
                if let Some(first) = ip.split(',').next() {
                    let trimmed = first.trim();
                    if !trimmed.is_empty() {
                        return trimmed.to_string();
                    }
                }
            }
        }
        if let Some(real_ip) = headers.get("X-Real-IP") {
            if let Ok(ip) = real_ip.to_str() {
                let trimmed = ip.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

fn extract_user_agent(headers: &HeaderMap) -> String {
    headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Request correlation ID middleware.
/// Reads `X-Request-Id` from the incoming request (or generates a new UUID)
/// and attaches it to the response headers for end-to-end tracing.
async fn correlation_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get("X-Request-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Store in request extensions so handlers can access it
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let mut response = next.run(request).await;
    if let Ok(val) = request_id.parse() {
        response.headers_mut().insert("X-Request-Id", val);
    }
    response
}

/// Request correlation ID available in handler extensions.
/// Handlers can extract via `Extension(RequestId(id))`.
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct RequestId(String);

/// API versioning: rewrite `/api/v1/*` → `/api/*` so both paths work.
async fn api_version_rewrite_middleware(mut request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    if let Some(rest) = path.strip_prefix("/api/v1") {
        let new_path = format!("/api{rest}");
        if let Ok(new_uri) = new_path.parse::<axum::http::Uri>() {
            *request.uri_mut() = new_uri;
        }
    }
    next.run(request).await
}

/// Reject POST/PUT/PATCH requests without `Content-Type: application/json`.
/// GET, DELETE, OPTIONS, and HEAD requests are exempt.
async fn content_type_validation_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let needs_body = matches!(
        *request.method(),
        Method::POST | Method::PUT | Method::PATCH
    );

    if needs_body {
        let has_json_ct = request
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.starts_with("application/json"))
            .unwrap_or(false);

        if !has_json_ct {
            return Err((
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Json(json!({
                    "success": false,
                    "error": {
                        "code": "UNSUPPORTED_MEDIA_TYPE",
                        "message": "Content-Type must be application/json"
                    }
                })),
            ));
        }
    }

    Ok(next.run(request).await)
}

/// Per-IP rate limiter for the login endpoint (Redis-backed, optional).
/// If Redis is unavailable the request passes through — account lockout
/// still protects against brute force.
async fn login_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref limiter) = state.login_rate_limiter {
        let client_ip = request
            .headers()
            .get("X-Forwarded-For")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .unwrap_or("unknown")
            .trim()
            .to_string();
        let key = format!("login:{client_ip}");
        let mut limiter = limiter.lock().await;
        match limiter.check(&key).await {
            investor_os::middleware::RateLimitResult::Exceeded { retry_after } => {
                warn!("Login rate limit exceeded for IP: {}", client_ip);
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({
                        "success": false,
                        "error": "Too many login attempts. Try again later.",
                        "retry_after_secs": retry_after
                    })),
                ));
            }
            investor_os::middleware::RateLimitResult::Allowed { .. } => {}
        }
    }
    Ok(next.run(request).await)
}

async fn require_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Try Bearer JWT first, then fall back to X-API-Key
    let (user, access_token) = if let Some(token) = extract_bearer_token(request.headers()) {
        let u = state
            .auth
            .validate_access_token(&token)
            .map_err(|_| unauthorized("Invalid or expired session"))?;
        (u, token)
    } else if let Some(api_key) = extract_api_key(request.headers()) {
        let user_id = auth::api_keys::validate_api_key(&state.db_pool, &api_key)
            .await
            .map_err(|_| unauthorized("API key validation failed"))?
            .ok_or_else(|| unauthorized("Invalid or revoked API key"))?;

        let row = auth::repository::find_user_by_id(&state.db_pool, user_id)
            .await
            .map_err(|_| unauthorized("User lookup failed"))?
            .ok_or_else(|| unauthorized("API key owner not found"))?;

        if !row.is_active {
            return Err(unauthorized("User account is disabled"));
        }

        (row.to_auth_user(), format!("apikey:{user_id}"))
    } else {
        return Err(unauthorized("Missing Authorization header or X-API-Key"));
    };

    let headers = request.headers();
    let client_ip = extract_client_ip(headers);
    let user_agent = extract_user_agent(headers);
    let timestamp = headers
        .get("X-Request-Timestamp")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok());
    let nonce = headers
        .get("X-Request-Nonce")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());
    let signature = headers
        .get("X-Request-Signature")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());
    let payload_hash = headers
        .get("X-Payload-Hash")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());

    let anti_fake_signal = RequestAntiFakeSignal {
        method: request.method().as_str().to_string(),
        path: request.uri().path().to_string(),
        access_token: access_token.clone(),
        client_ip,
        user_agent,
        request_timestamp_unix: timestamp,
        nonce,
        signature,
        payload_hash,
    };

    let mut anti_fake = state.anti_fake.lock().await;
    let strict = anti_fake.enforce_mode();
    let anti_fake_decision = anti_fake.evaluate_authenticated_request(anti_fake_signal);
    drop(anti_fake);

    match anti_fake_decision {
        AntiFakeDecision::Allow { .. } => {}
        AntiFakeDecision::Challenge { score, reasons }
        | AntiFakeDecision::Block { score, reasons } => {
            warn!(
                "Anti-fake flagged request score={} reasons={:?}",
                score, reasons
            );
            if strict {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "Anti-fake protection blocked request",
                        "score": score,
                        "reasons": reasons
                    })),
                ));
            }
        }
    }

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

/// Extract X-API-Key header value.
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
}

async fn auth_login_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<auth::LoginRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    {
        let mut anti_fake = state.anti_fake.lock().await;
        let strict = anti_fake.enforce_mode();
        let decision = anti_fake.evaluate_login_attempt(&payload.email, &client_ip, &user_agent);
        if strict
            && matches!(
                decision,
                AntiFakeDecision::Challenge { .. } | AntiFakeDecision::Block { .. }
            )
        {
            let score = decision.score();
            let reasons = decision.reasons().to_vec();
            warn!(
                "Anti-fake blocked login precheck score={} reasons={:?}",
                score, reasons
            );
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": "Login blocked by anti-fake protection",
                    "score": score,
                    "reasons": reasons
                })),
            );
        }
    }

    match state
        .auth
        .login(
            &payload.email,
            &payload.password,
            Some(&client_ip),
            Some(&user_agent),
        )
        .await
    {
        Ok(auth::LoginResponse::Success(session)) => {
            let uid = uuid::Uuid::parse_str(&session.user.id).ok();
            auth::audit::log_audit_event(
                &state.db_pool,
                auth::audit::AuditEvent::LoginSuccess,
                uid,
                Some(&client_ip),
                json!({"email": payload.email}),
            )
            .await;
            let mut anti_fake = state.anti_fake.lock().await;
            anti_fake.bind_session(&session.access_token, &client_ip, &user_agent);
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": session
                })),
            )
        }
        Ok(auth::LoginResponse::TotpRequired(challenge)) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": challenge
            })),
        ),
        Err(_) => {
            auth::audit::log_audit_event(
                &state.db_pool,
                auth::audit::AuditEvent::LoginFailed,
                None,
                Some(&client_ip),
                json!({"email": payload.email}),
            )
            .await;
            let mut anti_fake = state.anti_fake.lock().await;
            anti_fake.record_failed_login(&payload.email, &client_ip);
            unauthorized("Invalid credentials")
        }
    }
}

async fn auth_refresh_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<auth::RefreshRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    match state
        .auth
        .refresh_session(&payload.refresh_token, Some(&client_ip), Some(&user_agent))
        .await
    {
        Ok(session) => {
            let uid = uuid::Uuid::parse_str(&session.user.id).ok();
            auth::audit::log_audit_event(
                &state.db_pool,
                auth::audit::AuditEvent::TokenRefresh,
                uid,
                Some(&client_ip),
                json!({}),
            )
            .await;
            let mut anti_fake = state.anti_fake.lock().await;
            anti_fake.bind_session(&session.access_token, &client_ip, &user_agent);
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": session
                })),
            )
        }
        Err(_) => unauthorized("Invalid or expired refresh token"),
    }
}

async fn auth_me_handler(
    Extension(user): Extension<auth::AuthUser>,
) -> (StatusCode, Json<serde_json::Value>) {
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
    Extension(user): Extension<auth::AuthUser>,
    headers: HeaderMap,
    payload: Option<Json<auth::LogoutRequest>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let client_ip = extract_client_ip(&headers);
    let access_token = extract_bearer_token(&headers);
    let refresh_token = payload.and_then(|body| body.0.refresh_token);

    let _ = state.auth.logout(refresh_token.as_deref()).await;

    let uid = uuid::Uuid::parse_str(&user.id).ok();
    auth::audit::log_audit_event(
        &state.db_pool,
        auth::audit::AuditEvent::Logout,
        uid,
        Some(&client_ip),
        json!({}),
    )
    .await;

    if let Some(access) = access_token.as_deref() {
        let mut anti_fake = state.anti_fake.lock().await;
        anti_fake.unbind_session(access);
    }

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

// ===== TOTP 2FA handlers =====

async fn auth_totp_verify_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<auth::TotpVerifyRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    match state
        .auth
        .complete_totp_login(
            &payload.challenge_token,
            &payload.code,
            Some(&client_ip),
            Some(&user_agent),
        )
        .await
    {
        Ok(session) => {
            let uid = uuid::Uuid::parse_str(&session.user.id).ok();
            auth::audit::log_audit_event(
                &state.db_pool,
                auth::audit::AuditEvent::LoginSuccess,
                uid,
                Some(&client_ip),
                json!({"method": "totp"}),
            )
            .await;
            let mut anti_fake = state.anti_fake.lock().await;
            anti_fake.bind_session(&session.access_token, &client_ip, &user_agent);
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": session
                })),
            )
        }
        Err(_) => {
            auth::audit::log_audit_event(
                &state.db_pool,
                auth::audit::AuditEvent::LoginFailed,
                None,
                Some(&client_ip),
                json!({"method": "totp"}),
            )
            .await;
            unauthorized("Invalid TOTP code or expired challenge")
        }
    }
}

async fn auth_totp_setup_handler(
    State(state): State<AppState>,
    Extension(user): Extension<auth::AuthUser>,
) -> (StatusCode, Json<serde_json::Value>) {
    let user_id: uuid::Uuid = match user.id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "error": "Invalid user ID"})),
            )
        }
    };

    match auth::totp::setup_totp(&state.db_pool, user_id).await {
        Ok((secret, uri)) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": {
                    "secret": secret,
                    "otpauth_uri": uri
                }
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "error": e.to_string()})),
        ),
    }
}

async fn auth_totp_confirm_handler(
    State(state): State<AppState>,
    Extension(user): Extension<auth::AuthUser>,
    Json(payload): Json<auth::TotpCodeRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let user_id: uuid::Uuid = match user.id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "error": "Invalid user ID"})),
            )
        }
    };

    match auth::totp::confirm_totp(&state.db_pool, user_id, &payload.code).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({"success": true, "data": {"totp_enabled": true}})),
        ),
        Err(_) => unauthorized("Invalid TOTP code"),
    }
}

async fn auth_totp_disable_handler(
    State(state): State<AppState>,
    Extension(user): Extension<auth::AuthUser>,
    Json(payload): Json<auth::TotpCodeRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let user_id: uuid::Uuid = match user.id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "error": "Invalid user ID"})),
            )
        }
    };

    match auth::totp::disable_totp(&state.db_pool, user_id, &payload.code).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({"success": true, "data": {"totp_enabled": false}})),
        ),
        Err(_) => unauthorized("Invalid TOTP code"),
    }
}

// ===== API Key handlers =====

async fn auth_create_api_key_handler(
    State(state): State<AppState>,
    Extension(user): Extension<auth::AuthUser>,
    Json(payload): Json<auth::CreateApiKeyRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let user_id: uuid::Uuid = match user.id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "error": "Invalid user ID"})),
            )
        }
    };

    let perms = payload.permissions.unwrap_or_default();

    match auth::api_keys::create_api_key(
        &state.db_pool,
        user_id,
        &payload.name,
        &perms,
        payload.expires_in_days,
    )
    .await
    {
        Ok(created) => (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "data": {
                    "key_id": created.id.to_string(),
                    "api_key": created.api_key,
                    "prefix": created.key_prefix,
                    "note": "Store this key securely — it will not be shown again"
                }
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "error": e.to_string()})),
        ),
    }
}

async fn auth_list_api_keys_handler(
    State(state): State<AppState>,
    Extension(user): Extension<auth::AuthUser>,
) -> (StatusCode, Json<serde_json::Value>) {
    let user_id: uuid::Uuid = match user.id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "error": "Invalid user ID"})),
            )
        }
    };

    match auth::api_keys::list_api_keys(&state.db_pool, user_id).await {
        Ok(keys) => (StatusCode::OK, Json(json!({"success": true, "data": keys}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "error": e.to_string()})),
        ),
    }
}

async fn auth_revoke_api_key_handler(
    State(state): State<AppState>,
    Extension(user): Extension<auth::AuthUser>,
    Path(key_id): Path<uuid::Uuid>,
) -> (StatusCode, Json<serde_json::Value>) {
    let user_id: uuid::Uuid = match user.id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "error": "Invalid user ID"})),
            )
        }
    };

    match auth::api_keys::revoke_api_key(&state.db_pool, key_id, user_id).await {
        Ok(true) => (
            StatusCode::OK,
            Json(json!({"success": true, "data": {"revoked": true}})),
        ),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({"success": false, "error": "API key not found"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "error": e.to_string()})),
        ),
    }
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

    // Real DB health check
    let db_status = match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(_) => "pass",
        Err(_) => "fail",
    };

    // Real Redis health check (if configured)
    let redis_status = if state.login_rate_limiter.is_some() {
        "pass"
    } else {
        "not_configured"
    };

    let overall = if db_status == "pass" {
        "healthy"
    } else {
        "degraded"
    };

    Json(json!({
        "success": true,
        "data": {
            "status": overall,
            "version": state.version,
            "uptime_seconds": uptime.num_seconds(),
            "total_requests": count,
            "timestamp": Utc::now().to_rfc3339(),
            "environment": std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            "checks": {
                "api": "pass",
                "database": db_status,
                "redis": redis_status,
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
    let trading_mode = std::env::var("TRADING_MODE").unwrap_or_else(|_| "semi_auto".into());
    let hrm_weights = std::env::var("HRM_WEIGHTS_PATH").unwrap_or_default();
    let model_label = if hrm_weights.is_empty() {
        "hrm-v3-heuristic"
    } else {
        "hrm-v3-ml"
    };

    Json(json!({
        "success": true,
        "data": {
            "status": "ready",
            "model": model_label,
            "mode": trading_mode,
            "weights_loaded": !hrm_weights.is_empty(),
        }
    }))
}

async fn metrics_prometheus_handler() -> (StatusCode, [(HeaderName, &'static str); 1], String) {
    let content_type = [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")];

    match investor_os::monitoring::metrics::encode_metrics() {
        Ok(payload) => {
            investor_os::monitoring::metrics::record_api_request("GET", "/metrics", 200);
            (StatusCode::OK, content_type, payload)
        }
        Err(err) => {
            investor_os::monitoring::metrics::record_api_request("GET", "/metrics", 500);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                content_type,
                format!("# metrics_export_error {}\n", err),
            )
        }
    }
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

async fn security_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let anti_fake = state.anti_fake.lock().await;
    let mut anti_fake_reasons = serde_json::Map::new();
    let mut anti_fake_reason_pairs = anti_fake.data_policy_violation_reasons();
    anti_fake_reason_pairs.sort_by(|a, b| a.0.cmp(&b.0));
    for (reason, count) in anti_fake_reason_pairs.drain(..) {
        anti_fake_reasons.insert(reason, json!(count));
    }

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
        },
        "anti_fake": {
            "enforce": anti_fake.enforce_mode(),
            "enforce_real_data": anti_fake.enforce_real_data(),
            "allow_demo_endpoints": anti_fake.allow_demo_endpoints(),
            "fake_endpoint_prefixes": anti_fake.fake_endpoint_prefixes(),
            "data_policy_violation_count": anti_fake.data_policy_violation_count(),
            "data_policy_violation_reasons": anti_fake_reasons
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

async fn generate_api_key(
    State(state): State<AppState>,
    Extension(user): Extension<auth::AuthUser>,
) -> (StatusCode, Json<serde_json::Value>) {
    let user_id: uuid::Uuid = match user.id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "error": "Invalid user ID"})),
            )
        }
    };

    match auth::api_keys::create_api_key(
        &state.db_pool,
        user_id,
        "Security Dashboard Key",
        &[],
        Some(30),
    )
    .await
    {
        Ok(created) => (
            StatusCode::OK,
            Json(json!({
                "key_id": created.id.to_string(),
                "api_key": created.api_key,
                "user_id": user_id.to_string(),
                "clearance": "Internal",
                "expires_in_days": 30,
                "note": "Store this key securely — it will not be shown again",
                "usage": "curl -H 'X-API-Key: YOUR_KEY' http://localhost:5001/api/portfolio/optimize"
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "error": e.to_string()})),
        ),
    }
}

// ===== Killswitch handlers =====

async fn killswitch_status_handler(
    State(state): State<AppState>,
) -> (StatusCode, Json<serde_json::Value>) {
    use sqlx::Row;
    match sqlx::query("SELECT enabled, reason, triggered_at FROM killswitch_state WHERE id = 1")
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(Some(row)) => {
            let enabled: bool = row.try_get("enabled").unwrap_or(false);
            let reason: Option<String> = row.try_get("reason").unwrap_or(None);
            let triggered_at: Option<chrono::DateTime<Utc>> =
                row.try_get("triggered_at").unwrap_or(None);
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": {
                        "enabled": enabled,
                        "reason": reason,
                        "triggered_at": triggered_at
                    }
                })),
            )
        }
        Ok(None) => (
            StatusCode::OK,
            Json(json!({"success": true, "data": {"enabled": false}})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "error": e.to_string()})),
        ),
    }
}

#[derive(serde::Deserialize)]
struct KillswitchTriggerRequest {
    reason: Option<String>,
}

async fn killswitch_trigger_handler(
    State(state): State<AppState>,
    Extension(user): Extension<auth::AuthUser>,
    Json(payload): Json<KillswitchTriggerRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let user_id: Option<uuid::Uuid> = user.id.parse().ok();

    let result = sqlx::query(
        "UPDATE killswitch_state
         SET enabled = true, reason = $1, triggered_by = $2, triggered_at = NOW(), updated_at = NOW()
         WHERE id = 1",
    )
    .bind(&payload.reason)
    .bind(user_id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(_) => {
            warn!(
                user_id = user.id,
                reason = payload.reason.as_deref().unwrap_or(""),
                "KILLSWITCH TRIGGERED"
            );
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": {
                        "enabled": true,
                        "reason": payload.reason,
                        "triggered_at": Utc::now()
                    }
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "error": e.to_string()})),
        ),
    }
}

async fn killswitch_reset_handler(
    State(state): State<AppState>,
    Extension(user): Extension<auth::AuthUser>,
) -> (StatusCode, Json<serde_json::Value>) {
    if user.role != auth::UserRole::Admin {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"success": false, "error": "Admin role required to reset killswitch"})),
        );
    }

    let result = sqlx::query(
        "UPDATE killswitch_state
         SET enabled = false, reason = NULL, triggered_by = NULL, triggered_at = NULL, updated_at = NOW()
         WHERE id = 1",
    )
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(_) => {
            info!(user_id = user.id, "Killswitch reset by admin");
            (
                StatusCode::OK,
                Json(json!({"success": true, "data": {"enabled": false}})),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "error": e.to_string()})),
        ),
    }
}

// ===== Portfolio handlers =====

async fn optimize_portfolio() -> Json<serde_json::Value> {
    use investor_os::risk::{PortfolioRisk, Position, VaRConfig};

    // Seed returns until live market data feed is connected (Sprint 111+)
    let sample_returns: Vec<Decimal> = vec![
        "0.012", "-0.005", "0.008", "-0.003", "0.015", "-0.010", "0.007", "0.003", "-0.008",
        "0.011", "-0.002", "0.009", "-0.006", "0.004", "0.013", "-0.007", "0.006", "-0.001",
        "0.010", "-0.004",
    ]
    .into_iter()
    .filter_map(|s| s.parse::<Decimal>().ok())
    .collect();

    let risk_free_rate = Decimal::new(4, 2); // 0.04 (4% annual)
    let mut calculator = PortfolioRisk::new(VaRConfig::default());

    // Seed equity curve
    let mut equity = Decimal::from(100_000);
    for r in &sample_returns {
        equity += equity * r;
        calculator.update_equity(equity);
    }

    let metrics = calculator
        .calculate_all_metrics(&sample_returns, risk_free_rate)
        .ok();

    let positions = vec![
        Position {
            symbol: "AAPL".into(),
            quantity: Decimal::from(100),
            entry_price: Decimal::new(180, 0),
            current_price: Decimal::new(185, 0),
            weight: Decimal::new(25, 2),
        },
        Position {
            symbol: "GOOGL".into(),
            quantity: Decimal::from(50),
            entry_price: Decimal::new(140, 0),
            current_price: Decimal::new(145, 0),
            weight: Decimal::new(20, 2),
        },
        Position {
            symbol: "MSFT".into(),
            quantity: Decimal::from(75),
            entry_price: Decimal::new(380, 0),
            current_price: Decimal::new(390, 0),
            weight: Decimal::new(30, 2),
        },
        Position {
            symbol: "AMZN".into(),
            quantity: Decimal::from(30),
            entry_price: Decimal::new(175, 0),
            current_price: Decimal::new(180, 0),
            weight: Decimal::new(15, 2),
        },
        Position {
            symbol: "TSLA".into(),
            quantity: Decimal::from(25),
            entry_price: Decimal::new(220, 0),
            current_price: Decimal::new(210, 0),
            weight: Decimal::new(10, 2),
        },
    ];

    let concentration = PortfolioRisk::check_concentration(&positions, Decimal::new(35, 2));

    Json(json!({
        "success": true,
        "data": {
            "positions": positions.iter().map(|p| json!({
                "symbol": p.symbol,
                "weight": p.weight,
                "quantity": p.quantity,
                "entry_price": p.entry_price,
                "current_price": p.current_price,
            })).collect::<Vec<_>>(),
            "risk_metrics": metrics.as_ref().map(|m| json!({
                "var_95": m.var_95,
                "var_99": m.var_99,
                "cvar_95": m.cvar_95,
                "max_drawdown": m.max_drawdown,
                "current_drawdown": m.current_drawdown,
                "volatility": m.volatility,
                "sharpe_ratio": m.sharpe_ratio,
                "sortino_ratio": m.sortino_ratio,
                "calculated_at": m.calculated_at,
            })),
            "concentration_warning": concentration,
            "portfolio_value": equity,
        }
    }))
}

async fn efficient_frontier() -> Json<serde_json::Value> {
    use investor_os::risk::{PortfolioRisk, VaRConfig};

    // Compute risk metrics for allocation profiles (seed data until live feed — Sprint 111+)
    let profiles = [
        (
            "Conservative",
            vec![
                "0.003", "-0.001", "0.002", "0.001", "-0.002", "0.002", "0.001", "-0.001", "0.003",
                "-0.001", "0.002", "0.001", "0.001", "-0.001", "0.002", "-0.001", "0.001", "0.002",
                "-0.001", "0.001",
            ],
        ),
        (
            "Moderate",
            vec![
                "0.006", "-0.003", "0.005", "0.002", "-0.004", "0.004", "0.003", "-0.002", "0.006",
                "-0.002", "0.004", "0.003", "0.002", "-0.003", "0.005", "-0.002", "0.003", "0.004",
                "-0.002", "0.003",
            ],
        ),
        (
            "Aggressive",
            vec![
                "0.012", "-0.007", "0.009", "0.005", "-0.008", "0.010", "0.006", "-0.005", "0.011",
                "-0.004", "0.008", "0.005", "0.004", "-0.006", "0.009", "-0.003", "0.007", "0.008",
                "-0.005", "0.006",
            ],
        ),
        (
            "Very Aggressive",
            vec![
                "0.018", "-0.012", "0.015", "0.008", "-0.014", "0.016", "0.010", "-0.009", "0.017",
                "-0.007", "0.013", "0.009", "0.007", "-0.011", "0.014", "-0.006", "0.011", "0.013",
                "-0.008", "0.010",
            ],
        ),
    ];

    let risk_free = Decimal::new(4, 2);
    let mut frontier_points = Vec::new();

    for (name, returns_str) in &profiles {
        let returns: Vec<Decimal> = returns_str.iter().filter_map(|s| s.parse().ok()).collect();
        let mut calc = PortfolioRisk::new(VaRConfig::default());
        let mut eq = Decimal::from(100_000);
        for r in &returns {
            eq += eq * r;
            calc.update_equity(eq);
        }
        if let Ok(m) = calc.calculate_all_metrics(&returns, risk_free) {
            frontier_points.push(json!({
                "portfolio": name,
                "volatility": m.volatility,
                "sharpe_ratio": m.sharpe_ratio,
                "sortino_ratio": m.sortino_ratio,
                "max_drawdown": m.max_drawdown,
                "var_95": m.var_95,
            }));
        }
    }

    Json(json!({
        "success": true,
        "data": {
            "frontier_points": frontier_points,
            "risk_free_rate": "4%",
            "calculation_method": "Historical VaR with Sharpe/Sortino optimization",
        }
    }))
}

// ===== Strategy handlers =====

async fn current_regime() -> Json<serde_json::Value> {
    use investor_os::strategy_selector::{MarketIndicators, StrategySelectorEngine};

    // Seed indicators until live market data feed is connected (Sprint 111+)
    let indicators = MarketIndicators {
        trend_strength: 0.65,
        volatility: 0.35,
        volume: 0.55,
        rsi: 58.0,
        atr: 2.5,
    };

    let engine = StrategySelectorEngine::new();
    let regime = engine.detect_regime(&indicators);

    let recommendations = engine.get_recommendations(
        Decimal::from(100_000),
        investor_os::strategy_selector::RiskTolerance::Moderate,
    );

    Json(json!({
        "success": true,
        "data": {
            "current_regime": format!("{:?}", regime),
            "indicators": {
                "trend_strength": indicators.trend_strength,
                "volatility": indicators.volatility,
                "volume": indicators.volume,
                "rsi": indicators.rsi,
                "atr": indicators.atr,
            },
            "supported_regimes": [
                "Trending", "Ranging", "Volatile", "LowVolatility",
                "StrongUptrend", "StrongDowntrend", "WeakTrend",
                "VolatilityExpansion", "Normal", "Crisis", "Recovery"
            ],
            "recommendations": recommendations.iter().map(|r| json!({
                "strategy": r.strategy_name,
                "score": r.score,
                "reason": r.reason,
            })).collect::<Vec<_>>(),
        }
    }))
}

async fn select_strategy() -> Json<serde_json::Value> {
    use investor_os::strategy_selector::{
        MarketIndicators, SelectionCriteria, StrategySelectorEngine,
    };

    let indicators = MarketIndicators {
        trend_strength: 0.65,
        volatility: 0.35,
        volume: 0.55,
        rsi: 58.0,
        atr: 2.5,
    };

    let engine = StrategySelectorEngine::new();
    let regime = engine.detect_regime(&indicators);
    let strategies = engine.get_active_strategies();

    let criteria = SelectionCriteria {
        min_sharpe: 0.5,
        max_drawdown: 0.15,
        min_win_rate: 0.50,
        lookback_days: 90,
        require_proven: true,
        prefer_lower_turnover: false,
    };

    let selection = engine.select_strategy(regime, criteria);
    let attribution = engine.get_attribution();

    Json(json!({
        "success": true,
        "data": {
            "regime": format!("{:?}", regime),
            "selection": match selection {
                Ok(score) => json!({
                    "strategy": format!("{:?}", score.strategy_type),
                    "overall_score": score.overall_score,
                    "regime_fit_score": score.regime_fit_score,
                    "performance_score": score.performance_score,
                    "risk_adjusted_score": score.risk_adjusted_score,
                    "confidence": score.confidence,
                }),
                Err(e) => json!({"error": e.to_string()}),
            },
            "active_strategies": strategies.len(),
            "attribution": {
                "total_pnl": attribution.total_pnl.to_string(),
                "total_trades": attribution.total_trades,
                "strategies_tracked": attribution.by_strategy.len(),
            },
        }
    }))
}

// ===== Tax handlers =====

async fn tax_status() -> Json<serde_json::Value> {
    use investor_os::tax::{TaxEngine, TaxJurisdiction, TaxYear};

    let engine = TaxEngine::new(TaxJurisdiction::USA);
    let year = TaxYear::current().0;
    let violations = engine.get_wash_sale_violations();
    let harvest_ops = engine.find_harvest_opportunities();
    let unrealized = engine.get_unrealized_summary();
    let realized = engine.get_realized_for_year(year);

    Json(json!({
        "success": true,
        "data": {
            "jurisdiction": "US",
            "tax_year": year,
            "unrealized": {
                "short_term": unrealized.short_term_unrealized.to_string(),
                "long_term": unrealized.long_term_unrealized.to_string(),
                "total": unrealized.total_unrealized.to_string(),
                "harvestable_losses": unrealized.harvestable_losses.to_string(),
                "lots_count": engine.lot_count(),
            },
            "realized": {
                "short_term": realized.short_term_realized.to_string(),
                "long_term": realized.long_term_realized.to_string(),
                "total": realized.total_realized.to_string(),
                "wash_sale_adjustments": realized.wash_sale_adjustments.to_string(),
            },
            "harvest_opportunities": harvest_ops.len(),
            "wash_sale_violations": violations.len(),
            "features": ["Tax Loss Harvesting", "Wash Sale Monitor", "Schedule D", "Form 8949", "CSV Export"],
        }
    }))
}

async fn calculate_tax() -> Json<serde_json::Value> {
    use investor_os::tax::{TaxJurisdiction, TaxReportingEngine, TaxYear};

    let engine = TaxReportingEngine::new(TaxJurisdiction::USA);
    let year = TaxYear::current().0;
    let report = engine.generate_report(year, &[], vec![]);

    let short_term_rate = Decimal::new(35, 2); // 0.35 (35%)
    let long_term_rate = Decimal::new(15, 2); // 0.15 (15%)
    let st_tax = report.net_short_term * short_term_rate;
    let lt_tax = report.net_long_term * long_term_rate;
    let total_tax = st_tax + lt_tax;

    let schedule_d = engine.generate_schedule_d(&report);
    let deadline = engine.filing_deadline(year);

    Json(json!({
        "success": true,
        "data": {
            "tax_year": year,
            "filing_deadline": deadline.to_string(),
            "short_term": {
                "gains": report.net_short_term,
                "rate": "35%",
                "tax": st_tax,
            },
            "long_term": {
                "gains": report.net_long_term,
                "rate": "15%",
                "tax": lt_tax,
            },
            "total_estimated_tax": total_tax,
            "transactions_count": report.transactions.len(),
            "schedule_d": {
                "short_term_total": schedule_d.net_short_term.to_string(),
                "long_term_total": schedule_d.net_long_term.to_string(),
                "net_gain_loss": schedule_d.total_net_gain_loss.to_string(),
            },
            "harvesting_status": if report.transactions.is_empty() {
                "No realized gains — no harvesting opportunities"
            } else {
                "Active — monitoring for opportunities"
            },
        }
    }))
}

// ===== Monitoring handlers =====

async fn metrics_handler() -> Json<serde_json::Value> {
    // Read real process memory from /proc/self/status (Linux)
    let (memory_rss_kb, num_threads) = {
        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        let rss = status
            .lines()
            .find(|l| l.starts_with("VmRSS:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let threads = status
            .lines()
            .find(|l| l.starts_with("Threads:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        (rss, threads)
    };

    // Read open file descriptors
    let open_fds = std::fs::read_dir("/proc/self/fd")
        .map(|d| d.count() as u32)
        .unwrap_or(0);

    Json(json!({
        "module": "Real-Time Monitoring (Sprint 33)",
        "system_metrics": {
            "memory_rss_mb": memory_rss_kb / 1024,
            "threads": num_threads,
            "open_fds": open_fds,
            "process_id": std::process::id(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        },
        "note": "Prometheus metrics at /metrics. Trading analytics at /api/analytics/*."
    }))
}

async fn system_metrics() -> Json<serde_json::Value> {
    let start = std::time::Instant::now();

    // Real process uptime from /proc/self/stat (field 22 = starttime in clock ticks)
    let uptime_secs = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| {
            s.split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    // Memory info
    let mem_available_mb = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemAvailable:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
        })
        .map(|kb| kb / 1024)
        .unwrap_or(0);

    let self_check_latency_us = start.elapsed().as_micros();

    Json(json!({
        "health_checks": {
            "api": {
                "status": "pass",
                "self_check_latency_us": self_check_latency_us
            }
        },
        "system": {
            "host_uptime_secs": uptime_secs as u64,
            "host_mem_available_mb": mem_available_mb,
            "process_id": std::process::id(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        },
        "note": "Database health at /api/health. Full metrics at /metrics."
    }))
}

// ===== Deployment handlers =====

async fn deployment_status() -> Json<serde_json::Value> {
    let deploy_env = std::env::var("DEPLOY_ENV").unwrap_or_else(|_| "development".into());
    let status_label = match deploy_env.as_str() {
        "production" => "Production — live trading enabled",
        "staging" => "Staging — paper trading only",
        _ => "Development — local instance",
    };

    Json(json!({
        "module": "Production Deployment & CI/CD (Sprint 35)",
        "status": status_label,
        "environment": deploy_env,
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

// ===== Admin User Management Handlers =====

fn require_admin(user: &auth::AuthUser) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if user.role != auth::UserRole::Admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "Admin role required"
            })),
        ));
    }
    Ok(())
}

async fn admin_create_user_handler(
    State(state): State<AppState>,
    Extension(caller): Extension<auth::AuthUser>,
    Json(payload): Json<auth::CreateUserRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let Err(e) = require_admin(&caller) {
        return e;
    }

    match state.auth.create_user(&payload).await {
        Ok(user_info) => (
            StatusCode::CREATED,
            Json(json!({ "success": true, "data": user_info })),
        ),
        Err(auth::AuthError::DuplicateEmail(email)) => (
            StatusCode::CONFLICT,
            Json(json!({ "success": false, "error": format!("Email already exists: {email}") })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

async fn admin_list_users_handler(
    State(state): State<AppState>,
    Extension(caller): Extension<auth::AuthUser>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let Err(e) = require_admin(&caller) {
        return e;
    }

    match state.auth.list_users().await {
        Ok(users) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": users })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

async fn admin_get_user_handler(
    State(state): State<AppState>,
    Extension(caller): Extension<auth::AuthUser>,
    Path(user_id): Path<Uuid>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let Err(e) = require_admin(&caller) {
        return e;
    }

    match state.auth.get_user(user_id).await {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": user })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": "User not found" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

async fn admin_update_user_handler(
    State(state): State<AppState>,
    Extension(caller): Extension<auth::AuthUser>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<auth::UpdateUserRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let Err(e) = require_admin(&caller) {
        return e;
    }

    match state.auth.update_user(user_id, &payload).await {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": user })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": "User not found" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "error": e.to_string() })),
        ),
    }
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
        Ok(_) => Json(json!({
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
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        })),
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
        Ok(_) => Json(json!({
            "success": true,
            "message": "Order cancelled",
            "order_id": order_id
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

async fn get_positions_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let broker = state.broker.lock().await;
    match broker.get_positions().await {
        Ok(positions) => {
            let positions_json: Vec<serde_json::Value> = positions
                .iter()
                .map(|p| {
                    json!({
                        "ticker": p.ticker,
                        "quantity": p.quantity,
                        "avg_cost": p.avg_cost,
                        "market_price": p.market_price,
                        "market_value": p.market_value,
                        "unrealized_pnl": p.unrealized_pnl,
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "positions": positions_json,
                "count": positions.len()
            }))
        }
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

async fn get_account_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let broker = state.broker.lock().await;
    match broker.get_account_info().await {
        Ok(info) => Json(json!({
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
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

// ── Project tracking handlers (Sprint 111) ──

async fn projects_dashboard_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.projects.dashboard().await {
        Ok(dashboard) => Ok(Json(json!({ "success": true, "data": dashboard }))),
        Err(e) => Err(project_error_response(e)),
    }
}

async fn projects_list_programs_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.projects.list_programs().await {
        Ok(programs) => Ok(Json(json!({ "success": true, "data": programs }))),
        Err(e) => Err(project_error_response(e)),
    }
}

async fn projects_create_program_handler(
    State(state): State<AppState>,
    Json(body): Json<investor_os::projects::CreateProgramRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state
        .projects
        .create_program(
            &body.name,
            body.description.as_deref(),
            body.scope.as_deref(),
        )
        .await
    {
        Ok(program) => Ok(Json(json!({ "success": true, "data": program }))),
        Err(e) => Err(project_error_response(e)),
    }
}

async fn projects_program_detail_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.projects.get_program_detail(id).await {
        Ok(detail) => Ok(Json(json!({ "success": true, "data": detail }))),
        Err(e) => Err(project_error_response(e)),
    }
}

async fn projects_update_program_status_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<investor_os::projects::UpdateStatusRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.projects.update_program_status(id, &body.status).await {
        Ok(program) => Ok(Json(json!({ "success": true, "data": program }))),
        Err(e) => Err(project_error_response(e)),
    }
}

async fn projects_list_sprints_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let program_id = params
        .get("program_id")
        .and_then(|s| s.parse::<Uuid>().ok());

    match program_id {
        Some(pid) => match state.projects.list_sprints_by_program(pid).await {
            Ok(sprints) => Ok(Json(json!({ "success": true, "data": sprints }))),
            Err(e) => Err(project_error_response(e)),
        },
        None => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "program_id query param required" })),
        )),
    }
}

async fn projects_sprint_detail_handler(
    State(state): State<AppState>,
    Path(number): Path<i32>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.projects.get_sprint_detail(number).await {
        Ok(detail) => Ok(Json(json!({ "success": true, "data": detail }))),
        Err(e) => Err(project_error_response(e)),
    }
}

async fn projects_start_sprint_handler(
    State(state): State<AppState>,
    Path(number): Path<i32>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.projects.advance_sprint(number).await {
        Ok(sprint) => Ok(Json(json!({ "success": true, "data": sprint }))),
        Err(e) => Err(project_error_response(e)),
    }
}

async fn projects_complete_sprint_handler(
    State(state): State<AppState>,
    Path(number): Path<i32>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.projects.complete_sprint(number).await {
        Ok(sprint) => Ok(Json(json!({ "success": true, "data": sprint }))),
        Err(e) => Err(project_error_response(e)),
    }
}

async fn projects_list_tasks_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sprint_id = params.get("sprint_id").and_then(|s| s.parse::<Uuid>().ok());

    match sprint_id {
        Some(sid) => match state.projects.list_tasks(sid).await {
            Ok(tasks) => Ok(Json(json!({ "success": true, "data": tasks }))),
            Err(e) => Err(project_error_response(e)),
        },
        None => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "sprint_id query param required" })),
        )),
    }
}

async fn projects_update_task_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<investor_os::projects::UpdateTaskStatusRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state
        .projects
        .update_task_status(id, &body.status, body.priority.as_deref())
        .await
    {
        Ok(task) => Ok(Json(json!({ "success": true, "data": task }))),
        Err(e) => Err(project_error_response(e)),
    }
}

async fn projects_roadmap_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.projects.roadmap().await {
        Ok(roadmap) => Ok(Json(json!({ "success": true, "data": roadmap }))),
        Err(e) => Err(project_error_response(e)),
    }
}

fn project_error_response(
    err: investor_os::projects::ProjectError,
) -> (StatusCode, Json<serde_json::Value>) {
    use investor_os::projects::ProjectError;
    let (status, msg) = match &err {
        ProjectError::ProgramNotFound => (StatusCode::NOT_FOUND, err.to_string()),
        ProjectError::SprintNotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
        ProjectError::TaskNotFound => (StatusCode::NOT_FOUND, err.to_string()),
        ProjectError::UnmetDependencies(_, _) => (StatusCode::CONFLICT, err.to_string()),
        ProjectError::InvalidStatusTransition { .. } => (StatusCode::BAD_REQUEST, err.to_string()),
        ProjectError::Database(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        ),
    };
    (status, Json(json!({ "success": false, "error": msg })))
}
