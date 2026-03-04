//! OpenAI API Client
//!
//! GPT-4o-mini: $0.15/M input, $0.60/M output tokens
//! https://openai.com/pricing

use super::LLMError;

use serde_json::json;

#[derive(Debug)]
pub struct OpenAIClient {
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

impl OpenAIClient {
    pub async fn generate(&self, prompt: &str) -> Result<String, LLMError> {
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": "gpt-4o-mini",
                "messages": [{"role": "user", "content": prompt}],
                "temperature": 0.3,
                "max_tokens": 1000
            }))
            .send()
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        result["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| LLMError::ApiError("Invalid response".to_string()))
    }
}

/// Analyze earnings call transcript
pub async fn analyze_earnings(
    transcript: &str,
    api_key: &str,
) -> Result<EarningsAnalysis, LLMError> {
    let client = OpenAIClient::new(api_key.to_string());

    let prompt = format!(
        "Analyze this earnings call transcript:\n\n{}\n\n\
        Provide structured analysis with:\n\
        - Overall sentiment (positive/negative/neutral)\n\
        - Key metrics mentioned\n\
        - Forward guidance assessment\n\
        - Management tone and confidence\n\
        - Red flags or concerns",
        &transcript[..transcript.len().min(15000)]
    );

    let text = client.generate(&prompt).await?;

    // Parse structured response
    Ok(EarningsAnalysis {
        sentiment: extract_sentiment(&text),
        key_points: text.lines().map(|s| s.to_string()).collect(),
        raw_analysis: text,
    })
}

#[derive(Debug)]
pub struct EarningsAnalysis {
    pub sentiment: String,
    pub key_points: Vec<String>,
    pub raw_analysis: String,
}

fn extract_sentiment(text: &str) -> String {
    if text.to_lowercase().contains("positive") || text.to_lowercase().contains("bullish") {
        "positive".to_string()
    } else if text.to_lowercase().contains("negative") || text.to_lowercase().contains("bearish") {
        "negative".to_string()
    } else {
        "neutral".to_string()
    }
}
