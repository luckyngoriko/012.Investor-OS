//! Project tracking repository — all SQL queries (Sprint 111).

use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::error::ProjectError;
use super::types::*;

// ── Row mappers ──

fn map_program_row(row: sqlx::postgres::PgRow) -> Result<ProgramRow, sqlx::Error> {
    let status_str: String = row.try_get("status")?;
    Ok(ProgramRow {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        scope: row.try_get("scope")?,
        status: ProgramStatus::from_db(&status_str),
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        metadata: row.try_get("metadata")?,
    })
}

fn map_sprint_row(row: sqlx::postgres::PgRow) -> Result<SprintRow, sqlx::Error> {
    let status_str: String = row.try_get("status")?;
    Ok(SprintRow {
        id: row.try_get("id")?,
        program_id: row.try_get("program_id")?,
        sprint_number: row.try_get("sprint_number")?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        status: SprintStatus::from_db(&status_str),
        gate: row.try_get("gate")?,
        tasks_total: row.try_get("tasks_total")?,
        tasks_done: row.try_get("tasks_done")?,
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        metadata: row.try_get("metadata")?,
    })
}

fn map_task_row(row: sqlx::postgres::PgRow) -> Result<TaskRow, sqlx::Error> {
    let status_str: String = row.try_get("status")?;
    let priority_str: String = row.try_get("priority")?;
    Ok(TaskRow {
        id: row.try_get("id")?,
        sprint_id: row.try_get("sprint_id")?,
        wp_number: row.try_get("wp_number")?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        status: TaskStatus::from_db(&status_str),
        priority: TaskPriority::from_db(&priority_str),
        files_changed: row.try_get("files_changed")?,
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        metadata: row.try_get("metadata")?,
    })
}

// ── Programs ──

pub async fn insert_program(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
    scope: Option<&str>,
) -> Result<ProgramRow, ProjectError> {
    let row = sqlx::query(
        "INSERT INTO programs (name, description, scope)
         VALUES ($1, $2, $3)
         RETURNING id, name, description, scope, status,
                   started_at, completed_at, created_at, updated_at, metadata",
    )
    .bind(name)
    .bind(description)
    .bind(scope)
    .fetch_one(pool)
    .await?;
    Ok(map_program_row(row)?)
}

pub async fn list_programs(pool: &PgPool) -> Result<Vec<ProgramRow>, ProjectError> {
    let rows = sqlx::query(
        "SELECT id, name, description, scope, status,
                started_at, completed_at, created_at, updated_at, metadata
         FROM programs ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| map_program_row(r).map_err(ProjectError::from))
        .collect()
}

pub async fn find_program_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<ProgramRow>, ProjectError> {
    let row = sqlx::query(
        "SELECT id, name, description, scope, status,
                started_at, completed_at, created_at, updated_at, metadata
         FROM programs WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(map_program_row(r)?)),
        None => Ok(None),
    }
}

pub async fn find_program_by_name(
    pool: &PgPool,
    name: &str,
) -> Result<Option<ProgramRow>, ProjectError> {
    let row = sqlx::query(
        "SELECT id, name, description, scope, status,
                started_at, completed_at, created_at, updated_at, metadata
         FROM programs WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(map_program_row(r)?)),
        None => Ok(None),
    }
}

pub async fn update_program_status(
    pool: &PgPool,
    id: Uuid,
    status: &str,
) -> Result<ProgramRow, ProjectError> {
    let started = if status == "active" {
        Some(chrono::Utc::now())
    } else {
        None
    };
    let completed = if status == "completed" {
        Some(chrono::Utc::now())
    } else {
        None
    };

    let row = sqlx::query(
        "UPDATE programs
         SET status = $2,
             started_at = COALESCE($3, started_at),
             completed_at = COALESCE($4, completed_at),
             updated_at = NOW()
         WHERE id = $1
         RETURNING id, name, description, scope, status,
                   started_at, completed_at, created_at, updated_at, metadata",
    )
    .bind(id)
    .bind(status)
    .bind(started)
    .bind(completed)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(map_program_row(r)?),
        None => Err(ProjectError::ProgramNotFound),
    }
}

// ── Sprints ──

pub async fn insert_sprint(
    pool: &PgPool,
    program_id: Uuid,
    sprint_number: i32,
    title: &str,
    description: Option<&str>,
    gate: Option<&str>,
    tasks_total: i32,
) -> Result<SprintRow, ProjectError> {
    let row = sqlx::query(
        "INSERT INTO sprints_tracking (program_id, sprint_number, title, description, gate, tasks_total)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, program_id, sprint_number, title, description, status, gate,
                   tasks_total, tasks_done, started_at, completed_at, created_at, updated_at, metadata",
    )
    .bind(program_id)
    .bind(sprint_number)
    .bind(title)
    .bind(description)
    .bind(gate)
    .bind(tasks_total)
    .fetch_one(pool)
    .await?;
    Ok(map_sprint_row(row)?)
}

pub async fn list_sprints_by_program(
    pool: &PgPool,
    program_id: Uuid,
) -> Result<Vec<SprintRow>, ProjectError> {
    let rows = sqlx::query(
        "SELECT id, program_id, sprint_number, title, description, status, gate,
                tasks_total, tasks_done, started_at, completed_at, created_at, updated_at, metadata
         FROM sprints_tracking WHERE program_id = $1
         ORDER BY sprint_number ASC",
    )
    .bind(program_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| map_sprint_row(r).map_err(ProjectError::from))
        .collect()
}

pub async fn find_sprint_by_id(pool: &PgPool, id: Uuid) -> Result<Option<SprintRow>, ProjectError> {
    let row = sqlx::query(
        "SELECT id, program_id, sprint_number, title, description, status, gate,
                tasks_total, tasks_done, started_at, completed_at, created_at, updated_at, metadata
         FROM sprints_tracking WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(map_sprint_row(r)?)),
        None => Ok(None),
    }
}

pub async fn find_sprint_by_number(
    pool: &PgPool,
    number: i32,
) -> Result<Option<SprintRow>, ProjectError> {
    let row = sqlx::query(
        "SELECT id, program_id, sprint_number, title, description, status, gate,
                tasks_total, tasks_done, started_at, completed_at, created_at, updated_at, metadata
         FROM sprints_tracking WHERE sprint_number = $1",
    )
    .bind(number)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(map_sprint_row(r)?)),
        None => Ok(None),
    }
}

pub async fn update_sprint_status(
    pool: &PgPool,
    id: Uuid,
    status: &str,
) -> Result<SprintRow, ProjectError> {
    let started = if status == "active" {
        Some(chrono::Utc::now())
    } else {
        None
    };
    let completed = if status == "done" {
        Some(chrono::Utc::now())
    } else {
        None
    };

    let row = sqlx::query(
        "UPDATE sprints_tracking
         SET status = $2,
             started_at = COALESCE($3, started_at),
             completed_at = COALESCE($4, completed_at),
             updated_at = NOW()
         WHERE id = $1
         RETURNING id, program_id, sprint_number, title, description, status, gate,
                   tasks_total, tasks_done, started_at, completed_at, created_at, updated_at, metadata",
    )
    .bind(id)
    .bind(status)
    .bind(started)
    .bind(completed)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(map_sprint_row(r)?),
        None => Err(ProjectError::SprintNotFound(0)),
    }
}

pub async fn increment_tasks_done(
    pool: &PgPool,
    sprint_id: Uuid,
) -> Result<SprintRow, ProjectError> {
    let row = sqlx::query(
        "UPDATE sprints_tracking
         SET tasks_done = tasks_done + 1, updated_at = NOW()
         WHERE id = $1
         RETURNING id, program_id, sprint_number, title, description, status, gate,
                   tasks_total, tasks_done, started_at, completed_at, created_at, updated_at, metadata",
    )
    .bind(sprint_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(map_sprint_row(r)?),
        None => Err(ProjectError::SprintNotFound(0)),
    }
}

// ── Dependencies ──

pub async fn insert_sprint_dependency(
    pool: &PgPool,
    sprint_id: Uuid,
    depends_on_id: Uuid,
) -> Result<(), ProjectError> {
    sqlx::query(
        "INSERT INTO sprint_dependencies (sprint_id, depends_on_id)
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(sprint_id)
    .bind(depends_on_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_dependencies(pool: &PgPool, sprint_id: Uuid) -> Result<Vec<i32>, ProjectError> {
    let rows = sqlx::query(
        "SELECT st.sprint_number
         FROM sprint_dependencies sd
         JOIN sprints_tracking st ON st.id = sd.depends_on_id
         WHERE sd.sprint_id = $1
         ORDER BY st.sprint_number",
    )
    .bind(sprint_id)
    .fetch_all(pool)
    .await?;

    let nums: Vec<i32> = rows
        .iter()
        .map(|r| r.try_get("sprint_number"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(nums)
}

/// Check if all dependencies of a sprint are done.
pub async fn all_dependencies_done(
    pool: &PgPool,
    sprint_id: Uuid,
) -> Result<Vec<i32>, ProjectError> {
    // Returns sprint numbers of dependencies that are NOT done
    let rows = sqlx::query(
        "SELECT st.sprint_number
         FROM sprint_dependencies sd
         JOIN sprints_tracking st ON st.id = sd.depends_on_id
         WHERE sd.sprint_id = $1 AND st.status <> 'done'
         ORDER BY st.sprint_number",
    )
    .bind(sprint_id)
    .fetch_all(pool)
    .await?;

    let unmet: Vec<i32> = rows
        .iter()
        .map(|r| r.try_get("sprint_number"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(unmet)
}

// ── Tasks ──

pub async fn insert_task(
    pool: &PgPool,
    sprint_id: Uuid,
    wp_number: i32,
    title: &str,
    description: Option<&str>,
    priority: &str,
) -> Result<TaskRow, ProjectError> {
    let row = sqlx::query(
        "INSERT INTO tasks (sprint_id, wp_number, title, description, priority)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, sprint_id, wp_number, title, description, status, priority,
                   files_changed, started_at, completed_at, created_at, updated_at, metadata",
    )
    .bind(sprint_id)
    .bind(wp_number)
    .bind(title)
    .bind(description)
    .bind(priority)
    .fetch_one(pool)
    .await?;
    Ok(map_task_row(row)?)
}

pub async fn list_tasks_by_sprint(
    pool: &PgPool,
    sprint_id: Uuid,
) -> Result<Vec<TaskRow>, ProjectError> {
    let rows = sqlx::query(
        "SELECT id, sprint_id, wp_number, title, description, status, priority,
                files_changed, started_at, completed_at, created_at, updated_at, metadata
         FROM tasks WHERE sprint_id = $1
         ORDER BY wp_number ASC",
    )
    .bind(sprint_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| map_task_row(r).map_err(ProjectError::from))
        .collect()
}

pub async fn update_task_status(
    pool: &PgPool,
    task_id: Uuid,
    status: &str,
    priority: Option<&str>,
) -> Result<TaskRow, ProjectError> {
    let started = if status == "in_progress" {
        Some(chrono::Utc::now())
    } else {
        None
    };
    let completed = if status == "done" || status == "skipped" {
        Some(chrono::Utc::now())
    } else {
        None
    };

    let row = sqlx::query(
        "UPDATE tasks
         SET status = $2,
             priority = COALESCE($3, priority),
             started_at = COALESCE($4, started_at),
             completed_at = COALESCE($5, completed_at),
             updated_at = NOW()
         WHERE id = $1
         RETURNING id, sprint_id, wp_number, title, description, status, priority,
                   files_changed, started_at, completed_at, created_at, updated_at, metadata",
    )
    .bind(task_id)
    .bind(status)
    .bind(priority)
    .bind(started)
    .bind(completed)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(map_task_row(r)?),
        None => Err(ProjectError::TaskNotFound),
    }
}

pub async fn find_task_by_id(pool: &PgPool, id: Uuid) -> Result<Option<TaskRow>, ProjectError> {
    let row = sqlx::query(
        "SELECT id, sprint_id, wp_number, title, description, status, priority,
                files_changed, started_at, completed_at, created_at, updated_at, metadata
         FROM tasks WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(map_task_row(r)?)),
        None => Ok(None),
    }
}

// ── Dashboard aggregate ──

pub async fn dashboard_aggregate(pool: &PgPool) -> Result<Dashboard, ProjectError> {
    // Program summaries
    let prog_rows = sqlx::query(
        "SELECT p.id, p.name, p.scope, p.status,
                COUNT(st.id) AS total_sprints,
                COUNT(st.id) FILTER (WHERE st.status = 'done') AS completed_sprints
         FROM programs p
         LEFT JOIN sprints_tracking st ON st.program_id = p.id
         GROUP BY p.id
         ORDER BY p.created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    let programs: Vec<ProgramSummary> = prog_rows
        .into_iter()
        .map(|r| {
            let total: i64 = r.try_get("total_sprints").unwrap_or(0);
            let completed: i64 = r.try_get("completed_sprints").unwrap_or(0);
            let pct = if total > 0 {
                (completed as f32 / total as f32) * 100.0
            } else {
                0.0
            };
            let status_str: String = r.try_get("status").unwrap_or_default();
            Ok(ProgramSummary {
                id: r.try_get("id")?,
                name: r.try_get("name")?,
                scope: r.try_get("scope")?,
                status: ProgramStatus::from_db(&status_str),
                total_sprints: total,
                completed_sprints: completed,
                completion_pct: pct,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    // Global sprint counts
    let counts_row = sqlx::query(
        "SELECT COUNT(*) AS total,
                COUNT(*) FILTER (WHERE status = 'done') AS completed
         FROM sprints_tracking",
    )
    .fetch_one(pool)
    .await?;
    let total_sprints: i64 = counts_row.try_get("total").unwrap_or(0);
    let completed_sprints: i64 = counts_row.try_get("completed").unwrap_or(0);
    let completion_pct = if total_sprints > 0 {
        (completed_sprints as f32 / total_sprints as f32) * 100.0
    } else {
        0.0
    };

    // Active sprint (first one with status='active', lowest number)
    let active_row = sqlx::query(
        "SELECT id, program_id, sprint_number, title, description, status, gate,
                tasks_total, tasks_done, started_at, completed_at, created_at, updated_at, metadata
         FROM sprints_tracking WHERE status = 'active'
         ORDER BY sprint_number ASC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    let active_sprint = match active_row {
        Some(r) => Some(map_sprint_row(r)?),
        None => None,
    };

    // Blocked tasks
    let blocked_rows = sqlx::query(
        "SELECT id, sprint_id, wp_number, title, description, status, priority,
                files_changed, started_at, completed_at, created_at, updated_at, metadata
         FROM tasks WHERE status = 'blocked'
         ORDER BY created_at ASC LIMIT 20",
    )
    .fetch_all(pool)
    .await?;
    let blocked_tasks: Vec<TaskRow> = blocked_rows
        .into_iter()
        .map(|r| map_task_row(r).map_err(ProjectError::from))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Dashboard {
        programs,
        active_sprint,
        total_sprints,
        completed_sprints,
        completion_pct,
        blocked_tasks,
    })
}

// ── Roadmap ──

pub async fn roadmap(pool: &PgPool) -> Result<Vec<Roadmap>, ProjectError> {
    let programs = list_programs(pool).await?;
    let mut result = Vec::new();

    for prog in programs {
        let sprints = list_sprints_by_program(pool, prog.id).await?;
        let mut nodes = Vec::new();

        for s in &sprints {
            let deps = find_dependencies(pool, s.id).await?;
            nodes.push(RoadmapNode {
                sprint_number: s.sprint_number,
                title: s.title.clone(),
                status: s.status.clone(),
                depends_on: deps,
            });
        }

        result.push(Roadmap {
            program: prog.name,
            nodes,
        });
    }

    Ok(result)
}
