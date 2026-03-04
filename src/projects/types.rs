//! Project tracking domain types (Sprint 111).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Enums ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramStatus {
    Planned,
    Active,
    Completed,
    Cancelled,
}

impl ProgramStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            _ => Self::Planned,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SprintStatus {
    Planned,
    Active,
    Done,
    Blocked,
}

impl SprintStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Active => "active",
            Self::Done => "done",
            Self::Blocked => "blocked",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "done" => Self::Done,
            "blocked" => Self::Blocked,
            _ => Self::Planned,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
    Blocked,
    Skipped,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::Skipped => "skipped",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "done" => Self::Done,
            "blocked" => Self::Blocked,
            "skipped" => Self::Skipped,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl TaskPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "critical" => Self::Critical,
            "high" => Self::High,
            "low" => Self::Low,
            _ => Self::Medium,
        }
    }
}

// ── Row structs ──

#[derive(Debug, Clone, Serialize)]
pub struct ProgramRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub scope: Option<String>,
    pub status: ProgramStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct SprintRow {
    pub id: Uuid,
    pub program_id: Uuid,
    pub sprint_number: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: SprintStatus,
    pub gate: Option<String>,
    pub tasks_total: i32,
    pub tasks_done: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskRow {
    pub id: Uuid,
    pub sprint_id: Uuid,
    pub wp_number: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub files_changed: serde_json::Value,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

// ── Composite types ──

#[derive(Debug, Clone, Serialize)]
pub struct ProgramDetail {
    pub program: ProgramRow,
    pub sprints: Vec<SprintRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SprintDetail {
    pub sprint: SprintRow,
    pub tasks: Vec<TaskRow>,
    pub depends_on: Vec<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgramSummary {
    pub id: Uuid,
    pub name: String,
    pub scope: Option<String>,
    pub status: ProgramStatus,
    pub total_sprints: i64,
    pub completed_sprints: i64,
    pub completion_pct: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Dashboard {
    pub programs: Vec<ProgramSummary>,
    pub active_sprint: Option<SprintRow>,
    pub total_sprints: i64,
    pub completed_sprints: i64,
    pub completion_pct: f32,
    pub blocked_tasks: Vec<TaskRow>,
}

// ── Request types ──

#[derive(Debug, Deserialize)]
pub struct CreateProgramRequest {
    pub name: String,
    pub description: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskStatusRequest {
    pub status: String,
    pub priority: Option<String>,
}

// ── Roadmap types ──

#[derive(Debug, Clone, Serialize)]
pub struct RoadmapNode {
    pub sprint_number: i32,
    pub title: String,
    pub status: SprintStatus,
    pub depends_on: Vec<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Roadmap {
    pub program: String,
    pub nodes: Vec<RoadmapNode>,
}
