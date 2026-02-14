//! Model types and traits

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{FeatureVector, MlError, Result};

/// Model types supported
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum ModelType {
    Linear,
    Logistic,
    #[default]
    RandomForest,
    GradientBoosting,
    NeuralNetwork,
    Ensemble,
}


/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_type: ModelType,
    pub input_features: usize,
    pub output_classes: usize, // 1 for regression, >1 for classification
    pub hyperparameters: HashMap<String, f64>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_type: ModelType::RandomForest,
            input_features: 20,
            output_classes: 2, // Binary classification (up/down)
            hyperparameters: HashMap::new(),
        }
    }
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub model_type: ModelType,
    pub created_at: DateTime<Utc>,
    pub training_samples: usize,
    pub validation_accuracy: Decimal,
    pub feature_names: Vec<String>,
    pub description: Option<String>,
}

impl ModelMetadata {
    pub fn new(id: String, name: String, model_type: ModelType) -> Self {
        Self {
            id,
            name,
            version: "1.0.0".to_string(),
            model_type,
            created_at: Utc::now(),
            training_samples: 0,
            validation_accuracy: Decimal::ZERO,
            feature_names: vec![],
            description: None,
        }
    }
}

/// Model trait for all ML models
pub trait Model: Send + Sync {
    /// Get model metadata
    fn metadata(&self) -> &ModelMetadata;
    
    /// Train the model
    fn train(&mut self, features: &[FeatureVector], labels: &[Decimal]) -> Result<()>;
    
    /// Make a prediction
    fn predict(&self, features: &FeatureVector) -> Result<Prediction>;
    
    /// Batch prediction
    fn predict_batch(&self, features: &[FeatureVector]) -> Result<Vec<Prediction>> {
        features.iter().map(|f| self.predict(f)).collect()
    }
    
    /// Evaluate model performance
    fn evaluate(&self, features: &[FeatureVector], labels: &[Decimal]) -> Result<EvaluationMetrics>;
    
    /// Get feature importance (if available)
    fn feature_importance(&self) -> Option<HashMap<String, Decimal>> {
        None
    }
    
    /// Serialize model to bytes
    fn serialize(&self) -> Result<Vec<u8>>;
    
    /// Deserialize model from bytes
    fn deserialize(bytes: &[u8]) -> Result<Self> where Self: Sized;
}

/// Prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub value: Decimal,
    pub confidence: Decimal, // 0.0 - 1.0
    pub probabilities: Option<Vec<Decimal>>, // For classification
    pub timestamp: DateTime<Utc>,
}

impl Prediction {
    pub fn new(value: Decimal, confidence: Decimal) -> Self {
        Self {
            value,
            confidence,
            probabilities: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_probabilities(mut self, probs: Vec<Decimal>) -> Self {
        self.probabilities = Some(probs);
        self
    }

    /// Get predicted class for classification
    pub fn predicted_class(&self) -> usize {
        if let Some(ref probs) = self.probabilities {
            probs.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(idx, _)| idx)
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Check if prediction is confident enough
    pub fn is_confident(&self, threshold: Decimal) -> bool {
        self.confidence >= threshold
    }
}

/// Evaluation metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvaluationMetrics {
    pub accuracy: Decimal,
    pub precision: Decimal,
    pub recall: Decimal,
    pub f1_score: Decimal,
    pub mse: Decimal, // Mean Squared Error for regression
    pub mae: Decimal, // Mean Absolute Error
    pub rmse: Decimal,
    pub r2: Decimal,
}

/// Simple linear model implementation
#[derive(Debug, Clone)]
pub struct LinearModel {
    metadata: ModelMetadata,
    weights: Vec<Decimal>,
    bias: Decimal,
    config: ModelConfig,
}

impl LinearModel {
    pub fn new(config: ModelConfig) -> Self {
        let metadata = ModelMetadata::new(
            uuid::Uuid::new_v4().to_string(),
            "LinearModel".to_string(),
            ModelType::Linear,
        );

        Self {
            metadata,
            weights: vec![Decimal::ZERO; config.input_features],
            bias: Decimal::ZERO,
            config,
        }
    }

    /// Simple gradient descent training
    fn fit(&mut self, features: &[FeatureVector], labels: &[Decimal], epochs: usize, lr: Decimal) {
        for _ in 0..epochs {
            for (feat_vec, label) in features.iter().zip(labels.iter()) {
                let prediction = self.forward(feat_vec);
                let error = prediction - *label;

                // Update weights
                for (i, weight) in self.weights.iter_mut().enumerate() {
                    if let Some(val) = feat_vec.values.get(i) {
                        *weight -= lr * error * *val;
                    }
                }
                self.bias -= lr * error;
            }
        }
    }

    fn forward(&self, features: &FeatureVector) -> Decimal {
        let mut sum = self.bias;
        for (i, weight) in self.weights.iter().enumerate() {
            if let Some(val) = features.values.get(i) {
                sum += weight * *val;
            }
        }
        sum
    }
}

impl Model for LinearModel {
    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }

    fn train(&mut self, features: &[FeatureVector], labels: &[Decimal]) -> Result<()> {
        if features.is_empty() || labels.is_empty() {
            return Err(MlError::TrainingError("Empty training data".to_string()));
        }

        if features.len() != labels.len() {
            return Err(MlError::TrainingError(
                format!("Feature/label mismatch: {} vs {}", features.len(), labels.len())
            ));
        }

        let lr = Decimal::try_from(self.config.hyperparameters.get("learning_rate").copied().unwrap_or(0.01)).unwrap();
        let epochs = self.config.hyperparameters.get("epochs").copied().unwrap_or(100.0) as usize;

        self.fit(features, labels, epochs, lr);
        
        self.metadata.training_samples = features.len();
        
        Ok(())
    }

    fn predict(&self, features: &FeatureVector) -> Result<Prediction> {
        if features.len() != self.config.input_features {
            return Err(MlError::InvalidFeatureVector {
                expected: self.config.input_features,
                actual: features.len(),
            });
        }

        let value = self.forward(features);
        // Simple confidence based on magnitude
        let confidence = Decimal::ONE.min(value.abs() + Decimal::try_from(0.5).unwrap());

        Ok(Prediction::new(value, confidence))
    }

    fn evaluate(&self, features: &[FeatureVector], labels: &[Decimal]) -> Result<EvaluationMetrics> {
        let mut mse = Decimal::ZERO;
        let mut mae = Decimal::ZERO;

        for (feat, label) in features.iter().zip(labels.iter()) {
            let pred = self.forward(feat);
            let error = pred - *label;
            mse += error * error;
            mae += error.abs();
        }

        let n = Decimal::from(features.len() as i64);
        mse /= n;
        mae /= n;
        let rmse = approx_sqrt(mse);

        Ok(EvaluationMetrics {
            mse,
            mae,
            rmse,
            ..Default::default()
        })
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        // Simplified serialization
        let json = serde_json::to_string(&(self.weights.clone(), self.bias))
            .map_err(|e| MlError::InferenceError(e.to_string()))?;
        Ok(json.into_bytes())
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        let (weights, bias): (Vec<Decimal>, Decimal) = serde_json::from_slice(bytes)
            .map_err(|e| MlError::InferenceError(e.to_string()))?;

        let config = ModelConfig::default();
        let metadata = ModelMetadata::new(
            uuid::Uuid::new_v4().to_string(),
            "LinearModel".to_string(),
            ModelType::Linear,
        );

        Ok(Self {
            metadata,
            weights,
            bias,
            config,
        })
    }
}

/// Mock model for testing
#[derive(Debug, Clone)]
pub struct MockModel {
    metadata: ModelMetadata,
    config: ModelConfig,
    fixed_prediction: Decimal,
}

impl MockModel {
    pub fn new(fixed_prediction: Decimal) -> Self {
        let metadata = ModelMetadata::new(
            uuid::Uuid::new_v4().to_string(),
            "MockModel".to_string(),
            ModelType::Linear,
        );

        Self {
            metadata,
            config: ModelConfig::default(),
            fixed_prediction,
        }
    }
}

impl Model for MockModel {
    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }

    fn train(&mut self, _features: &[FeatureVector], _labels: &[Decimal]) -> Result<()> {
        self.metadata.training_samples = 100;
        self.metadata.validation_accuracy = Decimal::try_from(0.85).unwrap();
        Ok(())
    }

    fn predict(&self, _features: &FeatureVector) -> Result<Prediction> {
        Ok(Prediction::new(self.fixed_prediction, Decimal::try_from(0.9).unwrap()))
    }

    fn evaluate(&self, _features: &[FeatureVector], _labels: &[Decimal]) -> Result<EvaluationMetrics> {
        Ok(EvaluationMetrics {
            accuracy: Decimal::try_from(0.85).unwrap(),
            ..Default::default()
        })
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    fn deserialize(_bytes: &[u8]) -> Result<Self> {
        Ok(Self::new(Decimal::ZERO))
    }
}

fn approx_sqrt(value: Decimal) -> Decimal {
    if value.is_zero() {
        return Decimal::ZERO;
    }

    let mut low = Decimal::ZERO;
    let mut high = value.max(Decimal::ONE);
    let epsilon = Decimal::try_from(0.0001).unwrap();

    if value < Decimal::ONE {
        low = value;
        high = Decimal::ONE;
    }

    for _ in 0..50 {
        let mid = (low + high) / Decimal::from(2);
        let mid_sq = mid * mid;

        if (mid_sq - value).abs() < epsilon {
            return mid;
        }

        if mid_sq < value {
            low = mid;
        } else {
            high = mid;
        }
    }

    (low + high) / Decimal::from(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_creation() {
        let pred = Prediction::new(Decimal::from(100), Decimal::try_from(0.95).unwrap());
        assert_eq!(pred.value, Decimal::from(100));
        assert!(pred.is_confident(Decimal::try_from(0.9).unwrap()));
    }

    #[test]
    fn test_prediction_with_probabilities() {
        let probs = vec![Decimal::try_from(0.2).unwrap(), Decimal::try_from(0.8).unwrap()];
        let pred = Prediction::new(Decimal::ONE, Decimal::try_from(0.8).unwrap())
            .with_probabilities(probs);
        
        assert_eq!(pred.predicted_class(), 1);
    }

    #[test]
    fn test_linear_model_prediction() {
        let config = ModelConfig {
            input_features: 3,
            ..Default::default()
        };
        let model = LinearModel::new(config);
        
        let features = FeatureVector::new(
            vec![Decimal::ONE, Decimal::from(2), Decimal::from(3)],
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
        
        let pred = model.predict(&features).unwrap();
        assert!(pred.confidence >= Decimal::ZERO);
    }

    #[test]
    fn test_linear_model_train() {
        let config = ModelConfig {
            input_features: 2,
            hyperparameters: {
                let mut h = HashMap::new();
                h.insert("learning_rate".to_string(), 0.01);
                h.insert("epochs".to_string(), 10.0);
                h
            },
            ..Default::default()
        };
        let mut model = LinearModel::new(config);
        
        // Simple linear relationship: y = 2x1 + 3x2
        let features: Vec<FeatureVector> = (0..10)
            .map(|i| {
                let x1 = Decimal::from(i);
                let x2 = Decimal::from(i + 1);
                FeatureVector::new(
                    vec![x1, x2],
                    vec!["x1".to_string(), "x2".to_string()],
                )
            })
            .collect();
        
        let labels: Vec<Decimal> = (0..10)
            .map(|i| Decimal::from(2 * i + 3 * (i + 1)))
            .collect();
        
        model.train(&features, &labels).unwrap();
        
        assert_eq!(model.metadata().training_samples, 10);
    }

    #[test]
    fn test_mock_model() {
        let model = MockModel::new(Decimal::from(42));
        
        let features = FeatureVector::new(vec![], vec![]);
        let pred = model.predict(&features).unwrap();
        
        assert_eq!(pred.value, Decimal::from(42));
    }

    #[test]
    fn test_evaluation_metrics() {
        let metrics = EvaluationMetrics {
            accuracy: Decimal::try_from(0.85).unwrap(),
            precision: Decimal::try_from(0.82).unwrap(),
            recall: Decimal::try_from(0.88).unwrap(),
            f1_score: Decimal::try_from(0.85).unwrap(),
            ..Default::default()
        };
        
        assert!(metrics.accuracy > Decimal::ZERO);
        assert!(metrics.precision > Decimal::ZERO);
    }
}
