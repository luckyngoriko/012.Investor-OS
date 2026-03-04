//! HRM Weight Loading - Sprint 39
//!
//! Loading trained weights from SafeTensors into burn network.

use super::{HRMConfig, HRMError, Result};
use burn::prelude::*;
use burn::tensor::backend::Backend;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Weight loader
pub struct WeightLoader;

impl WeightLoader {
    pub fn new() -> Self {
        Self
    }

    /// Load weights from file
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<ModelWeights> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(HRMError::WeightLoadError(format!(
                "Weight file not found: {}",
                path.display()
            )));
        }

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "safetensors" => self.load_safetensors(path),
            "json" => self.load_json(path),
            _ => Err(HRMError::WeightLoadError(format!(
                "Unsupported format: {}. Use .safetensors",
                extension
            ))),
        }
    }

    /// Load SafeTensors format
    fn load_safetensors<P: AsRef<Path>>(&self, path: P) -> Result<ModelWeights> {
        use safetensors::SafeTensors;

        let path = path.as_ref();
        let data = fs::read(path)
            .map_err(|e| HRMError::WeightLoadError(format!("Failed to read file: {}", e)))?;

        let tensors = SafeTensors::deserialize(&data).map_err(|e| {
            HRMError::WeightLoadError(format!("Failed to parse SafeTensors: {}", e))
        })?;

        let mut weights = HashMap::new();

        for name in tensors.names() {
            let view = tensors.tensor(name).map_err(|e| {
                HRMError::WeightLoadError(format!("Failed to get tensor {}: {}", name, e))
            })?;

            let shape: Vec<usize> = view.shape().to_vec();

            // Convert f32 data
            let data: Vec<f32> = match view.dtype() {
                safetensors::Dtype::F32 => {
                    let bytes = view.data();
                    bytes
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                        .collect()
                }
                _ => continue, // Skip non-f32 tensors
            };

            weights.insert(name.to_string(), TensorData { shape, data });
        }

        let metadata = fs::metadata(path)
            .map_err(|e| HRMError::WeightLoadError(format!("Failed to read metadata: {}", e)))?;

        Ok(ModelWeights {
            format: WeightFormat::SafeTensors,
            file_size: metadata.len(),
            tensors: weights,
        })
    }

    /// Load JSON format
    fn load_json<P: AsRef<Path>>(&self, path: P) -> Result<ModelWeights> {
        let content = fs::read_to_string(&path)
            .map_err(|e| HRMError::WeightLoadError(format!("Failed to read JSON: {}", e)))?;

        let weights: JsonWeights = serde_json::from_str(&content)
            .map_err(|e| HRMError::WeightLoadError(format!("Failed to parse JSON: {}", e)))?;

        Ok(ModelWeights {
            format: WeightFormat::Json,
            file_size: content.len() as u64,
            tensors: weights.tensors,
        })
    }

    /// Verify compatibility
    pub fn verify_compatibility(&self, weights: &ModelWeights, _config: &HRMConfig) -> Result<()> {
        // Check required tensors exist
        let required = [
            "fc1.weight",
            "fc1.bias",
            "fc2.weight",
            "fc2.bias",
            "fc3.weight",
            "fc3.bias",
        ];

        for tensor_name in &required {
            if !weights.tensors.contains_key(*tensor_name) {
                return Err(HRMError::WeightLoadError(format!(
                    "Missing required tensor: {}",
                    tensor_name
                )));
            }
        }

        Ok(())
    }
}

impl Default for WeightLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Model weights container
#[derive(Debug, Clone)]
pub struct ModelWeights {
    pub format: WeightFormat,
    pub file_size: u64,
    pub tensors: HashMap<String, TensorData>,
}

impl ModelWeights {
    /// Get tensor by name
    pub fn get(&self, name: &str) -> Option<&TensorData> {
        self.tensors.get(name)
    }

    /// Get tensor as 1D burn tensor
    pub fn get_tensor_1d<B: Backend>(
        &self,
        name: &str,
        device: &B::Device,
    ) -> Option<Tensor<B, 1>> {
        let data = self.tensors.get(name)?;
        let tensor: Tensor<B, 1> = Tensor::from_data(data.data.as_slice(), device);
        Some(tensor.reshape([data.data.len()]))
    }

    /// Get tensor as 2D burn tensor
    pub fn get_tensor_2d<B: Backend>(
        &self,
        name: &str,
        device: &B::Device,
    ) -> Option<Tensor<B, 2>> {
        let data = self.tensors.get(name)?;
        if data.shape.len() != 2 {
            return None;
        }
        let tensor: Tensor<B, 1> = Tensor::from_data(data.data.as_slice(), device);
        Some(tensor.reshape([data.shape[0], data.shape[1]]))
    }

    /// Total parameter count
    pub fn parameter_count(&self) -> usize {
        self.tensors.values().map(|t| t.data.len()).sum()
    }

    /// List tensor names
    pub fn tensor_names(&self) -> Vec<&String> {
        self.tensors.keys().collect()
    }
}

/// Weight format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightFormat {
    SafeTensors,
    Json,
}

/// Tensor data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorData {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

/// JSON format
#[derive(Debug, Serialize, Deserialize)]
struct JsonWeights {
    pub version: String,
    pub format: String,
    pub tensors: HashMap<String, TensorData>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_nonexistent() {
        let loader = WeightLoader::new();
        let result = loader.load("/nonexistent/model.safetensors");
        assert!(matches!(result, Err(HRMError::WeightLoadError(_))));
    }

    #[test]
    fn test_load_json() {
        let mut temp = NamedTempFile::with_suffix(".json").unwrap();
        let weights = JsonWeights {
            version: "1.0".to_string(),
            format: "hrm".to_string(),
            tensors: {
                let mut m = HashMap::new();
                m.insert(
                    "w".to_string(),
                    TensorData {
                        shape: vec![2, 3],
                        data: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
                    },
                );
                m
            },
        };
        temp.write_all(serde_json::to_string(&weights).unwrap().as_bytes())
            .unwrap();

        let loader = WeightLoader::new();
        let result = loader.load(temp.path()).unwrap();

        assert_eq!(result.format, WeightFormat::Json);
        assert_eq!(result.parameter_count(), 6);
    }

    #[test]
    #[ignore = "Requires trained model file"]
    fn test_load_safetensors_real() {
        let loader = WeightLoader::new();
        let result = loader.load("models/hrm_synthetic_v1.safetensors");

        if let Ok(weights) = result {
            assert_eq!(weights.format, WeightFormat::SafeTensors);
            println!("Loaded {} parameters", weights.parameter_count());
            println!("Tensors: {:?}", weights.tensor_names());

            // Verify required tensors exist
            assert!(weights.tensors.contains_key("fc1.weight"));
            assert!(weights.tensors.contains_key("fc1.bias"));
            assert!(weights.tensors.contains_key("fc2.weight"));
            assert!(weights.tensors.contains_key("fc2.bias"));
            assert!(weights.tensors.contains_key("fc3.weight"));
            assert!(weights.tensors.contains_key("fc3.bias"));
        }
    }
}
