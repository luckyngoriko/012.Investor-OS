//! Hierarchical Reasoning Model (HRM) - Sprint 39
//!
//! Full neural network with weight loading from Python.

use super::{DeviceConfig, HRMConfig, HRMError, InferenceResult, MarketRegime, Result};
use super::inference::InferenceEngine;
use super::lstm::HRMNetwork;
use super::weights::{ModelWeights, WeightLoader};

use burn::prelude::*;
use burn_ndarray::NdArray;

/// HRM Backend
pub type HRMBackend = NdArray<f32>;

/// Hierarchical Reasoning Model
#[derive(Debug)]
pub struct HRM {
    config: HRMConfig,
    engine: InferenceEngine,
    network: Option<HRMNetwork<HRMBackend>>,
    weights: Option<ModelWeights>,
}

impl HRM {
    /// Create new HRM
    pub fn new(config: &HRMConfig) -> Result<Self> {
        config.validate()?;

        let engine = InferenceEngine::new()
            .with_gpu(matches!(config.device, DeviceConfig::Cuda | DeviceConfig::Auto))
            .with_timeout(5000);

        // Load network if weights provided
        let (network, weights) = if let Some(ref path) = config.weights_path {
            match Self::load_network(path, config) {
                Ok((net, w)) => {
                    println!("✅ Loaded HRM network from: {}", path);
                    (Some(net), Some(w))
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to load weights: {}. Using placeholder.", e);
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        Ok(Self {
            config: config.clone(),
            engine,
            network,
            weights,
        })
    }

    /// Load network with weights
    fn load_network(
        path: &str,
        _config: &HRMConfig,
    ) -> Result<(HRMNetwork<HRMBackend>, ModelWeights)> {
        let loader = WeightLoader::new();
        let weights = loader.load(path)?;
        loader.verify_compatibility(&weights, _config)?;

        let device = <HRMBackend as Backend>::Device::default();
        
        // Create network from loaded weights
        let network = HRMNetwork::from_weights(&weights, &device)
            .map_err(HRMError::WeightLoadError)?;

        Ok((network, weights))
    }

    /// Load weights
    pub fn load_weights<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        let (network, weights) = Self::load_network(
            path.as_ref().to_str().unwrap(),
            &self.config
        )?;
        
        self.network = Some(network);
        self.weights = Some(weights);
        
        Ok(())
    }

    /// Run inference
    pub fn infer(&self, signals: &[f32]) -> Result<InferenceResult> {
        if let Some(ref network) = self.network {
            self.infer_network(network, signals)
        } else {
            self.engine.infer(signals)
        }
    }

    /// Neural network inference
    fn infer_network(
        &self,
        network: &HRMNetwork<HRMBackend>,
        signals: &[f32],
    ) -> Result<InferenceResult> {
        use std::time::Instant;
        
        let start = Instant::now();
        
        if signals.len() != 6 {
            return Err(HRMError::InvalidInputShape {
                expected: 6,
                actual: signals.len(),
            });
        }
        
        // Convert to tensor [1, 6]
        let device = <HRMBackend as Backend>::Device::default();
        let input_vec: Vec<f32> = signals.to_vec();
        let input_1d = Tensor::<HRMBackend, 1>::from_data(input_vec.as_slice(), &device);
        let input: Tensor<HRMBackend, 2> = input_1d.reshape([1, 6]);
        
        // Forward pass
        let output = network.infer(input);
        
        // Extract values
        let output_data = output.to_data();
        let values: Vec<f32> = output_data.to_vec()
            .map_err(|e| HRMError::InferenceError(format!("Tensor error: {:?}", e)))?;
        
        if values.len() < 3 {
            return Err(HRMError::InferenceError("Invalid output size".to_string()));
        }
        
        let elapsed = start.elapsed().as_micros() as u64;
        
        // Output is already sigmoid-activated
        let conviction = values[0];
        let confidence = values[1];
        let regime = classify_regime(values[2]);
        
        Ok(InferenceResult {
            conviction,
            confidence,
            regime,
            latency_us: elapsed,
        })
    }

    /// Batch inference
    pub fn infer_batch(&self, batch: &[Vec<f32>]) -> Result<Vec<InferenceResult>> {
        batch.iter().map(|signals| self.infer(signals)).collect()
    }

    /// Check if ready
    pub fn is_ready(&self) -> bool {
        self.network.is_some()
    }

    /// Get config
    pub fn config(&self) -> &HRMConfig {
        &self.config
    }

    /// Get stats
    pub fn stats(&self) -> HRMStats {
        HRMStats {
            parameters: if let Some(ref w) = self.weights {
                w.parameter_count()
            } else {
                self.config.estimated_parameters()
            },
            weights_loaded: self.weights.is_some(),
            network_loaded: self.network.is_some(),
            input_size: self.config.input_size,
            output_size: self.config.output_size,
            gpu_enabled: cfg!(feature = "cuda"),
        }
    }

    /// Get weights
    pub fn weights(&self) -> Option<&ModelWeights> {
        self.weights.as_ref()
    }
}

/// HRM Stats
#[derive(Debug, Clone)]
pub struct HRMStats {
    pub parameters: usize,
    pub weights_loaded: bool,
    pub network_loaded: bool,
    pub input_size: usize,
    pub output_size: usize,
    pub gpu_enabled: bool,
}

/// Builder
pub struct HRMBuilder {
    config: HRMConfig,
}

impl HRMBuilder {
    pub fn new() -> Self {
        Self {
            config: HRMConfig::default(),
        }
    }

    pub fn with_config(mut self, config: HRMConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_input_size(mut self, size: usize) -> Self {
        self.config.input_size = size;
        self
    }

    pub fn with_hidden_sizes(mut self, high: usize, low: usize) -> Self {
        self.config.high_hidden_size = high;
        self.config.low_hidden_size = low;
        self
    }

    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.config.confidence_threshold = threshold;
        self
    }

    pub fn with_device(mut self, device: DeviceConfig) -> Self {
        self.config.device = device;
        self
    }

    pub fn with_weights<P: Into<String>>(mut self, path: P) -> Self {
        self.config.weights_path = Some(path.into());
        self
    }

    pub fn build(self) -> Result<HRM> {
        HRM::new(&self.config)
    }
}

impl Default for HRMBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn classify_regime(logit: f32) -> MarketRegime {
    match logit {
        x if x < 0.5 => MarketRegime::Bull,
        x if x < 1.5 => MarketRegime::Bear,
        x if x < 2.5 => MarketRegime::Sideways,
        _ => MarketRegime::Crisis,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_regime() {
        assert_eq!(classify_regime(0.0), MarketRegime::Bull);
        assert_eq!(classify_regime(1.0), MarketRegime::Bear);
        assert_eq!(classify_regime(2.0), MarketRegime::Sideways);
        assert_eq!(classify_regime(3.0), MarketRegime::Crisis);
    }
}
