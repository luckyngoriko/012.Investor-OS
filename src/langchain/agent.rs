//! Agent implementation with ReAct pattern
//!
//! ReAct = Reasoning + Acting
//! LLM мисли стъпка по стъпка и извиква tools когато е необходимо

use super::{Chain, ChainContext, ChainError, ChainResult, ExecutionMetadata, prompts::Message, tools::{Tool, ToolRegistry, ToolResult}};
use async_trait::async_trait;
use std::sync::Arc;

/// Agent с ReAct pattern
pub struct Agent {
    llm: Arc<dyn LLM>,
    tools: ToolRegistry,
    max_iterations: u32,
    system_prompt: String,
}

/// LLM interface за agent
#[async_trait]
pub trait LLM: Send + Sync {
    async fn generate(&self, messages: &[Message]) -> Result<String, LLMError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limit")]
    RateLimit,
}

impl Agent {
    pub fn new(llm: Arc<dyn LLM>, tools: ToolRegistry) -> Self {
        Self {
            llm,
            tools,
            max_iterations: 10,
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
        }
    }
    
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }
    
    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }
}

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a helpful trading assistant. You have access to tools that can help you analyze markets and make trading decisions.

When you need information, use a tool by responding with:
Action: tool_name
Action Input: {"key": "value"}

After receiving the observation, analyze it and either:
1. Use another tool if you need more information
2. Provide your final answer with:
Final Answer: your response

Be concise and analytical in your reasoning."#;

#[async_trait]
impl Chain for Agent {
    async fn run(&self, context: ChainContext) -> Result<ChainResult, ChainError> {
        let start_time = std::time::Instant::now();
        let mut iterations = 0u32;
        let mut tool_calls = 0u32;
        
        let question = context.get("question")
            .ok_or_else(|| ChainError::ExecutionError("Missing 'question' in context".to_string()))?;
        
        // Build conversation history
        let mut messages = vec![
            Message::system(&self.system_prompt),
            Message::system(format!("Available tools:\n{}", self.tools.describe())),
            Message::user(question),
        ];
        
        let final_output = loop {
            if iterations >= self.max_iterations {
                return Err(ChainError::ExecutionError("Max iterations exceeded".to_string()));
            }
            iterations += 1;
            
            // Get LLM response
            let response = self.llm.generate(&messages).await
                .map_err(|e| ChainError::ExecutionError(e.to_string()))?;
            
            // Check for Action
            if let Some((tool_name, tool_input)) = parse_action(&response) {
                // Execute tool
                let result = self.tools.execute(&tool_name, &tool_input).await
                    .unwrap_or_else(|e| ToolResult::error(e.to_string()));
                tool_calls += 1;
                
                // Add to conversation
                messages.push(Message::assistant(&response));
                messages.push(Message::user(format!(
                    "Observation: {}",
                    result.output
                )));
            } else if let Some(answer) = parse_final_answer(&response) {
                // Got final answer
                break answer;
            } else {
                // Neither action nor final answer
                messages.push(Message::assistant(&response));
                messages.push(Message::user("Please provide either an Action or a Final Answer."));
            }
        };
        
        let elapsed = start_time.elapsed().as_millis() as u64;
        
        Ok(ChainResult {
            output: final_output,
            parsed_output: None,
            metadata: ExecutionMetadata {
                llm_calls: iterations,
                tool_calls,
                tokens_used: 0,
                execution_time_ms: elapsed,
                model: "agent".to_string(),
            },
        })
    }
}

fn parse_action(response: &str) -> Option<(String, String)> {
    let lines: Vec<_> = response.lines().collect();
    
    let mut action_idx = None;
    let mut input_idx = None;
    
    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("Action:") {
            action_idx = Some(i);
        }
        if line.trim().starts_with("Action Input:") {
            input_idx = Some(i);
        }
    }
    
    match (action_idx, input_idx) {
        (Some(ai), Some(ii)) => {
            let action = lines[ai].trim_start_matches("Action:").trim().to_string();
            let input = lines[ii].trim_start_matches("Action Input:").trim().to_string();
            Some((action, input))
        }
        _ => None,
    }
}

fn parse_final_answer(response: &str) -> Option<String> {
    response.lines()
        .find(|l| l.starts_with("Final Answer:"))
        .map(|line| line.trim_start_matches("Final Answer:").trim().to_string())
}

/// Builder за Agent
pub struct AgentBuilder {
    llm: Option<Arc<dyn LLM>>,
    tools: ToolRegistry,
    system_prompt: Option<String>,
    max_iterations: u32,
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self {
            llm: None,
            tools: ToolRegistry::new(),
            system_prompt: None,
            max_iterations: 10,
        }
    }
    
    pub fn with_llm(mut self, llm: Arc<dyn LLM>) -> Self {
        self.llm = Some(llm);
        self
    }
    
    pub fn with_tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.tools.register(tool);
        self
    }
    
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }
    
    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }
    
    pub fn build(self) -> Result<Agent, ChainError> {
        let llm = self.llm.ok_or(ChainError::MissingLLM)?;
        
        let mut agent = Agent::new(llm, self.tools);
        agent.max_iterations = self.max_iterations;
        
        if let Some(prompt) = self.system_prompt {
            agent.system_prompt = prompt;
        }
        
        Ok(agent)
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_action() {
        let response = r#"I need to check the portfolio.
Action: get_portfolio
Action Input: {}"#;
        
        let (tool, input) = parse_action(response).unwrap();
        assert_eq!(tool, "get_portfolio");
        assert_eq!(input, "{}");
    }
    
    #[test]
    fn test_parse_final_answer() {
        let response = "Final Answer: The portfolio value is $10,000.";
        
        let answer = parse_final_answer(response).unwrap();
        assert_eq!(answer, "The portfolio value is $10,000.");
    }
}
