use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u8 = 1;
pub const AUDIENCE: &str = "deploy-go:deployment-release";
pub const MAX_TTL_SECONDS: i64 = 86_460;
pub const CLOCK_SKEW_SECONDS: i64 = 5;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileDigest {
    pub relative_path: String,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub schema_version: u8,
    pub audience: String,
    pub authorization_id: String,
    pub nonce: String,
    pub deployment_id: String,
    pub target_run_id: String,
    pub target_id: String,
    pub node_id: String,
    pub agent_id: String,
    pub snapshot_hash: String,
    pub commit_sha: String,
    pub checkout_tree_digest: String,
    pub artifact_manifest_digest: String,
    pub artifacts: Vec<FileDigest>,
    pub env_files: Vec<FileDigest>,
    pub environment: String,
    pub release_version: String,
    pub modules: Vec<String>,
    pub task_payload_digest: String,
    pub cancel_file: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub deadline_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_environment: Option<SecretEnvironmentClaims>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecretEnvironmentClaims {
    pub purpose: String,
    pub variable_names: Vec<String>,
    pub descriptor_digest: String,
    pub value_digest: String,
    pub credential_version: u64,
    pub template_id: String,
    pub template_version: String,
    pub template_digest: String,
    pub release_stage: String,
    pub executor_audience: String,
    pub target_process: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedBinding<'a> {
    pub deployment_id: &'a str,
    pub target_run_id: &'a str,
    pub target_id: &'a str,
    pub node_id: &'a str,
    pub agent_id: &'a str,
    pub snapshot_hash: &'a str,
    pub commit_sha: &'a str,
    pub task_payload_digest: &'a str,
    pub deadline_at: i64,
    pub secret_environment: Option<ExpectedSecretEnvironmentBinding<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedSecretEnvironmentBinding<'a> {
    pub purpose: &'a str,
    pub variable_names: &'a [String],
    pub descriptor_digest: &'a str,
    pub value_digest: &'a str,
    pub credential_version: u64,
    pub template_id: &'a str,
    pub template_version: &'a str,
    pub template_digest: &'a str,
    pub release_stage: &'a str,
    pub executor_audience: &'a str,
    pub target_process: &'a str,
}

#[derive(Clone)]
pub struct ReleaseSigner(SigningKey);

#[derive(Clone)]
pub struct ReleaseVerifier(VerifyingKey);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthorizationError {
    #[error("invalid release authorization key")]
    InvalidKey,
    #[error("invalid release authorization format")]
    InvalidFormat,
    #[error("invalid release authorization signature")]
    InvalidSignature,
    #[error("invalid release authorization claims")]
    InvalidClaims,
    #[error("release authorization binding mismatch")]
    BindingMismatch,
    #[error("release authorization is not currently valid")]
    InvalidTime,
}

impl ReleaseSigner {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self(SigningKey::from_bytes(&seed))
    }

    pub fn sign(&self, claims: &Claims) -> Result<String, AuthorizationError> {
        validate_claims(claims)?;
        let payload = serde_json::to_vec(claims).map_err(|_| AuthorizationError::InvalidClaims)?;
        let signature = self.0.sign(&payload);
        Ok(format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }

    pub fn public_key_base64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0.verifying_key().to_bytes())
    }

    pub fn verifier(&self) -> ReleaseVerifier {
        ReleaseVerifier(self.0.verifying_key())
    }
}

impl ReleaseVerifier {
    pub fn from_base64(value: &str) -> Result<Self, AuthorizationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| AuthorizationError::InvalidKey)?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| AuthorizationError::InvalidKey)?;
        let key = VerifyingKey::from_bytes(&bytes).map_err(|_| AuthorizationError::InvalidKey)?;
        Ok(Self(key))
    }

    pub fn verify(
        &self,
        token: &str,
        expected: &ExpectedBinding<'_>,
        now: i64,
    ) -> Result<Claims, AuthorizationError> {
        let (payload, signature) = token
            .split_once('.')
            .filter(|(_, signature)| !signature.contains('.'))
            .ok_or(AuthorizationError::InvalidFormat)?;
        let payload = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| AuthorizationError::InvalidFormat)?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| AuthorizationError::InvalidFormat)?;
        let signature =
            Signature::from_slice(&signature).map_err(|_| AuthorizationError::InvalidFormat)?;
        self.0
            .verify(&payload, &signature)
            .map_err(|_| AuthorizationError::InvalidSignature)?;
        let claims: Claims =
            serde_json::from_slice(&payload).map_err(|_| AuthorizationError::InvalidClaims)?;
        validate_claims(&claims)?;
        validate_binding(&claims, expected)?;
        if claims.issued_at > now.saturating_add(CLOCK_SKEW_SECONDS)
            || claims.expires_at <= now
            || claims.deadline_at <= now
            || claims.expires_at > claims.deadline_at
            || claims.expires_at.saturating_sub(claims.issued_at) > MAX_TTL_SECONDS
        {
            return Err(AuthorizationError::InvalidTime);
        }
        Ok(claims)
    }
}

fn validate_binding(
    claims: &Claims,
    expected: &ExpectedBinding<'_>,
) -> Result<(), AuthorizationError> {
    if claims.deployment_id != expected.deployment_id
        || claims.target_run_id != expected.target_run_id
        || claims.target_id != expected.target_id
        || claims.node_id != expected.node_id
        || claims.agent_id != expected.agent_id
        || claims.snapshot_hash != expected.snapshot_hash
        || claims.commit_sha != expected.commit_sha
        || claims.task_payload_digest != expected.task_payload_digest
        || claims.deadline_at != expected.deadline_at
    {
        return Err(AuthorizationError::BindingMismatch);
    }
    match (
        claims.secret_environment.as_ref(),
        expected.secret_environment.as_ref(),
    ) {
        (Some(claim), Some(expected))
            if claim.purpose == expected.purpose
                && claim.variable_names == expected.variable_names
                && claim.descriptor_digest == expected.descriptor_digest
                && claim.value_digest == expected.value_digest
                && claim.credential_version == expected.credential_version
                && claim.template_id == expected.template_id
                && claim.template_version == expected.template_version
                && claim.template_digest == expected.template_digest
                && claim.release_stage == expected.release_stage
                && claim.executor_audience == expected.executor_audience
                && claim.target_process == expected.target_process => {}
        (None, None) => {}
        _ => return Err(AuthorizationError::BindingMismatch),
    }
    Ok(())
}

fn validate_claims(claims: &Claims) -> Result<(), AuthorizationError> {
    if claims.schema_version != SCHEMA_VERSION
        || claims.audience != AUDIENCE
        || !valid_id(&claims.authorization_id, "release_auth_")
        || !valid_id(&claims.nonce, "release_nonce_")
        || !valid_id(&claims.deployment_id, "")
        || !valid_id(&claims.target_run_id, "")
        || !valid_id(&claims.target_id, "")
        || !valid_id(&claims.node_id, "")
        || !valid_id(&claims.agent_id, "")
        || !valid_sha256(&claims.snapshot_hash)
        || !valid_commit(&claims.commit_sha)
        || !valid_sha256(&claims.checkout_tree_digest)
        || !valid_sha256(&claims.artifact_manifest_digest)
        || !valid_sha256(&claims.task_payload_digest)
        || !valid_absolute_path(&claims.cancel_file)
        || claims.artifacts.is_empty()
        || claims.modules.is_empty()
        || claims.expires_at <= claims.issued_at
        || claims.deadline_at < claims.expires_at
        || !valid_environment(&claims.environment)
        || claims.release_version.is_empty()
        || claims.release_version.len() > 256
        || claims.artifacts.len() > 256
        || claims.env_files.len() > 128
        || claims.modules.len() > 128
        || !claims.artifacts.iter().all(valid_file_digest)
        || !claims.env_files.iter().all(valid_file_digest)
        || !claims
            .modules
            .iter()
            .all(|module| valid_component(module, 128))
        || !claims
            .secret_environment
            .as_ref()
            .is_none_or(valid_secret_environment_claims)
    {
        return Err(AuthorizationError::InvalidClaims);
    }
    Ok(())
}

fn valid_secret_environment_claims(claims: &SecretEnvironmentClaims) -> bool {
    matches!(
        claims.purpose.as_str(),
        "etcd-init" | "config-center-connection"
    ) && !claims.variable_names.is_empty()
        && claims.variable_names.len() <= 8
        && claims
            .variable_names
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        && claims.credential_version > 0
        && valid_sha256(&claims.descriptor_digest)
        && valid_sha256(&claims.value_digest)
        && valid_component(&claims.template_id, 128)
        && !claims.template_version.is_empty()
        && claims.template_version.len() <= 64
        && valid_sha256(&claims.template_digest)
        && claims.release_stage == "release"
        && matches!(
            claims.executor_audience.as_str(),
            "release_executor" | "etcd_template_bootstrap"
        )
        && valid_component(&claims.target_process, 128)
}

fn valid_file_digest(file: &FileDigest) -> bool {
    valid_relative_path(&file.relative_path) && valid_sha256(&file.digest)
}

fn valid_id(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix) && valid_component(value, 128)
}

fn valid_component(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn valid_sha256(value: &str) -> bool {
    let digest = value.strip_prefix("sha256:").unwrap_or(value);
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && !value.starts_with('/')
        && value
            .split('/')
            .all(|part| !matches!(part, "" | "." | ".."))
}

fn valid_absolute_path(value: &str) -> bool {
    value.starts_with('/')
        && value.len() <= 4096
        && value
            .split('/')
            .skip(1)
            .all(|part| !matches!(part, "" | "." | ".."))
}

fn valid_environment(value: &str) -> bool {
    matches!(value, "dev" | "test" | "staging" | "prod")
}
