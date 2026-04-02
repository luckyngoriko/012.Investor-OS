//! Price persistence: aggregates ticks into 1-minute OHLCV candles
//! and writes them to the prices TimescaleDB hypertable.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, DurationRound, Utc};
use sqlx::PgPool;
use tracing::{debug, error, info};

use super::TradingSignal;
use crate::nats::{NatsPublisher, PriceCandle};

/// In-progress 1-minute candle being assembled from ticks.
#[derive(Debug, Clone)]
struct CandleBuilder {
    symbol: String,
    minute_start: DateTime<Utc>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    trades: i32,
}

impl CandleBuilder {
    fn new(symbol: &str, price: f64, volume: f64, time: DateTime<Utc>) -> Self {
        let minute_start = time.duration_trunc(Duration::minutes(1)).unwrap_or(time);
        Self {
            symbol: symbol.to_string(),
            minute_start,
            open: price,
            high: price,
            low: price,
            close: price,
            volume,
            trades: 1,
        }
    }

    fn update(&mut self, price: f64, volume: f64) {
        if price > self.high {
            self.high = price;
        }
        if price < self.low {
            self.low = price;
        }
        self.close = price;
        self.volume += volume;
        self.trades += 1;
    }

    fn is_complete(&self, now: DateTime<Utc>) -> bool {
        let candle_end = self.minute_start + Duration::minutes(1);
        now >= candle_end
    }
}

/// Spawn a background task that listens on the signal broadcast channel,
/// aggregates price data into 1-minute candles, and persists to postgres.
///
/// This runs alongside the main tick_processing_loop without modifying it.
pub fn spawn_price_writer(
    pool: PgPool,
    mut signal_rx: tokio::sync::broadcast::Receiver<TradingSignal>,
    nats_publisher: Option<Arc<NatsPublisher>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("Price writer started — aggregating 1m candles to TimescaleDB + NATS");

        let mut candles: HashMap<String, CandleBuilder> = HashMap::new();
        let mut flush_interval = tokio::time::interval(std::time::Duration::from_secs(10));

        loop {
            tokio::select! {
                result = signal_rx.recv() => {
                    match result {
                        Ok(signal) => {
                            let now = Utc::now();
                            let price = signal.cq_score; // cq_score carries notional/price info
                            if price <= 0.0 {
                                continue; // Skip zero-price signals
                            }

                            let symbol = &signal.symbol;

                            // Check if current candle is complete
                            if let Some(candle) = candles.get(symbol) {
                                if candle.is_complete(now) {
                                    // Flush completed candle
                                    let c = candles.remove(symbol).unwrap();
                                    flush_candle(&pool, &c, &nats_publisher).await;
                                }
                            }

                            // Update or create candle
                            if let Some(candle) = candles.get_mut(symbol) {
                                candle.update(price, 0.0);
                            } else {
                                candles.insert(
                                    symbol.to_string(),
                                    CandleBuilder::new(symbol, price, 0.0, now),
                                );
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            debug!("Price writer lagged by {n} signals — catching up");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            info!("Price writer: signal channel closed, flushing remaining candles");
                            for (_, candle) in candles.drain() {
                                flush_candle(&pool, &candle, &nats_publisher).await;
                            }
                            break;
                        }
                    }
                }
                _ = flush_interval.tick() => {
                    // Periodic flush: write completed candles
                    let now = Utc::now();
                    let completed: Vec<String> = candles
                        .iter()
                        .filter(|(_, c)| c.is_complete(now))
                        .map(|(k, _)| k.clone())
                        .collect();

                    for symbol in completed {
                        if let Some(candle) = candles.remove(&symbol) {
                            flush_candle(&pool, &candle, &nats_publisher).await;
                        }
                    }
                }
            }
        }
    })
}

async fn flush_candle(
    pool: &PgPool,
    candle: &CandleBuilder,
    nats_publisher: &Option<Arc<NatsPublisher>>,
) {
    let result = sqlx::query(
        "INSERT INTO prices (time, symbol, open, high, low, close, volume, trades, source)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'binance')
         ON CONFLICT DO NOTHING",
    )
    .bind(candle.minute_start)
    .bind(&candle.symbol)
    .bind(candle.open)
    .bind(candle.high)
    .bind(candle.low)
    .bind(candle.close)
    .bind(candle.volume)
    .bind(candle.trades)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            debug!(
                symbol = %candle.symbol,
                time = %candle.minute_start,
                close = candle.close,
                trades = candle.trades,
                "Flushed 1m candle to prices table"
            );

            // Publish to NATS if available
            if let Some(publisher) = nats_publisher {
                let price_candle = PriceCandle {
                    open: candle.open,
                    high: candle.high,
                    low: candle.low,
                    close: candle.close,
                    volume: candle.volume,
                    trades: candle.trades,
                };
                if let Err(e) = publisher.publish_price(&candle.symbol, price_candle).await {
                    debug!(symbol = %candle.symbol, error = %e, "NATS price publish failed (non-fatal)");
                }
            }
        }
        Err(e) => {
            error!(
                symbol = %candle.symbol,
                error = %e,
                "Failed to flush candle to prices table"
            );
        }
    }
}
