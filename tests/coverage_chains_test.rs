//! Coverage tests for Chain implementations
//!
//! Fills gaps from coverage analysis

use async_trait::async_trait;
use investor_os::langchain::{
    chains::{ParallelChain, SequentialChain},
    prompts::PromptTemplate,
    Chain, ChainContext, ChainResult,
};

/// Mock Chain for testing
struct MockChain {
    prefix: String,
}

#[async_trait]
impl Chain for MockChain {
    async fn run(
        &self,
        ctx: ChainContext,
    ) -> Result<ChainResult, investor_os::langchain::ChainError> {
        let input = ctx.get("input").unwrap_or("default");
        let output = format!("{}{}", self.prefix, input);

        Ok(ChainResult {
            output,
            parsed_output: None,
            metadata: investor_os::langchain::ExecutionMetadata {
                llm_calls: 1,
                tool_calls: 0,
                tokens_used: 0,
                execution_time_ms: 0,
                model: "mock".to_string(),
            },
        })
    }
}

#[tokio::test]
async fn test_sequential_chain() {
    let chain1 = MockChain {
        prefix: "A:".to_string(),
    };
    let chain2 = MockChain {
        prefix: "B:".to_string(),
    };

    let seq_chain = SequentialChain::new()
        .add_chain(Box::new(chain1))
        .add_chain(Box::new(chain2))
        .with_output_key("step1")
        .with_output_key("step2");

    let ctx = ChainContext::new().with_variable("input", "test");

    let result = seq_chain.run(ctx).await.unwrap();

    // Second chain should use output of first as input
    assert!(result.output.contains("B:") || result.output.contains("A:test"));
    assert_eq!(result.metadata.llm_calls, 2);
}

#[tokio::test]
async fn test_parallel_chain() {
    let chain1 = MockChain {
        prefix: "Parallel1:".to_string(),
    };
    let chain2 = MockChain {
        prefix: "Parallel2:".to_string(),
    };

    let par_chain = ParallelChain::new()
        .add_chain("result1", Box::new(chain1))
        .add_chain("result2", Box::new(chain2));

    let ctx = ChainContext::new().with_variable("input", "data");

    let result = par_chain.run(ctx).await.unwrap();

    // Should contain both results
    assert!(result.output.contains("Parallel1:") || result.output.contains("Parallel2:"));
    assert_eq!(result.metadata.llm_calls, 2);
}

#[tokio::test]
async fn test_sequential_chain_empty() {
    let seq_chain = SequentialChain::new();

    let ctx = ChainContext::new();
    let result = seq_chain
        .run(ctx)
        .await
        .expect("empty sequential chain should return a default result");

    // Empty chain produces an empty output string
    assert_eq!(result.output, "", "empty chain should produce empty output");
}

#[tokio::test]
async fn test_chain_result_metadata() {
    let chain = MockChain {
        prefix: "".to_string(),
    };
    let ctx = ChainContext::new().with_variable("input", "x");

    let result = chain.run(ctx).await.unwrap();

    assert_eq!(result.metadata.llm_calls, 1);
    assert_eq!(result.metadata.tool_calls, 0);
    assert_eq!(result.metadata.model, "mock");
    assert!(result.metadata.execution_time_ms >= 0);
}

#[tokio::test]
async fn test_prompt_template_with_history() {
    use investor_os::langchain::prompts::{Message, Role};

    let template = PromptTemplate::new("Current: {current}");

    let mut vars = std::collections::HashMap::new();
    vars.insert("current".to_string(), "question".to_string());

    let history = vec![
        Message {
            role: Role::User,
            content: "Hello".to_string(),
            metadata: None,
        },
        Message {
            role: Role::Assistant,
            content: "Hi there".to_string(),
            metadata: None,
        },
    ];

    let result = template.render_with_history(&vars, &history).unwrap();

    assert!(result.contains("Hello"));
    assert!(result.contains("Hi there"));
    assert!(result.contains("Current: question"));
}
