//! Google Gemini API Client
//! 
//! Free tier: 1000 requests/day
//! https://ai.google.dev/

use super::LLMError;
use serde_json::json;

#[derive(Debug)]
pub struct GeminiClient {
    api_key: String,
    client: reqwest::Client,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

impl GeminiClient {
    pub async fn generate(&self, prompt: &str) -> Result<String, LLMError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent?key={}",
            self.api_key
        );
        
        let body = json!({
            "contents": [{"parts": [{"text": prompt}]}],
            "generationConfig": {
                "temperature": 0.3,
                "maxOutputTokens": 1000
            }
        });
        
        let response = self.client.post(&url).json(&body).send().await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        let result: serde_json::Value = response.json().await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        result["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| LLMError::ApiError("Invalid response".to_string()))
    }
    
    pub async fn generate_json<T: serde::de::DeserializeOwned>(&self, prompt: &str) -> Result<T, LLMError> {
        let text = self.generate(prompt).await?;
        serde_json::from_str(&text).map_err(|e| LLMError::ParseError(e.to_string()))
    }
}

/// Analyze SEC filing using Gemini
pub async fn analyze_sec_filing(text: &str, api_key: &str) -> Result<SECFilingAnalysis, LLMError> {
    let client = GeminiClient::new(api_key.to_string());
    
    let prompt = format!(
        "Analyze this SEC filing and extract key insights:\n\n{}\n\n\
        Provide analysis in this JSON format:\n\
        {{\n\
          \"sentiment\": \"positive|negative|neutral\",\n\
          \"risk_factors\": [\"risk1\", \"risk2\"],\n\
          \"key_developments\": [\"dev1\", \"dev2\"],\n\
          \"management_quality\": 1-10,\n\
          \"red_flags\": [\"flag1\"]\n\
        }}",
        &text[..text.len().min(10000)] // Limit text length
    );
    
    client.generate_json(&prompt).await
}

#[derive(Debug, serde::Deserialize)]
pub struct SECFilingAnalysis {
    pub sentiment: String,
    pub risk_factors: Vec<String>,
    pub key_developments: Vec<String>,
    pub management_quality: u8,
    pub red_flags: Vec<String>,
}
