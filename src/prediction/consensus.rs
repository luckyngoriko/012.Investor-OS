//! Prediction Consensus Engine (Sprint 124).
//!
//! Combines predictions from multiple ML models into a weighted consensus.
//! Each model's vote is weighted by its confidence and historical accuracy.

use serde::Serialize;

/// A single model's prediction contribution to the consensus.
#[derive(Debug, Clone)]
pub struct ModelPrediction {
    pub model_name: String,
    pub predicted_return: f64,
    pub confidence: f64,
    /// Historical accuracy weight (0-1). Default 1.0 for new models.
    pub accuracy_weight: f64,
}

/// Direction of the consensus prediction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConsensusDirection {
    Long,
    Short,
    Neutral,
}

/// Result of combining predictions from multiple models.
#[derive(Debug, Clone, Serialize)]
pub struct ConsensusResult {
    pub direction: ConsensusDirection,
    pub predicted_return: f64,
    pub confidence: f64,
    pub agreement: f64,
    pub n_models: usize,
    pub model_contributions: Vec<ModelContribution>,
}

/// How much each model contributed to the consensus.
#[derive(Debug, Clone, Serialize)]
pub struct ModelContribution {
    pub model_name: String,
    pub weight: f64,
    pub predicted_return: f64,
    pub agrees_with_consensus: bool,
}

/// Compute weighted consensus from multiple model predictions.
///
/// Each prediction is weighted by `confidence * accuracy_weight`.
/// Agreement measures how many models agree on direction.
pub fn compute_consensus(predictions: &[ModelPrediction]) -> Option<ConsensusResult> {
    if predictions.is_empty() {
        return None;
    }

    // Single model — return directly
    if predictions.len() == 1 {
        let p = &predictions[0];
        let direction = classify_direction(p.predicted_return);
        return Some(ConsensusResult {
            direction,
            predicted_return: p.predicted_return,
            confidence: p.confidence * 0.8, // discount for single model
            agreement: 1.0,
            n_models: 1,
            model_contributions: vec![ModelContribution {
                model_name: p.model_name.clone(),
                weight: 1.0,
                predicted_return: p.predicted_return,
                agrees_with_consensus: true,
            }],
        });
    }

    // Compute weights
    let weights: Vec<f64> = predictions
        .iter()
        .map(|p| p.confidence * p.accuracy_weight)
        .collect();

    let total_weight: f64 = weights.iter().sum();
    if total_weight <= 0.0 {
        return None;
    }

    let norm_weights: Vec<f64> = weights.iter().map(|w| w / total_weight).collect();

    // Weighted average return
    let predicted_return: f64 = predictions
        .iter()
        .zip(norm_weights.iter())
        .map(|(p, &w)| p.predicted_return * w)
        .sum();

    let consensus_direction = classify_direction(predicted_return);

    // Agreement: fraction of models that agree on direction
    let agreeing = predictions
        .iter()
        .filter(|p| classify_direction(p.predicted_return) == consensus_direction)
        .count();
    let agreement = agreeing as f64 / predictions.len() as f64;

    // Weighted average confidence, boosted by agreement
    let base_confidence: f64 = predictions
        .iter()
        .zip(norm_weights.iter())
        .map(|(p, &w)| p.confidence * w)
        .sum();

    // Agreement factor: full agreement (1.0) → boost 1.2x, half (0.5) → reduce 0.8x
    let agreement_factor = 0.6 + 0.6 * agreement;
    let confidence = (base_confidence * agreement_factor).clamp(0.0, 1.0);

    // Model contributions
    let contributions: Vec<ModelContribution> = predictions
        .iter()
        .zip(norm_weights.iter())
        .map(|(p, &w)| ModelContribution {
            model_name: p.model_name.clone(),
            weight: (w * 10000.0).round() / 10000.0,
            predicted_return: p.predicted_return,
            agrees_with_consensus: classify_direction(p.predicted_return) == consensus_direction,
        })
        .collect();

    Some(ConsensusResult {
        direction: consensus_direction,
        predicted_return: (predicted_return * 1_000_000.0).round() / 1_000_000.0,
        confidence: (confidence * 10000.0).round() / 10000.0,
        agreement: (agreement * 10000.0).round() / 10000.0,
        n_models: predictions.len(),
        model_contributions: contributions,
    })
}

/// Convert a predicted return to a direction.
fn classify_direction(predicted_return: f64) -> ConsensusDirection {
    if predicted_return > 0.001 {
        ConsensusDirection::Long
    } else if predicted_return < -0.001 {
        ConsensusDirection::Short
    } else {
        ConsensusDirection::Neutral
    }
}

/// Convert a ConsensusResult to a 0-100 quality score for TickerSignals.
pub fn consensus_to_quality_score(result: &ConsensusResult) -> u8 {
    // Map predicted_return to 0-100:
    // -5% → 0, 0% → 50, +5% → 100
    let return_score = ((result.predicted_return / 0.05) * 50.0 + 50.0).clamp(0.0, 100.0);
    // Weight by confidence and agreement
    let weighted = return_score * result.confidence * result.agreement;
    // Blend: 70% return signal, 30% neutral (50)
    let blended = weighted * 0.7 + 50.0 * 0.3;
    blended.clamp(0.0, 100.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_model_returns_discounted_confidence() {
        let preds = vec![ModelPrediction {
            model_name: "catboost".into(),
            predicted_return: 0.02,
            confidence: 0.8,
            accuracy_weight: 1.0,
        }];
        let result = compute_consensus(&preds).unwrap();
        assert_eq!(result.direction, ConsensusDirection::Long);
        assert_eq!(result.n_models, 1);
        assert!(result.confidence < 0.8); // discounted
    }

    #[test]
    fn two_agreeing_models_boost_confidence() {
        let preds = vec![
            ModelPrediction {
                model_name: "catboost".into(),
                predicted_return: 0.02,
                confidence: 0.8,
                accuracy_weight: 1.0,
            },
            ModelPrediction {
                model_name: "ets".into(),
                predicted_return: 0.015,
                confidence: 0.7,
                accuracy_weight: 0.9,
            },
        ];
        let result = compute_consensus(&preds).unwrap();
        assert_eq!(result.direction, ConsensusDirection::Long);
        assert_eq!(result.agreement, 1.0);
        assert!(result.confidence > 0.7); // boosted by agreement
    }

    #[test]
    fn disagreeing_models_reduce_confidence() {
        let preds = vec![
            ModelPrediction {
                model_name: "catboost".into(),
                predicted_return: 0.02,
                confidence: 0.8,
                accuracy_weight: 1.0,
            },
            ModelPrediction {
                model_name: "garch".into(),
                predicted_return: -0.01,
                confidence: 0.6,
                accuracy_weight: 1.0,
            },
        ];
        let result = compute_consensus(&preds).unwrap();
        assert!(result.agreement < 1.0);
        // Consensus direction follows the higher-weighted model
        assert_eq!(result.direction, ConsensusDirection::Long);
    }

    #[test]
    fn accuracy_weight_affects_contribution() {
        let preds = vec![
            ModelPrediction {
                model_name: "proven".into(),
                predicted_return: 0.03,
                confidence: 0.7,
                accuracy_weight: 1.0, // high accuracy
            },
            ModelPrediction {
                model_name: "new".into(),
                predicted_return: -0.03,
                confidence: 0.9,
                accuracy_weight: 0.2, // low accuracy
            },
        ];
        let result = compute_consensus(&preds).unwrap();
        // Despite "new" having higher confidence, "proven" should dominate
        assert_eq!(result.direction, ConsensusDirection::Long);
    }

    #[test]
    fn empty_predictions_returns_none() {
        assert!(compute_consensus(&[]).is_none());
    }

    #[test]
    fn three_models_full_agreement() {
        let preds = vec![
            ModelPrediction {
                model_name: "a".into(),
                predicted_return: 0.01,
                confidence: 0.8,
                accuracy_weight: 1.0,
            },
            ModelPrediction {
                model_name: "b".into(),
                predicted_return: 0.02,
                confidence: 0.7,
                accuracy_weight: 0.9,
            },
            ModelPrediction {
                model_name: "c".into(),
                predicted_return: 0.015,
                confidence: 0.6,
                accuracy_weight: 0.8,
            },
        ];
        let result = compute_consensus(&preds).unwrap();
        assert_eq!(result.n_models, 3);
        assert_eq!(result.agreement, 1.0);
        assert!(result.predicted_return > 0.01);
        assert_eq!(result.model_contributions.len(), 3);
    }

    #[test]
    fn consensus_to_score_neutral() {
        let result = ConsensusResult {
            direction: ConsensusDirection::Neutral,
            predicted_return: 0.0,
            confidence: 0.5,
            agreement: 1.0,
            n_models: 2,
            model_contributions: vec![],
        };
        let score = consensus_to_quality_score(&result);
        // Neutral return → ~50
        assert!(score >= 30 && score <= 70, "Expected ~50, got {score}");
    }

    #[test]
    fn consensus_to_score_strong_long() {
        let result = ConsensusResult {
            direction: ConsensusDirection::Long,
            predicted_return: 0.04,
            confidence: 0.9,
            agreement: 1.0,
            n_models: 3,
            model_contributions: vec![],
        };
        let score = consensus_to_quality_score(&result);
        assert!(score > 60, "Expected >60, got {score}");
    }
}
