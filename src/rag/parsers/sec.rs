//! SEC Filings Parser
//!
//! Parses 10-K, 10-Q, and 8-K filings into structured chunks

use super::{chunk_text, contains_forward_looking, extract_financial_metrics, DocumentChunk, DocumentMetadata, DocumentType, Result};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Parser for SEC filings
pub struct SecParser;

impl SecParser {
    /// Create a new SEC parser
    pub fn new() -> Self {
        Self
    }
    
    /// Parse an SEC filing into chunks
    pub async fn parse(
        &self,
        content: &str,
        document_type: DocumentType,
        ticker: &str,
        filing_date: DateTime<Utc>,
    ) -> Result<Vec<DocumentChunk>> {
        // Extract sections based on document type
        let sections = match document_type {
            DocumentType::Form10K => self.extract_10k_sections(content),
            DocumentType::Form10Q => self.extract_10q_sections(content),
            DocumentType::Form8K => self.extract_8k_sections(content),
            _ => vec![("Full Text".to_string(), content.to_string())],
        };
        
        let mut chunks = Vec::new();
        let mut global_chunk_index = 0;
        
        for (section_name, section_content) in sections {
            // Chunk the section content
            let section_chunks = chunk_text(&section_content, super::CHUNK_SIZE, super::CHUNK_OVERLAP);
            let total_section_chunks = section_chunks.len();
            
            for (i, chunk_content) in section_chunks.into_iter().enumerate() {
                let metadata = DocumentMetadata {
                    section: Some(section_name.clone()),
                    page_number: None,
                    sentiment_score: None,
                    financial_metrics: extract_financial_metrics(&chunk_content),
                    contains_forward_looking: contains_forward_looking(&chunk_content),
                };
                
                chunks.push(DocumentChunk {
                    id: Uuid::new_v4(),
                    ticker: ticker.to_string(),
                    document_type,
                    document_date: filing_date,
                    source_url: None,
                    content: chunk_content,
                    embedding: None,
                    chunk_index: global_chunk_index,
                    total_chunks: 0, // Will be set after counting
                    metadata,
                    created_at: Utc::now(),
                });
                
                global_chunk_index += 1;
            }
        }
        
        // Update total chunk count
        let total = chunks.len();
        for chunk in &mut chunks {
            chunk.total_chunks = total;
        }
        
        Ok(chunks)
    }
    
    /// Extract sections from a 10-K filing
    fn extract_10k_sections(&self, content: &str) -> Vec<(String, String)> {
        let mut sections = Vec::new();
        
        // Common 10-K section markers
        let section_markers = [
            ("Business", vec!["ITEM 1.", "ITEM 1 - BUSINESS", "BUSINESS"]),
            ("Risk Factors", vec!["ITEM 1A.", "ITEM 1A - RISK FACTORS", "RISK FACTORS"]),
            ("Properties", vec!["ITEM 2.", "ITEM 2 - PROPERTIES"]),
            ("Legal Proceedings", vec!["ITEM 3.", "ITEM 3 - LEGAL PROCEEDINGS"]),
            ("MD&A", vec!["ITEM 7.", "ITEM 7 - MANAGEMENT", "MANAGEMENT'S DISCUSSION"]),
            ("Financial Statements", vec!["ITEM 8.", "ITEM 8 - FINANCIAL STATEMENTS"]),
            ("Controls", vec!["ITEM 9A.", "ITEM 9A - CONTROLS"]),
        ];
        
        let content_upper = content.to_uppercase();
        
        for (name, markers) in &section_markers {
            for marker in markers {
                if let Some(start_pos) = content_upper.find(&marker.to_uppercase()) {
                    // Find the next section marker
                    let section_start = start_pos;
                    let mut section_end = content.len();
                    
                    for (_, next_markers) in &section_markers {
                        for next_marker in next_markers {
                            if let Some(pos) = content_upper[section_start + marker.len()..].find(&next_marker.to_uppercase()) {
                                let absolute_pos = section_start + marker.len() + pos;
                                if absolute_pos < section_end {
                                    section_end = absolute_pos;
                                }
                            }
                        }
                    }
                    
                    let section_content = content[section_start..section_end].trim().to_string();
                    if section_content.len() > 100 {
                        sections.push((name.to_string(), section_content));
                    }
                    break;
                }
            }
        }
        
        // If no sections found, treat as single document
        if sections.is_empty() {
            sections.push(("Full Document".to_string(), content.to_string()));
        }
        
        sections
    }
    
    /// Extract sections from a 10-Q filing
    fn extract_10q_sections(&self, content: &str) -> Vec<(String, String)> {
        let mut sections = Vec::new();
        
        let section_markers = [
            ("Financial Statements", vec!["ITEM 1.", "FINANCIAL STATEMENTS"]),
            ("MD&A", vec!["ITEM 2.", "MANAGEMENT'S DISCUSSION"]),
            ("Controls", vec!["ITEM 4.", "CONTROLS AND PROCEDURES"]),
        ];
        
        let content_upper = content.to_uppercase();
        
        for (name, markers) in &section_markers {
            for marker in markers {
                if let Some(start_pos) = content_upper.find(&marker.to_uppercase()) {
                    let section_start = start_pos;
                    let mut section_end = content.len();
                    
                    for (_, next_markers) in &section_markers {
                        for next_marker in next_markers {
                            if let Some(pos) = content_upper[section_start + marker.len()..].find(&next_marker.to_uppercase()) {
                                let absolute_pos = section_start + marker.len() + pos;
                                if absolute_pos < section_end {
                                    section_end = absolute_pos;
                                }
                            }
                        }
                    }
                    
                    let section_content = content[section_start..section_end].trim().to_string();
                    if section_content.len() > 100 {
                        sections.push((name.to_string(), section_content));
                    }
                    break;
                }
            }
        }
        
        if sections.is_empty() {
            sections.push(("Full Document".to_string(), content.to_string()));
        }
        
        sections
    }
    
    /// Extract sections from an 8-K filing
    fn extract_8k_sections(&self, content: &str) -> Vec<(String, String)> {
        // 8-Ks are typically event-driven, extract based on item numbers
        let mut sections = Vec::new();
        
        let item_markers: Vec<(String, Vec<&str>)> = (1..=9)
            .map(|i| (format!("Item {}", i), vec![&format!("ITEM {}.", i), &format!("ITEM {}", i)]))
            .collect();
        
        let content_upper = content.to_uppercase();
        
        for (name, markers) in &item_markers {
            for marker in markers {
                if let Some(start_pos) = content_upper.find(&marker.to_uppercase()) {
                    let section_start = start_pos;
                    let mut section_end = content.len();
                    
                    for (next_name, _) in &item_markers {
                        if next_name != name {
                            for next_marker in &[&format!("ITEM {}.", next_name.chars().last().unwrap()), &format!("ITEM {}", next_name.chars().last().unwrap())] {
                                if let Some(pos) = content_upper[section_start + marker.len()..].find(&next_marker.to_uppercase()) {
                                    let absolute_pos = section_start + marker.len() + pos;
                                    if absolute_pos < section_end {
                                        section_end = absolute_pos;
                                    }
                                }
                            }
                        }
                    }
                    
                    let section_content = content[section_start..section_end].trim().to_string();
                    if section_content.len() > 50 {
                        sections.push((name.clone(), section_content));
                    }
                    break;
                }
            }
        }
        
        if sections.is_empty() {
            sections.push(("Full Document".to_string(), content.to_string()));
        }
        
        sections
    }
}

impl Default for SecParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_10k_sections() {
        let parser = SecParser::new();
        let content = r#"
ITEM 1. BUSINESS
We are a technology company.

ITEM 1A. RISK FACTORS
There are many risks.

ITEM 7. MANAGEMENT'S DISCUSSION
Revenue increased.
"#;
        
        let sections = parser.extract_10k_sections(content);
        
        assert!(!sections.is_empty());
        assert!(sections.iter().any(|(name, _)| name == "Business"));
        assert!(sections.iter().any(|(name, _)| name == "Risk Factors"));
    }

    #[tokio::test]
    async fn test_parse_10k() {
        let parser = SecParser::new();
        let content = "ITEM 1. BUSINESS\n\nWe operate in tech. Revenue was $100M.\n\nITEM 1A. RISK FACTORS\n\nMarket risks exist.";
        
        let chunks = parser.parse(
            content,
            DocumentType::Form10K,
            "AAPL",
            Utc::now(),
        ).await.unwrap();
        
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].ticker, "AAPL");
        assert_eq!(chunks[0].document_type, DocumentType::Form10K);
    }
}
