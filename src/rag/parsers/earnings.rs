//! Earnings Call Parser
//!
//! S5-D7: Earnings Analyzer - FinBERT sentiment on transcripts

use super::{
    chunk_text, contains_forward_looking, extract_financial_metrics, DocumentChunk,
    DocumentMetadata, DocumentType, Result,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Parser for earnings call transcripts
pub struct EarningsParser;

/// A segment of an earnings call (prepared remarks or Q&A)
#[derive(Debug, Clone)]
pub struct EarningsSegment {
    pub speaker: String,
    pub speaker_role: SpeakerRole,
    pub content: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeakerRole {
    Ceo,
    Cfo,
    Cio,
    Executive,
    Analyst,
    Operator,
    Unknown,
}

impl EarningsParser {
    /// Create a new earnings parser
    pub fn new() -> Self {
        Self
    }

    /// Parse an earnings call transcript
    pub async fn parse(
        &self,
        transcript: &str,
        ticker: &str,
        call_date: DateTime<Utc>,
    ) -> Result<Vec<DocumentChunk>> {
        // Parse the transcript into segments
        let segments = self.parse_transcript(transcript);

        let mut chunks = Vec::new();
        let mut global_chunk_index = 0;

        // Combine prepared remarks and Q&A into sections
        let prepared_remarks: Vec<_> = segments
            .iter()
            .filter(|s| {
                matches!(
                    s.speaker_role,
                    SpeakerRole::Ceo | SpeakerRole::Cfo | SpeakerRole::Cio | SpeakerRole::Executive
                )
            })
            .map(|s| {
                format!(
                    "{} ({}): {}",
                    s.speaker,
                    self.role_to_string(&s.speaker_role),
                    s.content
                )
            })
            .collect();

        let qa_exchange: Vec<_> = segments
            .iter()
            .filter(|s| matches!(s.speaker_role, SpeakerRole::Analyst))
            .map(|s| format!("Analyst ({}): {}", s.speaker, s.content))
            .collect();

        // Process prepared remarks section
        if !prepared_remarks.is_empty() {
            let prepared_content = prepared_remarks.join("\n\n");
            let prepared_chunks =
                chunk_text(&prepared_content, super::CHUNK_SIZE, super::CHUNK_OVERLAP);

            for chunk_content in prepared_chunks {
                let metadata = DocumentMetadata {
                    section: Some("Prepared Remarks".to_string()),
                    page_number: None,
                    sentiment_score: None,
                    financial_metrics: extract_financial_metrics(&chunk_content),
                    contains_forward_looking: contains_forward_looking(&chunk_content),
                };

                chunks.push(DocumentChunk {
                    id: Uuid::new_v4(),
                    ticker: ticker.to_string(),
                    document_type: DocumentType::EarningsCall,
                    document_date: call_date,
                    source_url: None,
                    content: chunk_content,
                    embedding: None,
                    chunk_index: global_chunk_index,
                    total_chunks: 0,
                    metadata,
                    created_at: Utc::now(),
                });

                global_chunk_index += 1;
            }
        }

        // Process Q&A section
        if !qa_exchange.is_empty() {
            let qa_content = qa_exchange.join("\n\n");
            let qa_chunks = chunk_text(&qa_content, super::CHUNK_SIZE, super::CHUNK_OVERLAP);

            for chunk_content in qa_chunks {
                let metadata = DocumentMetadata {
                    section: Some("Q&A".to_string()),
                    page_number: None,
                    sentiment_score: None,
                    financial_metrics: extract_financial_metrics(&chunk_content),
                    contains_forward_looking: contains_forward_looking(&chunk_content),
                };

                chunks.push(DocumentChunk {
                    id: Uuid::new_v4(),
                    ticker: ticker.to_string(),
                    document_type: DocumentType::EarningsCall,
                    document_date: call_date,
                    source_url: None,
                    content: chunk_content,
                    embedding: None,
                    chunk_index: global_chunk_index,
                    total_chunks: 0,
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

    /// Parse transcript into segments with speaker identification
    fn parse_transcript(&self, transcript: &str) -> Vec<EarningsSegment> {
        let mut segments = Vec::new();
        let lines: Vec<&str> = transcript.lines().collect();

        let mut current_speaker: Option<String> = None;
        let mut current_role = SpeakerRole::Unknown;
        let mut current_content = String::new();
        let mut current_timestamp: Option<String> = None;

        for line in lines {
            let trimmed = line.trim();

            // Skip empty lines and operator instructions
            if trimmed.is_empty() || trimmed.starts_with('[') && trimmed.contains("Operator") {
                continue;
            }

            // Check for speaker line patterns
            // Common patterns: "John Smith - CEO", "Jane Doe:", "Operator:"
            if let Some(speaker_info) = self.extract_speaker(trimmed) {
                // Save previous segment if exists
                if let Some(speaker) = current_speaker.take() {
                    if !current_content.trim().is_empty() {
                        segments.push(EarningsSegment {
                            speaker,
                            speaker_role: current_role,
                            content: current_content.trim().to_string(),
                            timestamp: current_timestamp.clone(),
                        });
                    }
                }

                current_speaker = Some(speaker_info.name);
                current_role = speaker_info.role;
                current_timestamp = speaker_info.timestamp;
                current_content = speaker_info.remainder.unwrap_or_default();
            } else {
                // Continue current speaker's content
                current_content.push('\n');
                current_content.push_str(trimmed);
            }
        }

        // Don't forget the last segment
        if let Some(speaker) = current_speaker {
            if !current_content.trim().is_empty() {
                segments.push(EarningsSegment {
                    speaker,
                    speaker_role: current_role,
                    content: current_content.trim().to_string(),
                    timestamp: current_timestamp,
                });
            }
        }

        segments
    }

    /// Extract speaker information from a line
    fn extract_speaker(&self, line: &str) -> Option<SpeakerInfo> {
        // Pattern: "Name - Title" or "Name:" or "Name -- Title"
        let patterns = [
            r"^([A-Z][a-z]+\s+[A-Z][a-z]+)\s*[-–—]\s*(CEO|CFO|CIO|Chief|President|VP|Analyst)\s*:?\s*(.*)$",
            r"^([A-Z][a-z]+\s+[A-Z][a-z]+):\s*(.*)$",
            r"^([A-Z][a-z]+\s+[A-Z][a-z]+)\s*--\s*(.*)$",
        ];

        for _pattern in &patterns {
            // Simple pattern matching (in production would use regex)
            if line.contains(" - ") || line.contains(" -- ") {
                let parts: Vec<&str> = line.split(" - ").collect();
                if parts.len() >= 2 {
                    let name = parts[0].trim();
                    let rest = parts[1..].join(" - ");

                    let role = self.infer_role(&rest);
                    let remainder = if rest.contains(':') {
                        rest.split_once(':').map(|(_, r)| r.trim().to_string())
                    } else {
                        None
                    };

                    return Some(SpeakerInfo {
                        name: name.to_string(),
                        role,
                        timestamp: None,
                        remainder,
                    });
                }
            }

            if let Some((name, rest)) = line.split_once(':') {
                let name = name.trim();
                // Check if it looks like a name (starts with capital, contains space)
                if name
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_uppercase())
                    .unwrap_or(false)
                    && name.contains(' ')
                    && !name.to_lowercase().contains("operator")
                {
                    return Some(SpeakerInfo {
                        name: name.to_string(),
                        role: self.infer_role(name),
                        timestamp: None,
                        remainder: Some(rest.trim().to_string()),
                    });
                }
            }
        }

        None
    }

    /// Infer speaker role from context
    fn infer_role(&self, context: &str) -> SpeakerRole {
        let lower = context.to_lowercase();

        if lower.contains("analyst") || lower.contains("from") && lower.contains("capital") {
            SpeakerRole::Analyst
        } else if lower.contains("chief executive") || lower.contains("ceo") {
            SpeakerRole::Ceo
        } else if lower.contains("chief financial") || lower.contains("cfo") {
            SpeakerRole::Cfo
        } else if lower.contains("chief investment") || lower.contains("cio") {
            SpeakerRole::Cio
        } else if lower.contains("operator") {
            SpeakerRole::Operator
        } else if lower.contains("president") || lower.contains("vp") || lower.contains("executive")
        {
            SpeakerRole::Executive
        } else {
            SpeakerRole::Unknown
        }
    }

    fn role_to_string(&self, role: &SpeakerRole) -> &'static str {
        match role {
            SpeakerRole::Ceo => "CEO",
            SpeakerRole::Cfo => "CFO",
            SpeakerRole::Cio => "CIO",
            SpeakerRole::Executive => "Executive",
            SpeakerRole::Analyst => "Analyst",
            SpeakerRole::Operator => "Operator",
            SpeakerRole::Unknown => "Unknown",
        }
    }
}

#[derive(Debug)]
struct SpeakerInfo {
    name: String,
    role: SpeakerRole,
    timestamp: Option<String>,
    remainder: Option<String>,
}

impl Default for EarningsParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_speaker() {
        let parser = EarningsParser::new();

        let line = "John Smith - CEO: Welcome to our earnings call.";
        let info = parser.extract_speaker(line);

        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.name, "John Smith");
        assert!(matches!(info.role, SpeakerRole::Ceo));
    }

    #[test]
    fn test_parse_transcript() {
        let parser = EarningsParser::new();
        let transcript = r#"Operator: Good morning. Welcome to the Q1 earnings call.

Tim Cook - CEO: Thank you for joining us today.
We had a great quarter with revenue of $100 billion.

Luca Maestri - CFO: Let me walk through the financial details.

Analyst from Goldman Sachs: Can you discuss margins?

Tim Cook - CEO: Our margins improved significantly."#;

        let segments = parser.parse_transcript(transcript);

        assert!(!segments.is_empty());
        assert!(segments.iter().any(|s| s.speaker == "Tim Cook"));
        assert!(segments
            .iter()
            .any(|s| matches!(s.speaker_role, SpeakerRole::Ceo)));
    }

    #[tokio::test]
    async fn test_parse_earnings_call() {
        let parser = EarningsParser::new();
        let transcript = r#"Tim Cook - CEO: Welcome everyone.
We achieved record revenue of $100B this quarter.

Luca Maestri - CFO: Our EPS was $2.50."#;

        let chunks = parser.parse(transcript, "AAPL", Utc::now()).await.unwrap();

        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].ticker, "AAPL");
        assert_eq!(chunks[0].document_type, DocumentType::EarningsCall);
        assert!(chunks[0]
            .metadata
            .financial_metrics
            .iter()
            .any(|m| m.metric_type == "revenue"));
    }
}
