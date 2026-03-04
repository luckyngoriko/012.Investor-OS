//! LangChain-inspired AI Component Framework for Rust
//!
//! Превръща LLM интеграциите в композируеми "вериги" (chains)
//! подобно на LangChain, но type-safe и async-native.
//!
//! Основни концепции:
//! - Chain: Композируема единица работа
//! - PromptTemplate: Шаблони с променливи
//! - Tool: Агенти могат да извикват външни функции
//! - OutputParser: Структуриран output от LLM

pub mod agent;
pub mod chains;
pub mod memory;
pub mod parsers;
pub mod prompts;
pub mod tools;

pub use agent::LLMError as AgentLLMError;
pub use agent::{Agent, AgentBuilder, LLM};
pub use chains::{Chain, LLMChain, ParallelChain, SequentialChain};
pub use memory::{BufferMemory, ConversationMemory, VectorStoreMemory};
pub use parsers::{JsonParser, OutputParser, StructuredParser};
pub use prompts::{Message, PromptTemplate, Role};
pub use tools::{Tool, ToolCall, ToolRegistry, ToolResult};

use crate::ml::apis::{LLMError, LLMProvider};
use std::collections::HashMap;

/// Основен контекст за изпълнение на chain
#[derive(Debug, Clone, Default)]
pub struct ChainContext {
    /// Променливи за prompt темплейти
    pub variables: HashMap<String, String>,
    /// Метаданни за изпълнението
    pub metadata: HashMap<String, serde_json::Value>,
    /// Conversation history (ако има memory)
    pub history: Vec<Message>,
}

impl ChainContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.variables.get(key).map(|s| s.as_str())
    }
}

/// Резултат от изпълнение на chain
#[derive(Debug, Clone)]
pub struct ChainResult {
    pub output: String,
    pub parsed_output: Option<serde_json::Value>,
    pub metadata: ExecutionMetadata,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionMetadata {
    pub llm_calls: u32,
    pub tool_calls: u32,
    pub tokens_used: u64,
    pub execution_time_ms: u64,
    pub model: String,
}

/// Builder за създаване на complex chains
pub struct ChainBuilder {
    llm: Option<LLMProvider>,
    memory: Option<Box<dyn ConversationMemory>>,
    tools: ToolRegistry,
}

impl Default for ChainBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainBuilder {
    pub fn new() -> Self {
        Self {
            llm: None,
            memory: None,
            tools: ToolRegistry::new(),
        }
    }

    pub fn with_llm(mut self, llm: LLMProvider) -> Self {
        self.llm = Some(llm);
        self
    }

    pub fn with_memory<M: ConversationMemory + 'static>(mut self, memory: M) -> Self {
        self.memory = Some(Box::new(memory));
        self
    }

    pub fn with_tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.tools.register(tool);
        self
    }

    /// Създава LLMChain с prompt template
    pub fn build_llm_chain(self, template: PromptTemplate) -> Result<LLMChain, ChainError> {
        let llm = self.llm.ok_or(ChainError::MissingLLM)?;
        Ok(LLMChain::new(llm, template, self.memory))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    #[error("Missing LLM provider")]
    MissingLLM,
    #[error("LLM error: {0}")]
    LLMError(#[from] LLMError),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Tool error: {0}")]
    ToolError(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

// Примерен usage за Investor OS:
//
// ```rust,ignore
// // Анализ на SEC filing
// let chain = ChainBuilder::new()
//     .with_llm(LLMProvider::Claude(claude_client))
//     .with_memory(BufferMemory::new(10))
//     .build_llm_chain(PromptTemplate::new(r#"
//         Analyze this SEC filing for {ticker}:
//
//         {filing_content}
//
//         Provide:
//         1. Key risks (0-1 score)
//         2. Growth indicators (0-1 score)
//         3. Insider activity sentiment
//
//         Output as JSON.
//     "#))?;
//
// let result = chain.run(ChainContext::new()
//     .with_variable("ticker", "AAPL")
//     .with_variable("filing_content", sec_text)
// ).await?;
//
// let analysis: SecAnalysis = serde_json::from_str(&result.output)?;
// ```
