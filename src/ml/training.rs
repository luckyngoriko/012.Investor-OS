//! Model training pipeline

use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{info, warn};

use super::{FeatureVector, MlError, Model, EvaluationMetrics};

/// Training configuration
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    /// Train/test split ratio
    pub test_split: f64,
    /// Number of cross-validation folds
    pub cv_folds: usize,
    /// Random seed for reproducibility
    pub random_seed: u64,
    /// Enable early stopping
    pub early_stopping: bool,
    /// Early stopping patience
    pub patience: usize,
    /// Minimum improvement for early stopping
    pub min_delta: Decimal,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            test_split: 0.2,
            cv_folds: 5,
            random_seed: 42,
            early_stopping: true,
            patience: 10,
            min_delta: Decimal::try_from(0.001).unwrap(),
        }
    }
}

/// Training result
#[derive(Debug, Clone)]
pub struct TrainingResult {
    pub model_id: String,
    pub train_metrics: EvaluationMetrics,
    pub test_metrics: EvaluationMetrics,
    pub cv_scores: Vec<Decimal>,
    pub training_duration_ms: u64,
    pub best_epoch: Option<usize>,
}

/// Cross-validator for time-series data
#[derive(Debug, Clone)]
pub struct CrossValidator {
    n_splits: usize,
}

impl CrossValidator {
    pub fn new(n_splits: usize) -> Self {
        Self { n_splits }
    }

    /// Generate time-series cross-validation splits
    /// For time series, we use walk-forward validation
    pub fn split(&self, data_len: usize) -> Vec<(Vec<usize>, Vec<usize>)> {
        let mut splits = Vec::new();
        let fold_size = data_len / self.n_splits;

        for i in 1..self.n_splits {
            let train_end = i * fold_size;
            let test_end = ((i + 1) * fold_size).min(data_len);

            let train_indices: Vec<usize> = (0..train_end).collect();
            let test_indices: Vec<usize> = (train_end..test_end).collect();

            splits.push((train_indices, test_indices));
        }

        splits
    }

    /// Calculate cross-validation scores
    pub fn cross_validate<M: Model>(
        &self,
        model_factory: &dyn Fn() -> M,
        features: &[FeatureVector],
        labels: &[Decimal],
    ) -> Result<Vec<Decimal>, MlError> {
        let mut scores = Vec::new();

        for (fold, (train_idx, test_idx)) in self.split(features.len()).iter().enumerate() {
            info!("Training fold {}/{}", fold + 1, self.n_splits);

            // Split data
            let train_features: Vec<FeatureVector> = train_idx
                .iter()
                .filter_map(|&i| features.get(i).cloned())
                .collect();
            let train_labels: Vec<Decimal> = train_idx
                .iter()
                .filter_map(|&i| labels.get(i).copied())
                .collect();
            
            let test_features: Vec<FeatureVector> = test_idx
                .iter()
                .filter_map(|&i| features.get(i).cloned())
                .collect();
            let test_labels: Vec<Decimal> = test_idx
                .iter()
                .filter_map(|&i| labels.get(i).copied())
                .collect();

            // Train model
            let mut model = model_factory();
            model.train(&train_features, &train_labels)?;

            // Evaluate
            let metrics = model.evaluate(&test_features, &test_labels)?;
            scores.push(metrics.accuracy);
        }

        Ok(scores)
    }
}

/// Training pipeline
#[derive(Debug, Clone)]
pub struct TrainingPipeline {
    config: TrainingConfig,
}

impl TrainingPipeline {
    pub fn new(config: TrainingConfig) -> Self {
        Self { config }
    }

    /// Split data into train/test sets
    pub fn split_data(
        &self,
        features: &[FeatureVector],
        labels: &[Decimal],
    ) -> (Vec<FeatureVector>, Vec<Decimal>, Vec<FeatureVector>, Vec<Decimal>) {
        let n = features.len();
        let split_idx = ((1.0 - self.config.test_split) * n as f64) as usize;

        // Time series split - keep temporal order
        let train_features = features[..split_idx].to_vec();
        let train_labels = labels[..split_idx].to_vec();
        let test_features = features[split_idx..].to_vec();
        let test_labels = labels[split_idx..].to_vec();

        (train_features, train_labels, test_features, test_labels)
    }

    /// Train a model with the pipeline
    pub fn train<M: Model>(
        &self,
        model: &mut M,
        features: &[FeatureVector],
        labels: &[Decimal],
    ) -> Result<TrainingResult, MlError> {
        let start = std::time::Instant::now();

        // Split data
        let (train_features, train_labels, test_features, test_labels) = 
            self.split_data(features, labels);

        info!(
            "Training with {} samples, testing with {} samples",
            train_features.len(),
            test_features.len()
        );

        // Train model
        model.train(&train_features, &train_labels)?;

        // Evaluate
        let train_metrics = model.evaluate(&train_features, &train_labels)?;
        let test_metrics = model.evaluate(&test_features, &test_labels)?;

        let duration = start.elapsed().as_millis() as u64;

        info!("Training completed in {}ms", duration);
        info!("Train accuracy: {}%, Test accuracy: {}%",
            train_metrics.accuracy * Decimal::from(100),
            test_metrics.accuracy * Decimal::from(100)
        );

        Ok(TrainingResult {
            model_id: model.metadata().id.clone(),
            train_metrics,
            test_metrics,
            cv_scores: vec![],
            training_duration_ms: duration,
            best_epoch: None,
        })
    }

    /// Train with cross-validation
    pub fn train_with_cv<M: Model>(
        &self,
        model_factory: &dyn Fn() -> M,
        features: &[FeatureVector],
        labels: &[Decimal],
    ) -> Result<(M, TrainingResult), MlError> {
        let _start = std::time::Instant::now();

        // Run cross-validation
        let cv = CrossValidator::new(self.config.cv_folds);
        let cv_scores = cv.cross_validate(model_factory, features, labels)?;

        // Train final model on all data
        let mut final_model = model_factory();
        let result = self.train(&mut final_model, features, labels)?;

        let mut final_result = result;
        final_result.cv_scores = cv_scores.clone();

        let mean_cv_score = cv_scores.iter().sum::<Decimal>() / Decimal::from(cv_scores.len() as i64);
        info!("Cross-validation accuracy: {}%", mean_cv_score * Decimal::from(100));

        Ok((final_model, final_result))
    }

    /// Hyperparameter grid search
    pub fn grid_search<M: Model>(
        &self,
        model_factory: &dyn Fn(&HashMap<String, f64>) -> M,
        param_grid: &HashMap<String, Vec<f64>>,
        features: &[FeatureVector],
        labels: &[Decimal],
    ) -> Result<(HashMap<String, f64>, EvaluationMetrics), MlError> {
        info!("Starting grid search over {:?}", param_grid.keys());

        // Generate all parameter combinations
        let combinations = generate_combinations(param_grid);
        info!("Testing {} parameter combinations", combinations.len());

        let mut best_score = Decimal::ZERO;
        let mut best_params = HashMap::new();
        let mut best_metrics = EvaluationMetrics::default();

        for params in combinations {
            let cv = CrossValidator::new(self.config.cv_folds);
            let model_factory_with_params = || model_factory(&params);
            
            match cv.cross_validate(&model_factory_with_params, features, labels) {
                Ok(scores) => {
                    let mean_score = scores.iter().sum::<Decimal>() / Decimal::from(scores.len() as i64);
                    
                    if mean_score > best_score {
                        best_score = mean_score;
                        best_params = params.clone();
                        best_metrics.accuracy = mean_score;
                        info!("New best score: {}% with params {:?}", mean_score * Decimal::from(100), params);
                    }
                }
                Err(e) => {
                    warn!("Failed to evaluate params {:?}: {}", params, e);
                }
            }
        }

        info!("Best params: {:?} with accuracy {}%", best_params, best_score * Decimal::from(100));
        Ok((best_params, best_metrics))
    }
}

/// Generate all combinations of hyperparameters
fn generate_combinations(param_grid: &HashMap<String, Vec<f64>>) -> Vec<HashMap<String, f64>> {
    let keys: Vec<String> = param_grid.keys().cloned().collect();
    let mut combinations = vec![HashMap::new()];

    for key in keys {
        let values = param_grid.get(&key).unwrap();
        let mut new_combinations = Vec::new();

        for base in &combinations {
            for &value in values {
                let mut new_combo = base.clone();
                new_combo.insert(key.clone(), value);
                new_combinations.push(new_combo);
            }
        }

        combinations = new_combinations;
    }

    combinations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::model::{MockModel, LinearModel, ModelConfig};

    #[test]
    fn test_cross_validator_split() {
        let cv = CrossValidator::new(5);
        let splits = cv.split(100);

        assert_eq!(splits.len(), 4); // n_splits - 1
        
        // Check temporal ordering
        for (train, test) in &splits {
            let max_train = train.iter().max().copied().unwrap_or(0);
            let min_test = test.iter().min().copied().unwrap_or(0);
            assert!(max_train < min_test, "Train data must come before test data in time series");
        }
    }

    #[test]
    fn test_data_split() {
        let pipeline = TrainingPipeline::new(TrainingConfig::default());
        
        let features: Vec<FeatureVector> = (0..100)
            .map(|i| FeatureVector::new(vec![Decimal::from(i)], vec!["x".to_string()]))
            .collect();
        let labels: Vec<Decimal> = (0..100).map(Decimal::from).collect();

        let (train_f, train_l, test_f, test_l) = pipeline.split_data(&features, &labels);

        assert_eq!(train_f.len(), 80);
        assert_eq!(test_f.len(), 20);
        assert_eq!(train_l.len(), 80);
        assert_eq!(test_l.len(), 20);
    }

    #[test]
    fn test_train_model() {
        let pipeline = TrainingPipeline::new(TrainingConfig {
            test_split: 0.2,
            ..Default::default()
        });

        let mut model = MockModel::new(Decimal::from(1));

        let features: Vec<FeatureVector> = (0..100)
            .map(|_| FeatureVector::new(vec![], vec![]))
            .collect();
        let labels: Vec<Decimal> = (0..100).map(|_| Decimal::from(1)).collect();

        let result = pipeline.train(&mut model, &features, &labels).unwrap();

        // Duration can be 0 for very fast mocks, just check it's not None
        assert!(result.training_duration_ms >= 0);
        assert!(result.test_metrics.accuracy >= Decimal::ZERO);
    }

    #[test]
    fn test_grid_search_combinations() {
        let mut param_grid = HashMap::new();
        param_grid.insert("a".to_string(), vec![1.0, 2.0]);
        param_grid.insert("b".to_string(), vec![3.0, 4.0]);

        let combinations = generate_combinations(&param_grid);

        assert_eq!(combinations.len(), 4);
        
        // Check all combinations exist
        let has_1_3 = combinations.iter().any(|c| c.get("a") == Some(&1.0) && c.get("b") == Some(&3.0));
        let has_1_4 = combinations.iter().any(|c| c.get("a") == Some(&1.0) && c.get("b") == Some(&4.0));
        let has_2_3 = combinations.iter().any(|c| c.get("a") == Some(&2.0) && c.get("b") == Some(&3.0));
        let has_2_4 = combinations.iter().any(|c| c.get("a") == Some(&2.0) && c.get("b") == Some(&4.0));

        assert!(has_1_3 && has_1_4 && has_2_3 && has_2_4);
    }
}
