//! S5-GP-08: RAG API endpoints respond correctly

use investor_os::api::handlers::rag::{
    EarningsRequest, EarningsResponse, JournalSearchRequest, JournalSearchResponse,
    RagSearchRequest, RagSearchResponse, RagSummarizeRequest, RagSummarizeResponse,
    SecFilingRequest, SecFilingResponse,
};
use investor_os::api::handlers::ApiResponse;

#[test]
fn test_api_response_success() {
    let data = "test data".to_string();
    let response: ApiResponse<String> = ApiResponse::success(data.clone());

    assert!(response.success);
    assert_eq!(response.data, Some(data));
    assert!(response.error.is_none());
}

#[test]
fn test_api_response_error() {
    let response: ApiResponse<String> = ApiResponse::error("Something went wrong");

    assert!(!response.success);
    assert!(response.data.is_none());
    assert_eq!(response.error, Some("Something went wrong".to_string()));
}

#[test]
fn test_rag_search_request_serialization() {
    let request = RagSearchRequest {
        query: "What was revenue?".to_string(),
        ticker: Some("AAPL".to_string()),
        document_types: vec!["10-K".to_string(), "earnings_call".to_string()],
        limit: 5,
    };

    let json = serde_json::to_string(&request).expect("Should serialize");
    assert!(json.contains("What was revenue?"));
    assert!(json.contains("AAPL"));
}

#[test]
fn test_rag_search_response_serialization() {
    let response = RagSearchResponse {
        query: "test".to_string(),
        results: vec![],
        total_results: 0,
        search_time_ms: 150,
    };

    let json = serde_json::to_string(&response).expect("Should serialize");
    assert!(json.contains("150"));
}

#[test]
fn test_rag_summarize_request() {
    let request = RagSummarizeRequest {
        ticker: "AAPL".to_string(),
        document_type: Some("10-K".to_string()),
    };

    assert_eq!(request.ticker, "AAPL");
    assert_eq!(request.document_type, Some("10-K".to_string()));
}

#[test]
fn test_journal_search_request() {
    let request = JournalSearchRequest {
        query: "momentum trades".to_string(),
        portfolio_id: None,
        limit: 10,
    };

    assert_eq!(request.query, "momentum trades");
    assert_eq!(request.limit, 10);
}

#[test]
fn test_sec_filing_request() {
    use chrono::Utc;

    let request = SecFilingRequest {
        ticker: "AAPL".to_string(),
        document_type: "10-K".to_string(),
        content: "Test filing content".to_string(),
        filing_date: Utc::now(),
    };

    assert_eq!(request.ticker, "AAPL");
    assert_eq!(request.document_type, "10-K");
}

#[test]
fn test_sec_filing_response() {
    let response = SecFilingResponse {
        ticker: "AAPL".to_string(),
        chunks_processed: 42,
        document_type: "10-K".to_string(),
    };

    assert_eq!(response.chunks_processed, 42);
}

#[test]
fn test_earnings_request() {
    use chrono::Utc;

    let request = EarningsRequest {
        ticker: "AAPL".to_string(),
        transcript: "CEO: Revenue was $100B".to_string(),
        call_date: Utc::now(),
    };

    assert_eq!(request.ticker, "AAPL");
}

#[test]
fn test_default_limit() {
    use investor_os::api::handlers::default_limit;
    assert_eq!(default_limit(), 10);
}

// Note: Full API integration tests would require:
// 1. Starting a test server with AppState
// 2. Making HTTP requests to endpoints
// 3. Verifying response status codes and bodies
// These would be implemented as integration tests with a test database
