//! Inference Engine for serving predictions

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::{FeatureVector, MlError, Model, Prediction};

/// Inference configuration
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    /// Model cache size
    pub cache_size: usize,
    /// Prediction timeout in milliseconds
    pub timeout_ms: u64,
    /// Enable batch inference
    pub enable_batching: bool,
    /// Batch size
    pub batch_size: usize,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            cache_size: 10,
            timeout_ms: 100,
            enable_batching: true,
            batch_size: 32,
        }
    }
}

/// Batch prediction request/response
#[derive(Debug, Clone)]
pub struct BatchPrediction {
    pub predictions: Vec<Prediction>,
    pub latency_ms: u64,
    pub batch_size: usize,
}

/// Model cache entry
#[derive(Debug, Clone)]
struct ModelCacheEntry {
    model_id: String,
    last_used: DateTime<Utc>,
    use_count: u64,
}

/// Inference Engine
pub struct InferenceEngine {
    config: InferenceConfig,
    /// Loaded models cache
    models: Arc<RwLock<HashMap<String, Box<dyn Model + Send + Sync>>>>,
    /// Model metadata cache
    model_cache: Arc<RwLock<HashMap<String, ModelCacheEntry>>>,
}

impl std::fmt::Debug for InferenceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InferenceEngine")
            .field("config", &self.config)
            .field("model_count", &"<async>")
            .finish()
    }
}

impl InferenceEngine {
    /// Create a new inference engine
    pub fn new(config: InferenceConfig) -> Self {
        Self {
            config,
            models: Arc::new(RwLock::new(HashMap::new())),
            model_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a model with the engine
    pub async fn register_model(&self, model_id: String, model: Box<dyn Model + Send + Sync>) {
        let model_id_for_log = model_id.clone();
        let mut models = self.models.write().await;
        models.insert(model_id.clone(), model);
        
        let mut cache = self.model_cache.write().await;
        cache.insert(model_id.clone(), ModelCacheEntry {
            model_id,
            last_used: Utc::now(),
            use_count: 0,
        });
        
        info!("Registered model: {}", model_id_for_log);
    }

    /// Unregister a model
    pub async fn unregister_model(&self, model_id: &str) {
        let mut models = self.models.write().await;
        models.remove(model_id);
        
        let mut cache = self.model_cache.write().await;
        cache.remove(model_id);
        
        info!("Unregistered model: {}", model_id);
    }

    /// Get prediction from a model
    pub async fn predict(
        &self,
        model_id: &str,
        features: &FeatureVector,
    ) -> Result<Prediction, MlError> {
        let start = std::time::Instant::now();

        // Get model
        let models = self.models.read().await;
        let model = models.get(model_id)
            .ok_or_else(|| MlError::ModelNotFound(model_id.to_string()))?;

        // Make prediction
        let prediction = model.predict(features)?;

        // Update cache stats
        drop(models);
        self.update_model_stats(model_id).await;

        let latency = start.elapsed().as_millis() as u64;
        debug!("Prediction latency: {}ms", latency);

        if latency > self.config.timeout_ms {
            warn!("Prediction timeout: {}ms > {}ms", latency, self.config.timeout_ms);
        }

        Ok(prediction)
    }

    /// Batch prediction
    pub async fn predict_batch(
        &self,
        model_id: &str,
        features: &[FeatureVector],
    ) -> Result<BatchPrediction, MlError> {
        let start = std::time::Instant::now();

        // Get model
        let models = self.models.read().await;
        let model = models.get(model_id)
            .ok_or_else(|| MlError::ModelNotFound(model_id.to_string()))?;

        // Process in batches
        let mut all_predictions = Vec::new();
        
        for chunk in features.chunks(self.config.batch_size) {
            let predictions = model.predict_batch(chunk)?;
            all_predictions.extend(predictions);
        }

        drop(models);
        self.update_model_stats(model_id).await;

        let latency = start.elapsed().as_millis() as u64;
        
        Ok(BatchPrediction {
            predictions: all_predictions,
            latency_ms: latency,
            batch_size: features.len(),
        })
    }

    /// Get model metadata
    pub async fn get_model_info(&self, model_id: &str) -> Result<super::ModelMetadata, MlError> {
        let models = self.models.read().await;
        let model = models.get(model_id)
            .ok_or_else(|| MlError::ModelNotFound(model_id.to_string()))?;
        
        Ok(model.metadata().clone())
    }

    /// List all registered models
    pub async fn list_models(&self) -> Vec<String> {
        let models = self.models.read().await;
        models.keys().cloned().collect()
    }

    /// Get model cache statistics
    pub async fn get_cache_stats(&self) -> HashMap<String, (u64, DateTime<Utc>)> {
        let cache = self.model_cache.read().await;
        cache.iter()
            .map(|(k, v)| (k.clone(), (v.use_count, v.last_used)))
            .collect()
    }

    /// Clear model cache
    pub async fn clear_cache(&self) {
        let mut models = self.models.write().await;
        let mut cache = self.model_cache.write().await;
        models.clear();
        cache.clear();
        info!("Model cache cleared");
    }

    /// Update model usage statistics
    async fn update_model_stats(&self, model_id: &str) {
        let mut cache = self.model_cache.write().await;
        if let Some(entry) = cache.get_mut(model_id) {
            entry.use_count += 1;
            entry.last_used = Utc::now();
        }
    }

    /// Evict least recently used models if cache is full
    pub async fn evict_if_needed(&self) {
        let cache = self.model_cache.read().await;
        if cache.len() <= self.config.cache_size {
            return;
        }

        // Find LRU model
        let to_evict = cache.iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(id, _)| id.clone());

        drop(cache);

        if let Some(id) = to_evict {
            info!("Evicting model from cache: {}", id);
            self.unregister_model(&id).await;
        }
    }
}

/// A/B Testing support
#[derive(Debug, Clone)]
pub struct ABTestConfig {
    pub test_id: String,
    pub model_a_id: String,
    pub model_b_id: String,
    pub traffic_split: f64, // 0.0 - 1.0 (percentage to model B)
}

/// A/B Test manager
#[derive(Debug)]
pub struct ABTestManager {
    config: ABTestConfig,
    engine: Arc<InferenceEngine>,
}

impl ABTestManager {
    pub fn new(config: ABTestConfig, engine: Arc<InferenceEngine>) -> Self {
        Self { config, engine }
    }

    /// Get prediction using A/B test routing
    pub async fn predict(&self, features: &FeatureVector, user_id: Option<&str>) -> Result<(String, Prediction), MlError> {
        // Deterministic routing based on user_id or random
        let use_model_b = match user_id {
            Some(id) => {
                // Hash-based routing for consistency
                let hash = id.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
                (hash as f64 / u32::MAX as f64) < self.config.traffic_split
            }
            None => rand::random::<f64>() < self.config.traffic_split,
        };

        let model_id = if use_model_b {
            &self.config.model_b_id
        } else {
            &self.config.model_a_id
        };

        let prediction = self.engine.predict(model_id, features).await?;
        Ok((model_id.clone(), prediction))
    }
}

/// Feature store for caching computed features
#[derive(Debug, Clone)]
pub struct FeatureStore {
    cache: Arc<RwLock<HashMap<String, FeatureVector>>>,
    ttl_seconds: u64,
}

impl FeatureStore {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl_seconds,
        }
    }

    /// Store features
    pub async fn store(&self, key: String, features: FeatureVector) {
        let mut cache = self.cache.write().await;
        cache.insert(key, features);
    }

    /// Retrieve features
    pub async fn get(&self, key: &str) -> Option<FeatureVector> {
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    /// Clear expired entries
    pub async fn cleanup(&self) {
        // In production, check timestamps and remove expired entries
        // For now, just limit cache size
        let mut cache = self.cache.write().await;
        if cache.len() > 1000 {
            cache.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::model::MockModel;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_inference_engine() {
        let engine = InferenceEngine::new(InferenceConfig::default());
        let model = MockModel::new(Decimal::from(42));
        let model_id = model.metadata().id.clone();

        engine.register_model(model_id.clone(), Box::new(model)).await;

        let features = FeatureVector::new(vec![], vec![]);
        let prediction = engine.predict(&model_id, &features).await.unwrap();

        assert_eq!(prediction.value, Decimal::from(42));
    }

    #[tokio::test]
    async fn test_model_not_found() {
        let engine = InferenceEngine::new(InferenceConfig::default());
        
        let features = FeatureVector::new(vec![], vec![]);
        let result = engine.predict("nonexistent", &features).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_prediction() {
        let engine = InferenceEngine::new(InferenceConfig::default());
        let model = MockModel::new(Decimal::from(1));
        let model_id = model.metadata().id.clone();

        engine.register_model(model_id.clone(), Box::new(model)).await;

        let features: Vec<FeatureVector> = (0..10)
            .map(|_| FeatureVector::new(vec![], vec![]))
            .collect();

        let batch = engine.predict_batch(&model_id, &features).await.unwrap();

        assert_eq!(batch.predictions.len(), 10);
        assert_eq!(batch.batch_size, 10);
    }

    #[tokio::test]
    async fn test_list_models() {
        let engine = InferenceEngine::new(InferenceConfig::default());
        let model = MockModel::new(Decimal::ZERO);
        let model_id = model.metadata().id.clone();

        engine.register_model(model_id.clone(), Box::new(model)).await;

        let models = engine.list_models().await;
        assert!(models.contains(&model_id));
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let engine = InferenceEngine::new(InferenceConfig::default());
        let model = MockModel::new(Decimal::ZERO);
        let model_id = model.metadata().id.clone();

        engine.register_model(model_id.clone(), Box::new(model)).await;

        // Make some predictions
        let features = FeatureVector::new(vec![], vec![]);
        for _ in 0..5 {
            let _ = engine.predict(&model_id, &features).await;
        }

        let stats = engine.get_cache_stats().await;
        let (count, _) = stats.get(&model_id).unwrap();
        assert_eq!(*count, 5);
    }

    #[tokio::test]
    async fn test_feature_store() {
        let store = FeatureStore::new(60);
        
        let features = FeatureVector::new(
            vec![Decimal::ONE, Decimal::from(2)],
            vec!["a".to_string(), "b".to_string()],
        );

        store.store("test_key".to_string(), features.clone()).await;

        let retrieved = store.get("test_key").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 2);
    }
}
