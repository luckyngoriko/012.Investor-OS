//! Tools - функции, които LLM може да извиква

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Резултат от изпълнение на tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: String,
    pub success: bool,
    pub metadata: Option<serde_json::Value>,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: true,
            metadata: None,
        }
    }
    
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            output: message.into(),
            success: false,
            metadata: None,
        }
    }
}

/// Tool call representation
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub input: String,
}

/// Trait за tools
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value; // JSON Schema
    async fn execute(&self, input: &str) -> ToolResult;
}

/// Tool registry - съхранява налични tools
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }
    
    pub async fn execute(&self, name: &str, input: &str) -> Result<ToolResult, super::ChainError> {
        let tool = self.tools.get(name)
            .ok_or_else(|| super::ChainError::ToolError(
                format!("Tool '{}' not found", name)
            ))?;
        
        Ok(tool.execute(input).await)
    }
    
    /// Описание на всички tools за prompt
    pub fn describe(&self) -> String {
        self.tools.values()
            .map(|t| format!("- {}: {}\n  Parameters: {}", 
                t.name(), 
                t.description(),
                t.parameters()
            ))
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Trading Tools ====================

/// Tool: Получаване на текущ портфейл
pub struct PortfolioTool {
    portfolio_service: std::sync::Arc<dyn PortfolioService>,
}

#[async_trait]
pub trait PortfolioService: Send + Sync {
    async fn get_positions(&self) -> Vec<Position>;
    async fn get_balance(&self) -> rust_decimal::Decimal;
}

#[derive(Debug, Clone)]
pub struct Position {
    pub ticker: String,
    pub quantity: rust_decimal::Decimal,
    pub avg_price: rust_decimal::Decimal,
    pub current_price: rust_decimal::Decimal,
}

#[async_trait]
impl Tool for PortfolioTool {
    fn name(&self) -> &str {
        "get_portfolio"
    }
    
    fn description(&self) -> &str {
        "Get current portfolio positions and cash balance"
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    
    async fn execute(&self, _input: &str) -> ToolResult {
        let positions = self.portfolio_service.get_positions().await;
        let balance = self.portfolio_service.get_balance().await;
        
        let output = serde_json::json!({
            "cash_balance": balance.to_string(),
            "positions": positions.iter().map(|p| serde_json::json!({
                "ticker": p.ticker,
                "quantity": p.quantity.to_string(),
                "avg_price": p.avg_price.to_string(),
                "current_price": p.current_price.to_string(),
                "unrealized_pnl": ((p.current_price - p.avg_price) * p.quantity).to_string()
            })).collect::<Vec<_>>()
        });
        
        ToolResult::success(output.to_string())
    }
}

/// Tool: Получаване на пазарни данни
pub struct MarketDataTool {
    data_service: std::sync::Arc<dyn MarketDataService>,
}

#[async_trait]
pub trait MarketDataService: Send + Sync {
    async fn get_price(&self, ticker: &str) -> Option<rust_decimal::Decimal>;
    async fn get_ohlcv(&self, ticker: &str, timeframe: &str) -> Vec<OHLCV>;
}

#[derive(Debug, Clone)]
pub struct OHLCV {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub open: rust_decimal::Decimal,
    pub high: rust_decimal::Decimal,
    pub low: rust_decimal::Decimal,
    pub close: rust_decimal::Decimal,
    pub volume: u64,
}

#[async_trait]
impl Tool for MarketDataTool {
    fn name(&self) -> &str {
        "get_market_data"
    }
    
    fn description(&self) -> &str {
        "Get current price and OHLCV data for a ticker. Input: JSON with 'ticker' and optional 'timeframe'"
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "ticker": {"type": "string", "description": "Stock ticker symbol"},
                "timeframe": {"type": "string", "enum": ["1d", "1w", "1m"], "default": "1d"}
            },
            "required": ["ticker"]
        })
    }
    
    async fn execute(&self, input: &str) -> ToolResult {
        let params: serde_json::Value = match serde_json::from_str(input) {
            Ok(v) => v,
            Err(e) => return ToolResult::error(format!("Invalid JSON: {}", e)),
        };
        
        let ticker = params["ticker"].as_str()
            .unwrap_or("UNKNOWN");
        let timeframe = params["timeframe"].as_str()
            .unwrap_or("1d");
        
        let price = self.data_service.get_price(ticker).await;
        let ohlcv = self.data_service.get_ohlcv(ticker, timeframe).await;
        
        let output = serde_json::json!({
            "ticker": ticker,
            "current_price": price.map(|p| p.to_string()),
            "timeframe": timeframe,
            "data_points": ohlcv.len(),
            "latest": ohlcv.last().map(|o| serde_json::json!({
                "open": o.open.to_string(),
                "high": o.high.to_string(),
                "low": o.low.to_string(),
                "close": o.close.to_string(),
                "volume": o.volume
            }))
        });
        
        ToolResult::success(output.to_string())
    }
}

/// Tool: Изпълнение на поръчка (симулация или реална)
pub struct PlaceOrderTool {
    broker: std::sync::Arc<dyn BrokerService>,
    simulate: bool,
}

#[async_trait]
pub trait BrokerService: Send + Sync {
    async fn place_order(&self, order: OrderRequest) -> Result<OrderResponse, String>;
}

#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub ticker: String,
    pub action: Action,
    pub quantity: u32,
    pub order_type: OrderType,
}

#[derive(Debug, Clone)]
pub enum Action { Buy, Sell }

#[derive(Debug, Clone)]
pub enum OrderType {
    Market,
    Limit(rust_decimal::Decimal),
    Stop(rust_decimal::Decimal),
}

#[derive(Debug, Clone)]
pub struct OrderResponse {
    pub order_id: String,
    pub status: String,
    pub filled_quantity: u32,
    pub avg_price: rust_decimal::Decimal,
}

#[async_trait]
impl Tool for PlaceOrderTool {
    fn name(&self) -> &str {
        "place_order"
    }
    
    fn description(&self) -> &str {
        if self.simulate {
            "Simulate placing an order (paper trading)"
        } else {
            "Place a real order (LIVE TRADING - use with caution!)"
        }
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "ticker": {"type": "string"},
                "action": {"type": "string", "enum": ["buy", "sell"]},
                "quantity": {"type": "integer"},
                "order_type": {"type": "string", "enum": ["market", "limit", "stop"]}
            },
            "required": ["ticker", "action", "quantity"]
        })
    }
    
    async fn execute(&self, input: &str) -> ToolResult {
        // Парсване и валидация
        let _params: serde_json::Value = match serde_json::from_str(input) {
            Ok(v) => v,
            Err(e) => return ToolResult::error(format!("Invalid JSON: {}", e)),
        };
        
        // Създаваме OrderRequest...
        // (simplified)
        
        ToolResult::success(format!("Order simulated: {}", input))
    }
}

/// Tool: RAG търсене в SEC filings
pub struct SecSearchTool {
    rag_service: std::sync::Arc<dyn RagSearchService>,
}

#[async_trait]
pub trait RagSearchService: Send + Sync {
    async fn search_sec(&self, ticker: &str, query: &str) -> Vec<SearchResult>;
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub content: String,
    pub source: String,
    pub relevance: f32,
}

#[async_trait]
impl Tool for SecSearchTool {
    fn name(&self) -> &str {
        "search_sec_filings"
    }
    
    fn description(&self) -> &str {
        "Search SEC filings for a ticker. Input: JSON with 'ticker' and 'query'"
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "ticker": {"type": "string"},
                "query": {"type": "string", "description": "What to search for"}
            },
            "required": ["ticker", "query"]
        })
    }
    
    async fn execute(&self, input: &str) -> ToolResult {
        let params: serde_json::Value = match serde_json::from_str(input) {
            Ok(v) => v,
            Err(e) => return ToolResult::error(format!("Invalid JSON: {}", e)),
        };
        
        let ticker = params["ticker"].as_str().unwrap_or("");
        let query = params["query"].as_str().unwrap_or("");
        
        let results = self.rag_service.search_sec(ticker, query).await;
        
        let output = serde_json::json!({
            "results": results.iter().map(|r| serde_json::json!({
                "content": r.content,
                "source": r.source,
                "relevance": r.relevance
            })).collect::<Vec<_>>()
        });
        
        ToolResult::success(output.to_string())
    }
}

/// Tool: Анализ на sentiment
pub struct SentimentTool {
    sentiment_service: std::sync::Arc<dyn SentimentService>,
}

#[async_trait]
pub trait SentimentService: Send + Sync {
    async fn analyze(&self, ticker: &str) -> SentimentResult;
}

#[derive(Debug, Clone)]
pub struct SentimentResult {
    pub news_sentiment: f32,   // -1.0 to 1.0
    pub social_sentiment: f32, // -1.0 to 1.0
    pub overall: f32,
}

#[async_trait]
impl Tool for SentimentTool {
    fn name(&self) -> &str {
        "get_sentiment"
    }
    
    fn description(&self) -> &str {
        "Get sentiment analysis for a ticker"
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "ticker": {"type": "string"}
            },
            "required": ["ticker"]
        })
    }
    
    async fn execute(&self, input: &str) -> ToolResult {
        let params: serde_json::Value = match serde_json::from_str(input) {
            Ok(v) => v,
            Err(e) => return ToolResult::error(format!("Invalid JSON: {}", e)),
        };
        
        let ticker = params["ticker"].as_str().unwrap_or("");
        let result = self.sentiment_service.analyze(ticker).await;
        
        let output = serde_json::json!({
            "ticker": ticker,
            "news_sentiment": result.news_sentiment,
            "social_sentiment": result.social_sentiment,
            "overall": result.overall
        });
        
        ToolResult::success(output.to_string())
    }
}

/// Builder за trading tools
pub struct TradingToolsBuilder {
    registry: ToolRegistry,
}

impl TradingToolsBuilder {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }
    
    pub fn with_portfolio_service(self, _service: std::sync::Arc<dyn PortfolioService>) -> Self {
        // self.registry.register(Box::new(PortfolioTool { portfolio_service: service }));
        self
    }
    
    pub fn build(self) -> ToolRegistry {
        self.registry
    }
}

impl Default for TradingToolsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
