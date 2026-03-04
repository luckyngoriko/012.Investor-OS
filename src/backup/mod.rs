//! Backup & Recovery Module
//!
//! Additional functionality for production readiness:
//! - Automated backups
//! - Point-in-time recovery
//! - Data export/import

use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub enabled: bool,
    pub schedule: String, // cron expression
    pub retention_days: i32,
    pub destination: BackupDestination,
    pub compression: bool,
    pub encryption: bool,
}

/// Backup destination
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupDestination {
    Local { path: String },
    S3 { bucket: String, region: String },
    Gcs { bucket: String },
    Azure { container: String },
}

/// Backup job
#[derive(Debug, Clone)]
pub struct BackupJob {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: BackupStatus,
    pub size_bytes: i64,
    pub tables_backed_up: Vec<String>,
}

/// Backup status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    id: Uuid,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
    status: BackupStatus,
    size_bytes: i64,
    tables_backed_up: Vec<String>,
}

/// Backup service
pub struct BackupService {
    config: BackupConfig,
}

impl BackupService {
    /// Create new backup service
    pub fn new(config: BackupConfig) -> Self {
        Self { config }
    }

    /// Perform full backup
    pub async fn backup_full(&self) -> anyhow::Result<BackupJob> {
        if !self.config.enabled {
            anyhow::bail!("backup is disabled")
        }

        let started_at = Utc::now();
        let id = Uuid::new_v4();
        let mut job = BackupJob {
            id,
            started_at,
            completed_at: None,
            status: BackupStatus::Running,
            size_bytes: 0,
            tables_backed_up: vec![],
        };

        let backup_root = self.local_backup_root()?;
        fs::create_dir_all(&backup_root)?;

        let backup_dir = backup_root.join(format!("{}-{}", started_at.format("%Y%m%dT%H%M%S"), id));
        fs::create_dir_all(&backup_dir)?;

        let source_items = ["data", "config"];
        for item in &source_items {
            let src = Path::new(item);
            let dst = backup_dir.join(item);
            if src.exists() {
                let copied = copy_dir_recursive(src, &dst)?;
                if copied > 0 {
                    job.tables_backed_up.push((*item).to_string());
                    job.size_bytes += copied;
                }
            } else {
                info!(source = %item, "skipping backup source path because it does not exist");
            }
        }

        self.prune_expired_backups(&backup_root)?;

        job.status = BackupStatus::Completed;
        job.completed_at = Some(Utc::now());

        let manifest = BackupManifest {
            id: job.id,
            started_at: job.started_at,
            completed_at: job.completed_at.unwrap(),
            status: job.status,
            size_bytes: job.size_bytes,
            tables_backed_up: job.tables_backed_up.clone(),
        };
        let manifest_path = backup_dir.join("manifest.json");
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
        fs::write(manifest_path, manifest_bytes)?;

        info!(backup_id = %job.id, size_bytes = job.size_bytes, "full backup completed");

        Ok(job)
    }

    /// Restore from backup
    pub async fn restore(&self, backup_id: Uuid) -> anyhow::Result<()> {
        if !self.config.enabled {
            anyhow::bail!("backup is disabled")
        }

        let backup_root = self.local_backup_root()?;
        let backup_dir = self.find_backup_dir(&backup_root, backup_id)?;
        let manifest = read_manifest(&backup_dir.join("manifest.json"))?;
        if manifest.status != BackupStatus::Completed {
            anyhow::bail!("backup {} is not in completed state", backup_id);
        }

        for source in &manifest.tables_backed_up {
            let src = backup_dir.join(source);
            let dst = Path::new(source);

            if !src.exists() {
                warn!(source = %source, "backup source missing, skipping restore item");
                continue;
            }

            if dst.exists() {
                fs::remove_dir_all(dst)?;
            }
            copy_dir_recursive(&src, dst)?;
        }

        info!(backup_id = %backup_id, "restore completed");
        Ok(())
    }

    /// List available backups
    pub async fn list_backups(&self) -> anyhow::Result<Vec<BackupJob>> {
        if !self.config.enabled {
            anyhow::bail!("backup is disabled")
        }

        let mut jobs = vec![];
        let backup_root = self.local_backup_root()?;
        if !backup_root.exists() {
            return Ok(jobs);
        }

        for dir in fs::read_dir(&backup_root)? {
            let dir = dir?;
            if !dir.file_type()?.is_dir() {
                continue;
            }

            let manifest_path = dir.path().join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }

            match read_manifest(&manifest_path) {
                Ok(manifest) => {
                    jobs.push(BackupJob {
                        id: manifest.id,
                        started_at: manifest.started_at,
                        completed_at: Some(manifest.completed_at),
                        status: manifest.status,
                        size_bytes: manifest.size_bytes,
                        tables_backed_up: manifest.tables_backed_up,
                    });
                }
                Err(err) => {
                    warn!(path = %manifest_path.display(), error = ?err, "skipping unreadable backup manifest");
                }
            }
        }

        jobs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(jobs)
    }

    fn find_backup_dir(&self, backup_root: &Path, id: Uuid) -> anyhow::Result<PathBuf> {
        for item in fs::read_dir(backup_root)? {
            let item = item?;
            if !item.file_type()?.is_dir() {
                continue;
            }
            let manifest_path = item.path().join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            let manifest = read_manifest(&manifest_path)?;
            if manifest.id == id {
                return Ok(item.path());
            }
        }
        anyhow::bail!("backup {} not found", id)
    }

    fn local_backup_root(&self) -> anyhow::Result<PathBuf> {
        match &self.config.destination {
            BackupDestination::Local { path } => Ok(PathBuf::from(path)),
            other => anyhow::bail!("unsupported backup destination: {other:?}"),
        }
    }

    fn prune_expired_backups(&self, backup_root: &Path) -> anyhow::Result<()> {
        let retention_days = self.config.retention_days;
        if retention_days <= 0 {
            return Ok(());
        }

        let cutoff = Utc::now() - Duration::days(retention_days as i64);
        let mut candidate_dirs = vec![];

        for item in fs::read_dir(backup_root)? {
            let item = item?;
            if !item.file_type()?.is_dir() {
                continue;
            }
            let manifest_path = item.path().join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            let manifest = match read_manifest(&manifest_path) {
                Ok(manifest) => manifest,
                Err(err) => {
                    warn!(path = %manifest_path.display(), error = ?err, "ignoring malformed manifest during retention cleanup");
                    continue;
                }
            };

            if manifest.completed_at < cutoff {
                candidate_dirs.push(item.path());
            }
        }

        for dir in candidate_dirs {
            match fs::remove_dir_all(&dir) {
                Ok(_) => {
                    info!(path = %dir.display(), "removed expired backup");
                }
                Err(err) => {
                    warn!(path = %dir.display(), error = ?err, "failed to remove expired backup");
                }
            }
        }

        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<i64> {
    let mut copied_bytes = 0_i64;

    if src.is_file() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        copied_bytes += fs::copy(src, dst)? as i64;
        return Ok(copied_bytes);
    }

    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copied_bytes += copy_dir_recursive(&entry_path, &target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            copied_bytes += fs::copy(&entry_path, &target_path)? as i64;
        }
    }

    Ok(copied_bytes)
}

fn read_manifest(path: &Path) -> anyhow::Result<BackupManifest> {
    let bytes = fs::read(path)?;
    let manifest = serde_json::from_slice(&bytes)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    struct CurrentDirGuard {
        previous_dir: std::path::PathBuf,
    }

    impl CurrentDirGuard {
        fn new(path: &Path) -> anyhow::Result<Self> {
            let previous_dir = env::current_dir()?;
            env::set_current_dir(path)?;
            Ok(Self { previous_dir })
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.previous_dir);
        }
    }

    fn local_backup_config(path: String) -> BackupConfig {
        BackupConfig {
            enabled: true,
            schedule: "0 0 * * *".to_string(),
            retention_days: 7,
            destination: BackupDestination::Local { path },
            compression: false,
            encryption: false,
        }
    }

    fn create_source_tree(base: &Path) -> anyhow::Result<()> {
        let data = base.join("data");
        let config = base.join("config");
        fs::create_dir_all(data.join("sub"))?;
        fs::create_dir_all(&config)?;
        fs::write(data.join("snapshot.json"), "{\"k\":\"v\"}")?;
        fs::write(data.join("sub").join("prices.csv"), "a,b\n1,2")?;
        fs::write(config.join("settings.toml"), "strategy = \"alpha\"")?;
        Ok(())
    }

    #[tokio::test]
    async fn backup_and_list_and_restore_flow() -> anyhow::Result<()> {
        let repo = tempdir().expect("repo");
        let backup_root = tempdir().expect("backup root");

        let _guard = CurrentDirGuard::new(repo.path())?;

        create_source_tree(repo.path())?;
        let service = BackupService::new(local_backup_config(
            backup_root.path().to_string_lossy().to_string(),
        ));

        let backup = service.backup_full().await?;
        assert!(backup.size_bytes > 0);

        let backups = service.list_backups().await?;
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].id, backup.id);

        fs::remove_dir_all(repo.path().join("data"))?;
        fs::remove_dir_all(repo.path().join("config"))?;
        assert!(!repo.path().join("data").exists());

        service.restore(backup.id).await?;
        let restored_data = repo.path().join("data").join("snapshot.json");
        let restored_config = repo.path().join("config").join("settings.toml");
        assert!(restored_data.exists());
        assert!(restored_config.exists());
        Ok(())
    }

    #[tokio::test]
    async fn backup_calls_are_blocked_when_disabled() -> anyhow::Result<()> {
        let backup_root = tempdir().expect("backup root");
        let service = BackupService::new(BackupConfig {
            enabled: false,
            schedule: "0 0 * * *".to_string(),
            retention_days: 7,
            destination: BackupDestination::Local {
                path: backup_root.path().to_string_lossy().to_string(),
            },
            compression: false,
            encryption: false,
        });

        assert_eq!(
            service.backup_full().await.unwrap_err().to_string(),
            "backup is disabled"
        );
        assert_eq!(
            service.list_backups().await.unwrap_err().to_string(),
            "backup is disabled"
        );
        assert_eq!(
            service.restore(Uuid::nil()).await.unwrap_err().to_string(),
            "backup is disabled"
        );
        Ok(())
    }
}
