//! HuggingFace API Client
//!
//! Free tier: Rate limited
//! Models: FinBERT, BERT, etc.
//! https://huggingface.co/inference-api

use super::LLMError;


#[derive(Debug)]
pub struct HFClient {
    token: String,
    client: reqwest::Client,
}

impl HFClient {
    pub fn new(token: String) -> Self {
        Self {
            token,
            client: reqwest::Client::new(),
        }
    }
    
    /// FinBERT sentiment analysis
    pub async fn finbert_sentiment(&self, text: &str) -> Result<SentimentResult, LLMError> {
        let url = "https://api-inference.huggingface.co/models/ProsusAI/finbert";
        
        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({"inputs": text}))
            .send()
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        let result: Vec<Vec<serde_json::Value>> = response.json().await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        // Parse sentiment scores
        let mut positive = 0.0;
        let mut negative = 0.0;
        let mut neutral = 0.0;
        
        if let Some(scores) = result.first() {
            for score in scores {
                if let Some(label) = score["label"].as_str() {
                    if let Some(val) = score["score"].as_f64() {
                        match label {
                            "positive" => positive = val,
                            "negative" => negative = val,
                            "neutral" => neutral = val,
                            _ => {}
                        }
                    }
                }
            }
        }
        
        Ok(SentimentResult {
            positive,
            negative,
            neutral,
            overall: if positive > negative { "positive" } else { "negative" }.to_string(),
        })
    }
}


impl HFClient {
    pub async fn generate(&self, _prompt: &str) -> Result<String, LLMError> {
        // HuggingFace Inference API is mainly for models, not text generation
        Err(LLMError::ApiError("Use specific model methods".to_string()))
    }
    
}

#[derive(Debug)]
pub struct SentimentResult {
    pub positive: f64,
    pub negative: f64,
    pub neutral: f64,
    pub overall: String,
}

/// Batch sentiment analysis for multiple texts
pub async fn batch_sentiment(texts: &[String], token: &str) -> Vec<Result<SentimentResult, LLMError>> {
    let client = HFClient::new(token.to_string());
    let mut results = vec![];
    
    for text in texts {
        results.push(client.finbert_sentiment(text).await);
    }
    
    results
}
