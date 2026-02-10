//! Integration tests for LangChain module
//!
//! Sprint 1: LangChain Core Tests

use investor_os::langchain::{
    ChainContext,
    prompts::PromptTemplate,
};

#[tokio::test]
async fn test_prompt_template_render() {
    let template = PromptTemplate::new("Hello {name}, you have {count} messages!");
    
    let mut vars = std::collections::HashMap::new();
    vars.insert("name".to_string(), "Alice".to_string());
    vars.insert("count".to_string(), "5".to_string());
    
    let result = template.render(&vars).unwrap();
    assert_eq!(result, "Hello Alice, you have 5 messages!");
}

#[tokio::test]
async fn test_prompt_template_missing_variable() {
    let template = PromptTemplate::new("Hello {name}!");
    let vars = std::collections::HashMap::new();
    
    assert!(template.render(&vars).is_err());
}

#[tokio::test]
async fn test_chain_context_builder() {
    let ctx = ChainContext::new()
        .with_variable("ticker", "AAPL")
        .with_variable("price", "150.00")
        .with_metadata("timestamp", serde_json::json!("2024-01-01T00:00:00Z"));
    
    assert_eq!(ctx.get("ticker"), Some("AAPL"));
    assert_eq!(ctx.get("price"), Some("150.00"));
}

#[tokio::test]
async fn test_trading_prompts_exist() {
    // Verify trading prompt templates are valid
    use investor_os::langchain::prompts::trading_prompts;
    
    let sec_template = trading_prompts::sec_analysis();
    let vars = sec_template.required_variables();
    assert!(vars.contains(&"ticker".to_string()));
    assert!(vars.contains(&"filing_content".to_string()));
    
    let signal_template = trading_prompts::trading_signal();
    let signal_vars = signal_template.required_variables();
    assert!(signal_vars.contains(&"ticker".to_string()));
    assert!(signal_vars.contains(&"cq".to_string()));
}
