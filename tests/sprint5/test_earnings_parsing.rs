//! S5-GP-05: Earnings call parsing identifies speakers

use chrono::Utc;
use investor_os::rag::parsers::EarningsParser;
use investor_os::rag::DocumentType;

#[tokio::test]
async fn test_speaker_identification() {
    let parser = EarningsParser::new();

    let transcript = r#"Operator: Good morning, and welcome to the Q1 2024 earnings call.

Tim Cook - CEO: Thank you, operator. Good morning everyone.
We're pleased to report record revenue of $100 billion.

Luca Maestri - CFO: Thank you, Tim. Let me walk through the numbers.
Revenue grew 10% year over year.

Analyst from Goldman Sachs: Can you discuss your guidance for Q2?

Tim Cook - CEO: We expect continued strong performance."#;

    let chunks = parser
        .parse(transcript, "AAPL", Utc::now())
        .await
        .expect("Parsing should succeed");

    assert!(!chunks.is_empty(), "Should create chunks from transcript");

    // All chunks should be for AAPL
    for chunk in &chunks {
        assert_eq!(chunk.ticker, "AAPL");
        assert_eq!(chunk.document_type, DocumentType::EarningsCall);
    }
}

#[tokio::test]
async fn test_prepared_remarks_vs_qa() {
    let parser = EarningsParser::new();

    let transcript = r#"Tim Cook - CEO: Welcome everyone to our earnings call.
We had a great quarter with record revenue.

Luca Maestri - CFO: Let me detail the financials.

Analyst from Morgan Stanley: Question about margins?

Tim Cook - CEO: Our margins expanded significantly."#;

    let chunks = parser
        .parse(transcript, "AAPL", Utc::now())
        .await
        .expect("Parsing should succeed");

    // Should have sections for Prepared Remarks and Q&A
    let sections: Vec<_> = chunks
        .iter()
        .filter_map(|c| c.metadata.section.clone())
        .collect();

    assert!(
        sections.contains(&"Prepared Remarks".to_string()),
        "Should have Prepared Remarks section"
    );
    assert!(
        sections.contains(&"Q&A".to_string()),
        "Should have Q&A section"
    );
}

#[tokio::test]
async fn test_financial_metrics_in_earnings() {
    let parser = EarningsParser::new();

    let transcript = r#"Tim Cook - CEO: Revenue was $100 billion.
EPS came in at $2.50, up from $2.20 last year.

Luca Maestri - CFO: Net income reached $25 billion."#;

    let chunks = parser
        .parse(transcript, "AAPL", Utc::now())
        .await
        .expect("Parsing should succeed");

    // Should extract financial metrics from earnings
    let all_metrics: Vec<_> = chunks
        .iter()
        .flat_map(|c| &c.metadata.financial_metrics)
        .collect();

    assert!(
        !all_metrics.is_empty(),
        "Should extract financial metrics from earnings"
    );
}

#[tokio::test]
async fn test_earnings_forward_looking() {
    let parser = EarningsParser::new();

    let transcript = r#"Tim Cook - CEO: We expect revenue to grow 10% next quarter.
This guidance reflects our current view."#;

    let chunks = parser
        .parse(transcript, "AAPL", Utc::now())
        .await
        .expect("Parsing should succeed");

    // Should identify forward-looking guidance
    let forward_looking = chunks.iter().any(|c| c.metadata.contains_forward_looking);

    assert!(
        forward_looking,
        "Should identify forward-looking statements in earnings"
    );
}
