//! Chain implementations - композируеми единици работа

use super::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::time::Instant;

/// Базов trait за всички chains
#[async_trait]
pub trait Chain: Send + Sync {
    async fn run(&self, context: ChainContext) -> Result<ChainResult, ChainError>;
}

/// Прост LLM chain - подава prompt към LLM
pub struct LLMChain {
    llm: LLMProvider,
    template: PromptTemplate,
    memory: Option<Box<dyn ConversationMemory>>,
    parser: Option<Box<dyn OutputParser>>,
}

impl LLMChain {
    pub fn new(
        llm: LLMProvider,
        template: PromptTemplate,
        memory: Option<Box<dyn ConversationMemory>>,
    ) -> Self {
        Self {
            llm,
            template,
            memory,
            parser: None,
        }
    }

    pub fn with_parser<P: OutputParser + 'static>(mut self, parser: P) -> Self {
        self.parser = Some(Box::new(parser));
        self
    }
}

#[async_trait]
impl Chain for LLMChain {
    async fn run(&self, context: ChainContext) -> Result<ChainResult, ChainError> {
        let start = Instant::now();

        // Build prompt с историята ако има memory
        let prompt = if let Some(memory) = &self.memory {
            let history = memory.load().await;
            self.template
                .render_with_history(&context.variables, &history)?
        } else {
            self.template.render(&context.variables)?
        };

        // Call LLM
        let output = self.llm.generate(&prompt).await?;

        // Parse ако има parser
        let parsed_output = if let Some(parser) = &self.parser {
            Some(parser.parse(&output)?)
        } else {
            None
        };

        // Save to memory
        if let Some(memory) = &self.memory {
            memory.save(&prompt, &output).await;
        }

        let elapsed = start.elapsed().as_millis() as u64;

        let (input_tokens, output_tokens) = crate::ml::apis::estimate_tokens(&prompt, &output);

        Ok(ChainResult {
            output,
            parsed_output,
            metadata: ExecutionMetadata {
                llm_calls: 1,
                tool_calls: 0,
                tokens_used: input_tokens + output_tokens,
                execution_time_ms: elapsed,
                model: self.llm.name().to_string(),
            },
        })
    }
}

/// Последователна верига - изходът от chain N → вход за chain N+1
pub struct SequentialChain {
    chains: Vec<Box<dyn Chain>>,
    output_keys: Vec<String>, // Кои ключове да се запазят между стъпките
}

impl Default for SequentialChain {
    fn default() -> Self {
        Self::new()
    }
}

impl SequentialChain {
    pub fn new() -> Self {
        Self {
            chains: vec![],
            output_keys: vec![],
        }
    }

    pub fn add_chain(mut self, chain: Box<dyn Chain>) -> Self {
        self.chains.push(chain);
        self
    }

    pub fn with_output_key(mut self, key: impl Into<String>) -> Self {
        self.output_keys.push(key.into());
        self
    }
}

#[async_trait]
impl Chain for SequentialChain {
    async fn run(&self, mut context: ChainContext) -> Result<ChainResult, ChainError> {
        let start = Instant::now();
        let mut total_llm_calls = 0u32;
        let mut total_tool_calls = 0u32;
        let mut last_output = String::new();
        let mut last_parsed = None;

        for (i, chain) in self.chains.iter().enumerate() {
            // Добавяме изхода от предишната стъпка като input за следващата
            if i > 0 {
                context.variables.insert(
                    self.output_keys
                        .get(i - 1)
                        .cloned()
                        .unwrap_or_else(|| format!("step_{}", i - 1)),
                    last_output.clone(),
                );
            }

            let result = chain.run(context.clone()).await?;
            total_llm_calls += result.metadata.llm_calls;
            total_tool_calls += result.metadata.tool_calls;
            last_output = result.output;
            last_parsed = result.parsed_output;
        }

        let elapsed = start.elapsed().as_millis() as u64;

        Ok(ChainResult {
            output: last_output,
            parsed_output: last_parsed,
            metadata: ExecutionMetadata {
                llm_calls: total_llm_calls,
                tool_calls: total_tool_calls,
                tokens_used: 0,
                execution_time_ms: elapsed,
                model: "sequential".to_string(),
            },
        })
    }
}

/// Паралелна верига - изпълнява няколко chains едновременно
pub struct ParallelChain {
    chains: Vec<(String, Box<dyn Chain>)>, // (output_key, chain)
}

impl Default for ParallelChain {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelChain {
    pub fn new() -> Self {
        Self { chains: vec![] }
    }

    pub fn add_chain(mut self, output_key: impl Into<String>, chain: Box<dyn Chain>) -> Self {
        self.chains.push((output_key.into(), chain));
        self
    }
}

#[async_trait]
impl Chain for ParallelChain {
    async fn run(&self, context: ChainContext) -> Result<ChainResult, ChainError> {
        let start = Instant::now();

        // Изпълняваме всички chains паралелно
        let futures = self.chains.iter().map(|(key, chain)| {
            let ctx = context.clone();
            async move {
                let result = chain.run(ctx).await?;
                Ok::<(String, ChainResult), ChainError>((key.clone(), result))
            }
        });

        let results = futures::future::try_join_all(futures).await?;

        // Комбинираме резултатите в JSON
        let mut combined = serde_json::Map::new();
        let mut total_llm_calls = 0u32;
        let mut total_tool_calls = 0u32;

        for (key, result) in results {
            combined.insert(key, serde_json::Value::String(result.output));
            total_llm_calls += result.metadata.llm_calls;
            total_tool_calls += result.metadata.tool_calls;
        }

        let elapsed = start.elapsed().as_millis() as u64;
        let output = serde_json::to_string(&combined).unwrap_or_default();

        Ok(ChainResult {
            output: output.clone(),
            parsed_output: Some(serde_json::Value::Object(combined)),
            metadata: ExecutionMetadata {
                llm_calls: total_llm_calls,
                tool_calls: total_tool_calls,
                tokens_used: 0,
                execution_time_ms: elapsed,
                model: "parallel".to_string(),
            },
        })
    }
}

/// Tool-using chain (ReAct pattern) - LLM може да извиква tools
pub struct AgentChain {
    llm: LLMProvider,
    tools: ToolRegistry,
    max_iterations: u32,
}

impl AgentChain {
    pub fn new(llm: LLMProvider, tools: ToolRegistry) -> Self {
        Self {
            llm,
            tools,
            max_iterations: 5,
        }
    }

    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }
}

#[async_trait]
impl Chain for AgentChain {
    async fn run(&self, context: ChainContext) -> Result<ChainResult, ChainError> {
        let start = Instant::now();
        let mut iterations = 0u32;
        let mut tool_calls = 0u32;

        // ReAct loop: Thought → Action → Observation → ... → Final Answer
        let mut current_prompt = format!(
            "You are a trading analysis agent. You have access to these tools:\n{}\n\n\
            Question: {}\n\n\
            Think step by step. When you need information, use a tool. \
            Format your response as:\n\
            Thought: [your reasoning]\n\
            Action: [tool_name]|[input]\n\
            or\n\
            Thought: [your reasoning]\n\
            Final Answer: [your conclusion]",
            self.tools.describe(),
            context.get("question").unwrap_or("Analyze the market")
        );

        loop {
            if iterations >= self.max_iterations {
                return Err(ChainError::ExecutionError(
                    "Max iterations exceeded".to_string(),
                ));
            }

            let response = self.llm.generate(&current_prompt).await?;
            iterations += 1;

            // Parse за Action или Final Answer
            if let Some(action) = extract_action(&response) {
                // Изпълняваме tool
                let result = self.tools.execute(&action.tool_name, &action.input).await?;
                tool_calls += 1;

                current_prompt
                    .push_str(&format!("\n\n{}\nObservation: {}", response, result.output));
            } else if let Some(answer) = extract_final_answer(&response) {
                // Готово!
                let elapsed = start.elapsed().as_millis() as u64;
                return Ok(ChainResult {
                    output: answer,
                    parsed_output: None,
                    metadata: ExecutionMetadata {
                        llm_calls: iterations,
                        tool_calls,
                        tokens_used: 0,
                        execution_time_ms: elapsed,
                        model: self.llm.name().to_string(),
                    },
                });
            } else {
                // Нито action, нито final answer - молим за clarification
                current_prompt.push_str("\n\nPlease provide either an Action or a Final Answer.");
            }
        }
    }
}

fn extract_action(response: &str) -> Option<ToolCall> {
    // Parse "Action: tool_name|input"
    response
        .lines()
        .find(|l| l.starts_with("Action:"))
        .and_then(|line| {
            let parts: Vec<_> = line
                .trim_start_matches("Action:")
                .trim()
                .splitn(2, '|')
                .collect();
            if parts.len() == 2 {
                Some(ToolCall {
                    tool_name: parts[0].to_string(),
                    input: parts[1].to_string(),
                })
            } else {
                None
            }
        })
}

fn extract_final_answer(response: &str) -> Option<String> {
    response
        .lines()
        .find(|l| l.starts_with("Final Answer:"))
        .map(|line| line.trim_start_matches("Final Answer:").trim().to_string())
}

// ToolCall is now defined in tools.rs

/// RAG Chain - използва neurocod-rag за retrieval + LLM за generation
pub struct RAGChain {
    llm: LLMProvider,
    retriever: Arc<dyn DocumentRetriever>,
    template: PromptTemplate,
}

#[async_trait]
pub trait DocumentRetriever: Send + Sync {
    async fn retrieve(&self, query: &str, top_k: usize) -> Result<Vec<Document>, ChainError>;
}

#[derive(Debug, Clone)]
pub struct Document {
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub score: f32,
}

impl RAGChain {
    pub fn new(
        llm: LLMProvider,
        retriever: Arc<dyn DocumentRetriever>,
        template: PromptTemplate,
    ) -> Self {
        Self {
            llm,
            retriever,
            template,
        }
    }
}

#[async_trait]
impl Chain for RAGChain {
    async fn run(&self, context: ChainContext) -> Result<ChainResult, ChainError> {
        let start = Instant::now();

        // 1. Retrieve relevant documents
        let query = context
            .get("query")
            .ok_or_else(|| ChainError::ExecutionError("Missing 'query' in context".to_string()))?;

        let docs = self.retriever.retrieve(query, 5).await?;

        // 2. Format context from documents
        let context_text = docs
            .iter()
            .map(|d| {
                format!(
                    "[Source: {}]\n{}",
                    d.metadata.get("source").unwrap_or(&"unknown".to_string()),
                    d.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        // 3. Build prompt
        let mut ctx_with_docs = context.clone();
        ctx_with_docs
            .variables
            .insert("context".to_string(), context_text);
        let prompt = self.template.render(&ctx_with_docs.variables)?;

        // 4. Generate response
        let output = self.llm.generate(&prompt).await?;

        let elapsed = start.elapsed().as_millis() as u64;

        Ok(ChainResult {
            output,
            parsed_output: None,
            metadata: ExecutionMetadata {
                llm_calls: 1,
                tool_calls: 0,
                tokens_used: 0,
                execution_time_ms: elapsed,
                model: self.llm.name().to_string(),
            },
        })
    }
}
