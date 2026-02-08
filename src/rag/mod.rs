//! RAG (Retrieval-Augmented Generation) Module
//!
//! Sprint 5 Deliverables:
//! - S5-D6: SEC Filings Parser (10-K, 10-Q extraction)
//! - S5-D7: Earnings Analyzer (FinBERT sentiment)
//! - S5-D8: Journal AI Search (semantic search)
//! - S5-D9: API Endpoints (/api/rag/search, /api/rag/summarize)

pub mod embeddings;
pub mod parsers;
pub mod search;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur in the RAG module
#[derive(Error, Debug)]
pub enum RagError {
    #[error("Parser error: {0}")]
    Parser(String),
    
    #[error("Embedding generation failed: {0}")]
    Embedding(String),
    
    #[error("Search error: {0}")]
    Search(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("External API error: {0}")]
    ExternalApi(String),
}

/// Result type for RAG operations
pub type Result<T> = std::result::Result<T, RagError>;

/// Document types supported by the RAG system
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    /// Annual report (Form 10-K)
    Form10K,
    /// Quarterly report (Form 10-Q)
    Form10Q,
    /// Current report (Form 8-K)
    Form8K,
    /// Earnings call transcript
    EarningsCall,
    /// News article
    News,
    /// Analyst report
    AnalystReport,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentType::Form10K => "10-K",
            DocumentType::Form10Q => "10-Q",
            DocumentType::Form8K => "8-K",
            DocumentType::EarningsCall => "earnings_call",
            DocumentType::News => "news",
            DocumentType::AnalystReport => "analyst_report",
        }
    }
}

/// A chunk of a financial document with its embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: Uuid,
    pub ticker: String,
    pub document_type: DocumentType,
    pub document_date: DateTime<Utc>,
    pub source_url: Option<String>,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub metadata: DocumentMetadata,
    pub created_at: DateTime<Utc>,
}

/// Metadata for a document chunk
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Section of the document (e.g., "Risk Factors", "MD&A")
    pub section: Option<String>,
    /// Page number if available
    pub page_number: Option<u32>,
    /// Sentiment score if analyzed
    pub sentiment_score: Option<f32>,
    /// Key financial metrics mentioned
    pub financial_metrics: Vec<FinancialMetric>,
    /// Forward-looking statements flag
    pub contains_forward_looking: bool,
}

/// Financial metric extracted from a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialMetric {
    pub metric_type: String,
    pub value: String,
    pub unit: Option<String>,
    pub period: Option<String>,
}

/// Query for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub ticker: Option<String>,
    pub document_types: Vec<DocumentType>,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub limit: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            ticker: None,
            document_types: vec![],
            date_range: None,
            limit: 10,
        }
    }
}

/// Search result from semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: DocumentChunk,
    pub similarity_score: f32,
    pub relevance_explanation: Option<String>,
}

/// Summary of a document or collection of documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSummary {
    pub ticker: String,
    pub document_type: DocumentType,
    pub document_date: DateTime<Utc>,
    pub summary: String,
    pub key_points: Vec<String>,
    pub sentiment_analysis: Option<SentimentAnalysis>,
    pub risk_factors: Vec<String>,
    pub opportunities: Vec<String>,
}

/// Sentiment analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentAnalysis {
    pub overall_score: f32, // -1.0 to 1.0
    pub confidence: f32,
    pub label: SentimentLabel,
    pub breakdown: Vec<SentimentSegment>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SentimentLabel {
    Positive,
    Neutral,
    Negative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentSegment {
    pub text: String,
    pub score: f32,
    pub label: SentimentLabel,
}

/// RAG service for document processing and search
pub struct RagService {
    embedding_generator: embeddings::EmbeddingGenerator,
    document_search: search::DocumentSearch,
    sec_parser: parsers::SecParser,
    earnings_parser: parsers::EarningsParser,
}

impl RagService {
    /// Create a new RAG service
    pub async fn new(database_url: &str) -> Result<Self> {
        let embedding_generator = embeddings::EmbeddingGenerator::new().await?;
        let document_search = search::DocumentSearch::new(database_url).await?;
        let sec_parser = parsers::SecParser::new();
        let earnings_parser = parsers::EarningsParser::new();
        
        Ok(Self {
            embedding_generator,
            document_search,
            sec_parser,
            earnings_parser,
        })
    }
    
    /// Process and index an SEC filing
    pub async fn process_sec_filing(
        &self,
        ticker: &str,
        document_type: DocumentType,
        content: &str,
        filing_date: DateTime<Utc>,
    ) -> Result<Vec<DocumentChunk>> {
        // Parse the filing
        let chunks = match document_type {
            DocumentType::Form10K | DocumentType::Form10Q | DocumentType::Form8K => {
                self.sec_parser.parse(content, document_type, ticker, filing_date).await?
            }
            _ => return Err(RagError::Parser(format!(
                "Document type {:?} not supported for SEC parsing", 
                document_type
            ))),
        };
        
        // Generate embeddings and store
        let mut chunks_with_embeddings = Vec::new();
        for chunk in chunks {
            let embedding = self.embedding_generator.generate(&chunk.content).await?;
            let chunk_with_embedding = DocumentChunk {
                embedding: Some(embedding),
                ..chunk
            };
            
            self.document_search.store_chunk(&chunk_with_embedding).await?;
            chunks_with_embeddings.push(chunk_with_embedding);
        }
        
        Ok(chunks_with_embeddings)
    }
    
    /// Process and index an earnings call transcript
    pub async fn process_earnings_call(
        &self,
        ticker: &str,
        transcript: &str,
        call_date: DateTime<Utc>,
    ) -> Result<Vec<DocumentChunk>> {
        // Parse the transcript
        let chunks = self.earnings_parser.parse(transcript, ticker, call_date).await?;
        
        // Generate embeddings and store
        let mut chunks_with_embeddings = Vec::new();
        for chunk in chunks {
            let embedding = self.embedding_generator.generate(&chunk.content).await?;
            let chunk_with_embedding = DocumentChunk {
                embedding: Some(embedding),
                ..chunk
            };
            
            self.document_search.store_chunk(&chunk_with_embedding).await?;
            chunks_with_embeddings.push(chunk_with_embedding);
        }
        
        Ok(chunks_with_embeddings)
    }
    
    /// Search documents semantically
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // Generate embedding for query
        let query_embedding = self.embedding_generator.generate(&query.query).await?;
        
        // Search in database
        let results = self.document_search
            .search(&query_embedding, query)
            .await?;
        
        Ok(results)
    }
    
    /// Summarize documents for a ticker
    pub async fn summarize(
        &self,
        ticker: &str,
        document_type: Option<DocumentType>,
        date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    ) -> Result<Vec<DocumentSummary>> {
        // Fetch relevant chunks
        let chunks = self.document_search
            .fetch_for_ticker(ticker, document_type, date_range)
            .await?;
        
        // Generate summaries (would use LLM in production)
        let summaries = self.generate_summaries(chunks).await?;
        
        Ok(summaries)
    }
    
    /// Semantic search on decision journal
    pub async fn search_journal(
        &self,
        query: &str,
        portfolio_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<search::JournalSearchResult>> {
        let query_embedding = self.embedding_generator.generate(query).await?;
        
        self.document_search
            .search_journal(&query_embedding, portfolio_id, limit)
            .await
    }
    
    async fn generate_summaries(&self, chunks: Vec<DocumentChunk>) -> Result<Vec<DocumentSummary>> {
        // Group chunks by document
        use std::collections::HashMap;
        
        let mut by_document: HashMap<(String, DocumentType, DateTime<Utc>), Vec<DocumentChunk>> = HashMap::new();
        
        for chunk in chunks {
            let key = (chunk.ticker.clone(), chunk.document_type, chunk.document_date);
            by_document.entry(key).or_default().push(chunk);
        }
        
        // Generate summary for each document
        let mut summaries = Vec::new();
        for ((ticker, doc_type, date), doc_chunks) in by_document {
            // Sort chunks by index
            let mut sorted_chunks = doc_chunks;
            sorted_chunks.sort_by_key(|c| c.chunk_index);
            
            // Combine content
            let full_content: String = sorted_chunks.iter()
                .map(|c| c.content.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
            
            // Generate summary (simplified - would use LLM)
            let summary = format!(
                "Summary of {} {} filing from {} ({} chunks)",
                ticker,
                doc_type.as_str(),
                date.format("%Y-%m-%d"),
                sorted_chunks.len()
            );
            
            summaries.push(DocumentSummary {
                ticker,
                document_type: doc_type,
                document_date: date,
                summary,
                key_points: vec![],
                sentiment_analysis: None,
                risk_factors: vec![],
                opportunities: vec![],
            });
        }
        
        Ok(summaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_type_as_str() {
        assert_eq!(DocumentType::Form10K.as_str(), "10-K");
        assert_eq!(DocumentType::EarningsCall.as_str(), "earnings_call");
    }

    #[test]
    fn test_search_query_default() {
        let query = SearchQuery::default();
        assert!(query.query.is_empty());
        assert!(query.ticker.is_none());
        assert_eq!(query.limit, 10);
    }
}
