//! Prompt templates с променливи и message history

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Роля на съобщение
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Съобщение в разговор
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub metadata: Option<HashMap<String, String>>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            metadata: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            metadata: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            metadata: None,
        }
    }
}

/// Prompt template с променливи във формат {variable_name}
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    template: String,
    required_variables: Vec<String>,
}

impl PromptTemplate {
    pub fn new(template: impl Into<String>) -> Self {
        let template = template.into();
        let required_variables = Self::extract_variables(&template);

        Self {
            template,
            required_variables,
        }
    }

    /// Рендерира template с предоставени стойности
    pub fn render(&self, variables: &HashMap<String, String>) -> Result<String, super::ChainError> {
        // Проверяваме дали всички required променливи са налични
        for var in &self.required_variables {
            if !variables.contains_key(var) {
                return Err(super::ChainError::ExecutionError(format!(
                    "Missing required variable: {}",
                    var
                )));
            }
        }

        // Replace всички {variable} със стойностите
        let mut result = self.template.clone();
        for (key, value) in variables {
            result = result.replace(&format!("{{{}}}", key), value);
        }

        Ok(result)
    }

    /// Рендерира template с conversation history
    pub fn render_with_history(
        &self,
        variables: &HashMap<String, String>,
        history: &[Message],
    ) -> Result<String, super::ChainError> {
        let mut full_prompt = String::new();

        // Добавяме history
        for msg in history {
            let prefix = match msg.role {
                Role::System => "System: ",
                Role::User => "Human: ",
                Role::Assistant => "Assistant: ",
                Role::Tool => "Tool: ",
            };
            full_prompt.push_str(&format!("{}{}\n", prefix, msg.content));
        }

        // Добавяме текущия prompt
        let current = self.render(variables)?;
        full_prompt.push_str(&format!("Human: {}\nAssistant: ", current));

        Ok(full_prompt)
    }

    /// Извлича променливите от template
    fn extract_variables(template: &str) -> Vec<String> {
        let mut variables = vec![];
        let mut chars = template.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                let mut var_name = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '}' {
                        chars.next(); // consume '}'
                        break;
                    }
                    var_name.push(next_ch);
                    chars.next();
                }
                if !var_name.is_empty() && !var_name.contains('{') {
                    variables.push(var_name);
                }
            }
        }

        variables
    }

    pub fn required_variables(&self) -> &[String] {
        &self.required_variables
    }
}

/// Преддефинирани prompt-ове за Investor OS
pub mod trading_prompts {
    use super::*;

    /// Анализ на SEC filing
    pub fn sec_analysis() -> PromptTemplate {
        PromptTemplate::new(
            r#"You are a financial analyst. Analyze this SEC filing for {ticker}.

Filing Content:
{filing_content}

Filing Type: {filing_type}
Date: {filing_date}

Provide analysis in this JSON format:
{{
    "key_risks": ["risk1", "risk2"],
    "growth_indicators": ["indicator1"],
    "insider_sentiment": "bullish|bearish|neutral",
    "risk_score": 0.0-1.0,
    "opportunity_score": 0.0-1.0,
    "summary": "brief summary"
}}"#,
        )
    }

    /// Генериране на trading signal
    pub fn trading_signal() -> PromptTemplate {
        PromptTemplate::new(
            r#"Based on the following data, generate a trading signal.

Ticker: {ticker}
Current Price: {price}
Market Regime: {regime}

Signals:
- Quality Score: {quality_score}/100
- Insider Score: {insider_score}/100  
- Sentiment Score: {sentiment_score}/100
- Regime Fit: {regime_fit}/100
- Breakout Score: {breakout_score}/100
- ATR Trend: {atr_trend}/100

Conviction Quotient (CQ): {cq}/100

Recent News Context:
{news_context}

Provide signal as JSON:
{{
    "action": "BUY|SELL|HOLD",
    "confidence": 0.0-1.0,
    "position_size_suggestion": "small|medium|large",
    "rationale": "explanation",
    "risk_factors": ["factor1"]
}}"#,
        )
    }

    /// Phoenix стратегия generation
    pub fn strategy_generation() -> PromptTemplate {
        PromptTemplate::new(
            r#"You are the Phoenix Strategist. Generate a trading strategy based on market conditions.

Market Regime: {regime}
Volatility Environment: {volatility}
Trend Direction: {trend}

Recent Performance of Current Strategy:
{performance_summary}

Available Strategy Components:
- Entry: breakout, pullback, momentum, mean_reversion
- Exit: fixed_target, trailing_stop, time_based
- Sizing: fixed, volatility_adjusted, kelly

Generate a strategy configuration as JSON:
{{
    "name": "strategy_name",
    "description": "what it does",
    "entry_rules": {{"type": "...", "params": {{}}}},
    "exit_rules": {{"type": "...", "params": {{}}}},
    "position_sizing": {{"type": "...", "params": {{}}}},
    "max_positions": 5,
    "risk_per_trade": 0.01
}}"#,
        )
    }

    /// Decision journal analysis
    pub fn decision_journal() -> PromptTemplate {
        PromptTemplate::new(
            r#"Analyze this trading decision from the journal:

Decision Date: {date}
Ticker: {ticker}
Action: {action}
Conviction Quotient: {cq}
Expected Outcome: {expected}
Actual Outcome: {actual}
Rationale: {rationale}

Similar Past Decisions:
{similar_decisions}

Provide lessons learned:
{{
    "decision_quality": "good|poor|neutral",
    "bias_detected": "confirmation|overconfidence|loss_aversion|none",
    "lessons": ["lesson1"],
    "improvement_suggestions": ["suggestion1"]
}}"#,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_template() {
        let template = PromptTemplate::new("Hello {name}, your score is {score}!");

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("score".to_string(), "95".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "Hello Alice, your score is 95!");
    }

    #[test]
    fn test_missing_variable() {
        let template = PromptTemplate::new("Hello {name}!");
        let vars = HashMap::new();

        assert!(template.render(&vars).is_err());
    }
}
