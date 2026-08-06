use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::ArtifactConfig;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactManifest {
    schema_version: u32,
    release_version: String,
    commit_sha: String,
    artifacts: Vec<ArtifactManifestEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactManifestEntry {
    module: String,
    path: String,
    sha256: String,
    size: u64,
}

pub(super) fn verify_archive(
    path: &Path,
    manifest_json: &str,
    archive_digest: &str,
    config: &ArtifactConfig,
    recorded_total_size: i64,
    recorded_file_count: i64,
) -> Result<(), &'static str> {
    let manifest: ArtifactManifest =
        serde_json::from_str(manifest_json).map_err(|_| "artifact_manifest_invalid")?;
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/standards/deploy-artifact-manifest.schema.json"
    ))
    .map_err(|_| "artifact_manifest_invalid")?;
    let validator = jsonschema::validator_for(&schema).map_err(|_| "artifact_manifest_invalid")?;
    let manifest_value: serde_json::Value =
        serde_json::from_str(manifest_json).map_err(|_| "artifact_manifest_invalid")?;
    if !validator.is_valid(&manifest_value)
        || manifest.schema_version != 1
        || manifest.release_version.is_empty()
        || manifest.commit_sha.is_empty()
        || manifest.artifacts.is_empty()
        || manifest.artifacts.len() > config.max_files as usize
    {
        return Err("artifact_manifest_invalid");
    }
    let mut expected = HashMap::new();
    let mut modules = HashSet::new();
    let mut expected_total = 0_u64;
    for item in manifest.artifacts {
        if !modules.insert(item.module)
            || !safe_relative_path(&item.path)
            || item.size > config.max_file_bytes
            || validate_hex_digest(&item.sha256).is_err()
        {
            return Err("artifact_manifest_invalid");
        }
        expected_total = expected_total
            .checked_add(item.size)
            .ok_or("artifact_size_limit")?;
        if expected_total > config.max_total_bytes
            || expected
                .insert(item.path, (item.size, item.sha256))
                .is_some()
        {
            return Err("artifact_manifest_invalid");
        }
    }
    if u64::try_from(recorded_total_size).ok() != Some(expected_total)
        || usize::try_from(recorded_file_count).ok() != Some(expected.len())
    {
        return Err("artifact_manifest_facts_mismatch");
    }
    verify_archive_digest(path, archive_digest)?;
    verify_entries(path, &expected, config.max_files)
}

fn verify_archive_digest(path: &Path, expected_digest: &str) -> Result<(), &'static str> {
    let mut file = std::fs::File::open(path).map_err(|_| "artifact_archive_missing")?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut HashWriter(&mut hasher))
        .map_err(|_| "artifact_archive_read_failed")?;
    if format!("{:x}", hasher.finalize()) != expected_digest {
        return Err("artifact_archive_digest_mismatch");
    }
    Ok(())
}

fn verify_entries(
    path: &Path,
    expected: &HashMap<String, (u64, String)>,
    max_files: u32,
) -> Result<(), &'static str> {
    let file = std::fs::File::open(path).map_err(|_| "artifact_archive_missing")?;
    let mut archive = tar::Archive::new(file);
    let mut seen = HashSet::new();
    let entries = archive.entries().map_err(|_| "artifact_archive_invalid")?;
    for (index, entry) in entries.enumerate() {
        if index >= max_files as usize * 2 {
            return Err("artifact_archive_entry_limit");
        }
        let mut entry = entry.map_err(|_| "artifact_archive_invalid")?;
        let path = entry.path().map_err(|_| "artifact_archive_invalid")?;
        let path = path.to_str().ok_or("artifact_archive_invalid")?.to_owned();
        if !safe_relative_path(&path) {
            return Err("artifact_archive_path_invalid");
        }
        if entry.header().entry_type().is_dir() {
            continue;
        }
        if !entry.header().entry_type().is_file() || !seen.insert(path.clone()) {
            return Err("artifact_archive_entry_invalid");
        }
        let (expected_size, expected_digest) =
            expected.get(&path).ok_or("artifact_archive_unknown_file")?;
        if entry.size() != *expected_size {
            return Err("artifact_file_size_mismatch");
        }
        let mut hasher = Sha256::new();
        std::io::copy(&mut entry, &mut HashWriter(&mut hasher))
            .map_err(|_| "artifact_archive_read_failed")?;
        if format!("{:x}", hasher.finalize()) != *expected_digest {
            return Err("artifact_file_digest_mismatch");
        }
    }
    if seen.len() != expected.len() {
        return Err("artifact_archive_missing_file");
    }
    Ok(())
}

fn safe_relative_path(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn validate_hex_digest(value: &str) -> Result<(), ()> {
    (value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()))
    .then_some(())
    .ok_or(())
}

struct HashWriter<'a>(&'a mut Sha256);

impl std::io::Write for HashWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.update(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
