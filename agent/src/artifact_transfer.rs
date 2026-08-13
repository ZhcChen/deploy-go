use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use deploy_go_agent_protocol::ArtifactPrepared;
use futures_util::StreamExt;
use reqwest::{Client, StatusCode, header};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

use crate::{
    staging::{StagingLimits, verify_artifact_dir},
    token_refresh::AccessProvider,
};

const CHUNK_SIZE: usize = 1024 * 1024;

#[derive(Debug, Error)]
pub enum ArtifactTransferError {
    #[error("artifact 传输未启用")]
    Disabled,
    #[error("artifact 路径无效")]
    InvalidPath,
    #[error("artifact 响应无效")]
    InvalidResponse,
    #[error("artifact 服务拒绝请求")]
    Rejected,
    #[error("artifact 摘要不匹配")]
    DigestMismatch,
    #[error("artifact 本地 IO 失败: {0}")]
    Io(#[from] io::Error),
    #[error("artifact HTTP 传输失败")]
    Transport,
    #[error("artifact 本地校验失败")]
    Verification,
}

#[derive(Clone)]
pub struct ArtifactTransferClient {
    client: Client,
    api_base: Url,
    access_provider: Arc<dyn AccessProvider>,
    enabled: bool,
}

#[derive(Debug)]
pub struct PreparedArchive {
    pub path: PathBuf,
    pub notice: ArtifactPrepared,
}

pub struct ArchivePreparation<'a> {
    pub task_id: &'a str,
    pub authorization_id: &'a str,
    pub deployment_id: &'a str,
    pub artifact_dir: &'a Path,
    pub archive_path: &'a Path,
    pub expected_release: &'a str,
    pub expected_commit: &'a str,
    pub expected_modules: &'a [String],
    pub limits: &'a StagingLimits,
}

#[derive(Deserialize)]
struct UploadStatus {
    offset: u64,
    upload_size: u64,
}

impl ArtifactTransferClient {
    pub fn new(api_base: Url, access_provider: Arc<dyn AccessProvider>, enabled: bool) -> Self {
        Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(900))
                .build()
                .expect("固定 artifact HTTP client 配置有效"),
            api_base,
            access_provider,
            enabled,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn prepare_archive(
        &self,
        preparation: ArchivePreparation<'_>,
    ) -> Result<PreparedArchive, ArtifactTransferError> {
        if !self.enabled {
            return Err(ArtifactTransferError::Disabled);
        }
        let manifest = verify_artifact_dir(
            preparation.artifact_dir,
            preparation.expected_release,
            preparation.expected_commit,
            preparation.expected_modules,
            preparation.limits,
        )
        .map_err(|_| ArtifactTransferError::Verification)?;
        let manifest_bytes = fs::read(preparation.artifact_dir.join("deploy-go-artifact.json"))?;
        let manifest_digest = hex_digest(&manifest_bytes);
        create_deterministic_tar(preparation.artifact_dir, preparation.archive_path)?;
        let archive_size = fs::metadata(preparation.archive_path)?.len();
        let archive_digest = file_digest(preparation.archive_path)?;
        let total_size = manifest.artifacts.iter().map(|item| item.size).sum();
        let file_count = u32::try_from(manifest.artifacts.len())
            .map_err(|_| ArtifactTransferError::InvalidResponse)?;
        Ok(PreparedArchive {
            path: preparation.archive_path.to_owned(),
            notice: ArtifactPrepared {
                task_id: preparation.task_id.to_owned(),
                authorization_id: preparation.authorization_id.to_owned(),
                deployment_id: preparation.deployment_id.to_owned(),
                manifest_json: String::from_utf8(manifest_bytes)
                    .map_err(|_| ArtifactTransferError::InvalidResponse)?,
                manifest_digest,
                total_size,
                file_count,
                archive_size,
                archive_digest,
            },
        })
    }

    pub async fn upload(
        &self,
        lease_id: &str,
        archive: &PreparedArchive,
    ) -> Result<(), ArtifactTransferError> {
        let endpoint = self.endpoint(lease_id, "upload")?;
        let response = self
            .send_authenticated(self.client.post(endpoint.clone()).json(&serde_json::json!({
                "upload_size": archive.notice.archive_size,
                "archive_digest": archive.notice.archive_digest,
            })))
            .await?;
        let mut status = decode_status(response).await?;
        validate_upload_status(&status, archive.notice.archive_size, 0)?;
        let mut file = fs::File::open(&archive.path)?;
        let mut stalled = 0_u8;
        let mut resumable_failures = 0_u8;
        while status.offset < status.upload_size {
            let previous = status.offset;
            file.seek(SeekFrom::Start(status.offset))?;
            let remaining = status.upload_size - status.offset;
            let length = remaining.min(CHUNK_SIZE as u64) as usize;
            let mut chunk = vec![0; length];
            file.read_exact(&mut chunk)?;
            let end = status.offset + length as u64 - 1;
            let request = self
                .client
                .put(endpoint.clone())
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {}-{end}/{}", status.offset, status.upload_size),
                )
                .body(chunk);
            let upload_result = self.send_authenticated(request).await;
            let decoded = match upload_result {
                Ok(response) => decode_status(response).await,
                Err(error) => Err(error),
            };
            status = match decoded {
                Ok(status) => {
                    validate_upload_status(&status, archive.notice.archive_size, previous)?;
                    if status.offset != end + 1 {
                        return Err(ArtifactTransferError::InvalidResponse);
                    }
                    resumable_failures = 0;
                    status
                }
                Err(_) => {
                    resumable_failures = resumable_failures.saturating_add(1);
                    if resumable_failures > 3 {
                        return Err(ArtifactTransferError::Rejected);
                    }
                    let response = self
                        .send_authenticated(self.client.get(endpoint.clone()))
                        .await?;
                    let status = decode_status(response).await?;
                    validate_resume_upload_status(
                        &status,
                        archive.notice.archive_size,
                        previous,
                        end,
                    )?;
                    status
                }
            };
            if status.offset == previous {
                stalled = stalled.saturating_add(1);
                if stalled > 3 {
                    return Err(ArtifactTransferError::InvalidResponse);
                }
            } else {
                stalled = 0;
            }
        }
        let finalize = self.endpoint(lease_id, "upload/finalize")?;
        require_success(self.send_authenticated(self.client.post(finalize)).await?).await
    }

    pub async fn download(
        &self,
        lease_id: &str,
        archive_path: &Path,
        expected_archive_digest: &str,
    ) -> Result<(), ArtifactTransferError> {
        if !self.enabled {
            return Err(ArtifactTransferError::Disabled);
        }
        if archive_path.is_file() && file_digest(archive_path)? == expected_archive_digest {
            return Ok(());
        }
        remove_if_exists(archive_path)?;
        let partial_path = append_suffix(archive_path, ".part")?;
        let metadata_path = append_suffix(&partial_path, ".meta")?;
        if fs::read_to_string(&metadata_path).ok().as_deref() != Some(expected_archive_digest) {
            remove_if_exists(&partial_path)?;
            remove_if_exists(&metadata_path)?;
        }
        fs::write(&metadata_path, expected_archive_digest.as_bytes())?;
        let mut result = self
            .download_once(lease_id, &partial_path, expected_archive_digest)
            .await;
        if matches!(
            &result,
            Err(ArtifactTransferError::DigestMismatch)
                | Err(ArtifactTransferError::InvalidResponse)
                | Err(ArtifactTransferError::Rejected)
        ) {
            remove_if_exists(&partial_path)?;
            result = self
                .download_once(lease_id, &partial_path, expected_archive_digest)
                .await;
        }
        if result.is_err() {
            remove_if_exists(&partial_path)?;
            remove_if_exists(&metadata_path)?;
        } else {
            fs::rename(&partial_path, archive_path)?;
            remove_if_exists(&metadata_path)?;
        }
        result
    }

    async fn download_once(
        &self,
        lease_id: &str,
        archive_path: &Path,
        expected_archive_digest: &str,
    ) -> Result<(), ArtifactTransferError> {
        let endpoint = self.endpoint(lease_id, "download")?;
        let mut offset = fs::metadata(archive_path)
            .map(|item| item.len())
            .unwrap_or(0);
        let mut interruptions = 0_u8;
        loop {
            let response = self.range_request(endpoint.clone(), offset).await?;
            if response.status() != StatusCode::PARTIAL_CONTENT {
                return Err(ArtifactTransferError::Rejected);
            }
            let (response_end, total) = parse_content_range(response.headers(), offset)?;
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(archive_path)?;
            let mut stream = response.bytes_stream();
            let mut interrupted = false;
            while let Some(bytes) = stream.next().await {
                let bytes = match bytes {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        interrupted = true;
                        break;
                    }
                };
                file.write_all(&bytes)?;
                offset = offset
                    .checked_add(bytes.len() as u64)
                    .ok_or(ArtifactTransferError::InvalidResponse)?;
            }
            file.sync_all()?;
            if interrupted {
                interruptions = interruptions.saturating_add(1);
                if interruptions > 3 {
                    return Err(ArtifactTransferError::Transport);
                }
                continue;
            }
            if offset != response_end.saturating_add(1) {
                return Err(ArtifactTransferError::InvalidResponse);
            }
            if offset == total {
                break;
            }
        }
        if file_digest(archive_path)? != expected_archive_digest {
            return Err(ArtifactTransferError::DigestMismatch);
        }
        Ok(())
    }

    async fn access_token(&self) -> Result<String, ArtifactTransferError> {
        self.access_provider
            .prepare()
            .await
            .map(|access| access.access_token)
            .map_err(|_| ArtifactTransferError::Transport)
    }

    async fn range_request(
        &self,
        endpoint: Url,
        offset: u64,
    ) -> Result<reqwest::Response, ArtifactTransferError> {
        let send = |token: String| {
            self.client
                .get(endpoint.clone())
                .bearer_auth(token)
                .header(header::RANGE, format!("bytes={offset}-"))
                .send()
        };
        let mut response = send(self.access_token().await?)
            .await
            .map_err(|_| ArtifactTransferError::Transport)?;
        if response.status() == StatusCode::UNAUTHORIZED {
            response = send(self.access_token().await?)
                .await
                .map_err(|_| ArtifactTransferError::Transport)?;
        }
        Ok(response)
    }

    async fn send_authenticated(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, ArtifactTransferError> {
        for attempt in 0..3 {
            let response = request
                .try_clone()
                .ok_or(ArtifactTransferError::InvalidResponse)?
                .bearer_auth(self.access_token().await?)
                .send()
                .await;
            match response {
                Ok(response) if response.status() != StatusCode::UNAUTHORIZED => {
                    return Ok(response);
                }
                Ok(_) | Err(_) if attempt < 2 => {
                    tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt + 1))).await;
                }
                Ok(_) => return Err(ArtifactTransferError::Rejected),
                Err(_) => return Err(ArtifactTransferError::Transport),
            }
        }
        Err(ArtifactTransferError::Transport)
    }

    fn endpoint(&self, lease_id: &str, suffix: &str) -> Result<Url, ArtifactTransferError> {
        if lease_id.is_empty()
            || !lease_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(ArtifactTransferError::InvalidPath);
        }
        self.api_base
            .join(&format!("api/v1/agent/artifact-leases/{lease_id}/{suffix}"))
            .map_err(|_| ArtifactTransferError::InvalidPath)
    }
}

fn append_suffix(path: &Path, suffix: &str) -> Result<PathBuf, ArtifactTransferError> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ArtifactTransferError::InvalidPath)?;
    Ok(path.with_file_name(format!("{name}{suffix}")))
}

pub fn extract_archive(
    archive_path: &Path,
    target_dir: &Path,
) -> Result<(), ArtifactTransferError> {
    if target_dir.exists() {
        fs::remove_dir_all(target_dir)?;
    }
    fs::create_dir_all(target_dir)?;
    let mut archive = tar::Archive::new(fs::File::open(archive_path)?);
    for item in archive.entries()? {
        let mut item = item?;
        let path = item.path()?.into_owned();
        if path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
            || !item.header().entry_type().is_file()
        {
            return Err(ArtifactTransferError::InvalidPath);
        }
        item.unpack_in(target_dir)?;
    }
    Ok(())
}

pub fn extract_archive_atomic(
    archive_path: &Path,
    target_dir: &Path,
) -> Result<(), ArtifactTransferError> {
    extract_archive_atomic_verified(archive_path, target_dir, |_| Ok(()))
}

pub fn extract_archive_atomic_verified(
    archive_path: &Path,
    target_dir: &Path,
    verify: impl FnOnce(&Path) -> Result<(), ArtifactTransferError>,
) -> Result<(), ArtifactTransferError> {
    let parent = target_dir
        .parent()
        .ok_or(ArtifactTransferError::InvalidPath)?;
    fs::create_dir_all(parent)?;
    let name = target_dir
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ArtifactTransferError::InvalidPath)?;
    let temporary = parent.join(format!(".{name}.extracting"));
    if temporary.exists() {
        fs::remove_dir_all(&temporary)?;
    }
    extract_archive(archive_path, &temporary)?;
    if let Err(error) = verify(&temporary) {
        let _ = fs::remove_dir_all(&temporary);
        return Err(error);
    }
    if target_dir.exists() {
        let previous = parent.join(format!(".{name}.previous"));
        if previous.exists() {
            fs::remove_dir_all(&previous)?;
        }
        fs::rename(target_dir, &previous)?;
        if let Err(error) = fs::rename(&temporary, target_dir) {
            let _ = fs::rename(&previous, target_dir);
            return Err(ArtifactTransferError::Io(error));
        }
        fs::remove_dir_all(previous)?;
    } else {
        fs::rename(temporary, target_dir)?;
    }
    Ok(())
}

fn create_deterministic_tar(root: &Path, archive_path: &Path) -> Result<(), ArtifactTransferError> {
    let mut files = Vec::new();
    collect_regular_files(root, root, &mut files)?;
    files.sort();
    let output = fs::File::create(archive_path)?;
    let mut builder = tar::Builder::new(output);
    for relative in files {
        let path = root.join(&relative);
        let mut source = fs::File::open(&path)?;
        let metadata = source.metadata()?;
        let mut header = tar::Header::new_gnu();
        header.set_size(metadata.len());
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        builder.append_data(&mut header, relative, &mut source)?;
    }
    builder.finish()?;
    builder.into_inner()?.sync_all()?;
    Ok(())
}

fn collect_regular_files(
    root: &Path,
    current: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), ArtifactTransferError> {
    for item in fs::read_dir(current)? {
        let item = item?;
        let path = item.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(ArtifactTransferError::InvalidPath);
        }
        if metadata.is_dir() {
            collect_regular_files(root, &path, output)?;
        } else if metadata.is_file() {
            output.push(
                path.strip_prefix(root)
                    .map_err(|_| ArtifactTransferError::InvalidPath)?
                    .to_owned(),
            );
        } else {
            return Err(ArtifactTransferError::InvalidPath);
        }
    }
    Ok(())
}

async fn decode_status(response: reqwest::Response) -> Result<UploadStatus, ArtifactTransferError> {
    if !response.status().is_success() {
        return Err(ArtifactTransferError::Rejected);
    }
    response
        .json()
        .await
        .map_err(|_| ArtifactTransferError::InvalidResponse)
}

async fn require_success(response: reqwest::Response) -> Result<(), ArtifactTransferError> {
    response
        .status()
        .is_success()
        .then_some(())
        .ok_or(ArtifactTransferError::Rejected)
}

fn file_digest(path: &Path) -> Result<String, ArtifactTransferError> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    io::copy(&mut file, &mut digest)?;
    Ok(format!("{:x}", digest.finalize()))
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_upload_status(
    status: &UploadStatus,
    expected_size: u64,
    minimum_offset: u64,
) -> Result<(), ArtifactTransferError> {
    if status.upload_size != expected_size
        || status.offset > status.upload_size
        || status.offset < minimum_offset
    {
        return Err(ArtifactTransferError::InvalidResponse);
    }
    Ok(())
}

fn validate_resume_upload_status(
    status: &UploadStatus,
    expected_size: u64,
    previous_offset: u64,
    sent_end: u64,
) -> Result<(), ArtifactTransferError> {
    validate_upload_status(status, expected_size, previous_offset)?;
    if status.offset > sent_end + 1 {
        return Err(ArtifactTransferError::InvalidResponse);
    }
    Ok(())
}

fn remove_if_exists(path: &Path) -> Result<(), ArtifactTransferError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ArtifactTransferError::Io(error)),
    }
}

fn parse_content_range(
    headers: &reqwest::header::HeaderMap,
    expected_start: u64,
) -> Result<(u64, u64), ArtifactTransferError> {
    let value = headers
        .get(header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .ok_or(ArtifactTransferError::InvalidResponse)?;
    let value = value
        .strip_prefix("bytes ")
        .ok_or(ArtifactTransferError::InvalidResponse)?;
    let (range, total) = value
        .split_once('/')
        .ok_or(ArtifactTransferError::InvalidResponse)?;
    let (start, end) = range
        .split_once('-')
        .ok_or(ArtifactTransferError::InvalidResponse)?;
    let start = start
        .parse::<u64>()
        .map_err(|_| ArtifactTransferError::InvalidResponse)?;
    let end = end
        .parse::<u64>()
        .map_err(|_| ArtifactTransferError::InvalidResponse)?;
    let total = total
        .parse::<u64>()
        .map_err(|_| ArtifactTransferError::InvalidResponse)?;
    if start != expected_start || end < start || end >= total {
        return Err(ArtifactTransferError::InvalidResponse);
    }
    Ok((end, total))
}

#[cfg(test)]
mod tests {
    use super::{ArtifactTransferError, UploadStatus, validate_resume_upload_status};

    #[test]
    fn transport_error_get_status_cannot_jump_past_the_sent_chunk() {
        let status = UploadStatus {
            offset: 21,
            upload_size: 100,
        };
        assert!(matches!(
            validate_resume_upload_status(&status, 100, 10, 19),
            Err(ArtifactTransferError::InvalidResponse)
        ));
        assert!(
            validate_resume_upload_status(
                &UploadStatus {
                    offset: 20,
                    upload_size: 100,
                },
                100,
                10,
                19,
            )
            .is_ok()
        );
    }
}
