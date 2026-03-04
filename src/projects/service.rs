//! Project tracking business logic (Sprint 111).

use sqlx::PgPool;
use uuid::Uuid;

use super::error::ProjectError;
use super::repository;
use super::types::*;

/// Project tracking service — wraps repository with business rules.
pub struct ProjectService {
    pool: PgPool,
}

impl ProjectService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ── Programs ──

    pub async fn create_program(
        &self,
        name: &str,
        description: Option<&str>,
        scope: Option<&str>,
    ) -> Result<ProgramRow, ProjectError> {
        repository::insert_program(&self.pool, name, description, scope).await
    }

    pub async fn list_programs(&self) -> Result<Vec<ProgramRow>, ProjectError> {
        repository::list_programs(&self.pool).await
    }

    pub async fn get_program_detail(&self, id: Uuid) -> Result<ProgramDetail, ProjectError> {
        let program = repository::find_program_by_id(&self.pool, id)
            .await?
            .ok_or(ProjectError::ProgramNotFound)?;
        let sprints = repository::list_sprints_by_program(&self.pool, id).await?;
        Ok(ProgramDetail { program, sprints })
    }

    pub async fn update_program_status(
        &self,
        id: Uuid,
        status: &str,
    ) -> Result<ProgramRow, ProjectError> {
        repository::update_program_status(&self.pool, id, status).await
    }

    // ── Sprints ──

    pub async fn list_sprints_by_program(
        &self,
        program_id: Uuid,
    ) -> Result<Vec<SprintRow>, ProjectError> {
        repository::list_sprints_by_program(&self.pool, program_id).await
    }

    pub async fn get_sprint_detail(&self, number: i32) -> Result<SprintDetail, ProjectError> {
        let sprint = repository::find_sprint_by_number(&self.pool, number)
            .await?
            .ok_or(ProjectError::SprintNotFound(number))?;
        let tasks = repository::list_tasks_by_sprint(&self.pool, sprint.id).await?;
        let depends_on = repository::find_dependencies(&self.pool, sprint.id).await?;
        Ok(SprintDetail {
            sprint,
            tasks,
            depends_on,
        })
    }

    /// Advance a sprint to "active" status — checks all dependencies are done first.
    pub async fn advance_sprint(&self, number: i32) -> Result<SprintRow, ProjectError> {
        let sprint = repository::find_sprint_by_number(&self.pool, number)
            .await?
            .ok_or(ProjectError::SprintNotFound(number))?;

        let unmet = repository::all_dependencies_done(&self.pool, sprint.id).await?;
        if !unmet.is_empty() {
            return Err(ProjectError::UnmetDependencies(number, unmet));
        }

        repository::update_sprint_status(&self.pool, sprint.id, "active").await
    }

    /// Mark a sprint as done.
    pub async fn complete_sprint(&self, number: i32) -> Result<SprintRow, ProjectError> {
        let sprint = repository::find_sprint_by_number(&self.pool, number)
            .await?
            .ok_or(ProjectError::SprintNotFound(number))?;

        repository::update_sprint_status(&self.pool, sprint.id, "done").await
    }

    // ── Tasks ──

    pub async fn list_tasks(&self, sprint_id: Uuid) -> Result<Vec<TaskRow>, ProjectError> {
        repository::list_tasks_by_sprint(&self.pool, sprint_id).await
    }

    /// Update a task's status (and optionally priority).
    /// If the new status is "done", also increments the sprint's tasks_done counter.
    /// If tasks_done == tasks_total after increment, auto-completes the sprint.
    pub async fn update_task_status(
        &self,
        task_id: Uuid,
        status: &str,
        priority: Option<&str>,
    ) -> Result<TaskRow, ProjectError> {
        let task = repository::update_task_status(&self.pool, task_id, status, priority).await?;

        if status == "done" {
            let sprint = repository::increment_tasks_done(&self.pool, task.sprint_id).await?;
            // Auto-complete sprint when all tasks are done
            if sprint.tasks_done >= sprint.tasks_total && sprint.tasks_total > 0 {
                let _ = repository::update_sprint_status(&self.pool, sprint.id, "done").await;
            }
        }

        Ok(task)
    }

    // ── Dashboard ──

    pub async fn dashboard(&self) -> Result<Dashboard, ProjectError> {
        repository::dashboard_aggregate(&self.pool).await
    }

    // ── Roadmap ──

    pub async fn roadmap(&self) -> Result<Vec<Roadmap>, ProjectError> {
        repository::roadmap(&self.pool).await
    }
}
