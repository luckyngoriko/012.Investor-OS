//! Sprint 5 Golden Path Tests
//!
//! S5-GP-01: Covering index improves CQ query performance
//! S5-GP-02: TimescaleDB compression reduces storage
//! S5-GP-03: Materialized view refresh works
//! S5-GP-04: SEC filing parsing creates correct chunks
//! S5-GP-05: Earnings call parsing identifies speakers
//! S5-GP-06: Embedding generation produces normalized vectors
//! S5-GP-07: Semantic search returns relevant results
//! S5-GP-08: RAG API endpoints respond correctly

// Test modules
mod test_api;
mod test_compression;
mod test_earnings_parsing;
mod test_embeddings;
mod test_indexes;
mod test_materialized_views;
mod test_search;
mod test_sec_parsing;
