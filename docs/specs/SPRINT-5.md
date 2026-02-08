# Sprint 5: RAG Integration - Financial Intelligence

> **Duration:** Week 9-10
> **Goal:** Integrate neurocod-rag for SEC filings, earnings analysis, and decision journal AI search
> **Ref:** [SPEC-v1.0](./SPEC-v1.0.md) | [Golden Path](./GOLDEN-PATH.md) | [ROADMAP](../ROADMAP.md)

---

## Scope

### ✅ In Scope
- neurocod-rag crate integration
- SEC filings (10-K, 10-Q) ingestion and chunking
- Earnings call transcript analysis
- Vector embeddings with pgvector
- Decision journal semantic search
- Financial document Q&A
- Source attribution for AI responses

### ❌ Out of Scope
- Real-time news ingestion
- Social media sentiment analysis (already in Sprint 2)
- Multi-modal RAG (images, charts)
- Fine-tuning LLMs

---

## Deliverables

| ID | Deliverable | Acceptance Criteria |
|----|-------------|---------------------|
| S5-D1 | neurocod-rag integration | Crate dependency, connection pool |
| S5-D2 | SEC Filings ingestion | Download and parse 10-K, 10-Q from EDGAR |
| S5-D3 | Document chunker | Financial-aware chunking (sections, paragraphs) |
| S5-D4 | Vector embeddings | pgvector extension, embeddings storage |
| S5-D5 | RAG API endpoints | `/api/rag/query`, `/api/rag/ingest` |
| S5-D6 | Journal AI search | Semantic search past decisions by description |
| S5-D7 | Earnings Q&A | Ask questions about earnings transcripts |
| S5-D8 | Source attribution | Citations to source documents |

---

## Technical Implementation

### S5-D1: neurocod-rag Integration

```toml
# crates/investor-rag/Cargo.toml
[dependencies]
neurocod-rag = { path = "../../../neurocod-rag", features = ["pgvector"] }
pgvector = { version = "0.3", features = ["sqlx"] }
```

```rust
// crates/investor-rag/src/lib.rs
pub struct FinancialRag {
    client: neurocod_rag::Client,
    chunker: FinancialChunker,
}

impl FinancialRag {
    pub async fn query(&self, ticker: &str, question: &str) -> Result<RagResponse> {
        let context = self.client.search(
            &format!("{} {}", ticker, question),
            SearchOptions::default().limit(5)
        ).await?;
        
        self.generate_response(context, question).await
    }
}
```

### S5-D2: SEC Filings Ingestion

```rust
// crates/investor-collectors/src/sec_edgar.rs
pub struct SecEdgarClient {
    client: reqwest::Client,
}

impl SecEdgarClient {
    pub async fn fetch_10k(&self, ticker: &str, year: i32) -> Result<Filing10K> {
        // Download from EDGAR
        // Parse XBRL/XML
        // Extract: Business, Risk Factors, MD&A, Financials
    }
}
```

### S5-D3: Financial Document Chunker

```rust
pub struct FinancialChunker {
    max_chunk_size: usize,
    overlap: usize,
}

impl FinancialChunker {
    pub fn chunk_10k(&self, filing: &Filing10K) -> Vec<Chunk> {
        vec![
            Chunk::new("business", &filing.business_description),
            Chunk::new("risk_factors", &filing.risk_factors),
            Chunk::new("mdna", &filing.management_discussion),
            // ...
        ]
    }
}
```

### S5-D4: Vector Embeddings

```sql
-- migrations/002_pgvector.sql
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE document_embeddings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    ticker TEXT NOT NULL,
    document_type TEXT NOT NULL, -- '10-K', '10-Q', 'earnings'
    chunk_id TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding vector(1536), -- OpenAI text-embedding-3-small
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_embeddings_vector ON document_embeddings 
USING ivfflat (embedding vector_cosine_ops);

CREATE INDEX idx_embeddings_ticker ON document_embeddings (ticker);
```

### S5-D5: RAG API Endpoints

```rust
// crates/investor-api/src/handlers/rag.rs
pub async fn rag_query(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RagQueryRequest>,
) -> Json<ApiResponse<RagResponse>> {
    let response = state.rag.query(&req.ticker, &req.question).await?;
    
    Json(ApiResponse::success(response))
}

pub async fn rag_ingest(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RagIngestRequest>,
) -> Json<ApiResponse<IngestResponse>> {
    let chunks = state.chunker.chunk(&req.document);
    let embedded = state.embedder.embed(chunks).await?;
    state.rag.store(embedded).await?;
    
    Json(ApiResponse::success(IngestResponse { count: chunks.len() }))
}
```

### S5-D6: Journal Semantic Search

```rust
pub async fn search_journal(
    &self,
    query: &str,
) -> Result<Vec<JournalEntry>> {
    // Embed the query
    let query_embedding = self.embedder.embed_text(query).await?;
    
    // Find similar journal entries
    let similar = sqlx::query_as::<_, JournalEntry>(
        r#"
        SELECT j.* 
        FROM decision_journal j
        JOIN journal_embeddings je ON j.id = je.journal_id
        ORDER BY je.embedding <-> $1
        LIMIT 10
        "#
    )
    .bind(query_embedding)
    .fetch_all(&self.pool)
    .await?;
    
    Ok(similar)
}
```

### S5-D7: Earnings Q&A

```rust
pub async fn earnings_qa(
    &self,
    ticker: &str,
    quarter: &str,
    question: &str,
) -> Result<QAResponse> {
    // Load earnings transcript
    let transcript = self.load_transcript(ticker, quarter).await?;
    
    // Chunk by Q&A pairs
    let qa_pairs = self.chunker.chunk_earnings(&transcript);
    
    // Find relevant Q&A
    let relevant = self.find_relevant_qa(qa_pairs, question).await?;
    
    // Generate answer
    self.generate_answer(relevant, question).await
}
```

---

## Golden Path Tests

### S5-GP-01: SEC Filing Ingestion
```rust
#[tokio::test]
async fn test_sec_filing_ingestion() {
    let client = SecEdgarClient::new();
    let filing = client.fetch_10k("AAPL", 2024).await.unwrap();
    
    assert!(!filing.business_description.is_empty());
    assert!(!filing.risk_factors.is_empty());
}
```

### S5-GP-02: Document Chunking
```rust
#[test]
fn test_financial_chunker() {
    let chunker = FinancialChunker::new();
    let chunks = chunker.chunk_10k(&sample_filing());
    
    assert!(chunks.len() > 5);
    assert!(chunks.iter().all(|c| c.content.len() < 4000));
}
```

### S5-GP-03: Vector Embedding Storage
```rust
#[tokio::test]
async fn test_embedding_storage() {
    let rag = FinancialRag::new(&test_pool()).await.unwrap();
    
    let chunk = Chunk::new("test", "Apple reported strong Q4 earnings");
    let embedded = rag.embed(vec![chunk]).await.unwrap();
    
    rag.store(embedded).await.unwrap();
    
    let results = rag.search("Apple earnings", 5).await.unwrap();
    assert!(!results.is_empty());
}
```

### S5-GP-04: Journal Semantic Search
```rust
#[tokio::test]
async fn test_journal_semantic_search() {
    let rag = FinancialRag::new(&test_pool()).await.unwrap();
    
    let entries = rag.search_journal("tech stocks losing momentum").await.unwrap();
    
    // Should find entries about tech stock losses
    assert!(entries.iter().any(|e| e.ticker == "TSLA" || e.ticker == "META"));
}
```

### S5-GP-05: RAG API Query
```rust
#[tokio::test]
async fn test_rag_api_query() {
    let client = TestClient::new().await;
    
    let response = client
        .post("/api/rag/query")
        .json(&json!({
            "ticker": "AAPL",
            "question": "What are the main risk factors?"
        }))
        .send()
        .await;
    
    assert_eq!(response.status(), 200);
    let body: RagResponse = response.json().await;
    assert!(!body.answer.is_empty());
    assert!(!body.sources.is_empty());
}
```

### S5-GP-06: Source Attribution
```rust
#[tokio::test]
async fn test_source_attribution() {
    let rag = FinancialRag::new(&test_pool()).await.unwrap();
    
    let response = rag.query("AAPL", "revenue growth").await.unwrap();
    
    // Every claim should have a source
    assert!(response.sources.iter().all(|s| !s.document.is_empty()));
    assert!(response.sources.iter().all(|s| s.page_number.is_some()));
}
```

---

## Schedule

| Day | Focus |
|-----|-------|
| Day 1 | neurocod-rag integration, pgvector setup |
| Day 2 | SEC EDGAR client, filing download |
| Day 3 | Financial chunker implementation |
| Day 4 | Embedding service (OpenAI/local) |
| Day 5 | RAG API endpoints |
| Day 6 | Journal semantic search |
| Day 7 | Earnings Q&A feature |
| Day 8 | Source attribution, tests, docs |

---

## Exit Criteria

Sprint 5 is **COMPLETE** when:
- ✅ All 6 Golden Path tests pass
- ✅ Can ingest and query SEC 10-K filings
- ✅ Journal semantic search returns relevant results
- ✅ API endpoints return source attributions
- ✅ pgvector extension installed and working
- ✅ No critical/high bugs open

---

## Dependencies

- neurocod-rag crate (local path dependency)
- OpenAI API key (or local embedding model)
- EDGAR access (no API key needed, rate limited)
- pgvector PostgreSQL extension
