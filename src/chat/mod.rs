//! AI Chat (RAG) Module (Wave 2 Task 12).
//!
//! AI-powered chat that explains trading decisions using RAG
//! (Retrieval-Augmented Generation). Currently template-based;
//! LLM integration is a future milestone.
//!
//! Sub-modules:
//! - `context` — retrieves relevant context from PostgreSQL tables
//! - `responder` — generates template-based responses

pub mod context;
pub mod responder;

pub use context::{ChatContext, ChatContextResult};
pub use responder::generate_response;
