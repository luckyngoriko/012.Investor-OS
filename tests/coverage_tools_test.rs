//! Coverage tests for Tool implementations
//!
//! Fills gaps from coverage analysis

use investor_os::langchain::tools::{
    Tool, ToolRegistry, ToolResult, ToolCall,
};

/// Mock Tool for testing
struct MockTool {
    name: String,
    should_fail: bool,
}

#[async_trait::async_trait]
impl Tool for MockTool {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "A mock tool for testing"
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            }
        })
    }
    
    async fn execute(&self, input: &str) -> ToolResult {
        if self.should_fail {
            ToolResult::error("Tool execution failed")
        } else {
            ToolResult::success(format!("Processed: {}", input))
        }
    }
}

#[tokio::test]
async fn test_tool_registry_multiple_tools() {
    let mut registry = ToolRegistry::new();
    
    registry.register(Box::new(MockTool { 
        name: "tool1".to_string(), 
        should_fail: false 
    }));
    registry.register(Box::new(MockTool { 
        name: "tool2".to_string(), 
        should_fail: false 
    }));
    
    let tools = registry.list();
    assert_eq!(tools.len(), 2);
    assert!(tools.contains(&"tool1"));
    assert!(tools.contains(&"tool2"));
}

#[tokio::test]
async fn test_tool_execution_success() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(MockTool { 
        name: "test_tool".to_string(), 
        should_fail: false 
    }));
    
    let result = registry.execute("test_tool", "hello").await.unwrap();
    
    assert!(result.success);
    assert!(result.output.contains("Processed: hello"));
}

#[tokio::test]
async fn test_tool_execution_failure() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(MockTool { 
        name: "failing_tool".to_string(), 
        should_fail: true 
    }));
    
    let result = registry.execute("failing_tool", "input").await.unwrap();
    
    assert!(!result.success);
    assert!(result.output.contains("failed"));
}

#[tokio::test]
async fn test_tool_not_found() {
    let registry = ToolRegistry::new();
    
    let result = registry.execute("nonexistent", "input").await;
    
    assert!(result.is_err());
}

#[test]
fn test_tool_describe() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(MockTool { 
        name: "my_tool".to_string(), 
        should_fail: false 
    }));
    
    let description = registry.describe();
    
    assert!(description.contains("my_tool"));
    assert!(description.contains("A mock tool"));
}

#[test]
fn test_tool_result_construction() {
    let success_result = ToolResult::success("output data");
    assert!(success_result.success);
    assert_eq!(success_result.output, "output data");
    assert!(success_result.metadata.is_none());
    
    let error_result = ToolResult::error("something went wrong");
    assert!(!error_result.success);
    assert_eq!(error_result.output, "something went wrong");
}

#[test]
fn test_tool_call_construction() {
    let call = ToolCall {
        tool_name: "get_portfolio".to_string(),
        input: "{}".to_string(),
    };
    
    assert_eq!(call.tool_name, "get_portfolio");
    assert_eq!(call.input, "{}");
}

#[test]
fn test_empty_registry() {
    let registry = ToolRegistry::new();
    
    assert!(registry.list().is_empty());
    assert!(registry.describe().is_empty());
}
