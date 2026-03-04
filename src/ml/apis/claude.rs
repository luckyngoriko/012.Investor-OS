//! Anthropic Claude API Client
//!
//! Claude 3: $15/M tokens (expensive but powerful)
//! 200K context window for long documents
//! https://anthropic.com/pricing

use super::LLMError;

use serde_json::json;

#[derive(Debug)]
pub struct ClaudeClient {
    api_key: String,
    client: reqwest::Client,
}

impl ClaudeClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

impl ClaudeClient {
    pub async fn generate(&self, prompt: &str) -> Result<String, LLMError> {
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": "claude-3-opus-20240229",
                "max_tokens": 2000,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        result["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| LLMError::ApiError("Invalid response".to_string()))
    }
}

/// Deep analysis of 10-K filing
pub async fn analyze_10k(filing_text: &str, api_key: &str) -> Result<TenKAnalysis, LLMError> {
    let client = ClaudeClient::new(api_key.to_string());

    // Truncate if too long (200K context window but expensive)
    let truncated = &filing_text[..filing_text.len().min(50000)];

    let prompt = format!(
        "Perform a comprehensive analysis of this 10-K filing:\n\n{}\n\n\
        Focus on:\n\
        1. Business model and moat\n\
        2. Management quality and integrity\n\
        3. Financial health and risks\n\
        4. Competitive position\n\
        5. Growth opportunities\n\
        6. Red flags or concerns\n\n\
        Provide detailed investment thesis.",
        truncated
    );

    let analysis = client.generate(&prompt).await?;

    // Parse risk_rating from LLM analysis content
    let analysis_lower = analysis.to_lowercase();
    let risk_rating = if analysis_lower.contains("high risk")
        || analysis_lower.contains("significant risk")
        || analysis_lower.contains("elevated risk")
        || analysis_lower.contains("substantial risk")
    {
        "high"
    } else if analysis_lower.contains("low risk")
        || analysis_lower.contains("minimal risk")
        || analysis_lower.contains("limited risk")
        || analysis_lower.contains("negligible risk")
    {
        "low"
    } else {
        "medium"
    }
    .to_string();

    Ok(TenKAnalysis {
        summary: analysis.lines().take(3).collect::<Vec<_>>().join(" "),
        full_analysis: analysis,
        risk_rating,
    })
}

#[derive(Debug)]
pub struct TenKAnalysis {
    pub summary: String,
    pub full_analysis: String,
    pub risk_rating: String,
}
