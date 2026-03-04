//! HRM Configuration
//!
//! Defines hyperparameters and architectural constants for the
//! Hierarchical Reasoning Model.

use serde::{Deserialize, Serialize};

/// HRM Model Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HRMConfig {
    /// Input feature dimension (number of trading signals)
    pub input_size: usize,

    /// High-level LSTM hidden size (abstract planning)
    pub high_hidden_size: usize,

    /// Low-level LSTM hidden size (detailed execution)
    pub low_hidden_size: usize,

    /// Output dimension (cq_score, confidence, regime)
    pub output_size: usize,

    /// Number of LSTM layers in each module
    pub num_layers: usize,

    /// Dropout rate for regularization
    pub dropout: f32,

    /// Minimum confidence threshold for using HRM output
    /// Below this, falls back to static CQ calculation
    pub confidence_threshold: f32,

    /// Device preference: "cpu", "cuda", or "auto"
    pub device: DeviceConfig,

    /// Path to pre-trained weights file
    pub weights_path: Option<String>,
}

/// Device configuration for HRM inference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceConfig {
    /// Use CPU only
    Cpu,
    /// Use CUDA GPU if available
    Cuda,
    /// Use Metal GPU if available (macOS)
    Metal,
    /// Auto-detect best available device
    Auto,
}

impl Default for HRMConfig {
    fn default() -> Self {
        Self {
            input_size: 6,
            high_hidden_size: 128,
            low_hidden_size: 64,
            output_size: 3,
            num_layers: 1,
            dropout: 0.1,
            confidence_threshold: 0.7,
            device: DeviceConfig::Auto,
            weights_path: None,
        }
    }
}

impl HRMConfig {
    /// Create a new configuration with custom hidden sizes
    pub fn new(
        input_size: usize,
        high_hidden: usize,
        low_hidden: usize,
        output_size: usize,
    ) -> Self {
        Self {
            input_size,
            high_hidden_size: high_hidden,
            low_hidden_size: low_hidden,
            output_size,
            ..Default::default()
        }
    }

    /// Set the confidence threshold
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set the device configuration
    pub fn with_device(mut self, device: DeviceConfig) -> Self {
        self.device = device;
        self
    }

    /// Set the weights file path
    pub fn with_weights_path(mut self, path: impl Into<String>) -> Self {
        self.weights_path = Some(path.into());
        self
    }

    /// Validate configuration parameters
    pub fn validate(&self) -> crate::hrm::Result<()> {
        if self.input_size == 0 {
            return Err(crate::hrm::HRMError::ConfigError(
                "input_size must be > 0".to_string(),
            ));
        }
        if self.high_hidden_size == 0 {
            return Err(crate::hrm::HRMError::ConfigError(
                "high_hidden_size must be > 0".to_string(),
            ));
        }
        if self.low_hidden_size == 0 {
            return Err(crate::hrm::HRMError::ConfigError(
                "low_hidden_size must be > 0".to_string(),
            ));
        }
        if self.output_size == 0 {
            return Err(crate::hrm::HRMError::ConfigError(
                "output_size must be > 0".to_string(),
            ));
        }
        Ok(())
    }

    /// Total number of parameters (approximate)
    pub fn estimated_parameters(&self) -> usize {
        // LSTM parameters: 4 * (input_size * hidden_size + hidden_size * hidden_size + hidden_size)
        let high_lstm = 4
            * (self.input_size * self.high_hidden_size
                + self.high_hidden_size * self.high_hidden_size
                + self.high_hidden_size);

        let low_lstm = 4
            * (self.low_hidden_size * self.low_hidden_size
                + self.low_hidden_size * self.low_hidden_size
                + self.low_hidden_size);

        // Cross-connections
        let high_to_low = self.high_hidden_size * self.low_hidden_size + self.low_hidden_size;
        let low_to_high = self.low_hidden_size * self.high_hidden_size + self.high_hidden_size;

        // Output layer
        let output =
            (self.high_hidden_size + self.low_hidden_size) * self.output_size + self.output_size;

        high_lstm + low_lstm + high_to_low + low_to_high + output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HRMConfig::default();
        assert_eq!(config.input_size, 6);
        assert_eq!(config.high_hidden_size, 128);
        assert_eq!(config.low_hidden_size, 64);
        assert_eq!(config.output_size, 3);
        assert_eq!(config.confidence_threshold, 0.7);
        assert!(matches!(config.device, DeviceConfig::Auto));
    }

    #[test]
    fn test_custom_config() {
        let config = HRMConfig::new(10, 256, 128, 5);
        assert_eq!(config.input_size, 10);
        assert_eq!(config.high_hidden_size, 256);
        assert_eq!(config.low_hidden_size, 128);
        assert_eq!(config.output_size, 5);
    }

    #[test]
    fn test_config_validation() {
        let valid = HRMConfig::default();
        assert!(valid.validate().is_ok());

        let invalid = HRMConfig {
            input_size: 0,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_estimated_parameters() {
        let config = HRMConfig::default();
        let params = config.estimated_parameters();
        // Should be around ~100k parameters for default config
        assert!(params > 50000);
        assert!(params < 200000);
    }

    #[test]
    fn test_confidence_threshold_clamping() {
        let config = HRMConfig::default().with_confidence_threshold(1.5); // > 1.0
        assert_eq!(config.confidence_threshold, 1.0);

        let config = HRMConfig::default().with_confidence_threshold(-0.5); // < 0.0
        assert_eq!(config.confidence_threshold, 0.0);
    }
}
