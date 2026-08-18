use crate::protocol::{ReleaseStartRequest, SecretEnvironmentRequest, SecretEnvironmentValue};
use deploy_go_release_authorization::{
    AuthorizationError, Claims, ExpectedBinding, ExpectedSecretEnvironmentBinding, FileDigest,
    ReleaseVerifier,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::{fs::MetadataExt, fs::OpenOptionsExt, fs::PermissionsExt, io::AsRawFd},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};
use ulid::Ulid;
use zeroize::{Zeroize, Zeroizing};

pub const ARTIFACT_MANIFEST: &str = "deploy-go-artifact.json";
pub const FIXED_MAKE_PATH: &str = "/usr/bin/make";
pub const FIXED_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

#[derive(Clone)]
pub struct ReleaseAdmission {
    verifier: ReleaseVerifier,
    jobs_root: PathBuf,
    node_id: String,
    agent_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedRelease {
    pub job_dir: PathBuf,
    pub checkout_dir: PathBuf,
    pub artifact_dir: PathBuf,
    pub env_dir: PathBuf,
    pub claims: Claims,
    pub secret_environment: Vec<SecretEnvironmentValue>,
}

impl Drop for SealedRelease {
    fn drop(&mut self) {
        for variable in &mut self.secret_environment {
            variable.value.zeroize();
        }
    }
}

impl SealedRelease {
    pub fn command(&self, target_code: &str) -> Result<Command, ReleaseAdmissionError> {
        self.command_for(
            Path::new(FIXED_MAKE_PATH),
            &["--no-print-directory".into(), "deploy-go-release".into()],
            target_code,
        )
    }

    pub(crate) fn command_for(
        &self,
        program: &Path,
        arguments: &[String],
        target_code: &str,
    ) -> Result<Command, ReleaseAdmissionError> {
        if !valid_component(target_code) {
            return Err(ReleaseAdmissionError::InvalidRequest);
        }
        let cancel_file = self.job_dir.join("cancel");
        let mut command = Command::new(program);
        command
            .args(arguments)
            .current_dir(&self.checkout_dir)
            .env_clear()
            .env("PATH", FIXED_PATH)
            .env("DEPLOY_ID", &self.claims.deployment_id)
            .env("DEPLOY_ENVIRONMENT", &self.claims.environment)
            .env("DEPLOY_RELEASE_VERSION", &self.claims.release_version)
            .env("DEPLOY_COMMIT_SHA", &self.claims.commit_sha)
            .env("DEPLOY_MODULES", self.claims.modules.join(","))
            .env("DEPLOY_TARGET", target_code)
            .env("DEPLOY_ARTIFACT_DIR", &self.artifact_dir)
            .env("DEPLOY_ENV_DIR", &self.env_dir)
            .env("DEPLOY_CANCEL_FILE", cancel_file)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for variable in &self.secret_environment {
            command.env(&variable.name, &variable.value);
        }
        Ok(command)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReleaseAdmissionError {
    #[error("invalid release request")]
    InvalidRequest,
    #[error("invalid release authorization")]
    Authorization,
    #[error("release authorization binding mismatch")]
    Binding,
    #[error("release source path is outside the task root")]
    PathEscape,
    #[error("release source contains an unsupported file")]
    UnsafeFile,
    #[error("release source digest mismatch")]
    DigestMismatch,
    #[error("release job already exists")]
    JobConflict,
    #[error("release authorization nonce was already consumed")]
    Replayed,
    #[error("release admission storage unavailable")]
    Storage,
}

impl ReleaseAdmission {
    pub fn new(
        verifier: ReleaseVerifier,
        jobs_root: PathBuf,
        node_id: String,
        agent_id: String,
    ) -> Self {
        Self {
            verifier,
            jobs_root,
            node_id,
            agent_id,
        }
    }

    pub fn admit(
        &self,
        request: &ReleaseStartRequest,
        now: i64,
    ) -> Result<SealedRelease, ReleaseAdmissionError> {
        validate_request(request, &self.node_id, &self.agent_id)?;
        let expected = ExpectedBinding {
            deployment_id: &request.deployment_id,
            target_run_id: &request.target_run_id,
            target_id: &request.target_id,
            node_id: &request.node_id,
            agent_id: &request.agent_id,
            snapshot_hash: &request.snapshot_hash,
            commit_sha: &request.commit_sha,
            task_payload_digest: &request.task_payload_digest,
            deadline_at: request.deadline_at,
            secret_environment: request.secret_environment.as_ref().map(|secret| {
                ExpectedSecretEnvironmentBinding {
                    purpose: &secret.purpose,
                    variable_names: &secret.variable_names,
                    descriptor_digest: &secret.descriptor_digest,
                    value_digest: &secret.value_digest,
                    credential_version: secret.credential_version,
                    template_id: &secret.template_id,
                    template_version: &secret.template_version,
                    template_digest: &secret.template_digest,
                    release_stage: &secret.release_stage,
                    executor_audience: &secret.executor_audience,
                    target_process: &secret.target_process,
                }
            }),
        };
        let claims = self
            .verifier
            .verify(&request.authorization, &expected, now)
            .map_err(map_authorization_error)?;
        validate_claim_metadata(request, &claims)?;
        validate_secret_environment(request.secret_environment.as_ref(), &claims)?;
        let source_root = controlled_source_root(request)?;
        validate_source_root(&source_root)?;

        let checkout = Path::new(&request.checkout_dir);
        let artifact = Path::new(&request.artifact_dir);
        let env = Path::new(&request.env_dir);
        let checkout_digest = directory_digest(checkout, true)?;
        if !digest_matches(&claims.checkout_tree_digest, &checkout_digest) {
            return Err(ReleaseAdmissionError::DigestMismatch);
        }
        validate_claimed_directory(artifact, &claims.artifacts, true)?;
        validate_claimed_directory(env, &claims.env_files, false)?;
        let manifest_digest = file_digest(&artifact.join(ARTIFACT_MANIFEST))?;
        if !digest_matches(&claims.artifact_manifest_digest, &manifest_digest) {
            return Err(ReleaseAdmissionError::DigestMismatch);
        }
        validate_artifact_manifest(artifact, &claims)?;

        fs::create_dir_all(&self.jobs_root).map_err(|_| ReleaseAdmissionError::Storage)?;
        reject_symlink_path(&self.jobs_root)?;
        let final_dir = self.jobs_root.join(&request.job_id);
        let temporary = self.jobs_root.join(format!(".seal-{}", Ulid::new()));
        let result = seal_bundle(&temporary, checkout, artifact, env, &claims).and_then(|()| {
            let metadata =
                serde_json::to_vec(&claims).map_err(|_| ReleaseAdmissionError::Storage)?;
            write_read_only(&temporary.join("claims.json"), &metadata)?;
            make_tree_read_only(&temporary.join("bundle"))?;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o700))
                .map_err(|_| ReleaseAdmissionError::Storage)?;
            let _lock = AdmissionLock::acquire(&self.jobs_root)?;
            if final_dir.exists() {
                return Err(ReleaseAdmissionError::JobConflict);
            }
            if nonce_consumed(&self.jobs_root, &claims.nonce)? {
                return Err(ReleaseAdmissionError::Replayed);
            }
            fs::rename(&temporary, &final_dir).map_err(|_| ReleaseAdmissionError::Storage)
        });
        if let Err(error) = result {
            remove_sealed_tree(&temporary);
            return Err(error);
        }
        Ok(SealedRelease {
            checkout_dir: final_dir.join("bundle/checkout"),
            artifact_dir: final_dir.join("bundle/artifacts"),
            env_dir: final_dir.join("bundle/env"),
            job_dir: final_dir,
            claims,
            secret_environment: request
                .secret_environment
                .as_ref()
                .map(|value| value.variables.clone())
                .unwrap_or_default(),
        })
    }
}

fn validate_request(
    request: &ReleaseStartRequest,
    node_id: &str,
    agent_id: &str,
) -> Result<(), ReleaseAdmissionError> {
    if request.version != crate::protocol::PROTOCOL_VERSION
        || request.node_id != node_id
        || request.agent_id != agent_id
        || !valid_id(&request.job_id, "release_")
        || request.authorization.is_empty()
        || request.modules.is_empty()
        || request.modules.len() > 128
        || !matches!(
            request.environment.as_str(),
            "dev" | "test" | "staging" | "prod"
        )
        || !request.modules.iter().all(|value| valid_component(value))
        || !valid_component(&request.target_code)
    {
        return Err(ReleaseAdmissionError::InvalidRequest);
    }
    Ok(())
}

fn validate_secret_environment(
    secret: Option<&SecretEnvironmentRequest>,
    claims: &Claims,
) -> Result<(), ReleaseAdmissionError> {
    let Some(secret) = secret else {
        return Ok(());
    };
    if secret.variables.is_empty()
        || secret.variables.len() > 8
        || !matches!(
            secret.purpose.as_str(),
            "etcd-init" | "config-center-connection"
        )
        || !valid_sha256(&secret.descriptor_digest)
        || !valid_sha256(&secret.value_digest)
        || secret.credential_version == 0
        || !valid_component(&secret.template_id)
        || secret.template_version.is_empty()
        || secret.template_version.len() > 64
        || !valid_sha256(&secret.template_digest)
        || !matches!(secret.release_stage.as_str(), "release")
        || !matches!(
            secret.executor_audience.as_str(),
            "release_executor" | "etcd_template_bootstrap"
        )
        || !valid_component(&secret.target_process)
        || !valid_secret_environment_variables(secret)
        || claims.task_payload_digest.is_empty()
    {
        return Err(ReleaseAdmissionError::InvalidRequest);
    }
    let Some(claim) = claims.secret_environment.as_ref() else {
        return Err(ReleaseAdmissionError::Authorization);
    };
    if claim.descriptor_digest != secret.descriptor_digest
        || claim.purpose != secret.purpose
        || claim.variable_names != secret.variable_names
        || claim.value_digest != secret.value_digest
        || claim.credential_version != secret.credential_version
        || claim.template_id != secret.template_id
        || claim.template_version != secret.template_version
        || claim.template_digest != secret.template_digest
        || claim.release_stage != secret.release_stage
        || claim.executor_audience != secret.executor_audience
        || claim.target_process != secret.target_process
    {
        return Err(ReleaseAdmissionError::Binding);
    }
    let mut canonical = secret.variables.clone();
    canonical.sort_by(|left, right| left.name.cmp(&right.name));
    let value_digest = match serde_json::to_vec(&canonical) {
        Ok(bytes) => {
            let bytes = Zeroizing::new(bytes);
            format!("sha256:{:x}", Sha256::digest(bytes.as_slice()))
        }
        Err(_) => return Err(ReleaseAdmissionError::InvalidRequest),
    };
    for variable in &mut canonical {
        variable.value.zeroize();
    }
    if value_digest != secret.value_digest {
        return Err(ReleaseAdmissionError::Binding);
    }
    Ok(())
}

fn valid_secret_name(value: &str) -> bool {
    matches!(
        value,
        "ETCD_INIT_ROOT_USERNAME"
            | "ETCD_INIT_ROOT_PASSWORD"
            | "DEPLOY_CONFIG_CENTER_TYPE"
            | "DEPLOY_CONFIG_CENTER_ENDPOINTS"
            | "DEPLOY_CONFIG_CENTER_PREFIX"
            | "DEPLOY_CONFIG_CENTER_USERNAME"
            | "DEPLOY_CONFIG_CENTER_PASSWORD"
    )
}

fn valid_secret_environment_variables(secret: &SecretEnvironmentRequest) -> bool {
    let allowed: &[&str] = match (secret.purpose.as_str(), secret.executor_audience.as_str()) {
        ("etcd-init", "etcd_template_bootstrap") => {
            &["ETCD_INIT_ROOT_PASSWORD", "ETCD_INIT_ROOT_USERNAME"]
        }
        ("config-center-connection", "release_executor") => &[
            "DEPLOY_CONFIG_CENTER_ENDPOINTS",
            "DEPLOY_CONFIG_CENTER_PASSWORD",
            "DEPLOY_CONFIG_CENTER_PREFIX",
            "DEPLOY_CONFIG_CENTER_TYPE",
            "DEPLOY_CONFIG_CENTER_USERNAME",
        ],
        _ => return false,
    };
    let mut names = secret
        .variables
        .iter()
        .map(|variable| variable.name.as_str())
        .collect::<Vec<_>>();
    if names.iter().any(|name| !valid_secret_name(name))
        || secret
            .variables
            .iter()
            .any(|variable| variable.value.is_empty() || variable.value.len() > 65_536)
        || names.len() != allowed.len()
    {
        return false;
    }
    names.sort_unstable();
    names == allowed
        && secret
            .variable_names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            == allowed
}

fn valid_sha256(value: &str) -> bool {
    let value = value.strip_prefix("sha256:").unwrap_or(value);
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_claim_metadata(
    request: &ReleaseStartRequest,
    claims: &Claims,
) -> Result<(), ReleaseAdmissionError> {
    if claims.environment != request.environment
        || claims.release_version != request.release_version
        || claims.modules != request.modules
        || claims.cancel_file != request.cancel_file
    {
        return Err(ReleaseAdmissionError::Binding);
    }
    Ok(())
}

fn controlled_source_root(request: &ReleaseStartRequest) -> Result<PathBuf, ReleaseAdmissionError> {
    let checkout = normalized_absolute(Path::new(&request.checkout_dir))?;
    let artifact = normalized_absolute(Path::new(&request.artifact_dir))?;
    let env = normalized_absolute(Path::new(&request.env_dir))?;
    let cancel = normalized_absolute(Path::new(&request.cancel_file))?;
    let root = checkout.parent().ok_or(ReleaseAdmissionError::PathEscape)?;
    if artifact.parent() != Some(root)
        || env.parent() != Some(root)
        || cancel.parent() != Some(root)
    {
        return Err(ReleaseAdmissionError::PathEscape);
    }
    Ok(root.to_path_buf())
}

fn validate_source_root(root: &Path) -> Result<(), ReleaseAdmissionError> {
    reject_symlink_path(root)?;
    let metadata = fs::symlink_metadata(root).map_err(|_| ReleaseAdmissionError::PathEscape)?;
    if !metadata.is_dir() {
        return Err(ReleaseAdmissionError::PathEscape);
    }
    Ok(())
}

fn seal_bundle(
    destination: &Path,
    checkout: &Path,
    artifact: &Path,
    env: &Path,
    claims: &Claims,
) -> Result<(), ReleaseAdmissionError> {
    fs::create_dir(destination).map_err(|_| ReleaseAdmissionError::Storage)?;
    let bundle = destination.join("bundle");
    fs::create_dir(&bundle).map_err(|_| ReleaseAdmissionError::Storage)?;
    copy_checkout(checkout, &bundle.join("checkout"))?;
    copy_claimed(artifact, &bundle.join("artifacts"), &claims.artifacts, true)?;
    copy_claimed(env, &bundle.join("env"), &claims.env_files, false)?;
    let sealed_checkout_digest = directory_digest(&bundle.join("checkout"), true)?;
    if !digest_matches(&claims.checkout_tree_digest, &sealed_checkout_digest) {
        return Err(ReleaseAdmissionError::DigestMismatch);
    }
    validate_claimed_directory(&bundle.join("artifacts"), &claims.artifacts, true)?;
    validate_claimed_directory(&bundle.join("env"), &claims.env_files, false)?;
    let sealed_manifest_digest = file_digest(&bundle.join("artifacts").join(ARTIFACT_MANIFEST))?;
    if !digest_matches(&claims.artifact_manifest_digest, &sealed_manifest_digest) {
        return Err(ReleaseAdmissionError::DigestMismatch);
    }
    Ok(())
}

struct AdmissionLock(File);

impl AdmissionLock {
    fn acquire(jobs_root: &Path) -> Result<Self, ReleaseAdmissionError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(jobs_root.join(".admission.lock"))
            .map_err(|_| ReleaseAdmissionError::Storage)?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == -1 {
            return Err(ReleaseAdmissionError::Storage);
        }
        Ok(Self(file))
    }
}

impl Drop for AdmissionLock {
    fn drop(&mut self) {
        let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

fn nonce_consumed(jobs_root: &Path, nonce: &str) -> Result<bool, ReleaseAdmissionError> {
    for entry in fs::read_dir(jobs_root).map_err(|_| ReleaseAdmissionError::Storage)? {
        let entry = entry.map_err(|_| ReleaseAdmissionError::Storage)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(ReleaseAdmissionError::Storage);
        };
        if name.starts_with('.') {
            continue;
        }
        let metadata =
            fs::symlink_metadata(entry.path()).map_err(|_| ReleaseAdmissionError::Storage)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(ReleaseAdmissionError::Storage);
        }
        let claims: Claims = serde_json::from_slice(
            &fs::read(entry.path().join("claims.json"))
                .map_err(|_| ReleaseAdmissionError::Storage)?,
        )
        .map_err(|_| ReleaseAdmissionError::Storage)?;
        if claims.nonce == nonce {
            return Ok(true);
        }
    }
    Ok(false)
}

fn copy_checkout(source: &Path, destination: &Path) -> Result<(), ReleaseAdmissionError> {
    copy_tree(source, destination, true)
}

fn copy_tree(
    source: &Path,
    destination: &Path,
    skip_git: bool,
) -> Result<(), ReleaseAdmissionError> {
    validate_directory(source)?;
    fs::create_dir(destination).map_err(|_| ReleaseAdmissionError::Storage)?;
    let mut entries = fs::read_dir(source)
        .map_err(|_| ReleaseAdmissionError::UnsafeFile)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if skip_git && entry.file_name() == ".git" {
            continue;
        }
        let source_path = entry.path();
        let target_path = destination.join(entry.file_name());
        let metadata =
            fs::symlink_metadata(&source_path).map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
        if metadata.is_dir() {
            copy_tree(&source_path, &target_path, skip_git)?;
        } else if metadata.is_file() {
            copy_regular_file(&source_path, &target_path)?;
        } else {
            return Err(ReleaseAdmissionError::UnsafeFile);
        }
    }
    Ok(())
}

fn copy_claimed(
    source: &Path,
    destination: &Path,
    claims: &[FileDigest],
    include_manifest: bool,
) -> Result<(), ReleaseAdmissionError> {
    fs::create_dir(destination).map_err(|_| ReleaseAdmissionError::Storage)?;
    for claim in claims {
        let relative = normalized_relative(&claim.relative_path)?;
        let source_path = source.join(&relative);
        let target_path = destination.join(relative);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|_| ReleaseAdmissionError::Storage)?;
        }
        copy_regular_file(&source_path, &target_path)?;
    }
    if include_manifest {
        copy_regular_file(
            &source.join(ARTIFACT_MANIFEST),
            &destination.join(ARTIFACT_MANIFEST),
        )?;
    }
    Ok(())
}

fn copy_regular_file(source: &Path, destination: &Path) -> Result<(), ReleaseAdmissionError> {
    let before = fs::symlink_metadata(source).map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
    if !before.is_file() || before.nlink() != 1 {
        return Err(ReleaseAdmissionError::UnsafeFile);
    }
    let mut input = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(source)
        .map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
    let opened = input
        .metadata()
        .map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
    if !opened.is_file()
        || opened.nlink() != 1
        || opened.dev() != before.dev()
        || opened.ino() != before.ino()
        || opened.len() != before.len()
    {
        return Err(ReleaseAdmissionError::UnsafeFile);
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(if before.mode() & 0o111 == 0 {
            0o400
        } else {
            0o500
        })
        .open(destination)
        .map_err(|_| ReleaseAdmissionError::Storage)?;
    std::io::copy(&mut input, &mut output).map_err(|_| ReleaseAdmissionError::Storage)?;
    let after = input
        .metadata()
        .map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
    if after.len() != opened.len()
        || after.modified().ok() != opened.modified().ok()
        || after.dev() != opened.dev()
        || after.ino() != opened.ino()
    {
        return Err(ReleaseAdmissionError::UnsafeFile);
    }
    Ok(())
}

fn validate_claimed_directory(
    root: &Path,
    claims: &[FileDigest],
    allow_manifest: bool,
) -> Result<(), ReleaseAdmissionError> {
    validate_directory(root)?;
    let expected = claims
        .iter()
        .map(|claim| normalized_relative(&claim.relative_path))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut actual = BTreeMap::new();
    collect_files(root, root, &mut actual, false)?;
    if allow_manifest {
        actual.remove(Path::new(ARTIFACT_MANIFEST));
    }
    if actual.keys().cloned().collect::<BTreeSet<_>>() != expected {
        return Err(ReleaseAdmissionError::DigestMismatch);
    }
    for claim in claims {
        let relative = normalized_relative(&claim.relative_path)?;
        let digest = actual
            .get(&relative)
            .ok_or(ReleaseAdmissionError::DigestMismatch)?;
        if !digest_matches(&claim.digest, digest) {
            return Err(ReleaseAdmissionError::DigestMismatch);
        }
    }
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactManifest {
    schema_version: u32,
    release_version: String,
    commit_sha: String,
    artifacts: Vec<ArtifactEntry>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactEntry {
    module: String,
    path: String,
    sha256: String,
    size: u64,
}

fn validate_artifact_manifest(
    artifact_dir: &Path,
    claims: &Claims,
) -> Result<(), ReleaseAdmissionError> {
    let bytes = fs::read(artifact_dir.join(ARTIFACT_MANIFEST))
        .map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
    let manifest: ArtifactManifest =
        serde_json::from_slice(&bytes).map_err(|_| ReleaseAdmissionError::DigestMismatch)?;
    if manifest.schema_version != 1
        || manifest.release_version != claims.release_version
        || !manifest.commit_sha.eq_ignore_ascii_case(&claims.commit_sha)
        || manifest.artifacts.len() != claims.artifacts.len()
    {
        return Err(ReleaseAdmissionError::DigestMismatch);
    }
    let expected_modules = claims.modules.iter().cloned().collect::<BTreeSet<_>>();
    let actual_modules = manifest
        .artifacts
        .iter()
        .map(|entry| entry.module.clone())
        .collect::<BTreeSet<_>>();
    if expected_modules != actual_modules || actual_modules.len() != manifest.artifacts.len() {
        return Err(ReleaseAdmissionError::DigestMismatch);
    }
    let expected = claims
        .artifacts
        .iter()
        .map(|file| (file.relative_path.as_str(), file.digest.as_str()))
        .collect::<BTreeMap<_, _>>();
    for entry in manifest.artifacts {
        let digest = expected
            .get(entry.path.as_str())
            .ok_or(ReleaseAdmissionError::DigestMismatch)?;
        if !digest_matches(digest, &entry.sha256) {
            return Err(ReleaseAdmissionError::DigestMismatch);
        }
        let metadata = fs::symlink_metadata(artifact_dir.join(normalized_relative(&entry.path)?))
            .map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
        if metadata.len() != entry.size {
            return Err(ReleaseAdmissionError::DigestMismatch);
        }
    }
    Ok(())
}

pub fn directory_digest(root: &Path, skip_git: bool) -> Result<String, ReleaseAdmissionError> {
    validate_directory(root)?;
    let mut files = BTreeMap::new();
    collect_files(root, root, &mut files, skip_git)?;
    let mut digest = Sha256::new();
    for (path, file_digest) in files {
        let path = path.to_string_lossy();
        digest.update((path.len() as u64).to_be_bytes());
        digest.update(path.as_bytes());
        digest.update(file_digest.as_bytes());
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut BTreeMap<PathBuf, String>,
    skip_git: bool,
) -> Result<(), ReleaseAdmissionError> {
    validate_directory(current)?;
    for entry in fs::read_dir(current).map_err(|_| ReleaseAdmissionError::UnsafeFile)? {
        let entry = entry.map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
        if skip_git && current == root && entry.file_name() == ".git" {
            continue;
        }
        let path = entry.path();
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
        if metadata.is_dir() {
            collect_files(root, &path, files, skip_git)?;
        } else if metadata.is_file() && metadata.nlink() == 1 {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| ReleaseAdmissionError::PathEscape)?
                .to_path_buf();
            files.insert(relative, file_digest(&path)?);
        } else {
            return Err(ReleaseAdmissionError::UnsafeFile);
        }
    }
    Ok(())
}

pub fn file_digest(path: &Path) -> Result<String, ReleaseAdmissionError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(ReleaseAdmissionError::UnsafeFile);
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn validate_directory(path: &Path) -> Result<(), ReleaseAdmissionError> {
    reject_symlink_path(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| ReleaseAdmissionError::UnsafeFile)?;
    if !metadata.is_dir() {
        return Err(ReleaseAdmissionError::UnsafeFile);
    }
    Ok(())
}

fn reject_symlink_path(path: &Path) -> Result<(), ReleaseAdmissionError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::RootDir) {
            continue;
        }
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| ReleaseAdmissionError::PathEscape)?;
        if metadata.file_type().is_symlink() {
            return Err(ReleaseAdmissionError::PathEscape);
        }
    }
    Ok(())
}

fn normalized_absolute(path: &Path) -> Result<PathBuf, ReleaseAdmissionError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(ReleaseAdmissionError::PathEscape);
    }
    Ok(path.to_path_buf())
}

fn normalized_relative(value: &str) -> Result<PathBuf, ReleaseAdmissionError> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ReleaseAdmissionError::PathEscape);
    }
    Ok(path.to_path_buf())
}

fn digest_matches(expected: &str, actual: &str) -> bool {
    expected.strip_prefix("sha256:").unwrap_or(expected) == actual
}

fn valid_id(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix) && value.len() <= 128 && valid_component(value)
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn write_read_only(path: &Path, contents: &[u8]) -> Result<(), ReleaseAdmissionError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(path)
        .map_err(|_| ReleaseAdmissionError::Storage)?;
    file.write_all(contents)
        .map_err(|_| ReleaseAdmissionError::Storage)
}

fn make_tree_read_only(root: &Path) -> Result<(), ReleaseAdmissionError> {
    let mut directories = vec![root.to_path_buf()];
    let mut index = 0;
    while index < directories.len() {
        let directory = directories[index].clone();
        for entry in fs::read_dir(&directory).map_err(|_| ReleaseAdmissionError::Storage)? {
            let path = entry.map_err(|_| ReleaseAdmissionError::Storage)?.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|_| ReleaseAdmissionError::Storage)?;
            if metadata.is_dir() {
                directories.push(path);
            } else if metadata.is_file() {
                let mode = if metadata.mode() & 0o111 == 0 {
                    0o400
                } else {
                    0o500
                };
                fs::set_permissions(&path, fs::Permissions::from_mode(mode))
                    .map_err(|_| ReleaseAdmissionError::Storage)?;
            } else {
                return Err(ReleaseAdmissionError::UnsafeFile);
            }
        }
        index += 1;
    }
    for directory in directories.into_iter().rev() {
        fs::set_permissions(directory, fs::Permissions::from_mode(0o500))
            .map_err(|_| ReleaseAdmissionError::Storage)?;
    }
    Ok(())
}

fn remove_sealed_tree(root: &Path) {
    if !root.exists() {
        return;
    }
    let mut directories = vec![root.to_path_buf()];
    let mut index = 0;
    while index < directories.len() {
        let directory = directories[index].clone();
        let _ = fs::set_permissions(&directory, fs::Permissions::from_mode(0o700));
        if let Ok(entries) = fs::read_dir(&directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    directories.push(path);
                } else {
                    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
                }
            }
        }
        index += 1;
    }
    let _ = fs::remove_dir_all(root);
}

fn map_authorization_error(error: AuthorizationError) -> ReleaseAdmissionError {
    match error {
        AuthorizationError::BindingMismatch => ReleaseAdmissionError::Binding,
        AuthorizationError::InvalidKey
        | AuthorizationError::InvalidFormat
        | AuthorizationError::InvalidSignature
        | AuthorizationError::InvalidClaims
        | AuthorizationError::InvalidTime => ReleaseAdmissionError::Authorization,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deploy_go_release_authorization::{AUDIENCE, SCHEMA_VERSION, SecretEnvironmentClaims};

    fn claims(secret: SecretEnvironmentClaims) -> Claims {
        Claims {
            schema_version: SCHEMA_VERSION,
            audience: AUDIENCE.into(),
            authorization_id: "release_auth_test".into(),
            nonce: "release_nonce_test".into(),
            deployment_id: "deployment_test".into(),
            target_run_id: "run_test".into(),
            target_id: "target_test".into(),
            node_id: "node_test".into(),
            agent_id: "agent_test".into(),
            snapshot_hash: "sha256:snapshot".into(),
            commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
            checkout_tree_digest: "sha256:checkout".into(),
            artifact_manifest_digest: "sha256:manifest".into(),
            artifacts: vec![FileDigest {
                relative_path: "artifact".into(),
                digest: "sha256:artifact".into(),
            }],
            env_files: Vec::new(),
            environment: "test".into(),
            release_version: "release".into(),
            modules: vec!["api".into()],
            task_payload_digest: "sha256:payload".into(),
            cancel_file: "/tmp/cancel".into(),
            issued_at: 1,
            expires_at: 2,
            deadline_at: 2,
            secret_environment: Some(secret),
        }
    }

    fn secret_request(value_digest: String) -> SecretEnvironmentRequest {
        SecretEnvironmentRequest {
            purpose: "config-center-connection".into(),
            variable_names: vec![
                "DEPLOY_CONFIG_CENTER_ENDPOINTS".into(),
                "DEPLOY_CONFIG_CENTER_PASSWORD".into(),
                "DEPLOY_CONFIG_CENTER_PREFIX".into(),
                "DEPLOY_CONFIG_CENTER_TYPE".into(),
                "DEPLOY_CONFIG_CENTER_USERNAME".into(),
            ],
            descriptor_digest: format!("sha256:{}", "d".repeat(64)),
            value_digest,
            credential_version: 3,
            template_id: "etcd".into(),
            template_version: "3.6".into(),
            template_digest: format!("sha256:{}", "e".repeat(64)),
            release_stage: "release".into(),
            executor_audience: "release_executor".into(),
            target_process: "deploy-release".into(),
            variables: vec![
                SecretEnvironmentValue {
                    name: "DEPLOY_CONFIG_CENTER_TYPE".into(),
                    value: "etcd".into(),
                },
                SecretEnvironmentValue {
                    name: "DEPLOY_CONFIG_CENTER_ENDPOINTS".into(),
                    value: "[\"http://127.0.0.1:2379\"]".into(),
                },
                SecretEnvironmentValue {
                    name: "DEPLOY_CONFIG_CENTER_PREFIX".into(),
                    value: "/deploy-go/apps/a/test/".into(),
                },
                SecretEnvironmentValue {
                    name: "DEPLOY_CONFIG_CENTER_USERNAME".into(),
                    value: "a_test".into(),
                },
                SecretEnvironmentValue {
                    name: "DEPLOY_CONFIG_CENTER_PASSWORD".into(),
                    value: "password".into(),
                },
            ],
        }
    }

    fn secret_claims(request: &SecretEnvironmentRequest) -> SecretEnvironmentClaims {
        SecretEnvironmentClaims {
            purpose: request.purpose.clone(),
            variable_names: request.variable_names.clone(),
            descriptor_digest: request.descriptor_digest.clone(),
            value_digest: request.value_digest.clone(),
            credential_version: request.credential_version,
            template_id: request.template_id.clone(),
            template_version: request.template_version.clone(),
            template_digest: request.template_digest.clone(),
            release_stage: request.release_stage.clone(),
            executor_audience: request.executor_audience.clone(),
            target_process: request.target_process.clone(),
        }
    }

    #[test]
    fn secret_admission_recomputes_value_digest_and_rejects_unknown_names() {
        let mut canonical = secret_request(String::new());
        canonical
            .variables
            .sort_by(|left, right| left.name.cmp(&right.name));
        let digest = format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&canonical.variables).unwrap())
        );
        let request = secret_request(digest.clone());
        let claims = claims(secret_claims(&request));
        assert!(validate_secret_environment(Some(&request), &claims).is_ok());

        let wrong_digest = secret_request(format!("sha256:{}", "f".repeat(64)));
        assert_eq!(
            validate_secret_environment(Some(&wrong_digest), &claims),
            Err(ReleaseAdmissionError::Binding)
        );

        let mut unknown = request;
        unknown.variables[0].name = "PATH".into();
        assert_eq!(
            validate_secret_environment(Some(&unknown), &claims),
            Err(ReleaseAdmissionError::InvalidRequest)
        );
    }
}
