use std::{
    collections::HashSet,
    fs, io,
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deploy_go_agent_protocol::{DeploymentReleaseTask, DeploymentStage};
use tracing::{debug, warn};

use crate::{
    journal::{JournalError, JournalState, JournalStore, TaskJournal},
    runner::RunnerSpec,
};

pub const DEFAULT_TASK_RETENTION_SECONDS: u64 = 7 * 24 * 60 * 60;
pub const DEFAULT_DEPLOYMENT_RETENTION_SECONDS: u64 = 30 * 24 * 60 * 60;
pub const DEFAULT_STORAGE_CLEANUP_INTERVAL_SECONDS: u64 = 60 * 60;

const PRIVILEGED_TASK_FILE: &str = "privileged-release-task.json";
const RUNNER_SPEC_FILE: &str = "runner-spec.json";
const JOURNAL_FILE: &str = "journal.json";
const UPLOADED_ARTIFACT_FILE: &str = "artifact.tar";

#[derive(Clone, Debug)]
pub struct StorageCleanup {
    data_dir: PathBuf,
    tasks_root: PathBuf,
    deployments_root: PathBuf,
    task_retention: Duration,
    deployment_retention: Duration,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanupReport {
    pub removed_task_dirs: usize,
    pub removed_deployment_dirs: usize,
    pub removed_checkout_dirs: usize,
    pub removed_artifact_dirs: usize,
    pub removed_upload_archives: usize,
    pub reclaimed_deployments: usize,
}

#[derive(Clone, Debug)]
struct TaskReference {
    deployment_id: String,
    stage: DeploymentStage,
    checkout_dir: Option<PathBuf>,
    artifact_dir: Option<PathBuf>,
}

impl StorageCleanup {
    pub fn new(
        data_dir: PathBuf,
        task_retention: Duration,
        deployment_retention: Duration,
    ) -> Self {
        Self {
            tasks_root: data_dir.join("tasks"),
            deployments_root: data_dir.join("apps").join("deployments"),
            data_dir,
            task_retention,
            deployment_retention,
        }
    }

    pub fn run_once(&self) -> CleanupReport {
        let mut report = CleanupReport::default();
        if self.task_retention.is_zero() && self.deployment_retention.is_zero() {
            return report;
        }
        let now = unix_time();
        let entries = match fs::read_dir(&self.tasks_root) {
            Ok(entries) => entries.flatten().collect::<Vec<_>>(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                warn!(path = %self.tasks_root.display(), error = %error, "任务目录扫描失败");
                return report;
            }
        };
        let mut references: Vec<(PathBuf, TaskReference)> = Vec::new();
        let mut reclaimed_deployments = HashSet::new();
        let mut active_ids = HashSet::new();
        for entry in &entries {
            let path = entry.path();
            if !fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_dir())
                || !path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(valid_storage_name)
            {
                continue;
            }
            let Some(task_id) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let store = JournalStore::new(self.tasks_root.clone());
            let Ok(journal) = store.load(task_id) else {
                continue;
            };
            if active(&journal)
                && let Some(reference) = read_task_reference(&path)
            {
                active_ids.insert(reference.deployment_id);
            }
        }
        for entry in entries {
            let path = entry.path();
            if !entry.file_type().is_ok_and(|kind| kind.is_dir())
                || !valid_storage_name(entry.file_name().to_str().unwrap_or_default())
            {
                continue;
            }
            let store = JournalStore::new(self.tasks_root.clone());
            let Some(task_id) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let journal = match store.load(task_id) {
                Ok(journal) => journal,
                Err(JournalError::Missing) | Err(JournalError::InvalidJournal) => {
                    if path_age_secs(&path, now).is_some_and(|age| age >= task_retention_secs(self))
                        && remove_managed_tree(&path, &self.data_dir, &mut report) > 0
                    {
                        report.removed_task_dirs += 1;
                    }
                    continue;
                }
                Err(_) => continue,
            };
            let reference = read_task_reference(&path);
            let terminal = terminal(&journal.state);
            let confirmed = terminal
                && (journal.result_sequence.is_some()
                    || path_age_secs(&path, now)
                        .is_some_and(|age| age >= task_retention_secs(self)));
            if confirmed {
                let deployment_busy = reference
                    .as_ref()
                    .is_some_and(|reference| active_ids.contains(reference.deployment_id.as_str()));
                let reclaim_deployment = if deployment_busy {
                    false
                } else {
                    cleanup_terminal_sources_inner(
                        &path,
                        &journal,
                        reference.as_ref(),
                        &self.data_dir,
                        &mut report,
                    )
                };
                if reclaim_deployment {
                    report.reclaimed_deployments += 1;
                    if let Some(reference) = &reference {
                        reclaimed_deployments.insert(reference.deployment_id.clone());
                    }
                }
            }
            if terminal
                && path_age_secs(&path, now).is_some_and(|age| age >= task_retention_secs(self))
                && remove_managed_tree(&path, &self.data_dir, &mut report) > 0
            {
                report.removed_task_dirs += 1;
                continue;
            }
            if let Some(reference) = reference {
                references.push((path, reference));
            }
        }
        self.cleanup_deployment_roots(now, &references, &reclaimed_deployments, &mut report);
        report
    }

    fn cleanup_deployment_roots(
        &self,
        now: i64,
        references: &[(PathBuf, TaskReference)],
        reclaimed_deployments: &HashSet<String>,
        report: &mut CleanupReport,
    ) {
        let mut active_ids: HashSet<&str> = HashSet::new();
        let mut prepare_succeeded: HashSet<&str> = HashSet::new();
        for (path, reference) in references {
            let store = JournalStore::new(self.tasks_root.clone());
            let Some(task_id) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let Ok(journal) = store.load(task_id) else {
                continue;
            };
            if active(&journal) {
                active_ids.insert(&reference.deployment_id);
                continue;
            }
            if let (DeploymentStage::Prepare, JournalState::Succeeded) =
                (reference.stage.clone(), journal.state)
            {
                prepare_succeeded.insert(&reference.deployment_id);
            }
        }
        let entries = match fs::read_dir(&self.deployments_root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return,
            Err(error) => {
                warn!(path = %self.deployments_root.display(), error = %error, "部署工作目录扫描失败");
                return;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !valid_storage_name(&name) || !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                continue;
            }
            if active_ids.contains(name.as_str()) {
                continue;
            }
            if reclaimed_deployments.contains(&name) {
                if remove_managed_tree(&path, &self.data_dir, report) > 0 {
                    report.removed_deployment_dirs += 1;
                }
                continue;
            }
            if prepare_succeeded.contains(name.as_str())
                && path_age_secs(&path, now)
                    .is_some_and(|age| age < deployment_retention_secs(self))
            {
                // 保留可能仍处于手动等待发布阶段的 staging。
                continue;
            }
            // 不立即删除空目录：workspace 快照可能在 journal 写入前创建部署目录，
            // 空目录删除会与正在开始的 prepare 竞争。
            if path_age_secs(&path, now).is_some_and(|age| age >= deployment_retention_secs(self))
                && remove_managed_tree(&path, &self.data_dir, report) > 0
            {
                report.removed_deployment_dirs += 1;
            }
        }
    }
}

pub(crate) fn cleanup_executor_terminal_sources(
    task_dir: &Path,
    journal: &TaskJournal,
    data_dir: &Path,
) {
    let mut report = CleanupReport::default();
    let reference = read_task_reference(task_dir);
    if reference.as_ref().is_some_and(|reference| {
        active_same_deployment_exists(task_dir, data_dir, &reference.deployment_id)
    }) {
        return;
    }
    cleanup_terminal_sources_inner(task_dir, journal, reference.as_ref(), data_dir, &mut report);
}

fn cleanup_terminal_sources_inner(
    task_dir: &Path,
    journal: &TaskJournal,
    reference: Option<&TaskReference>,
    data_dir: &Path,
    report: &mut CleanupReport,
) -> bool {
    if !terminal(&journal.state)
        || journal.result_sequence.is_none()
        || task_dir
            .file_name()
            .and_then(|value| value.to_str())
            .is_none_or(|name| !valid_storage_name(name))
    {
        return false;
    }
    let Some(reference) = reference else {
        return false;
    };
    let uploaded = task_dir.join(UPLOADED_ARTIFACT_FILE).is_file();
    let mut reclaimed = false;
    let remove_checkout = reference.stage == DeploymentStage::Release
        || journal.state != JournalState::Succeeded
        || uploaded;
    if remove_checkout
        && let Some(checkout) = &reference.checkout_dir
        && source_path_allowed(checkout, task_dir, data_dir, &reference.deployment_id)
        && remove_managed_tree(checkout, data_dir, report) > 0
    {
        report.removed_checkout_dirs += 1;
        reclaimed = true;
    }
    let remove_artifact = reference.stage == DeploymentStage::Release
        || journal.state != JournalState::Succeeded
        || uploaded;
    if remove_artifact
        && let Some(artifact) = &reference.artifact_dir
        && source_path_allowed(artifact, task_dir, data_dir, &reference.deployment_id)
        && remove_managed_tree(artifact, data_dir, report) > 0
    {
        report.removed_artifact_dirs += 1;
        reclaimed = true;
    }
    if remove_artifact
        && uploaded
        && remove_managed_tree(&task_dir.join(UPLOADED_ARTIFACT_FILE), data_dir, report) > 0
    {
        report.removed_upload_archives += 1;
    }
    if reclaimed {
        remove_empty_deployment_parent(reference.artifact_dir.as_deref(), data_dir, report);
    }
    reclaimed
}

fn read_task_reference(task_dir: &Path) -> Option<TaskReference> {
    let privileged = task_dir.join(PRIVILEGED_TASK_FILE);
    if let Ok(bytes) = fs::read(&privileged)
        && let Ok(task) = serde_json::from_slice::<DeploymentReleaseTask>(&bytes)
    {
        return Some(TaskReference {
            deployment_id: task.deployment_id,
            stage: DeploymentStage::Release,
            checkout_dir: Some(PathBuf::from(task.checkout_dir)),
            artifact_dir: Some(PathBuf::from(task.artifact_dir)),
        });
    }
    let spec_path = task_dir.join(RUNNER_SPEC_FILE);
    let bytes = fs::read(spec_path).ok()?;
    let spec: RunnerSpec = serde_json::from_slice(&bytes).ok()?;
    let two_stage = spec.two_stage?;
    Some(TaskReference {
        deployment_id: spec.deployment_id,
        stage: two_stage.stage,
        checkout_dir: Some(two_stage.checkout_dir),
        artifact_dir: two_stage.artifact_dir,
    })
}

fn source_path_allowed(path: &Path, task_dir: &Path, data_dir: &Path, deployment_id: &str) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let in_task = path.starts_with(task_dir)
        && path != task_dir
        && relative_components_normal(path, data_dir).is_some();
    let deployments = data_dir.join("apps").join("deployments");
    let in_deployment = path.strip_prefix(&deployments).is_ok_and(|relative| {
        let mut components = relative.components();
        let deployment = components.next().and_then(|component| match component {
            Component::Normal(name) => name.to_str(),
            _ => None,
        });
        deployment == Some(deployment_id)
            && components
                .next()
                .is_some_and(|component| matches!(component, Component::Normal(_)))
    });
    in_task || in_deployment
}

fn active_same_deployment_exists(
    current_task_dir: &Path,
    data_dir: &Path,
    deployment_id: &str,
) -> bool {
    let tasks_root = data_dir.join("tasks");
    let Ok(entries) = fs::read_dir(&tasks_root) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path == current_task_dir
            || !path.is_dir()
            || !path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(valid_storage_name)
        {
            continue;
        }
        let Some(task_id) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let store = JournalStore::new(tasks_root.clone());
        let Ok(journal) = store.load(task_id) else {
            continue;
        };
        if active(&journal)
            && read_task_reference(&path)
                .is_some_and(|reference| reference.deployment_id == deployment_id)
        {
            return true;
        }
    }
    false
}

fn remove_empty_deployment_parent(
    path: Option<&Path>,
    data_dir: &Path,
    report: &mut CleanupReport,
) {
    let Some(path) = path else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    let deployments = data_dir.join("apps").join("deployments");
    if parent.parent() != Some(deployments.as_path()) {
        return;
    }
    if directory_empty(parent).unwrap_or(false) && remove_managed_tree(parent, data_dir, report) > 0
    {
        report.removed_deployment_dirs += 1;
    }
}

fn remove_managed_tree(path: &Path, root: &Path, _report: &mut CleanupReport) -> u64 {
    if managed_relative(path, root).is_none() {
        warn!(path = %path.display(), "拒绝清理非受控路径");
        return 0;
    }
    if !ancestors_symlink_free(root, path) {
        warn!(path = %path.display(), "拒绝清理含符号链接祖先的路径");
        return 0;
    }
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.file_type().is_symlink() {
        if fs::remove_file(path).is_ok() {
            return 1;
        }
        return 0;
    }
    if metadata.is_file() {
        return if fs::remove_file(path).is_ok() { 1 } else { 0 };
    }
    if !metadata.is_dir() {
        return 0;
    }
    match fs::remove_dir_all(path) {
        Ok(()) => {
            debug!(path = %path.display(), "已回收存储目录");
            1
        }
        Err(error) => {
            warn!(path = %path.display(), error = %error, "存储目录回收失败");
            0
        }
    }
}

fn managed_relative(path: &Path, root: &Path) -> Option<PathBuf> {
    if path == root {
        return None;
    }
    let relative = path.strip_prefix(root).ok()?;
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(relative.to_path_buf())
}

fn relative_components_normal(path: &Path, root: &Path) -> Option<()> {
    let relative = path.strip_prefix(root).ok()?;
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(())
}

fn ancestors_symlink_free(root: &Path, path: &Path) -> bool {
    if fs::symlink_metadata(root).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return false;
    }
    let Some(relative) = managed_relative(path, root) else {
        return false;
    };
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => return false,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => break,
            Err(_) => return false,
        }
    }
    true
}

fn directory_empty(path: &Path) -> io::Result<bool> {
    match fs::read_dir(path) {
        Ok(mut entries) => Ok(entries.next().transpose()?.is_none()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error),
    }
}

fn valid_storage_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn terminal(state: &JournalState) -> bool {
    matches!(
        state,
        JournalState::Succeeded
            | JournalState::Failed
            | JournalState::Canceled
            | JournalState::Interrupted
    )
}

fn active(journal: &TaskJournal) -> bool {
    matches!(
        journal.state,
        JournalState::Accepted | JournalState::Running
    ) || journal.transfer_phase.is_some()
        || journal.pid.is_some()
}

fn task_retention_secs(cleanup: &StorageCleanup) -> i64 {
    i64::try_from(cleanup.task_retention.as_secs()).unwrap_or(i64::MAX)
}

fn deployment_retention_secs(cleanup: &StorageCleanup) -> i64 {
    i64::try_from(cleanup.deployment_retention.as_secs()).unwrap_or(i64::MAX)
}

fn path_age_secs(path: &Path, now: i64) -> Option<i64> {
    let journal_path = path.join(JOURNAL_FILE);
    let metadata = fs::symlink_metadata(if journal_path.is_file() {
        &journal_path
    } else {
        path
    })
    .ok()?;
    let modified = metadata.modified().ok()?;
    let age = now
        .saturating_sub(
            modified
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        )
        .max(0);
    Some(age)
}

fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use deploy_go_agent_protocol::{Environment, MakeTarget, ReleaseCheckoutMode};
    use serde_json::json;
    use std::ffi::CString;

    #[cfg(unix)]
    fn set_mtime_to_epoch(path: &Path) {
        use std::os::unix::ffi::OsStrExt;
        let raw = CString::new(path.as_os_str().as_bytes()).unwrap();
        let times = [
            libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
        ];
        let result = unsafe { libc::utimensat(libc::AT_FDCWD, raw.as_ptr(), times.as_ptr(), 0) };
        assert_eq!(result, 0);
    }

    fn task_journal(store: &JournalStore, task_id: &str) -> TaskJournal {
        store
            .create(task_id, "idem_0123456789abcdef", "sha256:0123456789abcdef")
            .unwrap()
    }

    fn terminal_journal(store: &JournalStore, task_id: &str, state: JournalState) -> TaskJournal {
        let mut journal = task_journal(store, task_id);
        journal.state = state;
        journal.result_sequence = Some(1);
        store.store(&journal).unwrap();
        journal
    }

    fn write_runner_spec(task_dir: &Path, deployment_id: &str, stage: DeploymentStage) {
        write_runner_spec_paths(
            task_dir,
            deployment_id,
            stage,
            &task_dir.join("checkout"),
            &task_dir.join("staging"),
        );
    }

    fn write_runner_spec_paths(
        task_dir: &Path,
        deployment_id: &str,
        stage: DeploymentStage,
        checkout: &Path,
        artifact: &Path,
    ) {
        fs::create_dir_all(checkout).unwrap();
        fs::create_dir_all(artifact).unwrap();
        fs::write(checkout.join("Makefile"), "build:\n\ttrue\n").unwrap();
        fs::write(artifact.join("deploy-go-artifact.json"), "{}").unwrap();
        let spec = json!({
            "deployment_id": deployment_id,
            "script_path": "/usr/bin/make",
            "argument_tokens": ["deploy-go-prepare"],
            "environment_file_references": [],
            "timeout_seconds": 60,
            "log_budget_bytes": 1024,
            "two_stage": {
                "stage": match stage {
                    DeploymentStage::Prepare => "prepare",
                    DeploymentStage::Release => "release",
                },
                "checkout_dir": checkout.to_string_lossy(),
                "work_root": task_dir.to_string_lossy(),
                "repository_url": null,
                "commit_sha": "0123456789abcdef0123456789abcdef01234567",
                "credential_file": null,
                "environment": "test",
                "release_version": "20260905000000",
                "target_code": null,
                "modules": ["api"],
                "artifact_dir": artifact.to_string_lossy(),
                "staging_size_limit_bytes": 1024,
                "staging_max_files": 16,
                "git_lease_id": null
            }
        });
        fs::write(
            task_dir.join(RUNNER_SPEC_FILE),
            serde_json::to_vec(&spec).unwrap(),
        )
        .unwrap();
    }

    fn write_privileged_release_task(
        task_dir: &Path,
        deployment_id: &str,
        checkout: &Path,
        artifact: &Path,
    ) {
        fs::create_dir_all(checkout).unwrap();
        fs::create_dir_all(artifact).unwrap();
        fs::write(checkout.join("Makefile"), "build:\n\ttrue\n").unwrap();
        fs::write(artifact.join("deploy-go-artifact.json"), "{}").unwrap();
        let task = DeploymentReleaseTask {
            deployment_id: deployment_id.to_owned(),
            target_code: "test".to_owned(),
            work_root: task_dir.to_string_lossy().into_owned(),
            checkout_dir: checkout.to_string_lossy().into_owned(),
            artifact_dir: artifact.to_string_lossy().into_owned(),
            environment: Environment::Test,
            release_version: "20260905000000".to_owned(),
            commit_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            modules: vec!["api".to_owned()],
            make_target: MakeTarget::DeployGoRelease,
            timeout_seconds: 60,
            cancel_file: task_dir.join("cancel").to_string_lossy().into_owned(),
            privileged: true,
            privileged_context: None,
            artifact_download: None,
            repository_url: None,
            git_credential_lease_id: None,
            application_slug: None,
            required_env: Vec::new(),
            checkout_mode: ReleaseCheckoutMode::Artifact,
            secret_environment: None,
        };
        fs::write(
            task_dir.join(PRIVILEGED_TASK_FILE),
            serde_json::to_vec(&task).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn removes_old_terminal_task_and_keeps_active_task() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let tasks = data.join("tasks");
        let store = JournalStore::new(tasks);
        terminal_journal(&store, "task_old", JournalState::Succeeded);
        set_mtime_to_epoch(&store.task_dir("task_old").join("journal.json"));
        terminal_journal(&store, "task_active", JournalState::Running);

        let cleanup = StorageCleanup::new(data, Duration::from_secs(3600), Duration::from_secs(60));
        let report = cleanup.run_once();
        assert!(!store.task_dir("task_old").exists());
        assert!(store.task_dir("task_active").exists());
        assert_eq!(report.removed_task_dirs, 1);
    }

    #[test]
    fn prepare_success_keeps_staging_until_deployment_retention() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let tasks = data.join("tasks");
        let store = JournalStore::new(tasks.clone());
        let task_id = "task_prepare";
        let task_dir = store.task_dir(task_id);
        terminal_journal(&store, task_id, JournalState::Succeeded);
        write_runner_spec(&task_dir, "deployment_01", DeploymentStage::Prepare);

        let cleanup = StorageCleanup::new(
            data.clone(),
            Duration::from_secs(3600),
            Duration::from_secs(3600),
        );
        cleanup.run_once();
        assert!(task_dir.join("staging").is_dir());
        assert!(task_dir.join("checkout").exists());
        assert!(task_dir.exists());
    }

    #[test]
    fn uploaded_prepare_removes_checkout_staging_and_archive() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let tasks = data.join("tasks");
        let store = JournalStore::new(tasks.clone());
        let task_id = "task_upload";
        let task_dir = store.task_dir(task_id);
        terminal_journal(&store, task_id, JournalState::Succeeded);
        write_runner_spec(&task_dir, "deployment_01", DeploymentStage::Prepare);
        fs::write(task_dir.join("artifact.tar"), b"archive").unwrap();

        let cleanup = StorageCleanup::new(
            data.clone(),
            Duration::from_secs(3600),
            Duration::from_secs(3600),
        );
        cleanup.run_once();
        assert!(!task_dir.join("checkout").exists());
        assert!(!task_dir.join("staging").exists());
        assert!(!task_dir.join("artifact.tar").exists());
        assert!(task_dir.exists());
    }

    #[test]
    fn release_reclaims_deployment_workdir_after_seal() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let tasks = data.join("tasks");
        let deployments = data.join("apps").join("deployments").join("deployment_01");
        let store = JournalStore::new(tasks.clone());
        let task_id = "task_release_root";
        let task_dir = store.task_dir(task_id);
        terminal_journal(&store, task_id, JournalState::Succeeded);
        write_runner_spec_paths(
            &task_dir,
            "deployment_01",
            DeploymentStage::Release,
            &deployments.join("checkout"),
            &deployments.join("staging"),
        );

        let cleanup = StorageCleanup::new(
            data.clone(),
            Duration::from_secs(3600),
            Duration::from_secs(3600),
        );
        cleanup.run_once();
        assert!(!deployments.exists());
        assert!(task_dir.exists());
    }

    #[test]
    fn terminal_release_waits_while_same_deployment_is_active() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let tasks = data.join("tasks");
        let deployments = data.join("apps").join("deployments").join("deployment_01");
        let store = JournalStore::new(tasks.clone());
        let terminal_task = store.task_dir("task_terminal");
        terminal_journal(&store, "task_terminal", JournalState::Succeeded);
        write_runner_spec_paths(
            &terminal_task,
            "deployment_01",
            DeploymentStage::Release,
            &deployments.join("checkout"),
            &deployments.join("staging"),
        );
        let active_task = store.task_dir("task_active");
        let mut journal = task_journal(&store, "task_active");
        journal.state = JournalState::Running;
        store.store(&journal).unwrap();
        write_runner_spec_paths(
            &active_task,
            "deployment_01",
            DeploymentStage::Release,
            &deployments.join("checkout"),
            &deployments.join("staging"),
        );

        let cleanup =
            StorageCleanup::new(data, Duration::from_secs(3600), Duration::from_secs(3600));
        cleanup.run_once();
        assert!(deployments.join("checkout").join("Makefile").is_file());
        assert!(
            deployments
                .join("staging")
                .join("deploy-go-artifact.json")
                .is_file()
        );
        assert!(store.task_dir("task_terminal").exists());
        assert!(store.task_dir("task_active").exists());
    }

    #[test]
    fn task_cannot_reclaim_another_deployment_workdir() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let tasks = data.join("tasks");
        let other = data.join("apps").join("deployments").join("deployment_02");
        let store = JournalStore::new(tasks.clone());
        let task_id = "task_cross_deployment";
        let task_dir = store.task_dir(task_id);
        terminal_journal(&store, task_id, JournalState::Succeeded);
        write_runner_spec_paths(
            &task_dir,
            "deployment_01",
            DeploymentStage::Release,
            &other.join("checkout"),
            &other.join("staging"),
        );

        let cleanup =
            StorageCleanup::new(data, Duration::from_secs(3600), Duration::from_secs(3600));
        cleanup.run_once();
        assert!(other.join("checkout").join("Makefile").is_file());
        assert!(
            other
                .join("staging")
                .join("deploy-go-artifact.json")
                .is_file()
        );
        assert!(task_dir.exists());
    }

    #[test]
    fn privileged_release_reclaims_task_checkout_and_staging() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let tasks = data.join("tasks");
        let store = JournalStore::new(tasks.clone());
        let task_id = "task_privileged";
        let task_dir = store.task_dir(task_id);
        terminal_journal(&store, task_id, JournalState::Succeeded);
        write_privileged_release_task(
            &task_dir,
            "deployment_01",
            &task_dir.join("checkout"),
            &task_dir.join("staging"),
        );

        let cleanup = StorageCleanup::new(
            data.clone(),
            Duration::from_secs(3600),
            Duration::from_secs(3600),
        );
        cleanup.run_once();
        assert!(!task_dir.join("checkout").exists());
        assert!(!task_dir.join("staging").exists());
        assert!(task_dir.exists());
    }

    #[test]
    fn unreported_terminal_sources_are_not_reclaimed() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let tasks = data.join("tasks");
        let store = JournalStore::new(tasks.clone());
        let task_id = "task_unreported";
        let task_dir = store.task_dir(task_id);
        let mut journal = task_journal(&store, task_id);
        journal.state = JournalState::Succeeded;
        store.store(&journal).unwrap();
        write_runner_spec(&task_dir, "deployment_01", DeploymentStage::Release);

        let cleanup =
            StorageCleanup::new(data, Duration::from_secs(3600), Duration::from_secs(3600));
        cleanup.run_once();
        assert!(task_dir.join("checkout").is_dir());
        assert!(task_dir.join("staging").is_dir());
        assert!(task_dir.exists());
    }

    #[test]
    fn fresh_empty_deployment_is_not_deleted_before_journal_write() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let deployments = data.join("apps").join("deployments").join("deployment_01");
        fs::create_dir_all(&deployments).unwrap();

        let cleanup =
            StorageCleanup::new(data, Duration::from_secs(3600), Duration::from_secs(3600));
        cleanup.run_once();
        assert!(deployments.is_dir());
    }

    #[test]
    fn old_empty_deployment_is_reclaimed_after_retention() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let deployments = data.join("apps").join("deployments").join("deployment_01");
        fs::create_dir_all(&deployments).unwrap();
        set_mtime_to_epoch(&deployments);

        let cleanup = StorageCleanup::new(data, Duration::from_secs(3600), Duration::from_secs(60));
        cleanup.run_once();
        assert!(!deployments.exists());
    }

    #[test]
    fn release_terminal_reclaims_checkout_and_staging() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().to_path_buf();
        let tasks = data.join("tasks");
        let store = JournalStore::new(tasks.clone());
        let task_id = "task_release";
        let task_dir = store.task_dir(task_id);
        terminal_journal(&store, task_id, JournalState::Succeeded);
        write_runner_spec(&task_dir, "deployment_01", DeploymentStage::Release);

        let cleanup = StorageCleanup::new(
            data.clone(),
            Duration::from_secs(0),
            Duration::from_secs(3600),
        );
        cleanup.run_once();
        assert!(!task_dir.join("checkout").exists());
        assert!(!task_dir.join("staging").exists());
    }
}
