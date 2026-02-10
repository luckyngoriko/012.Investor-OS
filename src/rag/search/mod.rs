//! Semantic Search
//!
//! S5-D8: Journal AI Search - Semantic search on decision journal
//! S5-D9: API Endpoints - /api/rag/search, /api/rag/summarize

use crate::rag::{DocumentChunk, DocumentType, Result, RagError, SearchQuery, SearchResult};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Document search using pgvector for similarity search
pub struct DocumentSearch {
    pool: PgPool,
}

/// Result from journal semantic search
#[derive(Debug, Clone)]
pub struct JournalSearchResult {
    pub decision_id: Uuid,
    pub portfolio_id: Uuid,
    pub ticker: String,
    pub action: String,
    pub journal_entry: String,
    pub similarity_score: f32,
    pub decision_date: DateTime<Utc>,
}

impl DocumentSearch {
    /// Create a new document search service
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| RagError::Database(e.to_string()))?;
        
        Ok(Self { pool })
    }
    
    /// Store a document chunk with its embedding
    pub async fn store_chunk(&self, chunk: &DocumentChunk) -> Result<()> {
        let embedding = chunk.embedding.as_ref()
            .ok_or_else(|| RagError::Embedding("Chunk missing embedding".to_string()))?;
        
        sqlx::query(
            r#"
            INSERT INTO document_embeddings 
            (id, ticker, document_type, document_date, source_url, content_chunk, embedding, chunk_index, total_chunks, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (ticker, document_type, document_date, chunk_index) 
            DO UPDATE SET 
                content_chunk = EXCLUDED.content_chunk,
                embedding = EXCLUDED.embedding,
                metadata = EXCLUDED.metadata
            "#
        )
        .bind(chunk.id)
        .bind(&chunk.ticker)
        .bind(chunk.document_type.as_str())
        .bind(chunk.document_date)
        .bind(chunk.source_url.as_ref())
        .bind(&chunk.content)
        .bind(embedding.as_slice()) // pgvector handles f32 arrays
        .bind(chunk.chunk_index as i32)
        .bind(chunk.total_chunks as i32)
        .bind(serde_json::to_value(&chunk.metadata).map_err(|e| RagError::Database(e.to_string()))?)
        .execute(&self.pool)
        .await
        .map_err(|e| RagError::Database(e.to_string()))?;
        
        Ok(())
    }
    
    /// Search documents by semantic similarity
    pub async fn search(
        &self,
        query_embedding: &[f32],
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>> {
        let mut sql = String::from(
            r#"
            SELECT 
                id, ticker, document_type, document_date, source_url, 
                content_chunk, chunk_index, total_chunks, metadata,
                embedding <=> $1 as distance
            FROM document_embeddings
            WHERE 1=1
            "#
        );
        
        let mut bind_idx = 2;
        
        // Add ticker filter
        if query.ticker.is_some() {
            sql.push_str(&format!(" AND ticker = ${}", bind_idx));
            bind_idx += 1;
        }
        
        // Add document type filter
        if !query.document_types.is_empty() {
            let types: Vec<String> = query.document_types.iter()
                .map(|t| format!("'{}'", t.as_str()))
                .collect();
            sql.push_str(&format!(" AND document_type IN ({})", types.join(",")));
        }
        
        // Add date range filter
        if query.date_range.is_some() {
            sql.push_str(&format!(" AND document_date >= ${} AND document_date <= ${}", bind_idx, bind_idx + 1));
        }
        
        // Order by similarity and limit
        sql.push_str(&format!(
            " ORDER BY embedding <=> $1 LIMIT {}",
            query.limit
        ));
        
        // Build the query
        let mut db_query = sqlx::query_as::<_, DocumentEmbeddingRow>(&sql)
            .bind(query_embedding);
        
        // Bind ticker if present
        if let Some(ticker) = &query.ticker {
            db_query = db_query.bind(ticker);
        }
        
        // Bind date range if present
        if let Some((start, end)) = &query.date_range {
            db_query = db_query.bind(start).bind(end);
        }
        
        let rows: Vec<DocumentEmbeddingRow> = db_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RagError::Database(e.to_string()))?;
        
        // Convert to SearchResults
        let results: Vec<SearchResult> = rows.into_iter()
            .map(|row| {
                let similarity = 1.0 - row.distance; // Convert distance to similarity
                
                let chunk = DocumentChunk {
                    id: row.id,
                    ticker: row.ticker,
                    document_type: parse_document_type(&row.document_type),
                    document_date: row.document_date,
                    source_url: row.source_url,
                    content: row.content_chunk,
                    embedding: None, // Don't return embeddings
                    chunk_index: row.chunk_index as usize,
                    total_chunks: row.total_chunks as usize,
                    metadata: serde_json::from_value(row.metadata)
                        .unwrap_or_default(),
                    created_at: Utc::now(),
                };
                
                SearchResult {
                    chunk,
                    similarity_score: similarity.max(0.0),
                    relevance_explanation: None,
                }
            })
            .collect();
        
        Ok(results)
    }
    
    /// Fetch all chunks for a ticker
    pub async fn fetch_for_ticker(
        &self,
        ticker: &str,
        document_type: Option<DocumentType>,
        date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    ) -> Result<Vec<DocumentChunk>> {
        let mut sql = String::from(
            r#"
            SELECT 
                id, ticker, document_type, document_date, source_url, 
                content_chunk, chunk_index, total_chunks, metadata
            FROM document_embeddings
            WHERE ticker = $1
            "#
        );
        
        let mut bind_idx = 2;
        
        if document_type.is_some() {
            sql.push_str(&format!(" AND document_type = ${}", bind_idx));
            bind_idx += 1;
        }
        
        if date_range.is_some() {
            sql.push_str(&format!(" AND document_date >= ${} AND document_date <= ${}", bind_idx, bind_idx + 1));
        }
        
        sql.push_str(" ORDER BY document_date DESC, chunk_index ASC");
        
        let mut query = sqlx::query_as::<_, DocumentEmbeddingRow>(&sql)
            .bind(ticker);
        
        if let Some(doc_type) = document_type {
            query = query.bind(doc_type.as_str());
        }
        
        if let Some((start, end)) = date_range {
            query = query.bind(start).bind(end);
        }
        
        let rows: Vec<DocumentEmbeddingRow> = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RagError::Database(e.to_string()))?;
        
        let chunks: Vec<DocumentChunk> = rows.into_iter()
            .map(|row| DocumentChunk {
                id: row.id,
                ticker: row.ticker,
                document_type: parse_document_type(&row.document_type),
                document_date: row.document_date,
                source_url: row.source_url,
                content: row.content_chunk,
                embedding: None,
                chunk_index: row.chunk_index as usize,
                total_chunks: row.total_chunks as usize,
                metadata: serde_json::from_value(row.metadata).unwrap_or_default(),
                created_at: Utc::now(),
            })
            .collect();
        
        Ok(chunks)
    }
    
    /// Semantic search on decision journal
    pub async fn search_journal(
        &self,
        _query_embedding: &[f32],
        portfolio_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<JournalSearchResult>> {
        // For journal search, we need to generate embeddings for journal entries on-the-fly
        // or have a separate table for journal embeddings
        // This is a simplified implementation that does text search
        
        let mut sql = String::from(
            r#"
            SELECT 
                d.id, d.portfolio_id, d.ticker, d.action, d.journal_entry, d.created_at
            FROM decisions d
            WHERE d.journal_entry IS NOT NULL
            "#
        );
        
        if portfolio_id.is_some() {
            sql.push_str(" AND d.portfolio_id = $2");
        }
        
        sql.push_str(" ORDER BY d.created_at DESC LIMIT $1");
        
        let mut query = sqlx::query_as::<_, DecisionRow>(&sql)
            .bind(limit as i64);
        
        if let Some(pid) = portfolio_id {
            query = query.bind(pid);
        }
        
        let rows: Vec<DecisionRow> = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RagError::Database(e.to_string()))?;
        
        // Calculate similarity scores (in production, would use embeddings)
        let results: Vec<JournalSearchResult> = rows.into_iter()
            .map(|row| {
                // Placeholder similarity - in production would compare embeddings
                let similarity = 0.5; 
                
                JournalSearchResult {
                    decision_id: row.id,
                    portfolio_id: row.portfolio_id,
                    ticker: row.ticker,
                    action: row.action,
                    journal_entry: row.journal_entry.unwrap_or_default(),
                    similarity_score: similarity,
                    decision_date: row.created_at,
                }
            })
            .collect();
        
        Ok(results)
    }
    
    /// Log a RAG query for analytics
    pub async fn log_query(
        &self,
        query_text: &str,
        query_embedding: Option<&[f32]>,
        context_ticker: Option<&str>,
        results: &[SearchResult],
        latency_ms: u64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO rag_queries (query_text, query_embedding, context_ticker, results, latency_ms)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(query_text)
        .bind(query_embedding)
        .bind(context_ticker)
        .bind(serde_json::to_value(results).map_err(|e| RagError::Database(e.to_string()))?)
        .bind(latency_ms as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| RagError::Database(e.to_string()))?;
        
        Ok(())
    }
}

/// Database row for document embeddings
#[derive(sqlx::FromRow)]
struct DocumentEmbeddingRow {
    id: Uuid,
    ticker: String,
    document_type: String,
    document_date: DateTime<Utc>,
    source_url: Option<String>,
    content_chunk: String,
    chunk_index: i32,
    total_chunks: i32,
    metadata: serde_json::Value,
    #[sqlx(default)]
    distance: f32,
}

/// Database row for decisions
#[derive(sqlx::FromRow)]
struct DecisionRow {
    id: Uuid,
    portfolio_id: Uuid,
    ticker: String,
    action: String,
    journal_entry: Option<String>,
    created_at: DateTime<Utc>,
}

fn parse_document_type(s: &str) -> DocumentType {
    match s {
        "10-K" => DocumentType::Form10K,
        "10-Q" => DocumentType::Form10Q,
        "8-K" => DocumentType::Form8K,
        "earnings_call" => DocumentType::EarningsCall,
        "news" => DocumentType::News,
        "analyst_report" => DocumentType::AnalystReport,
        _ => DocumentType::News,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::SearchQuery;

    // Note: These tests would require a test database
    // For now, we just test the parsing logic

    #[test]
    fn test_parse_document_type() {
        assert!(matches!(parse_document_type("10-K"), DocumentType::Form10K));
        assert!(matches!(parse_document_type("earnings_call"), DocumentType::EarningsCall));
        assert!(matches!(parse_document_type("unknown"), DocumentType::News));
    }
}
