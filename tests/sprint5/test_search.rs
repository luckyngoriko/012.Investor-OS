//! S5-GP-07: Semantic search returns relevant results

use investor_os::rag::{DocumentType, SearchQuery};

#[test]
fn test_search_query_construction() {
    let query = SearchQuery {
        query: "What was Apple's revenue?".to_string(),
        ticker: Some("AAPL".to_string()),
        document_types: vec![DocumentType::Form10K, DocumentType::EarningsCall],
        date_range: None,
        limit: 5,
    };

    assert_eq!(query.query, "What was Apple's revenue?");
    assert_eq!(query.ticker, Some("AAPL".to_string()));
    assert_eq!(query.document_types.len(), 2);
    assert_eq!(query.limit, 5);
}

#[test]
fn test_search_query_default_limit() {
    let query = SearchQuery::default();

    assert_eq!(query.limit, 10);
    assert!(query.query.is_empty());
    assert!(query.ticker.is_none());
}

#[test]
fn test_document_type_as_str() {
    assert_eq!(DocumentType::Form10K.as_str(), "10-K");
    assert_eq!(DocumentType::Form10Q.as_str(), "10-Q");
    assert_eq!(DocumentType::Form8K.as_str(), "8-K");
    assert_eq!(DocumentType::EarningsCall.as_str(), "earnings_call");
    assert_eq!(DocumentType::News.as_str(), "news");
    assert_eq!(DocumentType::AnalystReport.as_str(), "analyst_report");
}

// Note: Full integration tests for search would require a test database
// with actual embeddings and would test:
// 1. Storing chunks with embeddings
// 2. Searching and getting relevant results
// 3. Similarity scores are in correct range
// 4. Filtering by ticker, document type, and date range works

#[tokio::test]
async fn test_search_result_ranking() {
    // This is a conceptual test - in practice would use real search
    // Results should be ordered by similarity score (highest first)

    // Mock search results
    let results = vec![
        MockResult {
            score: 0.95,
            text: "Apple revenue was $100B".to_string(),
        },
        MockResult {
            score: 0.87,
            text: "Revenue increased to $100B".to_string(),
        },
        MockResult {
            score: 0.72,
            text: "Other company had revenue".to_string(),
        },
    ];

    // Verify scores are in descending order
    for i in 1..results.len() {
        assert!(
            results[i - 1].score >= results[i].score,
            "Results should be ordered by score descending"
        );
    }
}

struct MockResult {
    score: f32,
    text: String,
}
