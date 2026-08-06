use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, Weak},
};

use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;

use crate::config::ArtifactConfig;

pub(crate) mod http;
mod verify;
pub use http::router;

#[derive(Clone, Debug)]
pub struct ArtifactStore {
    quarantine_root: PathBuf,
    objects_root: PathBuf,
    config: ArtifactConfig,
    download_pins: Arc<Mutex<HashMap<String, usize>>>,
    upload_locks: Arc<Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>>,
    verification_slots: Arc<tokio::sync::Semaphore>,
}

#[derive(Debug, Error)]
pub enum ArtifactStoreError {
    #[error("制品根目录必须是绝对路径")]
    RelativeRoot,
    #[error("无法初始化制品存储目录")]
    Initialize(#[source] std::io::Error),
    #[error("制品存储键格式无效")]
    InvalidStorageKey,
}

impl ArtifactStore {
    pub fn initialize(config: ArtifactConfig) -> Result<Self, ArtifactStoreError> {
        if !config.root.is_absolute() {
            return Err(ArtifactStoreError::RelativeRoot);
        }
        let quarantine_root = config.root.join("quarantine");
        let objects_root = config.root.join("objects");
        fs::create_dir_all(&quarantine_root).map_err(ArtifactStoreError::Initialize)?;
        fs::create_dir_all(&objects_root).map_err(ArtifactStoreError::Initialize)?;
        Ok(Self {
            quarantine_root,
            objects_root,
            config,
            download_pins: Arc::new(Mutex::new(HashMap::new())),
            upload_locks: Arc::new(Mutex::new(HashMap::new())),
            verification_slots: Arc::new(tokio::sync::Semaphore::new(2)),
        })
    }

    pub fn config(&self) -> &ArtifactConfig {
        &self.config
    }

    pub(crate) fn upload_path(&self, artifact_id: &str) -> Result<PathBuf, ArtifactStoreError> {
        validate_key(artifact_id)?;
        Ok(self.quarantine_root.join(format!("{artifact_id}.upload")))
    }

    pub(crate) fn object_path(&self, storage_key: &str) -> Result<PathBuf, ArtifactStoreError> {
        validate_key(storage_key)?;
        Ok(self.objects_root.join(storage_key))
    }

    pub(crate) fn pin_download(&self, storage_key: &str) -> ArtifactDownloadPin {
        let mut pins = self
            .download_pins
            .lock()
            .expect("artifact pin lock poisoned");
        *pins.entry(storage_key.to_owned()).or_default() += 1;
        ArtifactDownloadPin {
            storage_key: storage_key.to_owned(),
            pins: Arc::clone(&self.download_pins),
        }
    }

    fn is_download_pinned(&self, storage_key: &str) -> bool {
        self.download_pins
            .lock()
            .expect("artifact pin lock poisoned")
            .contains_key(storage_key)
    }

    pub(crate) fn upload_lock(&self, artifact_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self
            .upload_locks
            .lock()
            .expect("artifact upload lock poisoned");
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = locks.get(artifact_id).and_then(Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(tokio::sync::Mutex::new(()));
        locks.insert(artifact_id.to_owned(), Arc::downgrade(&lock));
        lock
    }

    pub(crate) async fn verification_permit(&self) -> tokio::sync::OwnedSemaphorePermit {
        Arc::clone(&self.verification_slots)
            .acquire_owned()
            .await
            .expect("artifact verification semaphore closed")
    }
}

pub(crate) struct ArtifactDownloadPin {
    storage_key: String,
    pins: Arc<Mutex<HashMap<String, usize>>>,
}

impl Drop for ArtifactDownloadPin {
    fn drop(&mut self) {
        let mut pins = self.pins.lock().expect("artifact pin lock poisoned");
        if let Some(count) = pins.get_mut(&self.storage_key) {
            *count -= 1;
            if *count == 0 {
                pins.remove(&self.storage_key);
            }
        }
    }
}

pub async fn reconcile_and_cleanup(
    pool: &SqlitePool,
    store: &ArtifactStore,
) -> Result<u64, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE artifact_leases SET status='expired' WHERE status='active' AND expires_at<=?",
    )
    .bind(&now)
    .execute(pool)
    .await?;
    sqlx::query("UPDATE deployment_artifacts SET status='failed',expires_at=?,updated_at=?,version=version+1 WHERE status='uploading' AND NOT EXISTS (SELECT 1 FROM artifact_leases lease WHERE lease.artifact_id=deployment_artifacts.id AND lease.purpose='artifact_upload' AND lease.status='active')")
        .bind(&now).bind(&now).execute(pool).await?;
    sqlx::query("UPDATE deployment_artifacts SET status=CASE WHEN storage_key IS NULL THEN 'failed' ELSE 'verified' END,updated_at=?,version=version+1 WHERE status='deleting'")
        .bind(&now).execute(pool).await?;

    let uploading: Vec<(String, i64)> = sqlx::query_as(
        "SELECT id,upload_offset FROM deployment_artifacts WHERE status='uploading'",
    )
    .fetch_all(pool)
    .await?;
    for (id, offset) in uploading {
        let Ok(path) = store.upload_path(&id) else {
            continue;
        };
        match tokio::fs::metadata(&path).await {
            Ok(metadata) if metadata.is_file() && metadata.len() >= offset as u64 => {
                if metadata.len() > offset as u64 {
                    let file = tokio::fs::OpenOptions::new().write(true).open(&path).await;
                    if let Ok(file) = file {
                        let _ = file.set_len(offset as u64).await;
                    }
                }
            }
            Ok(_) | Err(_) if offset > 0 => {
                sqlx::query("UPDATE deployment_artifacts SET status='failed',expires_at=?,updated_at=?,version=version+1 WHERE id=? AND status='uploading'")
                    .bind(&now).bind(&now).bind(&id).execute(pool).await?;
                sqlx::query("UPDATE artifact_leases SET status='failed' WHERE artifact_id=? AND purpose='artifact_upload' AND status='active'")
                    .bind(&id).execute(pool).await?;
            }
            _ => {}
        }
    }

    let verified: Vec<(String, String)> =
        sqlx::query_as("SELECT id,storage_key FROM deployment_artifacts WHERE status='verified'")
            .fetch_all(pool)
            .await?;
    for (id, storage_key) in verified {
        let missing = store
            .object_path(&storage_key)
            .ok()
            .and_then(|path| std::fs::metadata(path).ok())
            .is_none_or(|metadata| !metadata.is_file());
        if missing {
            sqlx::query("UPDATE deployment_artifacts SET status='failed',updated_at=?,version=version+1 WHERE id=? AND status='verified'")
                .bind(&now).bind(&id).execute(pool).await?;
            sqlx::query("UPDATE artifact_leases SET status='failed' WHERE artifact_id=? AND status='active'")
                .bind(&id).execute(pool).await?;
        }
    }

    let expired: Vec<(String, Option<String>, String)> = sqlx::query_as(
        "SELECT artifact.id,artifact.storage_key,artifact.status FROM deployment_artifacts artifact WHERE artifact.status IN ('verified','failed') AND artifact.expires_at<=? AND NOT EXISTS (SELECT 1 FROM deployment_target_runs run WHERE run.artifact_id=artifact.id AND run.status IN ('pending','downloading','running')) AND NOT EXISTS (SELECT 1 FROM artifact_leases lease WHERE lease.artifact_id=artifact.id AND lease.status='active')",
    )
    .bind(&now)
    .fetch_all(pool)
    .await?;
    let mut cleaned = 0;
    for (id, storage_key, previous_status) in expired {
        if storage_key
            .as_deref()
            .is_some_and(|key| store.is_download_pinned(key))
        {
            continue;
        }
        let claimed = sqlx::query("UPDATE deployment_artifacts SET status='deleting',updated_at=?,version=version+1 WHERE id=? AND status=? AND expires_at<=? AND NOT EXISTS (SELECT 1 FROM deployment_target_runs run WHERE run.artifact_id=deployment_artifacts.id AND run.status IN ('pending','downloading','running')) AND NOT EXISTS (SELECT 1 FROM artifact_leases lease WHERE lease.artifact_id=deployment_artifacts.id AND lease.status='active')")
            .bind(&now).bind(&id).bind(&previous_status).bind(&now).execute(pool).await?;
        if claimed.rows_affected() != 1 {
            continue;
        }
        if storage_key
            .as_deref()
            .is_some_and(|key| store.is_download_pinned(key))
        {
            sqlx::query("UPDATE deployment_artifacts SET status=?,updated_at=?,version=version+1 WHERE id=? AND status='deleting'")
                .bind(&previous_status).bind(&now).bind(&id).execute(pool).await?;
            continue;
        }

        let remove_upload = match store.upload_path(&id) {
            Ok(path) => remove_managed_file(&path).await,
            Err(_) => false,
        };
        let remove_object = if let Some(storage_key) = storage_key.as_deref() {
            let shared: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deployment_artifacts WHERE id<>? AND ((storage_key=? AND status IN ('verified','deleting')) OR (archive_digest=? AND status='uploading'))")
                .bind(&id).bind(storage_key).bind(storage_key).fetch_one(pool).await?;
            if shared > 0 {
                true
            } else {
                match store.object_path(storage_key) {
                    Ok(path) => remove_managed_file(&path).await,
                    Err(_) => false,
                }
            }
        } else {
            true
        };
        if remove_upload && remove_object {
            cleaned += sqlx::query("UPDATE deployment_artifacts SET status='expired',storage_key=NULL,updated_at=?,version=version+1 WHERE id=? AND status='deleting'")
                .bind(&now).bind(&id).execute(pool).await?.rows_affected();
        } else {
            sqlx::query("UPDATE deployment_artifacts SET status=?,updated_at=?,version=version+1 WHERE id=? AND status='deleting'")
                .bind(&previous_status).bind(&now).bind(&id).execute(pool).await?;
        }
    }
    cleanup_orphans(pool, store).await?;
    Ok(cleaned)
}

async fn cleanup_orphans(pool: &SqlitePool, store: &ArtifactStore) -> Result<(), sqlx::Error> {
    if let Ok(mut entries) = tokio::fs::read_dir(&store.quarantine_root).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some(artifact_id) = name.strip_suffix(".upload") else {
                continue;
            };
            if validate_key(artifact_id).is_err()
                || !entry.file_type().await.is_ok_and(|kind| kind.is_file())
            {
                continue;
            }
            let active: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM deployment_artifacts WHERE id=? AND status='uploading'",
            )
            .bind(artifact_id)
            .fetch_one(pool)
            .await?;
            if active == 0 {
                let _ = remove_managed_file(&entry.path()).await;
            }
        }
    }
    if let Ok(mut entries) = tokio::fs::read_dir(&store.objects_root).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let Some(storage_key) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if validate_key(&storage_key).is_err()
                || store.is_download_pinned(&storage_key)
                || !entry.file_type().await.is_ok_and(|kind| kind.is_file())
            {
                continue;
            }
            let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deployment_artifacts WHERE (storage_key=? AND status IN ('verified','deleting')) OR (archive_digest=? AND status='uploading')")
                .bind(&storage_key).bind(&storage_key).fetch_one(pool).await?;
            if active == 0 {
                let _ = remove_managed_file(&entry.path()).await;
            }
        }
    }
    Ok(())
}

async fn remove_managed_file(path: &std::path::Path) -> bool {
    match tokio::fs::remove_file(path).await {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(_) => false,
    }
}

fn validate_key(value: &str) -> Result<(), ArtifactStoreError> {
    if (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        Ok(())
    } else {
        Err(ArtifactStoreError::InvalidStorageKey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(root: PathBuf) -> ArtifactConfig {
        ArtifactConfig {
            root,
            max_file_bytes: 1024,
            max_total_bytes: 2048,
            max_files: 4,
            max_chunk_bytes: 512,
            upload_ttl_seconds: 60,
            retention_ttl_seconds: 120,
        }
    }

    #[test]
    fn storage_keys_cannot_escape_the_managed_roots() {
        let temp = tempfile::tempdir().unwrap();
        let store = ArtifactStore::initialize(config(temp.path().to_path_buf())).unwrap();
        assert!(
            store
                .upload_path("artifact_01")
                .unwrap()
                .starts_with(&store.config().root)
        );
        for key in ["", "../escape", "a/b", ".", "中文"] {
            assert!(store.upload_path(key).is_err(), "accepted {key}");
            assert!(store.object_path(key).is_err(), "accepted {key}");
        }
    }
}
