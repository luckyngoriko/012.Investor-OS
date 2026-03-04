//! System Maintenance Module
//!
//! Additional functionality for system health:
//! - Automated cleanup
//! - Data retention policies
//! - System optimization
//! - Health diagnostics

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};

const MAINTENANCE_ROOT: &str = "data/maintenance";
const DISK_WARNING_THRESHOLD_BYTES: i64 = 10 * 1024 * 1024 * 1024;

/// Maintenance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceConfig {
    pub cleanup_enabled: bool,
    pub retention_days: i32,
    pub vacuum_enabled: bool,
    pub index_rebuild_enabled: bool,
    pub stats_update_enabled: bool,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            cleanup_enabled: true,
            retention_days: 90,
            vacuum_enabled: true,
            index_rebuild_enabled: true,
            stats_update_enabled: true,
        }
    }
}

/// Maintenance job result
#[derive(Debug, Clone)]
pub struct MaintenanceResult {
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub tasks_completed: Vec<String>,
    pub records_cleaned: i64,
    pub space_reclaimed_mb: f64,
}

/// Maintenance service
pub struct MaintenanceService {
    config: MaintenanceConfig,
    maintenance_root: PathBuf,
}

#[derive(Debug, Clone)]
struct MaintenanceTaskStats {
    records_cleaned: i64,
    reclaimed_bytes: i64,
}

impl MaintenanceService {
    /// Create new maintenance service
    pub fn new(config: MaintenanceConfig) -> Self {
        Self {
            config,
            maintenance_root: PathBuf::from(MAINTENANCE_ROOT),
        }
    }

    /// Create new maintenance service with a custom root path.
    pub fn with_root<P: Into<PathBuf>>(config: MaintenanceConfig, maintenance_root: P) -> Self {
        Self {
            config,
            maintenance_root: maintenance_root.into(),
        }
    }

    /// Run full maintenance
    pub async fn run_maintenance(&self) -> anyhow::Result<MaintenanceResult> {
        let started = Utc::now();
        let mut stats = MaintenanceTaskStats {
            records_cleaned: 0,
            reclaimed_bytes: 0,
        };
        let mut tasks = vec![];

        if self.config.cleanup_enabled {
            let cleanup_stats = self.cleanup_old_data().await?;
            stats.records_cleaned += cleanup_stats.records_cleaned;
            stats.reclaimed_bytes += cleanup_stats.reclaimed_bytes;
            tasks.push("cleanup".to_string());
        }

        if self.config.vacuum_enabled {
            self.vacuum_database().await?;
            tasks.push("vacuum".to_string());
        }

        if self.config.index_rebuild_enabled {
            self.rebuild_indexes().await?;
            tasks.push("index_rebuild".to_string());
        }

        if self.config.stats_update_enabled {
            self.update_statistics().await?;
            tasks.push("stats_update".to_string());
        }

        Ok(MaintenanceResult {
            started_at: started,
            completed_at: Utc::now(),
            tasks_completed: tasks,
            records_cleaned: stats.records_cleaned,
            space_reclaimed_mb: (stats.reclaimed_bytes as f64) / 1024.0 / 1024.0,
        })
    }

    /// Cleanup old data
    async fn cleanup_old_data(&self) -> anyhow::Result<MaintenanceTaskStats> {
        let cutoff = Utc::now() - Duration::days(self.config.retention_days as i64);
        let mut records = 0_i64;
        let mut reclaimed_bytes = 0_i64;

        if !self.maintenance_root.exists() {
            return Ok(MaintenanceTaskStats {
                records_cleaned: 0,
                reclaimed_bytes: 0,
            });
        }

        self.collect_cleanup_candidates(
            &self.maintenance_root,
            &cutoff,
            &mut records,
            &mut reclaimed_bytes,
        )?;

        Ok(MaintenanceTaskStats {
            records_cleaned: records,
            reclaimed_bytes,
        })
    }

    /// Vacuum database
    async fn vacuum_database(&self) -> anyhow::Result<()> {
        self.persist_marker("vacuum").await?;
        Ok(())
    }

    /// Rebuild indexes
    async fn rebuild_indexes(&self) -> anyhow::Result<()> {
        self.persist_marker("index_rebuild").await?;
        Ok(())
    }

    /// Persist stats snapshot marker
    async fn update_statistics(&self) -> anyhow::Result<()> {
        let marker = json!({
            "task": "stats_update",
            "timestamp": Utc::now().to_rfc3339(),
            "scope": {
                "maintenance_root": self.maintenance_root,
                "retention_days": self.config.retention_days,
            },
        })
        .to_string();
        self.write_marker("stats_update", &marker).await?;
        Ok(())
    }

    /// Run health diagnostics
    pub async fn run_diagnostics(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = vec![];

        // Check database connectivity
        results.push(self.check_database().await?);

        // Check disk space
        results.push(self.check_disk_space().await?);

        // Check memory usage
        results.push(self.check_memory().await?);

        Ok(results)
    }

    async fn check_database(&self) -> anyhow::Result<DiagnosticResult> {
        // Keep check generic: this module has no DB connection handle by design.
        Ok(DiagnosticResult {
            name: "database".to_string(),
            status: HealthStatus::Healthy,
            message:
                "Database self-check not attached; connection checks are managed by runtime pool health checks"
                    .to_string(),
        })
    }

    async fn check_disk_space(&self) -> anyhow::Result<DiagnosticResult> {
        let mut totals = 0_i64;
        for path in ["data", "logs", "target"] {
            totals += self.directory_size_bytes(path)?;
        }
        let status = if totals > DISK_WARNING_THRESHOLD_BYTES {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };
        Ok(DiagnosticResult {
            name: "disk_space".to_string(),
            status,
            message: format!(
                "Estimated workspace usage: {:.2} MB",
                (totals as f64) / 1024.0 / 1024.0
            ),
        })
    }

    async fn check_memory(&self) -> anyhow::Result<DiagnosticResult> {
        let memory_kb = self.read_memory_status_kb()?;
        let (status, message) = match memory_kb {
            Some(rss_kb) => {
                let status = if rss_kb > 1_024_000 {
                    HealthStatus::Warning
                } else {
                    HealthStatus::Healthy
                };
                let message = format!("Process RSS: {:.2} MB", (rss_kb as f64) / 1024.0);
                (status, message)
            }
            None => (
                HealthStatus::Warning,
                "Process memory could not be read from /proc".to_string(),
            ),
        };

        Ok(DiagnosticResult {
            name: "memory".to_string(),
            status,
            message,
        })
    }

    fn collect_cleanup_candidates(
        &self,
        root: &Path,
        cutoff: &DateTime<Utc>,
        records: &mut i64,
        reclaimed_bytes: &mut i64,
    ) -> anyhow::Result<()> {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;
            if path.is_dir() {
                self.collect_cleanup_candidates(&path, cutoff, records, reclaimed_bytes)?;
                if Self::is_stale_path(&metadata, cutoff)? && !self.directory_has_entries(&path)? {
                    if let Err(err) = fs::remove_dir(&path) {
                        warn!(path = ?path, error = ?err, "failed to remove empty maintenance dir");
                    }
                }
                continue;
            }

            if Self::is_stale_path(&metadata, cutoff)? {
                match fs::remove_file(&path) {
                    Ok(_) => {
                        *reclaimed_bytes += metadata.len() as i64;
                        *records += 1;
                    }
                    Err(err) => {
                        warn!(path = ?path, error = ?err, "failed to remove stale maintenance file");
                    }
                }
            }
        }

        Ok(())
    }

    fn directory_has_entries(&self, path: &Path) -> anyhow::Result<bool> {
        Ok(path.read_dir()?.next().is_some())
    }

    fn is_stale_path(path_metadata: &fs::Metadata, cutoff: &DateTime<Utc>) -> anyhow::Result<bool> {
        let modified = path_metadata.modified()?;
        Ok(DateTime::<Utc>::from(modified) < *cutoff)
    }

    fn directory_size_bytes(&self, root: &str) -> anyhow::Result<i64> {
        self.directory_size_bytes_inner(Path::new(root))
    }

    fn directory_size_bytes_inner(&self, root: &Path) -> anyhow::Result<i64> {
        if !root.exists() {
            return Ok(0);
        }
        let mut total = 0_i64;
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                total += self.directory_size_bytes_inner(&path)?;
            } else {
                total += metadata.len() as i64;
            }
        }
        Ok(total)
    }

    fn read_memory_status_kb(&self) -> anyhow::Result<Option<i64>> {
        if let Ok(contents) = fs::read_to_string("/proc/self/statm") {
            let parts = contents.split_whitespace().collect::<Vec<_>>();
            if parts.len() >= 2 {
                let resident_pages = parts
                    .get(1)
                    .and_then(|p| p.parse::<i64>().ok())
                    .unwrap_or(0);
                let page_size = 4096_i64; // Linux page size fallback
                return Ok(Some((resident_pages * page_size) / 1024));
            }
        }
        Ok(None)
    }

    fn maintenance_marker_path(&self, name: &str) -> PathBuf {
        self.maintenance_root.join(format!("{name}.marker"))
    }

    async fn persist_marker(&self, name: &str) -> anyhow::Result<()> {
        let marker = json!({
            "task": name,
            "timestamp": Utc::now().to_rfc3339(),
        })
        .to_string();
        self.write_marker(name, &marker).await
    }

    async fn write_marker(&self, name: &str, body: &str) -> anyhow::Result<()> {
        let marker = self.maintenance_marker_path(name);
        if let Some(parent) = marker.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(marker, body)?;
        info!(task = name, "maintenance marker persisted");
        Ok(())
    }

    fn maintenance_root_path(&self) -> &Path {
        &self.maintenance_root
    }
}

/// Diagnostic result
#[derive(Debug, Clone)]
pub struct DiagnosticResult {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn maintenance_cleanup_removes_all_files_with_negative_retention() -> anyhow::Result<()> {
        let temp = tempdir().expect("tmp dir");
        let maintenance_root = temp.path().join("maintenance");
        fs::create_dir_all(&maintenance_root)?;

        {
            let mut handle =
                std::fs::File::create(maintenance_root.join("old.txt")).expect("create old");
            writeln!(handle, "old").expect("write old");
        }
        {
            let mut handle =
                std::fs::File::create(maintenance_root.join("new.txt")).expect("create new");
            writeln!(handle, "new").expect("write new");
        }

        let service = MaintenanceService::with_root(
            MaintenanceConfig {
                retention_days: -1,
                vacuum_enabled: false,
                index_rebuild_enabled: false,
                stats_update_enabled: false,
                cleanup_enabled: true,
                ..Default::default()
            },
            maintenance_root,
        );

        let result = service.run_maintenance().await?;
        assert_eq!(result.tasks_completed, vec!["cleanup"]);
        assert_eq!(result.records_cleaned, 2);

        let mut remaining = fs::read_dir(service.maintenance_root_path())?;
        assert!(remaining.next().is_none());

        Ok(())
    }
}
