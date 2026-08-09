use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
};

use deploy_go_terminal_capability::{CapabilityVerifier, ExpectedBinding};
use sha2::{Digest, Sha256};

use crate::protocol::OpenRequest;

pub struct CapabilityAuthorizer {
    verifier: CapabilityVerifier,
    node_id: String,
    agent_id: String,
    replay_dir: PathBuf,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthorizationError {
    #[error("invalid terminal capability")]
    InvalidCapability,
    #[error("terminal capability already consumed")]
    Replayed,
    #[error("terminal capability replay store unavailable")]
    ReplayStore,
}

impl CapabilityAuthorizer {
    pub fn new(
        verifier: CapabilityVerifier,
        node_id: String,
        agent_id: String,
        replay_dir: PathBuf,
    ) -> Self {
        Self {
            verifier,
            node_id,
            agent_id,
            replay_dir,
        }
    }

    pub fn authorize(&self, request: &OpenRequest, now: i64) -> Result<(), AuthorizationError> {
        let claims = self
            .verifier
            .verify(
                &request.capability,
                &ExpectedBinding {
                    node_id: &self.node_id,
                    agent_id: &self.agent_id,
                    session_id: &request.session_id,
                    connection_generation: request.connection_generation,
                },
                now,
            )
            .map_err(|_| AuthorizationError::InvalidCapability)?;
        self.prepare_replay_dir()?;
        let digest = Sha256::digest(request.capability.as_bytes());
        let marker = self.replay_dir.join(format!("{digest:x}"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(marker)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    AuthorizationError::Replayed
                } else {
                    AuthorizationError::ReplayStore
                }
            })?;
        writeln!(file, "{}", claims.expires_at).map_err(|_| AuthorizationError::ReplayStore)?;
        file.sync_all()
            .map_err(|_| AuthorizationError::ReplayStore)?;
        std::fs::File::open(&self.replay_dir)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| AuthorizationError::ReplayStore)?;
        Ok(())
    }

    fn prepare_replay_dir(&self) -> Result<(), AuthorizationError> {
        std::fs::create_dir_all(&self.replay_dir).map_err(|_| AuthorizationError::ReplayStore)?;
        let metadata = std::fs::symlink_metadata(&self.replay_dir)
            .map_err(|_| AuthorizationError::ReplayStore)?;
        let parent = self
            .replay_dir
            .parent()
            .ok_or(AuthorizationError::ReplayStore)?;
        let parent_metadata =
            std::fs::symlink_metadata(parent).map_err(|_| AuthorizationError::ReplayStore)?;
        let effective_uid = unsafe { libc::geteuid() };
        if parent_metadata.file_type().is_symlink()
            || !parent_metadata.is_dir()
            || parent_metadata.uid() != effective_uid
            || parent_metadata.mode() & 0o022 != 0
        {
            return Err(AuthorizationError::ReplayStore);
        }
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || metadata.uid() != effective_uid
        {
            return Err(AuthorizationError::ReplayStore);
        }
        std::fs::set_permissions(&self.replay_dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| AuthorizationError::ReplayStore)?;
        let secured = std::fs::symlink_metadata(&self.replay_dir)
            .map_err(|_| AuthorizationError::ReplayStore)?;
        if secured.file_type().is_symlink()
            || !secured.is_dir()
            || secured.uid() != effective_uid
            || secured.mode() & 0o777 != 0o700
        {
            return Err(AuthorizationError::ReplayStore);
        }
        Ok(())
    }
}
