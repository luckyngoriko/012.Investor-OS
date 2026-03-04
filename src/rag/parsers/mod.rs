//! Document Parsers
//!
//! S5-D6: SEC Filings Parser (10-K, 10-Q)
//! S5-D7: Earnings Analyzer (FinBERT sentiment on transcripts)

use super::{DocumentChunk, DocumentMetadata, DocumentType, FinancialMetric, Result};

mod earnings;
mod sec;

pub use earnings::EarningsParser;
pub use sec::SecParser;

/// Maximum characters per chunk
const CHUNK_SIZE: usize = 1000;
/// Overlap between chunks for context continuity
const CHUNK_OVERLAP: usize = 100;

/// Split text into chunks with overlap
fn chunk_text(text: &str, max_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let end = (start + max_size).min(text.len());

        // Try to find a natural break point (period + space or newline)
        let chunk_end = if end < text.len() {
            let search_start = end.saturating_sub(100);
            if let Some(pos) = text[search_start..end].rfind(". ") {
                search_start + pos + 2
            } else if let Some(pos) = text[search_start..end].rfind('\n') {
                search_start + pos + 1
            } else {
                end
            }
        } else {
            end
        };

        chunks.push(text[start..chunk_end].trim().to_string());

        // Move start forward with overlap, but ensure we make progress
        let next_start = chunk_end.saturating_sub(overlap);
        if next_start <= start {
            // Force progress if overlap would keep us at same position
            start = chunk_end;
        } else {
            start = next_start;
        }
    }

    chunks
}

/// Extract financial metrics from text
fn extract_financial_metrics(text: &str) -> Vec<FinancialMetric> {
    let mut metrics = Vec::new();

    // Simple regex-like patterns for common financial metrics
    let patterns = [
        (
            "revenue",
            r"(?:revenue|sales)\s*(?:of\s*)?\$?([\d,]+\.?\d*)\s*(million|billion|M|B)?",
        ),
        (
            "eps",
            r"(?:earnings per share|EPS)\s*(?:of\s*)?\$?([\d,]+\.?\d*)",
        ),
        (
            "net income",
            r"(?:net income|profit)\s*(?:of\s*)?\$?([\d,]+\.?\d*)\s*(million|billion|M|B)?",
        ),
        (
            "guidance",
            r"(?:guidance|outlook)\s*(?:for\s*)?(?:FY\s*)?(\d{4})?\s*(?:is\s*)?\$?([\d,]+\.?\d*)",
        ),
    ];

    for (metric_type, _pattern) in &patterns {
        // Simplified extraction - in production would use regex
        if text.to_lowercase().contains(metric_type) {
            // Look for numbers near the metric keyword
            let lower_text = text.to_lowercase();
            if let Some(pos) = lower_text.find(metric_type) {
                let context_start = pos.saturating_sub(50);
                let context_end = (pos + 100).min(text.len());
                let context = &text[context_start..context_end];

                // Try to find a number in the context
                if let Some(num) = extract_number(context) {
                    metrics.push(FinancialMetric {
                        metric_type: metric_type.to_string(),
                        value: num,
                        unit: None,
                        period: None,
                    });
                }
            }
        }
    }

    metrics
}

/// Extract a number from text context
fn extract_number(text: &str) -> Option<String> {
    // Simple number extraction - would be more sophisticated in production
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::new();
    let mut found_digit = false;

    for ch in chars {
        if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            result.push(ch);
            found_digit = true;
        } else if found_digit && ch.is_whitespace() {
            break;
        }
    }

    if found_digit {
        Some(result)
    } else {
        None
    }
}

/// Check if text contains forward-looking statements
fn contains_forward_looking(text: &str) -> bool {
    let lower = text.to_lowercase();
    let forward_indicators = [
        "will",
        "expect",
        "expects",
        "anticipates",
        "projects",
        "plans",
        "intends",
        "believes",
        "estimated",
        "future",
        "outlook",
        "guidance",
        "forecast",
        "target",
    ];

    forward_indicators.iter().any(|&word| lower.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text() {
        let text = "This is a test. ".repeat(100);
        let chunks = chunk_text(&text, 200, 20);

        assert!(!chunks.is_empty());
        assert!(chunks[0].len() <= 200);
    }

    #[test]
    fn test_extract_financial_metrics() {
        let text = "The company reported revenue of $100 million and EPS of $2.50.";
        let metrics = extract_financial_metrics(text);

        assert!(!metrics.is_empty());
        assert!(metrics.iter().any(|m| m.metric_type == "revenue"));
    }

    #[test]
    fn test_contains_forward_looking() {
        assert!(contains_forward_looking("We expect revenue to grow"));
        assert!(contains_forward_looking("Future outlook is positive"));
        assert!(!contains_forward_looking("Revenue was $100M"));
    }
}
