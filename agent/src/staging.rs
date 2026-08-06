use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

const MANIFEST_FILE: &str = "deploy-go-artifact.json";

#[derive(Clone, Debug)]
pub struct StagingLimits {
    pub size_limit_bytes: u64,
    pub max_files: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactManifest {
    pub schema_version: u32,
    pub release_version: String,
    pub commit_sha: String,
    pub artifacts: Vec<ArtifactEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactEntry {
    pub module: String,
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Error)]
pub enum StagingError {
    #[error("发布物目录无效")]
    InvalidDirectory,
    #[error("manifest 缺失")]
    MissingManifest,
    #[error("manifest 无效")]
    InvalidManifest,
    #[error("release version 与任务不一致")]
    ReleaseVersionMismatch,
    #[error("commit SHA 与任务不一致")]
    CommitMismatch,
    #[error("模块集合与任务不一致")]
    ModuleMismatch,
    #[error("manifest 声明重复模块")]
    DuplicateModule,
    #[error("发布物路径逃逸或非法")]
    PathEscape,
    #[error("发布物目录包含符号链接")]
    SymlinkForbidden,
    #[error("manifest 声明的文件缺失")]
    MissingFile,
    #[error("文件 SHA-256 不匹配")]
    ChecksumMismatch,
    #[error("文件大小不匹配")]
    SizeMismatch,
    #[error("发布物目录存在未声明文件")]
    UndeclaredFile,
    #[error("发布物超过限额")]
    LimitExceeded,
    #[error("发布物目录读取失败: {0}")]
    Io(#[from] io::Error),
}

pub fn verify_artifact_dir(
    artifact_dir: &Path,
    expected_release_version: &str,
    expected_commit_sha: &str,
    expected_modules: &[String],
    limits: &StagingLimits,
) -> Result<ArtifactManifest, StagingError> {
    let dir = fs::canonicalize(artifact_dir).map_err(|_| StagingError::InvalidDirectory)?;
    if fs::symlink_metadata(&dir)
        .map_err(StagingError::Io)?
        .file_type()
        .is_symlink()
        || !dir.is_dir()
    {
        return Err(StagingError::InvalidDirectory);
    }
    let manifest_path = dir.join(MANIFEST_FILE);
    let bytes = fs::read(&manifest_path).map_err(|_| StagingError::MissingManifest)?;
    validate_manifest_schema(&bytes)?;
    let manifest: ArtifactManifest =
        serde_json::from_slice(&bytes).map_err(|_| StagingError::InvalidManifest)?;
    if manifest.schema_version != 1 {
        return Err(StagingError::InvalidManifest);
    }
    if manifest.release_version != expected_release_version {
        return Err(StagingError::ReleaseVersionMismatch);
    }
    if !manifest
        .commit_sha
        .eq_ignore_ascii_case(expected_commit_sha)
    {
        return Err(StagingError::CommitMismatch);
    }
    validate_modules(&manifest, expected_modules)?;

    let (files, total_size) = collect_files(&dir)?;
    if total_size > limits.size_limit_bytes || files.len() > limits.max_files {
        return Err(StagingError::LimitExceeded);
    }
    let mut declared = HashSet::new();
    let mut modules = HashSet::new();
    for entry in &manifest.artifacts {
        let relative = normalize_relative_path(&entry.path)?;
        if !modules.insert(entry.module.as_str()) {
            return Err(StagingError::DuplicateModule);
        }
        let path = dir.join(&relative);
        let metadata = fs::symlink_metadata(&path).map_err(|_| StagingError::MissingFile)?;
        if metadata.file_type().is_symlink() {
            return Err(StagingError::SymlinkForbidden);
        }
        let canonical = fs::canonicalize(&path).map_err(|_| StagingError::MissingFile)?;
        if !canonical.starts_with(&dir) {
            return Err(StagingError::PathEscape);
        }
        let actual_size = metadata.len();
        if actual_size != entry.size {
            return Err(StagingError::SizeMismatch);
        }
        let actual_sha = file_sha256(&path)?;
        if actual_sha != entry.sha256.to_ascii_lowercase() {
            return Err(StagingError::ChecksumMismatch);
        }
        declared.insert(relative);
    }
    let mut expected_files = files.keys().cloned().collect::<HashSet<_>>();
    expected_files.remove(&PathBuf::from(MANIFEST_FILE));
    if expected_files != declared {
        return Err(StagingError::UndeclaredFile);
    }
    Ok(manifest)
}

fn validate_modules(
    manifest: &ArtifactManifest,
    expected_modules: &[String],
) -> Result<(), StagingError> {
    let actual = manifest
        .artifacts
        .iter()
        .map(|entry| entry.module.as_str())
        .collect::<HashSet<_>>();
    let expected = expected_modules
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if actual.len() != manifest.artifacts.len() || actual != expected {
        return Err(StagingError::ModuleMismatch);
    }
    Ok(())
}

fn normalize_relative_path(path: &str) -> Result<PathBuf, StagingError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path == "."
        || path.split('/').any(|part| part.is_empty() || part == "..")
    {
        return Err(StagingError::PathEscape);
    }
    Ok(PathBuf::from(path))
}

fn validate_manifest_schema(bytes: &[u8]) -> Result<(), StagingError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| StagingError::InvalidManifest)?;
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/standards/deploy-artifact-manifest.schema.json"
    ))
    .map_err(|_| StagingError::InvalidManifest)?;
    jsonschema::validator_for(&schema)
        .map_err(|_| StagingError::InvalidManifest)?
        .is_valid(&value)
        .then_some(())
        .ok_or(StagingError::InvalidManifest)
}

fn file_sha256(path: &Path) -> Result<String, StagingError> {
    let mut hasher = Sha256::new();
    let mut file = fs::File::open(path).map_err(StagingError::Io)?;
    io::copy(&mut file, &mut hasher).map_err(StagingError::Io)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_files(dir: &Path) -> Result<(HashMap<PathBuf, u64>, u64), StagingError> {
    let mut files = HashMap::new();
    let mut total = 0_u64;
    collect_files_inner(dir, dir, &mut files, &mut total)?;
    Ok((files, total))
}

fn collect_files_inner(
    dir: &Path,
    current: &Path,
    files: &mut HashMap<PathBuf, u64>,
    total: &mut u64,
) -> Result<(), StagingError> {
    for entry in fs::read_dir(current).map_err(StagingError::Io)? {
        let entry = entry.map_err(StagingError::Io)?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(StagingError::Io)?;
        if metadata.file_type().is_symlink() {
            return Err(StagingError::SymlinkForbidden);
        }
        if metadata.is_dir() {
            collect_files_inner(dir, &path, files, total)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(StagingError::InvalidDirectory);
        }
        let relative = path
            .strip_prefix(dir)
            .map_err(|_| StagingError::PathEscape)?;
        if relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        }) {
            return Err(StagingError::PathEscape);
        }
        *total = total.saturating_add(metadata.len());
        files.insert(relative.to_path_buf(), metadata.len());
    }
    Ok(())
}
