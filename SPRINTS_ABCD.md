# Sprint Опции A/B/C/D - Пълен Обзор

## 🚀 Опция A: REST API (Sprint 43)

### Какво се прави:
HTTP endpoint за HRM inference, който frontend и други services могат да използват.

### Имплементация:
```rust
// src/api/handlers/hrm.rs
use axum::{
    extract::Json,
    http::StatusCode,
    response::Json as AxumJson,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct HRMInferenceRequest {
    pub pegy: f32,
    pub insider: f32,
    pub sentiment: f32,
    pub vix: f32,
    pub regime: f32,
    pub time: f32,
}

#[derive(Debug, Serialize)]
pub struct HRMInferenceResponse {
    pub conviction: f32,
    pub confidence: f32,
    pub regime: String,
    pub should_trade: bool,
    pub recommended_strategy: String,
    pub latency_ms: f64,
}

pub async fn hrm_infer(
    State(engine): State<Arc<StrategySelectorEngine>>,
    Json(request): Json<HRMInferenceRequest>,
) -> Result<AxumJson<HRMInferenceResponse>, StatusCode> {
    let start = Instant::now();
    
    let signals = HRMInputSignals::new(
        request.pegy,
        request.insider,
        request.sentiment,
        request.vix,
        request.regime,
        request.time,
    );
    
    let result = engine.calculate_conviction(&signals);
    
    Ok(AxumJson(HRMInferenceResponse {
        conviction: result.conviction,
        confidence: result.confidence,
        regime: format!("{:?}", result.regime),
        should_trade: result.should_trade(0.7),
        recommended_strategy: select_strategy(&result.regime),
        latency_ms: start.elapsed().as_secs_f64() * 1000.0,
    }))
}
```

### Ползи:
- ✅ Frontend dashboard може да показва real-time conviction
- ✅ Други services могат да използват HRM
- ✅ Лесно тестване през curl/Postman

### Време: ~2-3 часа

---

## ⚡ Опция B: Real-time Market Data Pipeline (Sprint 44)

### Какво се прави:
WebSocket връзка към биржи → HRM анализ → Автоматични търговски решения

### Имплементация:
```rust
// src/streaming/hrm_pipeline.rs
pub struct HRMTradingPipeline {
    engine: Arc<StrategySelectorEngine>,
    broker: Arc<dyn Broker>,
    running: AtomicBool,
}

impl HRMTradingPipeline {
    pub async fn run(&self, symbol: &str) -> Result<()> {
        let mut stream = self.connect_market_data(symbol).await?;
        
        while self.running.load(Ordering::Relaxed) {
            if let Some(tick) = stream.next().await {
                // Convert tick to HRM signals
                let signals = self.adapt_tick_to_hrm(&tick);
                
                // Get ML conviction
                let conviction = self.engine.calculate_conviction(&signals);
                
                info!(
                    "{}: conviction={:.4}, confidence={:.4}, regime={:?}",
                    symbol, conviction.conviction, conviction.confidence, conviction.regime
                );
                
                // Trading decision
                if conviction.should_trade(0.7) && conviction.confidence > 0.8 {
                    let order = self.create_order(&conviction, &tick);
                    self.broker.execute(order).await?;
                    
                    info!("🚀 EXECUTED TRADE: {} @ {:.2}", symbol, tick.price);
                }
            }
        }
        
        Ok(())
    }
    
    fn adapt_tick_to_hrm(&self, tick: &MarketTick) -> HRMInputSignals {
        HRMInputSignals::new(
            0.5, // PEGY (would come from fundamentals API)
            tick.volume / 1000000.0, // Insider proxy from volume
            self.calculate_sentiment(tick), // Sentiment from price action
            self.calculate_vix_proxy(tick), // Volatility proxy
            self.detect_regime(tick),
            self.time_of_day(),
        )
    }
}
```

### Ползи:
- ✅ Напълно автоматизирана търговия
- ✅ Real-time ML-based decisions
- ✅ Не се налага ръчна интервенция

### Време: ~4-6 часа

---

## 📊 Опция C: Performance Benchmarks (Sprint 45)

### Какво се прави:
Измерване на latency, throughput, memory usage за HRM

### Имплементация:
```rust
// benches/hrm_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn hrm_inference_benchmark(c: &mut Criterion) {
    let engine = StrategySelectorEngine::new()
        .with_hrm_weights("models/hrm_synthetic_v1.safetensors")
        .unwrap();
    
    let signals = HRMInputSignals::new(0.8, 0.9, 0.7, 15.0, 0.0, 0.5);
    
    // Single inference latency
    c.bench_function("hrm_single_inference", |b| {
        b.iter(|| {
            black_box(engine.calculate_conviction(&signals))
        })
    });
    
    // Batch throughput
    c.bench_function("hrm_batch_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(engine.calculate_conviction(&signals));
            }
        })
    });
}

criterion_group!(benches, hrm_inference_benchmark);
criterion_main!(benches);
```

### Очаквани резултати:
```
Running benches/hrm_benchmark.rs
hrm_single_inference   time:   [290 µs 295 µs 301 µs]
hrm_batch_1000         time:   [285 ms 290 ms 295 ms]
                        thrpt:  [3.4K inferences/sec]

Memory: ~2MB model size
Latency p50: 0.3ms
Latency p99: 0.5ms
```

### Ползи:
- ✅ Доказателство че HRM е production-ready
- ✅ Можем да оптимизираме ако е бавно
- ✅ SLAs за latency

### Време: ~2-3 часа

---

## 🔄 Опция D: Model Versioning & A/B Testing (Sprint 46)

### Какво се прави:
Система за управление на множество модели и A/B тестване

### Имплементация:
```rust
// src/hrm/model_registry.rs
pub struct HRMRegistry {
    models: HashMap<String, Arc<HRM>>,
    active_model: String,
    ab_test_config: Option<ABTestConfig>,
}

pub struct ABTestConfig {
    pub model_a: String,
    pub model_b: String,
    pub split_ratio: f32, // 0.5 = 50/50
}

impl HRMRegistry {
    pub fn load_model(&mut self, name: &str, path: &str) -> Result<()> {
        let hrm = HRMBuilder::new().with_weights(path).build()?;
        self.models.insert(name.to_string(), Arc::new(hrm));
        Ok(())
    }
    
    pub fn select_model(&self, user_id: Option<&str>) -> Arc<HRM> {
        if let Some(config) = &self.ab_test_config {
            // Deterministic split based on user_id
            let bucket = user_id.map(|id| {
                let hash = fxhash::hash64(id.as_bytes());
                (hash % 100) as f32 / 100.0
            }).unwrap_or(0.5);
            
            if bucket < config.split_ratio {
                self.models.get(&config.model_a).cloned()
            } else {
                self.models.get(&config.model_b).cloned()
            }.unwrap_or_else(|| self.get_default())
        } else {
            self.get_default()
        }
    }
    
    // Compare model performance
    pub fn compare_models(&self, model_a: &str, model_b: &str) -> ModelComparison {
        // Run golden dataset on both
        // Return accuracy, latency, confidence metrics
    }
}
```

### Ползи:
- ✅ Можем да тестваме нови модели без риск
- ✅ Постепенно въвеждане на ML (50% heuristic / 50% neural)
- ✅ Rollback ако нов модел е по-лош

### Време: ~4-5 часа

---

## 🎯 Препоръка

### Ако искаш бърз win → **A (REST API)**
- Frontend веднага може да показва HRM данни
- Лесно за демонстрация

### Ако искаш full automation → **B (Real-time Pipeline)**
- Напълно автоматизирана система
- По-сложно, но мощно

### Ако искаш production confidence → **C (Benchmarks)**
- Доказателство че всичко работи бързо
- Важно преди live trading

### Ако планираш бъдещи подобрения → **D (A/B Testing)**
- Foundation за continuous improvement
- Позволява safe experimentation

---

## 💡 Моята Препоръка

**Започни с A (REST API)** защото:
1. Бързо се имплементира (2-3 часа)
2. Веднага виждаш резултати в browser
3. Подготвя основа за B (real-time)
4. Можеш да покажеш на инвеститори/екип

После B → C → D по приоритет.

---

**Кое избираш? (A/B/C/D или комбинация?)**