//! Free Data Aggregator
//!
//! Aggregates data from multiple free sources:
//! - Cross-validation between sources
//! - Data quality scoring
//! - AI pattern discovery
//! - Comparison with paid sources

use super::{DataSource, FreeMarketData, CrossSourcePrice, DataQualityMetrics};
use chrono::{DateTime, Utc, Duration};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::{HashMap, VecDeque};

/// Aggregates data from multiple free sources
pub struct FreeDataAggregator {
    /// Latest data from each source
    source_data: HashMap<DataSource, HashMap<String, FreeMarketData>>,
    /// Historical data for pattern detection
    history: HashMap<String, VecDeque<PricePoint>>,
    /// Data quality metrics per source
    quality_metrics: HashMap<DataSource, DataQualityMetrics>,
    /// Max history size per symbol
    max_history: usize,
}

#[derive(Debug, Clone)]
struct PricePoint {
    timestamp: DateTime<Utc>,
    source: DataSource,
    price: Decimal,
    volume: Option<Decimal>,
}

impl FreeDataAggregator {
    pub fn new() -> Self {
        Self {
            source_data: HashMap::new(),
            history: HashMap::new(),
            quality_metrics: HashMap::new(),
            max_history: 10000,
        }
    }
    
    /// Add data from a source
    pub fn add_data(&mut self, data: FreeMarketData) {
        let source = data.source;
        let symbol = data.symbol.clone();
        
        // Store latest data
        self.source_data
            .entry(source)
            .or_default()
            .insert(symbol.clone(), data.clone());
        
        // Add to history
        let history = self.history.entry(symbol).or_default();
        history.push_back(PricePoint {
            timestamp: data.timestamp,
            source,
            price: data.price,
            volume: data.volume_24h,
        });
        
        // Trim history
        while history.len() > self.max_history {
            history.pop_front();
        }
        
        // Update quality metrics
        self.update_quality_metrics(source);
    }
    
    /// Get consensus price across all sources
    pub fn get_consensus_price(&self, symbol: &str) -> Option<ConsensusPrice> {
        let mut prices = Vec::new();
        let mut sources = Vec::new();
        
        for (source, data_map) in &self.source_data {
            if let Some(data) = data_map.get(symbol) {
                // Check freshness (within 5 minutes)
                let age = Utc::now() - data.timestamp;
                if age < Duration::minutes(5) {
                    prices.push(data.price);
                    sources.push(*source);
                }
            }
        }
        
        if prices.len() < 2 {
            return None;
        }
        
        // Calculate median
        prices.sort();
        let median = if prices.len() % 2 == 0 {
            let mid = prices.len() / 2;
            (prices[mid - 1] + prices[mid]) / Decimal::from(2)
        } else {
            prices[prices.len() / 2]
        };
        
        // Calculate spread
        let min = prices[0];
        let max = prices[prices.len() - 1];
        let spread = max - min;
        
        // Calculate confidence based on spread
        let spread_pct: f64 = if median.is_zero() {
            0.0
        } else {
            let s: f64 = spread.try_into().unwrap_or(0.0);
            let m: f64 = median.try_into().unwrap_or(1.0);
            (s / m) * 100.0
        };
        
        let confidence = if spread_pct < 0.1 {
            1.0
        } else if spread_pct < 0.5 {
            0.9
        } else if spread_pct < 1.0 {
            0.7
        } else {
            0.5
        };
        
        Some(ConsensusPrice {
            symbol: symbol.to_string(),
            price: median,
            spread,
            spread_pct,
            sources,
            confidence,
            timestamp: Utc::now(),
        })
    }
    
    /// Get cross-source comparison
    pub fn get_cross_source_comparison(&self, symbol: &str) -> Option<CrossSourcePrice> {
        let mut cross = CrossSourcePrice::new(symbol);
        
        for (source, data_map) in &self.source_data {
            if let Some(data) = data_map.get(symbol) {
                cross.add_price(*source, data.price);
            }
        }
        
        if cross.prices.len() < 2 {
            return None;
        }
        
        cross.calculate_consensus();
        Some(cross)
    }
    
    /// Detect patterns from free data
    pub fn detect_patterns(&self, symbol: &str) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();
        
        let history = match self.history.get(symbol) {
            Some(h) if h.len() >= 10 => h,
            _ => return patterns,
        };
        
        // Pattern 1: Price discrepancy between sources
        if let Some(cross) = self.get_cross_source_comparison(symbol) {
            if cross.spread_pct > 0.5 {
                patterns.push(DetectedPattern {
                    pattern_type: PatternType::PriceDiscrepancy,
                    symbol: symbol.to_string(),
                    confidence: (cross.spread_pct / 5.0).min(1.0) as f32,
                    description: format!(
                        "Price discrepancy: {:.2}% spread across sources, outliers: {:?}",
                        cross.spread_pct, cross.outlier_sources
                    ),
                    timestamp: Utc::now(),
                });
            }
        }
        
        // Pattern 2: Volume spike
        let recent_volume: Decimal = history.iter().rev().take(10)
            .filter_map(|p| p.volume)
            .sum();
        let older_volume: Decimal = history.iter().rev().skip(10).take(10)
            .filter_map(|p| p.volume)
            .sum();
        
        if !older_volume.is_zero() {
            let volume_ratio: f64 = (recent_volume / older_volume).try_into().unwrap_or(1.0);
            if volume_ratio > 2.0 {
                patterns.push(DetectedPattern {
                    pattern_type: PatternType::VolumeSpike,
                    symbol: symbol.to_string(),
                    confidence: ((volume_ratio - 1.0) / 3.0).min(1.0) as f32,
                    description: format!("Volume spike: {:.1}x normal", volume_ratio),
                    timestamp: Utc::now(),
                });
            }
        }
        
        // Pattern 3: Price momentum
        let recent: Vec<_> = history.iter().rev().take(10).collect();
        let older: Vec<_> = history.iter().rev().skip(10).take(10).collect();
        
        let recent_sum: Decimal = recent.iter().map(|p| p.price).sum();
        let older_sum: Decimal = older.iter().map(|p| p.price).sum();
        
        if !recent.is_empty() && !older.is_empty() {
            let recent_avg = recent_sum / Decimal::from(recent.len() as i64);
            let older_avg = older_sum / Decimal::from(older.len() as i64);
            
            if !older_avg.is_zero() {
                let change_pct: f64 = ((recent_avg - older_avg) / older_avg).try_into().unwrap_or(0.0);
                if change_pct.abs() > 0.02 {
                    patterns.push(DetectedPattern {
                        pattern_type: PatternType::MomentumShift,
                        symbol: symbol.to_string(),
                        confidence: (change_pct.abs() / 0.05).min(1.0) as f32,
                        description: format!("Momentum shift: {:.2}%", change_pct * 100.0),
                        timestamp: Utc::now(),
                    });
                }
            }
        }
        
        patterns
    }
    
    /// Compare free data with paid source (for quality validation)
    pub fn validate_against_paid(&mut self, symbol: &str, paid_price: Decimal, _paid_source: &str) -> ValidationResult {
        let consensus = match self.get_consensus_price(symbol) {
            Some(c) => c,
            None => return ValidationResult::no_data(),
        };
        
        let deviation = (consensus.price - paid_price).abs();
        let deviation_pct: f64 = if paid_price.is_zero() {
            0.0
        } else {
            (deviation / paid_price).try_into().unwrap_or(0.0)
        };
        
        // Update quality metrics for each source
        for source in &consensus.sources {
            if let Some(data_map) = self.source_data.get(source) {
                if let Some(data) = data_map.get(symbol) {
                    let source_deviation = (data.price - paid_price).abs();
                    let source_dev_pct: f64 = if paid_price.is_zero() {
                        0.0
                    } else {
                        (source_deviation / paid_price).try_into().unwrap_or(0.0)
                    };
                    
                    let metrics = self.quality_metrics.entry(*source).or_default();
                    metrics.sample_count += 1;
                    // Exponential moving average of accuracy
                    let alpha = 0.1;
                    metrics.accuracy_vs_paid = 
                        metrics.accuracy_vs_paid * (1.0 - alpha) + 
                        (1.0 - source_dev_pct as f32) * alpha;
                }
            }
        }
        
        ValidationResult {
            symbol: symbol.to_string(),
            free_consensus: consensus.price,
            paid_price,
            deviation,
            deviation_pct,
            is_accurate: deviation_pct < 0.001, // < 0.1%
            source_count: consensus.sources.len(),
            timestamp: Utc::now(),
        }
    }
    
    /// Get data quality report
    pub fn get_quality_report(&self) -> Vec<SourceQualityReport> {
        self.quality_metrics
            .iter()
            .map(|(source, metrics)| SourceQualityReport {
                source: *source,
                accuracy_vs_paid: metrics.accuracy_vs_paid,
                sample_count: metrics.sample_count,
                reliability_score: metrics.accuracy_vs_paid * 
                    (1.0 - metrics.error_rate),
            })
            .collect()
    }
    
    fn update_quality_metrics(&mut self, source: DataSource) {
        let metrics = self.quality_metrics.entry(source).or_default();
        metrics.sample_count += 1;
    }
}

impl Default for FreeDataAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Consensus price across sources
#[derive(Debug, Clone)]
pub struct ConsensusPrice {
    pub symbol: String,
    pub price: Decimal,
    pub spread: Decimal,
    pub spread_pct: f64,
    pub sources: Vec<DataSource>,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

/// Detected pattern from data analysis
#[derive(Debug, Clone)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub symbol: String,
    pub confidence: f32,
    pub description: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum PatternType {
    PriceDiscrepancy,
    VolumeSpike,
    MomentumShift,
    CorrelationBreakdown,
    ArbitrageOpportunity,
}

/// Validation against paid data
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub symbol: String,
    pub free_consensus: Decimal,
    pub paid_price: Decimal,
    pub deviation: Decimal,
    pub deviation_pct: f64,
    pub is_accurate: bool,
    pub source_count: usize,
    pub timestamp: DateTime<Utc>,
}

impl ValidationResult {
    fn no_data() -> Self {
        Self {
            symbol: String::new(),
            free_consensus: Decimal::ZERO,
            paid_price: Decimal::ZERO,
            deviation: Decimal::ZERO,
            deviation_pct: 0.0,
            is_accurate: false,
            source_count: 0,
            timestamp: Utc::now(),
        }
    }
}

/// Quality report for a data source
#[derive(Debug, Clone)]
pub struct SourceQualityReport {
    pub source: DataSource,
    pub accuracy_vs_paid: f32,
    pub sample_count: u64,
    pub reliability_score: f32,
}

/// Data source quality ranking
pub struct DataSourceQuality;

impl DataSourceQuality {
    /// Rank sources by reliability
    pub fn rank_sources(reports: &[SourceQualityReport]) -> Vec<&SourceQualityReport> {
        let mut sorted: Vec<_> = reports.iter().collect();
        sorted.sort_by(|a, b| {
            b.reliability_score.partial_cmp(&a.reliability_score).unwrap()
        });
        sorted
    }
}

/// Cross-source validation
pub struct CrossSourceValidation;

impl CrossSourceValidation {
    /// Validate if sources agree within threshold
    pub fn validate_consensus(cross: &CrossSourcePrice, max_spread_pct: f64) -> bool {
        cross.spread_pct <= max_spread_pct
    }
    
    /// Find arbitrage opportunities between sources
    pub fn find_arbitrage(cross: &CrossSourcePrice, min_profit_pct: f64) -> Option<ArbitrageOp> {
        if cross.prices.len() < 2 {
            return None;
        }
        
        let mut min_price = Decimal::MAX;
        let mut max_price = Decimal::ZERO;
        let mut min_source = None;
        let mut max_source = None;
        
        for (source, price) in &cross.prices {
            if *price < min_price {
                min_price = *price;
                min_source = Some(*source);
            }
            if *price > max_price {
                max_price = *price;
                max_source = Some(*source);
            }
        }
        
        if min_price.is_zero() {
            return None;
        }
        
        let diff = max_price - min_price;
        let profit_pct: f64 = (diff / min_price).to_f64().unwrap_or(0.0) * 100.0;
        
        if profit_pct >= min_profit_pct {
            Some(ArbitrageOp {
                symbol: cross.symbol.clone(),
                buy_source: min_source?,
                sell_source: max_source?,
                buy_price: min_price,
                sell_price: max_price,
                profit_pct,
            })
        } else {
            None
        }
    }
}

/// Arbitrage opportunity between free sources
#[derive(Debug, Clone)]
pub struct ArbitrageOp {
    pub symbol: String,
    pub buy_source: DataSource,
    pub sell_source: DataSource,
    pub buy_price: Decimal,
    pub sell_price: Decimal,
    pub profit_pct: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_data(symbol: &str, source: DataSource, price: f64) -> FreeMarketData {
        FreeMarketData {
            symbol: symbol.to_string(),
            source,
            price: Decimal::try_from(price).unwrap(),
            change_24h: None,
            volume_24h: None,
            timestamp: Utc::now(),
            bid: None,
            ask: None,
        }
    }

    #[test]
    fn test_consensus_price() {
        let mut agg = FreeDataAggregator::new();
        
        agg.add_data(create_test_data("AAPL", DataSource::YahooFinance, 150.0));
        agg.add_data(create_test_data("AAPL", DataSource::AlphaVantage, 150.5));
        agg.add_data(create_test_data("AAPL", DataSource::Finnhub, 149.8));
        
        let consensus = agg.get_consensus_price("AAPL").unwrap();
        
        assert!(consensus.price > Decimal::ZERO);
        assert_eq!(consensus.sources.len(), 3);
        assert!(consensus.confidence > 0.0);
    }

    #[test]
    fn test_pattern_detection() {
        let mut agg = FreeDataAggregator::new();
        
        // Add historical data for pattern detection
        for i in 0..30 {
            let price = 150.0 + (i as f64 * 0.1);
            let volume = if i > 20 { 1000000.0 } else { 100000.0 }; // Volume spike
            
            let mut data = create_test_data("AAPL", DataSource::YahooFinance, price);
            data.volume_24h = Some(Decimal::try_from(volume).unwrap());
            agg.add_data(data);
        }
        
        let patterns = agg.detect_patterns("AAPL");
        
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_arbitrage_detection() {
        let mut cross = CrossSourcePrice::new("BTCUSD");
        
        // 2% price difference
        cross.add_price(DataSource::BinancePublic, Decimal::try_from(50000.0).unwrap());
        cross.add_price(DataSource::CryptoCompare, Decimal::try_from(51000.0).unwrap());
        cross.calculate_consensus();
        
        let arb = CrossSourceValidation::find_arbitrage(&cross, 0.5);
        
        assert!(arb.is_some(), "Should detect arbitrage with 2% spread");
        let arb = arb.unwrap();
        assert!(arb.profit_pct >= 0.5, "Profit should be at least 0.5%");
    }

    #[test]
    fn test_validation_against_paid() {
        let mut agg = FreeDataAggregator::new();
        
        agg.add_data(create_test_data("AAPL", DataSource::YahooFinance, 150.0));
        agg.add_data(create_test_data("AAPL", DataSource::AlphaVantage, 150.02));
        agg.add_data(create_test_data("AAPL", DataSource::Finnhub, 149.98));
        
        let validation = agg.validate_against_paid("AAPL", Decimal::try_from(150.0).unwrap(), "Bloomberg");
        
        assert!(validation.is_accurate || validation.deviation_pct < 0.01);
    }
}
