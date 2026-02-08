//! Interactive Brokers API Models

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// IB Order Request
#[derive(Debug, Clone, Serialize)]
pub struct IbOrderRequest {
    #[serde(rename = "acctId")]
    pub acct_id: String,
    pub conid: i64,
    #[serde(rename = "secType")]
    pub sec_type: String,
    #[serde(rename = "cOID")]
    pub c_oid: Option<String>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    #[serde(rename = "orderType")]
    pub order_type: String,
    #[serde(rename = "listingExchange")]
    pub listing_exchange: Option<String>,
    #[serde(rename = "isSingleGroup")]
    pub is_single_group: bool,
    #[serde(rename = "outsideRTH")]
    pub outside_rth: bool,
    pub price: Option<String>,
    #[serde(rename = "auxPrice")]
    pub aux_price: Option<String>,
    pub side: String,
    pub ticker: String,
    #[serde(rename = "tif")]
    pub tif: String,
    pub quantity: String,
    #[serde(rename = "useAdaptive")]
    pub use_adaptive: bool,
}

/// IB Order Response
#[derive(Debug, Clone, Deserialize)]
pub struct IbOrderResponse {
    #[serde(rename = "order_id")]
    pub order_id: String,
    #[serde(rename = "local_order_id")]
    pub local_order_id: Option<String>,
    #[serde(rename = "order_status")]
    pub order_status: Option<String>,
    #[serde(rename = "encrypt_message")]
    pub encrypt_message: Option<String>,
}

/// IB Order Status
#[derive(Debug, Clone, Deserialize)]
pub struct IbOrderStatus {
    #[serde(rename = "orderId")]
    pub order_id: String,
    pub status: String,
    #[serde(rename = "filledQuantity")]
    pub filled_quantity: Option<String>,
    #[serde(rename = "remainingQuantity")]
    pub remaining_quantity: Option<String>,
    #[serde(rename = "avgFillPrice")]
    pub avg_fill_price: Option<String>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
}

/// IB Position
#[derive(Debug, Clone, Deserialize)]
pub struct IbPosition {
    #[serde(rename = "acctId")]
    pub acct_id: String,
    #[serde(rename = "conid")]
    pub conid: i64,
    #[serde(rename = "contractDesc")]
    pub contract_desc: String,
    #[serde(rename = "position")]
    // Decimal serializes as number by default with serde feature
    pub position: Decimal,
    #[serde(rename = "mktPrice")]
    // Decimal serializes as number by default with serde feature
    pub mkt_price: Option<Decimal>,
    #[serde(rename = "mktValue")]
    // Decimal serializes as number by default with serde feature
    pub mkt_value: Option<Decimal>,
    #[serde(rename = "currency")]
    pub currency: String,
    #[serde(rename = "avgCost")]
    // Decimal serializes as number by default with serde feature
    pub avg_cost: Decimal,
    #[serde(rename = "avgPrice")]
    // Decimal serializes as number by default with serde feature
    pub avg_price: Option<Decimal>,
    #[serde(rename = "realizedPnl")]
    // Decimal serializes as number by default with serde feature
    pub realized_pnl: Option<Decimal>,
    #[serde(rename = "unrealizedPnl")]
    // Decimal serializes as number by default with serde feature
    pub unrealized_pnl: Option<Decimal>,
}

/// IB Account Summary
#[derive(Debug, Clone, Deserialize)]
pub struct IbAccountSummary {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "accountType")]
    pub account_type: Option<String>,
    #[serde(rename = "netLiquidation")]
    // Decimal serializes as number by default with serde feature
    pub net_liquidation: Option<Decimal>,
    #[serde(rename = "buyingPower")]
    // Decimal serializes as number by default with serde feature
    pub buying_power: Option<Decimal>,
    #[serde(rename = "cashBalance")]
    // Decimal serializes as number by default with serde feature
    pub cash_balance: Option<Decimal>,
    #[serde(rename = "equityWithLoanValue")]
    // Decimal serializes as number by default with serde feature
    pub equity_with_loan_value: Option<Decimal>,
    #[serde(rename = "previousDayEquityWithLoanValue")]
    // Decimal serializes as number by default with serde feature
    pub previous_day_equity_with_loan_value: Option<Decimal>,
    #[serde(rename = "grossPositionValue")]
    // Decimal serializes as number by default with serde feature
    pub gross_position_value: Option<Decimal>,
    #[serde(rename = "regTEquity")]
    // Decimal serializes as number by default with serde feature
    pub reg_t_equity: Option<Decimal>,
    #[serde(rename = "regTMargin")]
    // Decimal serializes as number by default with serde feature
    pub reg_t_margin: Option<Decimal>,
    #[serde(rename = "availableFunds")]
    // Decimal serializes as number by default with serde feature
    pub available_funds: Option<Decimal>,
    #[serde(rename = "excessLiquidity")]
    // Decimal serializes as number by default with serde feature
    pub excess_liquidity: Option<Decimal>,
    #[serde(rename = "dayTradesRemaining")]
    pub day_trades_remaining: Option<i32>,
    #[serde(rename = "maintenanceMarginReq")]
    // Decimal serializes as number by default with serde feature
    pub maintenance_margin_req: Option<Decimal>,
}

/// IB Market Data Snapshot
#[derive(Debug, Clone, Deserialize)]
pub struct IbMarketDataSnapshot {
    pub conid: i64,
    #[serde(rename = "_updated")]
    pub updated: Option<i64>,
    #[serde(rename = "31")] // Last price
    pub last_price: Option<String>,
    #[serde(rename = "83")] // Bid price
    pub bid_price: Option<String>,
    #[serde(rename = "84")] // Ask price
    pub ask_price: Option<String>,
    #[serde(rename = "85")] // Bid size
    pub bid_size: Option<String>,
    #[serde(rename = "86")] // Ask size
    pub ask_size: Option<String>,
    #[serde(rename = "88")] // Volume
    pub volume: Option<String>,
    #[serde(rename = "7295")] // Last size
    pub last_size: Option<String>,
}

/// IB Execution
#[derive(Debug, Clone, Deserialize)]
pub struct IbExecution {
    #[serde(rename = "executionId")]
    pub execution_id: String,
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "conid")]
    pub conid: i64,
    pub symbol: String,
    pub side: String,
    // Decimal serializes as number by default with serde feature
    pub shares: Decimal,
    // Decimal serializes as number by default with serde feature
    pub price: Decimal,
    // Decimal serializes as number by default with serde feature
    pub commission: Decimal,
    #[serde(rename = "execTime")]
    pub exec_time: String,
}
