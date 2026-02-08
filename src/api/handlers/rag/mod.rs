//! RAG API Handlers

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::AppState;
use crate::api::handlers::ApiResponse;
use crate::rag::{DocumentType, SearchQuery};

/// Default limit for search results
pub fn default_limit() -> usize {
    10
}

// ==================== RAG Endpoints (S5-D9) ====================

#[derive(Serialize, Deserialize)]
pub struct RagSearchRequest {
    pub query: String,
    pub ticker: Option<String>,
    pub document_types: Vec<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Serialize)]
pub struct RagSearchResponse {
    pub query: String,
    pub results: Vec<SearchResultView>,
    pub total_results: usize,
    pub search_time_ms: u64,
}

#[derive(Serialize)]
pub struct SearchResultView {
    pub ticker: String,
    pub document_type: String,
    pub document_date: String,
    pub content: String,
    pub similarity_score: f32,
    pub section: Option<String>,
}

/// POST /api/rag/search - Semantic search across documents
pub async fn rag_search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RagSearchRequest>,
) -> Result<Json<ApiResponse<RagSearchResponse>>, StatusCode> {
    let start = std::time::Instant::now();
    
    // Parse document types
    let doc_types: Vec<DocumentType> = req.document_types.iter()
        .filter_map(|t| match t.as_str() {
            "10-K" => Some(DocumentType::Form10K),
            "10-Q" => Some(DocumentType::Form10Q),
            "8-K" => Some(DocumentType::Form8K),
            "earnings_call" => Some(DocumentType::EarningsCall),
            "news" => Some(DocumentType::News),
            _ => None,
        })
        .collect();
    
    let query = SearchQuery {
        query: req.query.clone(),
        ticker: req.ticker,
        document_types: doc_types,
        date_range: None,
        limit: req.limit,
    };
    
    match state.rag_service.search(&query).await {
        Ok(results) => {
            let search_time = start.elapsed().as_millis() as u64;
            
            let views: Vec<SearchResultView> = results.iter()
                .map(|r| SearchResultView {
                    ticker: r.chunk.ticker.clone(),
                    document_type: r.chunk.document_type.as_str().to_string(),
                    document_date: r.chunk.document_date.format("%Y-%m-%d").to_string(),
                    content: r.chunk.content.clone(),
                    similarity_score: r.similarity_score,
                    section: r.chunk.metadata.section.clone(),
                })
                .collect();
            
            let response = RagSearchResponse {
                query: req.query,
                results: views,
                total_results: results.len(),
                search_time_ms: search_time,
            };
            
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("RAG search error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RagSummarizeRequest {
    pub ticker: String,
    pub document_type: Option<String>,
}

#[derive(Serialize)]
pub struct RagSummarizeResponse {
    pub ticker: String,
    pub summaries: Vec<SummaryView>,
}

#[derive(Serialize)]
pub struct SummaryView {
    pub document_type: String,
    pub document_date: String,
    pub summary: String,
    pub key_points: Vec<String>,
}

/// POST /api/rag/summarize - Summarize documents for a ticker
pub async fn rag_summarize(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RagSummarizeRequest>,
) -> Result<Json<ApiResponse<RagSummarizeResponse>>, StatusCode> {
    let doc_type = req.document_type.as_ref().and_then(|t| match t.as_str() {
        "10-K" => Some(DocumentType::Form10K),
        "10-Q" => Some(DocumentType::Form10Q),
        "8-K" => Some(DocumentType::Form8K),
        "earnings_call" => Some(DocumentType::EarningsCall),
        _ => None,
    });
    
    match state.rag_service.summarize(&req.ticker, doc_type, None).await {
        Ok(summaries) => {
            let views: Vec<SummaryView> = summaries.iter()
                .map(|s| SummaryView {
                    document_type: s.document_type.as_str().to_string(),
                    document_date: s.document_date.format("%Y-%m-%d").to_string(),
                    summary: s.summary.clone(),
                    key_points: s.key_points.clone(),
                })
                .collect();
            
            let response = RagSummarizeResponse {
                ticker: req.ticker,
                summaries: views,
            };
            
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("RAG summarize error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct JournalSearchRequest {
    pub query: String,
    pub portfolio_id: Option<uuid::Uuid>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Serialize)]
pub struct JournalSearchResponse {
    pub query: String,
    pub results: Vec<JournalResultView>,
}

#[derive(Serialize)]
pub struct JournalResultView {
    pub decision_id: uuid::Uuid,
    pub ticker: String,
    pub action: String,
    pub journal_entry: String,
    pub similarity_score: f32,
    pub decision_date: String,
}

/// POST /api/rag/journal-search - Semantic search on decision journal
pub async fn rag_journal_search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JournalSearchRequest>,
) -> Result<Json<ApiResponse<JournalSearchResponse>>, StatusCode> {
    match state.rag_service.search_journal(&req.query, req.portfolio_id, req.limit).await {
        Ok(results) => {
            let views: Vec<JournalResultView> = results.iter()
                .map(|r| JournalResultView {
                    decision_id: r.decision_id,
                    ticker: r.ticker.clone(),
                    action: r.action.clone(),
                    journal_entry: r.journal_entry.clone(),
                    similarity_score: r.similarity_score,
                    decision_date: r.decision_date.format("%Y-%m-%d").to_string(),
                })
                .collect();
            
            let response = JournalSearchResponse {
                query: req.query,
                results: views,
            };
            
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Journal search error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

// ==================== SEC Filings Endpoint ====================

#[derive(Serialize, Deserialize)]
pub struct SecFilingRequest {
    pub ticker: String,
    pub document_type: String,
    pub content: String,
    pub filing_date: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct SecFilingResponse {
    pub ticker: String,
    pub chunks_processed: usize,
    pub document_type: String,
}

/// POST /api/rag/sec-filings - Process and index an SEC filing
pub async fn process_sec_filing(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SecFilingRequest>,
) -> Result<Json<ApiResponse<SecFilingResponse>>, StatusCode> {
    let doc_type = match req.document_type.as_str() {
        "10-K" => DocumentType::Form10K,
        "10-Q" => DocumentType::Form10Q,
        "8-K" => DocumentType::Form8K,
        _ => {
            return Ok(Json(ApiResponse::error(format!(
                "Unsupported document type: {}",
                req.document_type
            ))));
        }
    };
    
    match state.rag_service.process_sec_filing(
        &req.ticker,
        doc_type,
        &req.content,
        req.filing_date,
    ).await {
        Ok(chunks) => {
            let response = SecFilingResponse {
                ticker: req.ticker,
                chunks_processed: chunks.len(),
                document_type: req.document_type,
            };
            
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("SEC filing processing error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

// ==================== Earnings Endpoint ====================

#[derive(Serialize, Deserialize)]
pub struct EarningsRequest {
    pub ticker: String,
    pub transcript: String,
    pub call_date: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct EarningsResponse {
    pub ticker: String,
    pub chunks_processed: usize,
}

/// POST /api/rag/earnings - Process and index an earnings call transcript
pub async fn process_earnings_call(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EarningsRequest>,
) -> Result<Json<ApiResponse<EarningsResponse>>, StatusCode> {
    match state.rag_service.process_earnings_call(
        &req.ticker,
        &req.transcript,
        req.call_date,
    ).await {
        Ok(chunks) => {
            let response = EarningsResponse {
                ticker: req.ticker,
                chunks_processed: chunks.len(),
            };
            
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Earnings processing error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}
