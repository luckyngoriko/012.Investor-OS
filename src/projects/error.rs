//! Project tracking errors (Sprint 111).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("program not found")]
    ProgramNotFound,

    #[error("sprint not found: {0}")]
    SprintNotFound(i32),

    #[error("task not found")]
    TaskNotFound,

    #[error("sprint {0} has unmet dependencies: {1:?}")]
    UnmetDependencies(i32, Vec<i32>),

    #[error("invalid status transition from {from} to {to}")]
    InvalidStatusTransition { from: String, to: String },

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}
