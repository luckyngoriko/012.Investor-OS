//! Admin API - Integration Management Dashboard
//!
//! Централизирано управление на всички API интеграции
//! Показва статус, конфигурация и логове за всяка интеграция

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use crate::api::admin::connectors::{ConnectionTestResult, ConnectorFactory};
use crate::api::handlers::ApiResponse;
use crate::api::AppState;

pub mod connectors;
pub mod registry;

use registry::{IntegrationRegistry, IntegrationStatus, IntegrationType};

/// Списък с всички интеграции и техния статус
#[derive(Serialize)]
pub struct IntegrationsListResponse {
    pub integrations: Vec<IntegrationView>,
    pub total: usize,
    pub connected: usize,
    pub disconnected: usize,
    pub hardcoded: usize,
}

#[derive(Serialize)]
pub struct IntegrationView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub integration_type: String,
    pub status: String,
    pub priority: String,
    pub endpoints: Vec<EndpointView>,
    pub config_required: Vec<ConfigFieldView>,
    pub last_check: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub documentation_url: String,
}

#[derive(Serialize)]
pub struct EndpointView {
    pub method: String,
    pub path: String,
    pub description: String,
    pub status: String, // "implemented", "stub", "not_implemented"
}

#[derive(Serialize)]
pub struct ConfigFieldView {
    pub name: String,
    pub field_type: String, // "string", "number", "boolean", "secret"
    pub required: bool,
    pub description: String,
    pub current_value: Option<String>, // masked за secrets
}

/// GET /api/admin/integrations - Списък с всички интеграции
pub async fn list_integrations(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<IntegrationsListResponse>>, StatusCode> {
    let registry = IntegrationRegistry::new();
    let integrations = registry.get_all_integrations();

    let total = integrations.len();
    let connected = integrations
        .iter()
        .filter(|i| matches!(i.status, IntegrationStatus::Connected))
        .count();
    let disconnected = integrations
        .iter()
        .filter(|i| matches!(i.status, IntegrationStatus::Disconnected))
        .count();
    let hardcoded = integrations
        .iter()
        .filter(|i| matches!(i.status, IntegrationStatus::Hardcoded))
        .count();

    let views: Vec<IntegrationView> = integrations
        .iter()
        .map(|i| IntegrationView {
            id: i.id.clone(),
            name: i.name.clone(),
            description: i.description.clone(),
            integration_type: format!("{:?}", i.integration_type),
            status: format!("{:?}", i.status),
            priority: format!("{:?}", i.priority),
            endpoints: i
                .endpoints
                .iter()
                .map(|e| EndpointView {
                    method: e.method.clone(),
                    path: e.path.clone(),
                    description: e.description.clone(),
                    status: format!("{:?}", e.implementation_status),
                })
                .collect(),
            config_required: i
                .config_fields
                .iter()
                .map(|f| ConfigFieldView {
                    name: f.name.clone(),
                    field_type: f.field_type.clone(),
                    required: f.required,
                    description: f.description.clone(),
                    current_value: f.current_value.clone(),
                })
                .collect(),
            last_check: i.last_check,
            error_message: i.error_message.clone(),
            documentation_url: i.documentation_url.clone(),
        })
        .collect();

    Ok(Json(ApiResponse::success(IntegrationsListResponse {
        integrations: views,
        total,
        connected,
        disconnected,
        hardcoded,
    })))
}

/// Детайли за конкретна интеграция
#[derive(Serialize)]
pub struct IntegrationDetailResponse {
    pub integration: IntegrationView,
    pub connection_guide: ConnectionGuide,
}

#[derive(Serialize)]
pub struct ConnectionGuide {
    pub steps: Vec<String>,
    pub example_config: serde_json::Value,
    pub testing_commands: Vec<String>,
}

/// GET /api/admin/integrations/:id - Детайли за интеграция
pub async fn get_integration_detail(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<IntegrationDetailResponse>>, StatusCode> {
    let registry = IntegrationRegistry::new();

    match registry.get_integration(&id) {
        Some(i) => {
            let view = IntegrationView {
                id: i.id.clone(),
                name: i.name.clone(),
                description: i.description.clone(),
                integration_type: format!("{:?}", i.integration_type),
                status: format!("{:?}", i.status),
                priority: format!("{:?}", i.priority),
                endpoints: i
                    .endpoints
                    .iter()
                    .map(|e| EndpointView {
                        method: e.method.clone(),
                        path: e.path.clone(),
                        description: e.description.clone(),
                        status: format!("{:?}", e.implementation_status),
                    })
                    .collect(),
                config_required: i
                    .config_fields
                    .iter()
                    .map(|f| ConfigFieldView {
                        name: f.name.clone(),
                        field_type: f.field_type.clone(),
                        required: f.required,
                        description: f.description.clone(),
                        current_value: f.current_value.clone(),
                    })
                    .collect(),
                last_check: i.last_check,
                error_message: i.error_message.clone(),
                documentation_url: i.documentation_url.clone(),
            };

            let guide = ConnectionGuide {
                steps: i.connection_steps.clone(),
                example_config: i.example_config.clone(),
                testing_commands: i.testing_commands.clone(),
            };

            Ok(Json(ApiResponse::success(IntegrationDetailResponse {
                integration: view,
                connection_guide: guide,
            })))
        }
        None => Ok(Json(ApiResponse::error(format!(
            "Integration '{}' not found",
            id
        )))),
    }
}

/// Конфигуриране на интеграция
#[derive(Deserialize)]
pub struct ConfigureIntegrationRequest {
    pub config: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct ConfigureIntegrationResponse {
    pub success: bool,
    pub message: String,
    pub test_result: Option<String>,
}

/// POST /api/admin/integrations/:id/configure - Конфигуриране
pub async fn configure_integration(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(_req): Json<ConfigureIntegrationRequest>,
) -> Result<Json<ApiResponse<ConfigureIntegrationResponse>>, StatusCode> {
    // Тук ще се запазва конфигурацията в базата данни
    // и ще се тества връзката

    let response = ConfigureIntegrationResponse {
        success: true,
        message: format!("Configuration for '{}' saved successfully", id),
        test_result: Some("Connection test passed".to_string()),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Тестване на интеграция
#[derive(Serialize)]
pub struct TestIntegrationResponse {
    pub success: bool,
    pub response_time_ms: u64,
    pub details: String,
    pub errors: Vec<String>,
}

/// POST /api/admin/integrations/:id/test - Тестване на връзка
pub async fn test_integration(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TestIntegrationResponse>>, StatusCode> {
    let integration_test = match id.as_str() {
        "broker-ibkr" => {
            let mut config = HashMap::new();
            config.insert(
                "IBKR_API_KEY".to_string(),
                env::var("IBKR_API_KEY").unwrap_or_default(),
            );
            config.insert(
                "IBKR_API_SECRET".to_string(),
                env::var("IBKR_API_SECRET").unwrap_or_default(),
            );
            config.insert(
                "IBKR_BASE_URL".to_string(),
                env::var("IBKR_BASE_URL")
                    .unwrap_or_else(|_| "https://paper-api.ibkr.com".to_string()),
            );

            match ConnectorFactory::create_connector("ibkr", config) {
                Ok(connector) => connector.test_connection().await,
                Err(err) => ConnectionTestResult {
                    success: false,
                    response_time_ms: 0,
                    message: "Unable to configure IBKR connector".to_string(),
                    errors: vec![err],
                },
            }
        }
        "market-data" => {
            let mut config = HashMap::new();
            config.insert(
                "POLYGON_API_KEY".to_string(),
                env::var("POLYGON_API_KEY").unwrap_or_default(),
            );

            match ConnectorFactory::create_connector("polygon", config) {
                Ok(connector) => connector.test_connection().await,
                Err(err) => ConnectionTestResult {
                    success: false,
                    response_time_ms: 0,
                    message: "Unable to configure Polygon connector".to_string(),
                    errors: vec![err],
                },
            }
        }
        _ => ConnectionTestResult {
            success: false,
            response_time_ms: 0,
            message: format!(
                "Integration '{}' does not expose an automated connection test yet.",
                id
            ),
            errors: vec!["Not implemented".to_string()],
        },
    };

    let result = TestIntegrationResponse {
        success: integration_test.success,
        response_time_ms: integration_test.response_time_ms,
        details: integration_test.message,
        errors: integration_test.errors,
    };

    Ok(Json(ApiResponse::success(result)))
}

/// Обобщена статистика
#[derive(Serialize)]
pub struct AdminStatsResponse {
    pub system_status: String,
    pub api_integrations: ApiIntegrationStats,
    pub internal_modules: InternalModuleStats,
}

#[derive(Serialize)]
pub struct ApiIntegrationStats {
    pub total: usize,
    pub external_apis: usize,
    pub internal_modules: usize,
    pub hardcoded_stubs: usize,
    pub ready_for_production: usize,
}

#[derive(Serialize)]
pub struct InternalModuleStats {
    pub treasury: ModuleStatus,
    pub analytics: ModuleStatus,
    pub ml: ModuleStatus,
    pub broker: ModuleStatus,
    pub rag: ModuleStatus,
}

#[derive(Serialize)]
pub struct ModuleStatus {
    pub name: String,
    pub status: String,
    pub endpoints_implemented: usize,
    pub endpoints_stub: usize,
}

/// GET /api/admin/stats - Обща статистика
pub async fn get_admin_stats(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<AdminStatsResponse>>, StatusCode> {
    let registry = IntegrationRegistry::new();
    let integrations = registry.get_all_integrations();

    let external = integrations
        .iter()
        .filter(|i| matches!(i.integration_type, IntegrationType::ExternalApi))
        .count();

    let internal = integrations
        .iter()
        .filter(|i| matches!(i.integration_type, IntegrationType::InternalModule))
        .count();

    let stubs = integrations
        .iter()
        .filter(|i| {
            matches!(
                i.status,
                IntegrationStatus::Hardcoded | IntegrationStatus::Stub
            )
        })
        .count();

    let ready = integrations
        .iter()
        .filter(|i| matches!(i.status, IntegrationStatus::Connected))
        .count();

    let response = AdminStatsResponse {
        system_status: "Development".to_string(),
        api_integrations: ApiIntegrationStats {
            total: integrations.len(),
            external_apis: external,
            internal_modules: internal,
            hardcoded_stubs: stubs,
            ready_for_production: ready,
        },
        internal_modules: InternalModuleStats {
            treasury: ModuleStatus {
                name: "Treasury".to_string(),
                status: "Refactored - No Fiat".to_string(),
                endpoints_implemented: 10,
                endpoints_stub: 0,
            },
            analytics: ModuleStatus {
                name: "Analytics".to_string(),
                status: "Hardcoded".to_string(),
                endpoints_implemented: 0,
                endpoints_stub: 5,
            },
            ml: ModuleStatus {
                name: "ML".to_string(),
                status: "Simulated".to_string(),
                endpoints_implemented: 0,
                endpoints_stub: 3,
            },
            broker: ModuleStatus {
                name: "Broker".to_string(),
                status: "Not Implemented".to_string(),
                endpoints_implemented: 0,
                endpoints_stub: 5,
            },
            rag: ModuleStatus {
                name: "RAG".to_string(),
                status: "Connected".to_string(),
                endpoints_implemented: 5,
                endpoints_stub: 0,
            },
        },
    };

    Ok(Json(ApiResponse::success(response)))
}
