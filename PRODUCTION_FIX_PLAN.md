# 🛠️ ПЛАН ЗА ПОПРАВКА - Production Ready

**Цел**: Премахване на ВСИЧКИ фалшиви имплементации, dummy-та и TODO-та  
**Методология**: Истински интеграции или пълно премахване на неработещ код  
**Времева рамка**: 4 седмици (приоритизирано)

---

## 🚨 Phase 1: Treasury Модул (Седмица 1)

### 1.1 Изтриване на FiatGateway (Нереалистично за MVP)

**Причина**: Банкови интеграции изискват:
- Лиценз за платежна институция (6-12 месеца)
- Banking partner (SWIFT, SEPA)
- KYC/AML инфраструктура
- PCI DSS compliance

**Действие**:
```bash
# Изтрий файлове
rm src/treasury/fiat.rs
rm src/treasury/fx.rs  # Зависи от fiat
```

**Заместване в src/treasury/mod.rs**:
```rust
// Премахни FiatGateway от публичния API
// Остави само CryptoCustody за крипто-native trading
```

---

### 1.2 Имплементация на CryptoCustody (Fireblocks/Copper Integration)

**Реална интеграция с Fireblocks SDK**:

```rust
// src/treasury/fireblocks.rs (НОВ ФАЙЛ)
use fireblocks_sdk::{FireblocksClient, TransferPeerPath, TransactionStatus};

pub struct FireblocksCustody {
    client: FireblocksClient,
    vault_account_id: String,
}

impl FireblocksCustody {
    pub async fn new(api_key: String, api_secret: String, vault_account_id: String) -> Result<Self> {
        let client = FireblocksClient::new(api_key, api_secret)
            .await
            .map_err(|e| TreasuryError::ConnectionFailed(e.to_string()))?;
            
        Ok(Self { client, vault_account_id })
    }
    
    pub async fn get_deposit_address(&self, asset: &str) -> Result<String> {
        // Реално извикване към Fireblocks API
        let address = self.client
            .get_deposit_address(&self.vault_account_id, asset)
            .await
            .map_err(|e| TreasuryError::ApiError(e.to_string()))?;
            
        Ok(address.address)
    }
    
    pub async fn create_transaction(
        &self,
        asset: &str,
        amount: Decimal,
        destination: &str,
    ) -> Result<String> {
        // Реално извикване за теглене
        let tx = self.client
            .create_transaction(
                asset,
                amount.to_string(),
                TransferPeerPath::VaultAccount { id: self.vault_account_id.clone() },
                TransferPeerPath::ExternalWallet { id: destination.to_string() },
            )
            .await
            .map_err(|e| TreasuryError::TransactionFailed(e.to_string()))?;
            
        Ok(tx.id)
    }
}
```

**Или ако нямаме Fireblocks акаунт** - премахни целия treasury модул:
```rust
// src/treasury/mod.rs
// Премахни всичко - използвай само Paper Trading за демо
```

---

## 🔴 Phase 2: API Handlers (Седмица 1-2)

### 2.1 Analytics API - Реална Интеграция

**Текущ проблем** (`src/api/handlers/analytics.rs:45`):
```rust
// ХАРДКОДНАТА ЗАГЛУШКА - ИЗТРИЙ ТОВА
let response = BacktestResponse {
    total_return: Decimal::from(15) / Decimal::from(100), // ФАЛШИВО
    ...
};
```

**Реална имплементация**:
```rust
use crate::analytics::backtest::BacktestEngine;
use crate::analytics::risk::RiskAnalyzer;

pub async fn run_backtest(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BacktestRequest>,
) -> Result<Json<ApiResponse<BacktestResponse>>, StatusCode> {
    // Реален backtest със съществуващия engine
    let engine = BacktestEngine::new(state.market_data.clone());
    
    let result = engine.run(
        &req.strategy,
        &req.tickers,
        req.start_date,
        req.end_date,
        req.initial_capital,
        req.commission_rate.unwrap_or(Decimal::try_from(0.001).unwrap()),
    ).await.map_err(|e| {
        tracing::error!("Backtest failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let response = BacktestResponse {
        total_return: result.total_return,
        annualized_return: result.annualized_return,
        sharpe_ratio: result.sharpe_ratio,
        max_drawdown: result.max_drawdown,
        total_trades: result.trades.len(),
        win_rate: result.win_rate,
    };
    
    Ok(Json(ApiResponse::success(response)))
}
```

### 2.2 Risk Metrics API - Реална Интеграция

```rust
pub async fn get_risk_metrics(
    State(state): State<Arc<AppState>>,
    Query(req): Query<RiskMetricsRequest>,
) -> Result<Json<ApiResponse<RiskMetricsResponse>>, StatusCode> {
    let portfolio = state.portfolio_service
        .get_portfolio(&req.portfolio_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    let analyzer = RiskAnalyzer::new();
    let metrics = analyzer.calculate_metrics(
        &portfolio.positions,
        req.lookback_days.unwrap_or(252) as usize,
    ).map_err(|e| {
        tracing::error!("Risk calculation failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let response = RiskMetricsResponse {
        var_95: metrics.var_95,
        var_99: metrics.var_99,
        sharpe_ratio: metrics.sharpe_ratio,
        sortino_ratio: metrics.sortino_ratio,
        max_drawdown: metrics.max_drawdown,
        volatility: metrics.volatility,
    };
    
    Ok(Json(ApiResponse::success(response)))
}
```

### 2.3 ML Prediction API - Реална Интеграция

```rust
use crate::ml::inference::InferenceEngine;
use crate::ml::features::FeatureEngine;

pub async fn get_ml_prediction(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MLPredictionRequest>,
) -> Result<Json<ApiResponse<MLPredictionResponse>>, StatusCode> {
    // Зареди реален модел от диск
    let model = state.ml_model_cache
        .get_model(&format!("{}/latest.onnx", req.ticker))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    
    // Вземи реални feature-и
    let price_history = state.market_data
        .get_price_history(&req.ticker, 252)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    let feature_engine = FeatureEngine::default();
    let features = feature_engine.extract_features(&price_history)
        .map_err(|e| {
            tracing::error!("Feature extraction failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Реално предсказание
    let prediction = model.predict(&features)
        .map_err(|e| {
            tracing::error!("Prediction failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let response = MLPredictionResponse {
        ticker: req.ticker,
        predicted_cq: prediction.value.try_into().unwrap_or(0.0),
        confidence: prediction.confidence.try_into().unwrap_or(0.0),
        should_trade: prediction.value > Decimal::from(70),
        feature_importance: model.get_feature_importance(),
    };
    
    Ok(Json(ApiResponse::success(response)))
}
```

---

## 🧠 Phase 3: ML Integration (Седмица 2-3)

### 3.1 Зареждане на ONNX Модели

```rust
// src/ml/model_loader.rs (НОВ ФАЙЛ)
use ort::{Environment, Session, Value};
use std::sync::Arc;

pub struct OnnxModel {
    session: Session,
    input_shape: Vec<i64>,
}

impl OnnxModel {
    pub fn load(path: &str) -> Result<Self, MlError> {
        let environment = Arc::new(
            Environment::builder()
                .with_name("InvestorOS")
                .build()
                .map_err(|e| MlError::ModelLoadFailed(e.to_string()))?
        );
        
        let session = Session::builder()
            .map_err(|e| MlError::ModelLoadFailed(e.to_string()))?
            .with_model_from_file(path)
            .map_err(|e| MlError::ModelLoadFailed(e.to_string()))?;
            
        let input_shape = session.inputs[0].dimensions().map(|d| d.unwrap() as i64).collect();
        
        Ok(Self { session, input_shape })
    }
    
    pub fn predict(&self, features: &[f32]) -> Result<f32, MlError> {
        let input = Value::from_array(
            self.session.allocator(),
            ndarray::Array::from_vec(features.to_vec()).into_dyn(),
        ).map_err(|e| MlError::InferenceFailed(e.to_string()))?;
        
        let outputs = self.session.run(vec![input])
            .map_err(|e| MlError::InferenceFailed(e.to_string()))?;
            
        let output = outputs[0].try_extract::<f32>()
            .map_err(|e| MlError::InferenceFailed(e.to_string()))?;
            
        Ok(output[0])
    }
}
```

### 3.2 Интеграция с InferenceEngine

```rust
// src/ml/inference.rs - Премахни mock

pub struct InferenceEngine {
    model_cache: Arc<RwLock<HashMap<String, OnnxModel>>>,
    feature_engine: FeatureEngine,
}

impl InferenceEngine {
    pub async fn predict(&self, symbol: &str, data: &[PriceData]) -> Result<Prediction, MlError> {
        // Зареди модел от кеш или диск
        let model = self.get_or_load_model(symbol).await?;
        
        // Извлечи feature-и
        let features = self.feature_engine.extract_features(data)?;
        let feature_vec = features.to_vec(); // Конвертирай към Vec<f32>
        
        // Реално предсказание с ONNX
        let raw_prediction = model.predict(&feature_vec)?;
        
        // Изчисли confidence от model uncertainty
        let confidence = self.calculate_confidence(&model, &feature_vec)?;
        
        Ok(Prediction {
            value: Decimal::try_from(raw_prediction).unwrap_or(Decimal::ZERO),
            confidence: Decimal::try_from(confidence).unwrap_or(Decimal::ZERO),
            timestamp: Utc::now(),
        })
    }
    
    async fn get_or_load_model(&self, symbol: &str) -> Result<OnnxModel, MlError> {
        // Провери кеш
        if let Some(model) = self.model_cache.read().await.get(symbol) {
            return Ok(model.clone());
        }
        
        // Зареди от диск
        let path = format!("./models/{}/model.onnx", symbol);
        let model = OnnxModel::load(&path)?;
        
        // Сложи в кеш
        self.model_cache.write().await.insert(symbol.to_string(), model.clone());
        
        Ok(model)
    }
}
```

---

## 🔧 Phase 4: unwrap() Рефакторинг (Седмица 3-4)

### 4.1 Систематично премахване

**Правило**: Всеки `unwrap()` се заменя с `?` или `match`.

**Пример** (`src/risk/portfolio_risk.rs:31`):
```rust
// ПРЕДИ:
let return_pct = (price - prev_price) / prev_price.unwrap(); // Може да паникне!

// СЛЕД:
let return_pct = if prev_price == Decimal::ZERO {
    Decimal::ZERO
} else {
    (price - prev_price) / prev_price
};
```

**Автоматизирана проверка**:
```bash
# Добави в CI
rust-clippy -- -D clippy::unwrap_used -D clippy::expect_used
```

---

## 📊 Phase 5: RAG Embeddings (Седмица 4)

### 5.1 Истинска Embeddings Интеграция

**Опция A**: OpenAI API (Платена, но работи веднага)
```rust
// src/rag/embeddings/openai.rs
use reqwest::Client;

pub struct OpenAiEmbedder {
    client: Client,
    api_key: String,
}

impl OpenAiEmbedder {
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, RagError> {
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "input": text,
                "model": "text-embedding-3-small"
            }))
            .send()
            .await
            .map_err(|e| RagError::ApiError(e.to_string()))?;
            
        let embedding = response.json::<EmbeddingResponse>()
            .await
            .map_err(|e| RagError::ParseError(e.to_string()))?;
            
        Ok(embedding.data[0].embedding)
    }
}
```

**Опция B**: Локален ONNX модел (Безплатна, но изисква ресурси)
```rust
// Зареди all-MiniLM-L6-v2.onnx
```

---

## 📋 ПРИОРИТИЗАЦИЯ

| Приоритет | Задача | Време | Блокер |
|-----------|--------|-------|--------|
| P0 | Изтрий FiatGateway | 1 ден | Необходим лиценз |
| P0 | API Handlers интеграция | 3 дни | - |
| P1 | ML ONNX зареждане | 5 дни | Нужни ML модели |
| P1 | unwrap() рефакторинг | 5 дни | - |
| P2 | RAG Embeddings | 3 дни | API ключ или ONNX |
| P2 | CryptoCustody (Fireblocks) | 5 дни | Fireblocks акаунт |

---

## 🎯 КРИТЕРИЙ ЗА ГОТОВНОСТ

След поправките трябва да няма:
- [ ] `// TODO:` в production код
- [ ] `unwrap()` извън тестове
- [ ] Хардкоднати стойности в API handlers
- [ ] `Mock` или `Simulate` в имената на функции
- [ ] `unimplemented!()` или `todo!()`

---

## 💰 БЮДЖЕТ (Ако трябва да плащаме за услуги)

| Услуга | Цена/месец | Необходимост |
|--------|------------|--------------|
| Fireblocks | $500+ | Само за реални пари |
| OpenAI API | $20-100 | За embeddings |
| Pinecone | $70+ | За векторно търсене |
| **Общо** | **$600+** | За production |

**Алтернатива**: Paper trading mode (безплатно) без Treasury модул.

---

**План създаден**: 2026-02-11  
**Изпълнител**: Старш Rust Developer  
**Преглед**: След всяка Phase
