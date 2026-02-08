//! Embedding Generation
//!
//! Generates vector embeddings for text using sentence-transformers models

use crate::rag::{Result, RagError};

/// Embedding generator using local or API-based models
pub struct EmbeddingGenerator {
    model: EmbeddingModel,
    dimension: usize,
}

/// Available embedding models
enum EmbeddingModel {
    /// Local ONNX model (all-MiniLM-L6-v2, 384-dim)
    #[cfg(feature = "local-embeddings")]
    Local(LocalModel),
    /// OpenAI API
    OpenAi { api_key: String, model: String },
    /// Hugging Face Inference API
    HuggingFace { api_token: String, model: String },
    /// Mock for testing
    Mock,
}

#[cfg(feature = "local-embeddings")]
struct LocalModel {
    // Would use ort crate for ONNX inference
    session: ort::Session,
    tokenizer: tokenizers::Tokenizer,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator
    pub async fn new() -> Result<Self> {
        // Try to use local model first, fall back to API
        #[cfg(feature = "local-embeddings")]
        {
            match Self::load_local_model().await {
                Ok(model) => {
                    return Ok(Self {
                        model: EmbeddingModel::Local(model),
                        dimension: 384,
                    });
                }
                Err(e) => {
                    tracing::warn!("Failed to load local model: {}, falling back to mock", e);
                }
            }
        }
        
        // Check for API keys
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            return Ok(Self {
                model: EmbeddingModel::OpenAi {
                    api_key,
                    model: "text-embedding-3-small".to_string(),
                },
                dimension: 1536,
            });
        }
        
        // Mock model for development/testing
        Ok(Self {
            model: EmbeddingModel::Mock,
            dimension: 384,
        })
    }
    
    /// Generate embedding for text
    pub async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        match &self.model {
            #[cfg(feature = "local-embeddings")]
            EmbeddingModel::Local(model) => {
                self.generate_local(text, model).await
            }
            EmbeddingModel::OpenAi { api_key, model } => {
                self.generate_openai(text, api_key, model).await
            }
            EmbeddingModel::HuggingFace { api_token, model } => {
                self.generate_hf(text, api_token, model).await
            }
            EmbeddingModel::Mock => {
                // Generate deterministic mock embedding
                Ok(self.generate_mock(text))
            }
        }
    }
    
    /// Generate embeddings for multiple texts (batch)
    pub async fn generate_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        
        for text in texts {
            embeddings.push(self.generate(text).await?);
        }
        
        Ok(embeddings)
    }
    
    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }
    
    #[cfg(feature = "local-embeddings")]
    async fn load_local_model() -> Result<LocalModel> {
        use ort::GraphOptimizationLevel;
        use tokenizers::Tokenizer;
        
        // Load ONNX model
        let session = ort::Session::builder()
            .map_err(|e| RagError::Embedding(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| RagError::Embedding(e.to_string()))?
            .commit_from_file("models/all-MiniLM-L6-v2.onnx")
            .map_err(|e| RagError::Embedding(format!("Failed to load ONNX model: {}", e)))?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file("models/tokenizer.json")
            .map_err(|e| RagError::Embedding(format!("Failed to load tokenizer: {}", e)))?;
        
        Ok(LocalModel { session, tokenizer })
    }
    
    #[cfg(feature = "local-embeddings")]
    async fn generate_local(&self, text: &str, model: &LocalModel) -> Result<Vec<f32>> {
        use ort::tensor::DynOrtTensor;
        
        // Tokenize
        let encoding = model.tokenizer
            .encode(text, true)
            .map_err(|e| RagError::Embedding(e.to_string()))?;
        
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&m| m as i64).collect();
        
        // Create input tensor
        let input_shape = vec![1, input_ids.len() as i64];
        let inputs = ort::inputs![
            "input_ids" => (input_shape.clone(), input_ids),
            "attention_mask" => (input_shape.clone(), attention_mask),
        ].map_err(|e| RagError::Embedding(e.to_string()))?;
        
        // Run inference
        let outputs: DynOrtTensor<f32, _> = model.session
            .run(inputs)
            .map_err(|e| RagError::Embedding(e.to_string()))?
            .extract_tensor(0)
            .map_err(|e| RagError::Embedding(e.to_string()))?;
        
        // Extract embedding
        let embedding: Vec<f32> = outputs.view().iter().copied().collect();
        
        // Normalize
        Ok(self.normalize(&embedding))
    }
    
    async fn generate_openai(&self, text: &str, api_key: &str, model: &str) -> Result<Vec<f32>> {
        use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
        
        let client = reqwest::Client::new();
        
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| RagError::Embedding(e.to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        let request_body = serde_json::json!({
            "input": text,
            "model": model,
            "encoding_format": "float"
        });
        
        let response = client
            .post("https://api.openai.com/v1/embeddings")
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| RagError::ExternalApi(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RagError::ExternalApi(format!("OpenAI API error: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RagError::ExternalApi(e.to_string()))?;
        
        let embedding = response_json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| RagError::ExternalApi("Invalid response format".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        
        Ok(embedding)
    }
    
    async fn generate_hf(&self, text: &str, api_token: &str, model: &str) -> Result<Vec<f32>> {
        use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
        
        let client = reqwest::Client::new();
        
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_token))
                .map_err(|e| RagError::Embedding(e.to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        let request_body = serde_json::json!({
            "inputs": text,
        });
        
        let url = format!(
            "https://api-inference.huggingface.co/pipeline/feature-extraction/{}",
            model
        );
        
        let response = client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| RagError::ExternalApi(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RagError::ExternalApi(format!("HF API error: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RagError::ExternalApi(e.to_string()))?;
        
        // HF returns array of embeddings, we take the first one
        let embedding: Vec<f32> = response_json[0]
            .as_array()
            .ok_or_else(|| RagError::ExternalApi("Invalid response format".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        
        // Normalize
        Ok(self.normalize(&embedding))
    }
    
    fn generate_mock(&self, text: &str) -> Vec<f32> {
        // Deterministic mock embedding based on text hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Generate 384-dimensional vector from hash
        let mut embedding = Vec::with_capacity(self.dimension);
        let mut seed = hash;
        
        for _ in 0..self.dimension {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let value = ((seed % 1000) as f32 / 1000.0) * 2.0 - 1.0;
            embedding.push(value);
        }
        
        self.normalize(&embedding)
    }
    
    fn normalize(&self, vector: &[f32]) -> Vec<f32> {
        let magnitude: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if magnitude > 0.0 {
            vector.iter().map(|x| x / magnitude).collect()
        } else {
            vector.to_vec()
        }
    }
}

/// Calculate cosine similarity between two embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a > 0.0 && norm_b > 0.0 {
        dot_product / (norm_a * norm_b)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        assert!((cosine_similarity(&a, &c)).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_mock_embedding() {
        let generator = EmbeddingGenerator {
            model: EmbeddingModel::Mock,
            dimension: 384,
        };
        
        let embedding1 = generator.generate("test text").await.unwrap();
        let embedding2 = generator.generate("test text").await.unwrap();
        
        assert_eq!(embedding1.len(), 384);
        assert_eq!(embedding1, embedding2); // Deterministic
        
        // Check normalization
        let magnitude: f32 = embedding1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_normalize() {
        let generator = EmbeddingGenerator {
            model: EmbeddingModel::Mock,
            dimension: 3,
        };
        
        let vector = vec![3.0, 4.0, 0.0];
        let normalized = generator.normalize(&vector);
        
        let magnitude: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001);
    }
}
