//! S5-GP-04: SEC filing parsing creates correct chunks

use investor_os::rag::parsers::SecParser;
use investor_os::rag::DocumentType;
use chrono::Utc;

#[tokio::test]
async fn test_10k_section_extraction() {
    let parser = SecParser::new();
    
    let content = r#"
ITEM 1. BUSINESS

We are Apple Inc., a technology company.
Our revenue was $365 billion in fiscal 2021.

ITEM 1A. RISK FACTORS

We face many risks including:
- Supply chain disruptions
- Regulatory changes
- Competition

ITEM 7. MANAGEMENT'S DISCUSSION AND ANALYSIS

Overview
Our business performed well this quarter.
EPS increased to $5.50.

ITEM 8. FINANCIAL STATEMENTS

Consolidated Balance Sheets...
"#;

    let chunks = parser.parse(content, DocumentType::Form10K, "AAPL", Utc::now())
        .await
        .expect("Parsing should succeed");

    // Should have chunks for each section
    assert!(!chunks.is_empty(), "Should create chunks");
    
    // Check that sections are properly identified
    let sections: Vec<_> = chunks.iter()
        .filter_map(|c| c.metadata.section.clone())
        .collect();
    
    assert!(sections.contains(&"Business".to_string()), "Should have Business section");
    assert!(sections.contains(&"Risk Factors".to_string()), "Should have Risk Factors section");
    assert!(sections.contains(&"MD&A".to_string()), "Should have MD&A section");
}

#[tokio::test]
async fn test_chunking_with_overlap() {
    let parser = SecParser::new();
    
    // Create content larger than chunk size
    let content = format!("ITEM 1. BUSINESS\n\n{}", "Word. ".repeat(2000));

    let chunks = parser.parse(&content, DocumentType::Form10K, "TEST", Utc::now())
        .await
        .expect("Parsing should succeed");

    // Should create multiple chunks for large content
    assert!(chunks.len() > 1, "Large content should be split into multiple chunks");
    
    // Chunks should have correct indices
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.chunk_index, i, "Chunk index should be sequential");
        assert_eq!(chunk.total_chunks, chunks.len(), "Total chunks should match");
    }
}

#[tokio::test]
async fn test_financial_metric_extraction() {
    let parser = SecParser::new();
    
    let content = r#"
ITEM 1. BUSINESS

Revenue was $100 billion for fiscal 2024.
EPS increased to $2.50 per share.
Net income reached $25 billion.
"#;

    let chunks = parser.parse(&content, DocumentType::Form10K, "TEST", Utc::now())
        .await
        .expect("Parsing should succeed");

    // Should extract financial metrics
    let all_metrics: Vec<_> = chunks.iter()
        .flat_map(|c| &c.metadata.financial_metrics)
        .collect();
    
    assert!(!all_metrics.is_empty(), "Should extract financial metrics");
    
    // Should detect revenue
    let has_revenue = all_metrics.iter()
        .any(|m| m.metric_type == "revenue");
    assert!(has_revenue, "Should detect revenue metric");
}

#[tokio::test]
async fn test_forward_looking_detection() {
    let parser = SecParser::new();
    
    let content = r#"
ITEM 1. BUSINESS

We expect revenue to grow 10% next year.
This is forward-looking guidance.

Past performance was strong with $100B revenue.
"#;

    let chunks = parser.parse(&content, DocumentType::Form10K, "TEST", Utc::now())
        .await
        .expect("Parsing should succeed");

    // Should identify forward-looking statements
    let forward_looking_chunks: Vec<_> = chunks.iter()
        .filter(|c| c.metadata.contains_forward_looking)
        .collect();
    
    assert!(!forward_looking_chunks.is_empty(), 
        "Should identify chunks with forward-looking statements");
}
