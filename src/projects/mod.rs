//! Enterprise Project Tracking System (Sprint 111).
//!
//! PostgreSQL-backed program/sprint/task tracking with dependency management,
//! dashboard aggregates, and roadmap visualization.

pub mod error;
pub mod repository;
pub mod seed;
pub mod service;
pub mod types;

pub use error::ProjectError;
pub use service::ProjectService;
pub use types::{
    CreateProgramRequest, Dashboard, ProgramDetail, ProgramRow, ProgramStatus, ProgramSummary,
    Roadmap, RoadmapNode, SprintDetail, SprintRow, SprintStatus, TaskPriority, TaskRow, TaskStatus,
    UpdateStatusRequest, UpdateTaskStatusRequest,
};
