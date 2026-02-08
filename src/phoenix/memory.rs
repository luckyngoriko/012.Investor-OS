//! Phoenix RAG Memory
//!
//! Experience storage and retrieval using neurocod-rag (Sprint 5)
//! Stores: winners, mistakes, regime patterns, strategies

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Experience stored in RAG memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingExperience {
    pub id: Uuid,
    pub market_condition: MarketCondition,
    pub decision: TradingDecision,
    pub outcome: TradeOutcome,
    pub lesson: String,
    pub regime: MarketRegime,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub embedding: Option<Vec<f32>>, // For vector search
}

/// Market condition when trade was made
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCondition {
    pub ticker: String,
    pub price: rust_decimal::Decimal,
    pub volume: u64,
    pub rsi: Option<f64>,
    pub macd: Option<f64>,
    pub vwap: Option<rust_decimal::Decimal>,
    pub trend: TrendDirection,
    pub volatility: VolatilityLevel,
}

/// Trade outcome after holding period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOutcome {
    pub profit_loss: rust_decimal::Decimal,
    pub profit_loss_pct: f64,
    pub holding_period_days: u32,
    pub max_favorable_excursion: rust_decimal::Decimal,
    pub max_adverse_excursion: rust_decimal::Decimal,
    pub exit_reason: ExitReason,
    pub success: bool,
}

/// Direction of trend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    StrongUptrend,
    Uptrend,
    Sideways,
    Downtrend,
    StrongDowntrend,
}

/// Volatility level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VolatilityLevel {
    Low,    // ATR < 2%
    Normal, // ATR 2-5%
    High,   // ATR 5-10%
    Extreme, // ATR > 10%
}

/// Market regime classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MarketRegime {
    RiskOn,      // Growth stocks rallying
    Uncertain,   // Mixed signals
    RiskOff,     // Flight to safety
    Crisis,      // High volatility, correlations → 1
}

/// Reason for exiting trade
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExitReason {
    TakeProfit,
    StopLoss,
    TrailingStop,
    TimeStop,
    SignalReversal,
    Manual,
}

/// RAG Memory for Phoenix - stores and retrieves trading experiences
pub struct RagMemory {
    winners: Vec<TradingExperience>,
    mistakes: Vec<TradingExperience>,
    regime_patterns: Vec<TradingExperience>,
    strategies: Vec<StrategyRecord>,
}

/// Strategy performance record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRecord {
    pub name: String,
    pub description: String,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub win_rate: f64,
    pub avg_profit: rust_decimal::Decimal,
    pub avg_loss: rust_decimal::Decimal,
    pub profit_factor: f64,
    pub best_regime: Option<MarketRegime>,
}

/// Query for similar experiences
#[derive(Debug, Clone)]
pub struct ExperienceQuery {
    pub ticker: Option<String>,
    pub regime: Option<MarketRegime>,
    pub trend: Option<TrendDirection>,
    pub volatility: Option<VolatilityLevel>,
    pub outcome_filter: OutcomeFilter,
    pub limit: usize,
}

/// Filter by outcome type
#[derive(Debug, Clone)]
pub enum OutcomeFilter {
    All,
    WinnersOnly,
    MistakesOnly,
    SimilarTo(TradeOutcome), // Find similar P&L patterns
}

impl Default for ExperienceQuery {
    fn default() -> Self {
        Self {
            ticker: None,
            regime: None,
            trend: None,
            volatility: None,
            outcome_filter: OutcomeFilter::All,
            limit: 10,
        }
    }
}

impl RagMemory {
    pub fn new() -> Self {
        Self {
            winners: Vec::new(),
            mistakes: Vec::new(),
            regime_patterns: Vec::new(),
            strategies: Vec::new(),
        }
    }
    
    /// Store a new trading experience
    pub fn store_experience(&mut self, experience: TradingExperience) {
        // Classify and store
        if experience.outcome.success {
            self.winners.push(experience.clone());
        } else {
            self.mistakes.push(experience.clone());
        }
        
        // Also store for regime pattern analysis
        self.regime_patterns.push(experience);
    }
    
    /// Query similar experiences
    pub fn query_similar_cases(&self, query: &ExperienceQuery) -> Vec<&TradingExperience> {
        let all_experiences: Vec<&TradingExperience> = self
            .winners
            .iter()
            .chain(self.mistakes.iter())
            .collect();
        
        let mut filtered: Vec<&TradingExperience> = all_experiences
            .into_iter()
            .filter(|exp| {
                // Apply filters
                if let Some(ref ticker) = query.ticker {
                    if exp.market_condition.ticker != *ticker {
                        return false;
                    }
                }
                
                if let Some(ref regime) = query.regime {
                    if exp.regime != *regime {
                        return false;
                    }
                }
                
                if let Some(ref trend) = query.trend {
                    if exp.market_condition.trend != *trend {
                        return false;
                    }
                }
                
                if let Some(ref vol) = query.volatility {
                    if exp.market_condition.volatility != *vol {
                        return false;
                    }
                }
                
                match query.outcome_filter {
                    OutcomeFilter::WinnersOnly => exp.outcome.success,
                    OutcomeFilter::MistakesOnly => !exp.outcome.success,
                    _ => true,
                }
            })
            .collect();
        
        // Sort by relevance (most recent first for now)
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Limit results
        filtered.truncate(query.limit);
        filtered
    }
    
    /// Get lessons learned from winners
    pub fn get_winner_lessons(&self, limit: usize) -> Vec<String> {
        self.winners
            .iter()
            .take(limit)
            .map(|exp| exp.lesson.clone())
            .collect()
    }
    
    /// Get lessons learned from mistakes
    pub fn get_mistake_lessons(&self, limit: usize) -> Vec<String> {
        self.mistakes
            .iter()
            .take(limit)
            .map(|exp| exp.lesson.clone())
            .collect()
    }
    
    /// Query what works in a specific regime
    pub fn what_works_in_regime(&self, regime: &MarketRegime) -> RegimeInsight {
        let regime_trades: Vec<&TradingExperience> = self
            .regime_patterns
            .iter()
            .filter(|exp| &exp.regime == regime)
            .collect();
        
        let winners: Vec<_> = regime_trades.iter().filter(|e| e.outcome.success).collect();
        let total = regime_trades.len();
        
        if total == 0 {
            return RegimeInsight {
                regime: regime.clone(),
                win_rate: 0.0,
                avg_return: 0.0,
                best_strategies: vec![],
                recommendation: "No data for this regime yet".to_string(),
            };
        }
        
        let win_rate = winners.len() as f64 / total as f64;
        let avg_return = regime_trades
            .iter()
            .map(|e| e.outcome.profit_loss_pct)
            .sum::<f64>() / total as f64;
        
        RegimeInsight {
            regime: regime.clone(),
            win_rate,
            avg_return,
            best_strategies: vec![], // Would analyze from actual trades
            recommendation: format!(
                "In {} regime: {:.1}% win rate, {:.2}% avg return",
                format!("{:?}", regime),
                win_rate * 100.0,
                avg_return
            ),
        }
    }
    
    /// Record strategy performance
    pub fn record_strategy(&mut self, strategy: StrategyRecord) {
        self.strategies.push(strategy);
    }
    
    /// Get best performing strategies
    pub fn get_best_strategies(&self, min_trades: u32) -> Vec<&StrategyRecord> {
        let mut valid_strategies: Vec<&StrategyRecord> = self
            .strategies
            .iter()
            .filter(|s| s.total_trades >= min_trades)
            .collect();
        
        valid_strategies.sort_by(|a, b| {
            b.profit_factor
                .partial_cmp(&a.profit_factor)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        valid_strategies
    }
    
    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            total_experiences: self.winners.len() + self.mistakes.len(),
            winners: self.winners.len(),
            mistakes: self.mistakes.len(),
            strategies: self.strategies.len(),
            win_rate: if self.winners.len() + self.mistakes.len() > 0 {
                self.winners.len() as f64 / (self.winners.len() + self.mistakes.len()) as f64
            } else {
                0.0
            },
        }
    }
}

/// Insight for a specific market regime
#[derive(Debug, Clone)]
pub struct RegimeInsight {
    pub regime: MarketRegime,
    pub win_rate: f64,
    pub avg_return: f64,
    pub best_strategies: Vec<String>,
    pub recommendation: String,
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_experiences: usize,
    pub winners: usize,
    pub mistakes: usize,
    pub strategies: usize,
    pub win_rate: f64,
}

impl Default for RagMemory {
    fn default() -> Self {
        Self::new()
    }
}

// Import from parent module
use super::{TradingDecision, Action};
