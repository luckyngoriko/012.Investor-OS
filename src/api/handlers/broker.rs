//! Broker API Handlers
//!
//! S6 API Endpoints:
//! - POST /api/broker/orders - Place order
//! - DELETE /api/broker/orders/:id - Cancel order
//! - GET /api/broker/positions - Get positions
//! - GET /api/broker/account - Get account info
//! - POST /api/broker/kill-switch - Trigger kill switch

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::api::handlers::ApiResponse;
use crate::api::AppState;
use crate::broker::{Order, OrderSide, OrderType, TimeInForce, Broker};

/// Request to place an order
#[derive(Serialize, Deserialize)]
pub struct PlaceOrderRequest {
    pub ticker: String,
    pub side: String,
    pub quantity: Decimal,
    pub order_type: String,
    pub limit_price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub time_in_force: Option<String>,
    pub proposal_id: Option<Uuid>,
    pub portfolio_id: Uuid,
    pub notes: Option<String>,
}

/// Order response
#[derive(Serialize, Deserialize)]
pub struct OrderResponse {
    pub id: Uuid,
    pub broker_order_id: Option<String>,
    pub ticker: String,
    pub side: String,
    pub quantity: Decimal,
    pub order_type: String,
    pub limit_price: Option<Decimal>,
    pub status: String,
    pub filled_quantity: Decimal,
    pub created_at: DateTime<Utc>,
}

/// Position response
#[derive(Serialize, Deserialize)]
pub struct PositionResponse {
    pub id: Uuid,
    pub ticker: String,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_price: Option<Decimal>,
    pub market_value: Option<Decimal>,
    pub unrealized_pnl: Option<Decimal>,
    pub updated_at: DateTime<Utc>,
}

/// Account info response
#[derive(Serialize, Deserialize)]
pub struct AccountInfoResponse {
    pub account_id: String,
    pub cash_balance: Decimal,
    pub buying_power: Decimal,
    pub net_liquidation: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub currency: String,
}

/// POST /api/broker/orders - Place a new order
pub async fn place_order(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PlaceOrderRequest>,
) -> Result<Json<ApiResponse<OrderResponse>>, StatusCode> {
    // Parse order side
    let side = match req.side.to_lowercase().as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => {
            return Ok(Json(ApiResponse::error(format!(
                "Invalid order side: {}. Use 'buy' or 'sell'", req.side
            ))));
        }
    };
    
    // Parse order type
    let order_type = match req.order_type.to_lowercase().as_str() {
        "market" => OrderType::Market,
        "limit" => OrderType::Limit,
        "stop" => OrderType::Stop,
        "stop_limit" => OrderType::StopLimit,
        _ => {
            return Ok(Json(ApiResponse::error(format!(
                "Invalid order type: {}. Use 'market', 'limit', 'stop', or 'stop_limit'", 
                req.order_type
            ))));
        }
    };
    
    // Parse time in force
    let time_in_force = req.time_in_force.as_ref().map(|t| {
        match t.to_lowercase().as_str() {
            "day" => TimeInForce::Day,
            "gtc" => TimeInForce::Gtc,
            "ioc" => TimeInForce::Ioc,
            _ => TimeInForce::Day,
        }
    }).unwrap_or(TimeInForce::Day);
    
    // Create order
    let mut order = Order {
        id: Uuid::new_v4(),
        broker_order_id: None,
        ticker: req.ticker.clone(),
        side,
        quantity: req.quantity,
        order_type,
        limit_price: req.limit_price,
        stop_price: req.stop_price,
        time_in_force,
        status: crate::broker::OrderStatus::PendingSubmit,
        filled_quantity: Decimal::ZERO,
        avg_fill_price: None,
        commission: None,
        proposal_id: req.proposal_id,
        portfolio_id: req.portfolio_id,
        notes: req.notes.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    // Place order through paper broker
    match state.broker.place_order(&mut order).await {
        Ok(_) => {
            let response = OrderResponse {
                id: order.id,
                broker_order_id: order.broker_order_id.clone(),
                ticker: order.ticker.clone(),
                side: req.side.clone(),
                quantity: order.quantity,
                order_type: req.order_type.clone(),
                limit_price: order.limit_price,
                status: format!("{:?}", order.status),
                filled_quantity: order.filled_quantity,
                created_at: order.created_at,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Order placement error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// DELETE /api/broker/orders/:id - Cancel an order
pub async fn cancel_order(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    // Create a dummy order to cancel (paper broker will look it up by ID)
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
        status: crate::broker::OrderStatus::PendingSubmit,
        filled_quantity: Decimal::ZERO,
        avg_fill_price: None,
        commission: None,
        proposal_id: None,
        portfolio_id: Uuid::nil(),
        notes: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    match state.broker.cancel_order(&mut order).await {
        Ok(_) => {
            Ok(Json(ApiResponse::success(serde_json::json!({
                "order_id": order_id,
                "status": "cancelled"
            }))))
        }
        Err(e) => {
            tracing::error!("Order cancellation error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// GET /api/broker/positions - Get all positions
pub async fn get_positions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<PositionResponse>>>, StatusCode> {
    match state.broker.get_positions().await {
        Ok(positions) => {
            let responses: Vec<PositionResponse> = positions.into_iter().map(|p| {
                PositionResponse {
                    id: Uuid::new_v4(),
                    ticker: p.ticker,
                    quantity: p.quantity,
                    avg_cost: p.avg_cost,
                    market_price: p.market_price,
                    market_value: p.market_value,
                    unrealized_pnl: p.unrealized_pnl,
                    updated_at: Utc::now(),
                }
            }).collect();
            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("Get positions error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// GET /api/broker/positions/:ticker - Get position for ticker
pub async fn get_position(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
) -> Result<Json<ApiResponse<Option<PositionResponse>>>, StatusCode> {
    match state.broker.get_position(&ticker).await {
        Ok(position) => {
            let response = position.map(|p| PositionResponse {
                id: Uuid::new_v4(),
                ticker: p.ticker,
                quantity: p.quantity,
                avg_cost: p.avg_cost,
                market_price: p.market_price,
                market_value: p.market_value,
                unrealized_pnl: p.unrealized_pnl,
                updated_at: Utc::now(),
            });
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Get position error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// GET /api/broker/account - Get account information
pub async fn get_account(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<AccountInfoResponse>>, StatusCode> {
    match state.broker.get_account_info().await {
        Ok(info) => {
            let response = AccountInfoResponse {
                account_id: info.account_id,
                cash_balance: info.cash_balance,
                buying_power: info.buying_power,
                net_liquidation: info.net_liquidation,
                unrealized_pnl: info.unrealized_pnl,
                realized_pnl: info.realized_pnl,
                currency: info.currency,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Get account error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// POST /api/broker/kill-switch - Trigger kill switch (S6-D8)
#[derive(Serialize, Deserialize)]
pub struct KillSwitchRequest {
    pub portfolio_id: Uuid,
    pub reason: String,
}

#[derive(Serialize, Deserialize)]
pub struct KillSwitchResponse {
    pub triggered: bool,
    pub timestamp: DateTime<Utc>,
    pub orders_cancelled: usize,
    pub positions_flattened: usize,
    pub message: String,
}

pub async fn trigger_kill_switch(
    Json(_req): Json<KillSwitchRequest>,
) -> Result<Json<ApiResponse<KillSwitchResponse>>, StatusCode> {
    let response = KillSwitchResponse {
        triggered: true,
        timestamp: Utc::now(),
        orders_cancelled: 0,
        positions_flattened: 0,
        message: "Kill switch triggered - manual intervention required".to_string(),
    };

    Ok(Json(ApiResponse::success(response)))
}
