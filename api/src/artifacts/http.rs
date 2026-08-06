use std::path::Path;

use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::{Extension, Path as AxumPath, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
    routing::post,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use utoipa::ToSchema;

use crate::{
    AppState, RequestId,
    agents::auth::authenticate_access,
    error::{ApiError, ApiResult},
};

use super::ArtifactStore;

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct InitiateUploadRequest {
    upload_size: u64,
    archive_digest: String,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct UploadStatusResponse {
    lease_id: String,
    artifact_id: String,
    offset: u64,
    upload_size: u64,
    status: String,
}

#[derive(FromRow)]
struct UploadLeaseRow {
    artifact_id: String,
    lease_status: String,
    lease_expires_at: String,
    artifact_status: String,
    upload_offset: i64,
    upload_size: Option<i64>,
    archive_digest: Option<String>,
    manifest_json: String,
    lease_manifest_digest: String,
    artifact_manifest_digest: String,
    total_size: i64,
    file_count: i64,
}

#[derive(FromRow)]
struct DownloadLeaseRow {
    artifact_id: String,
    lease_status: String,
    lease_expires_at: String,
    manifest_digest: String,
    artifact_manifest_digest: String,
    storage_key: String,
    upload_size: i64,
    target_status: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/agent/artifact-leases/{id}/upload",
            post(initiate_upload).get(upload_status).put(upload_chunk),
        )
        .route(
            "/agent/artifact-leases/{id}/upload/finalize",
            post(finalize_upload),
        )
        .route(
            "/agent/artifact-leases/{id}/download",
            axum::routing::get(download_artifact),
        )
}

#[utoipa::path(operation_id = "artifact_download", get, path = "/api/v1/agent/artifact-leases/{id}/download", params(("id" = String, Path), ("Authorization" = String, Header), ("Range" = Option<String>, Header)), responses((status = 200, body = Vec<u8>, content_type = "application/octet-stream"), (status = 206, body = Vec<u8>, content_type = "application/octet-stream"), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn download_artifact(
    State(state): State<AppState>,
    AxumPath(lease_id): AxumPath<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let identity = authenticate_access(state.pool(), &headers, request_id.as_str()).await?;
    let store = require_store(&state, request_id.as_str())?;
    let lease: DownloadLeaseRow = sqlx::query_as(
        "SELECT lease.artifact_id,lease.status AS lease_status,lease.expires_at AS lease_expires_at,lease.manifest_digest,artifact.manifest_digest AS artifact_manifest_digest,artifact.storage_key,artifact.upload_size,target.status AS target_status FROM artifact_leases lease JOIN deployment_artifacts artifact ON artifact.id=lease.artifact_id JOIN deployment_target_runs target ON target.id=lease.target_run_id AND target.agent_id=lease.agent_id AND target.artifact_id=lease.artifact_id WHERE lease.id=? AND lease.agent_id=? AND lease.purpose='artifact_download' AND artifact.status='verified'",
    )
    .bind(&lease_id)
    .bind(&identity.agent_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?
    .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    if lease.lease_status != "active"
        || lease.lease_expires_at <= Utc::now().to_rfc3339()
        || !matches!(
            lease.target_status.as_str(),
            "pending" | "downloading" | "running"
        )
        || lease.manifest_digest != lease.artifact_manifest_digest
    {
        return Err(ApiError::conflict(
            "artifact_download_lease_inactive",
            "制品下载 lease 已失效",
            request_id.as_str(),
        ));
    }
    let total =
        u64::try_from(lease.upload_size).map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (start, end, partial) =
        parse_range(headers.get(header::RANGE), total, request_id.as_str())?;
    let length = end - start + 1;
    let path = store
        .object_path(&lease.storage_key)
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let mut file = tokio::fs::File::open(path).await.map_err(|_| {
        ApiError::conflict(
            "artifact_object_missing",
            "制品对象不存在",
            request_id.as_str(),
        )
    })?;
    if file
        .metadata()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .len()
        != total
    {
        return Err(ApiError::conflict(
            "artifact_object_size_mismatch",
            "制品对象大小不一致",
            request_id.as_str(),
        ));
    }
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let pin = store.pin_download(&lease.storage_key);
    let stream = async_stream::stream! {
        let _pin = pin;
        let mut reader = file.take(length);
        let mut remaining = length;
        while remaining > 0 {
            let mut buffer = vec![0_u8; remaining.min(64 * 1024) as usize];
            let read = match reader.read(&mut buffer).await {
                Ok(read) => read,
                Err(error) => {
                    yield Err::<Vec<u8>, std::io::Error>(error);
                    return;
                }
            };
            if read == 0 {
                yield Err::<Vec<u8>, std::io::Error>(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "artifact changed during download",
                ));
                return;
            }
            buffer.truncate(read);
            remaining -= read as u64;
            yield Ok::<Vec<u8>, std::io::Error>(buffer);
        }
    };
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = if partial {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    response
        .headers_mut()
        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string())
            .map_err(|_| ApiError::internal(request_id.as_str()))?,
    );
    response.headers_mut().insert(
        "x-deploy-go-artifact-id",
        HeaderValue::from_str(&lease.artifact_id)
            .map_err(|_| ApiError::internal(request_id.as_str()))?,
    );
    if partial {
        response.headers_mut().insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{total}"))
                .map_err(|_| ApiError::internal(request_id.as_str()))?,
        );
    }
    Ok(response)
}

#[utoipa::path(operation_id = "artifact_upload_initiate", post, path = "/api/v1/agent/artifact-leases/{id}/upload", params(("id" = String, Path), ("Authorization" = String, Header)), request_body = InitiateUploadRequest, responses((status = 200, body = UploadStatusResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn initiate_upload(
    State(state): State<AppState>,
    AxumPath(lease_id): AxumPath<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    crate::http::ApiJson(payload): crate::http::ApiJson<InitiateUploadRequest>,
) -> ApiResult<Json<UploadStatusResponse>> {
    let identity = authenticate_access(state.pool(), &headers, request_id.as_str()).await?;
    let store = require_store(&state, request_id.as_str())?;
    let archive_limit = store
        .config()
        .max_total_bytes
        .saturating_add(u64::from(store.config().max_files).saturating_mul(1024))
        .saturating_add(1024);
    if payload.upload_size == 0 || payload.upload_size > archive_limit {
        return Err(ApiError::validation(
            "制品上传大小超出限制",
            request_id.as_str(),
        ));
    }
    validate_digest(&payload.archive_digest, request_id.as_str())?;
    let artifact_id: String = sqlx::query_scalar(
        "SELECT artifact_id FROM artifact_leases WHERE id=? AND agent_id=? AND purpose='artifact_upload'",
    )
    .bind(&lease_id)
    .bind(&identity.agent_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?
    .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let upload_lock = store.upload_lock(&artifact_id);
    let _upload_guard = upload_lock.lock().await;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let lease = load_upload_lease(
        &mut *transaction,
        &lease_id,
        &identity.agent_id,
        request_id.as_str(),
    )
    .await?;
    validate_active_upload(&lease, request_id.as_str())?;
    if let Some(existing_size) = lease.upload_size {
        if u64::try_from(existing_size).ok() != Some(payload.upload_size)
            || lease.archive_digest.as_deref() != Some(payload.archive_digest.as_str())
        {
            return Err(ApiError::conflict(
                "artifact_upload_session_conflict",
                "上传会话参数不一致",
                request_id.as_str(),
            ));
        }
    } else {
        sqlx::query("UPDATE deployment_artifacts SET upload_size=?,archive_digest=?,updated_at=?,version=version+1 WHERE id=? AND upload_size IS NULL")
            .bind(i64::try_from(payload.upload_size).map_err(|_| ApiError::validation("制品上传大小超出限制", request_id.as_str()))?)
            .bind(&payload.archive_digest).bind(Utc::now().to_rfc3339()).bind(&lease.artifact_id)
            .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    }
    let upload_path = store
        .upload_path(&lease.artifact_id)
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let file = tokio::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(upload_path)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let db_offset =
        u64::try_from(lease.upload_offset).map_err(|_| ApiError::internal(request_id.as_str()))?;
    let file_len = file
        .metadata()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .len();
    if file_len < db_offset {
        return Err(ApiError::conflict(
            "artifact_upload_corrupt",
            "上传会话文件短于已确认 offset",
            request_id.as_str(),
        ));
    }
    if file_len > db_offset {
        file.set_len(db_offset)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    }
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(UploadStatusResponse {
        lease_id,
        artifact_id: lease.artifact_id,
        offset: db_offset,
        upload_size: payload.upload_size,
        status: lease.artifact_status,
    }))
}

#[utoipa::path(operation_id = "artifact_upload_status", get, path = "/api/v1/agent/artifact-leases/{id}/upload", params(("id" = String, Path), ("Authorization" = String, Header)), responses((status = 200, body = UploadStatusResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn upload_status(
    State(state): State<AppState>,
    AxumPath(lease_id): AxumPath<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> ApiResult<Json<UploadStatusResponse>> {
    let identity = authenticate_access(state.pool(), &headers, request_id.as_str()).await?;
    let lease = load_upload_lease(
        state.pool(),
        &lease_id,
        &identity.agent_id,
        request_id.as_str(),
    )
    .await?;
    validate_active_upload(&lease, request_id.as_str())?;
    Ok(Json(upload_response(lease_id, lease, request_id.as_str())?))
}

#[utoipa::path(operation_id = "artifact_upload_chunk", put, path = "/api/v1/agent/artifact-leases/{id}/upload", params(("id" = String, Path), ("Authorization" = String, Header), ("Content-Range" = String, Header)), request_body(content = Vec<u8>, content_type = "application/octet-stream"), responses((status = 200, body = UploadStatusResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn upload_chunk(
    State(state): State<AppState>,
    AxumPath(lease_id): AxumPath<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    body: Body,
) -> ApiResult<Json<UploadStatusResponse>> {
    let identity = authenticate_access(state.pool(), &headers, request_id.as_str()).await?;
    let store = require_store(&state, request_id.as_str())?;
    let (start, end, total) = parse_content_range(&headers, request_id.as_str())?;
    let length = end
        .checked_sub(start)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| ApiError::validation("Content-Range 格式不正确", request_id.as_str()))?;
    if length > store.config().max_chunk_bytes {
        return Err(ApiError::validation(
            "上传分块超出限制",
            request_id.as_str(),
        ));
    }
    let bytes = to_bytes(body, usize::try_from(length).unwrap_or(usize::MAX) + 1)
        .await
        .map_err(|_| ApiError::validation("上传分块长度不正确", request_id.as_str()))?;
    if bytes.len() as u64 != length {
        return Err(ApiError::validation(
            "上传分块长度不正确",
            request_id.as_str(),
        ));
    }
    let artifact_id: String = sqlx::query_scalar(
        "SELECT artifact_id FROM artifact_leases WHERE id=? AND agent_id=? AND purpose='artifact_upload'",
    )
    .bind(&lease_id)
    .bind(&identity.agent_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?
    .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let upload_lock = store.upload_lock(&artifact_id);
    let _upload_guard = upload_lock.lock().await;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let lease = load_upload_lease(
        &mut *transaction,
        &lease_id,
        &identity.agent_id,
        request_id.as_str(),
    )
    .await?;
    validate_active_upload(&lease, request_id.as_str())?;
    let upload_size = lease
        .upload_size
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| {
            ApiError::conflict(
                "artifact_upload_not_initiated",
                "上传会话尚未初始化",
                request_id.as_str(),
            )
        })?;
    if total != upload_size || end >= total {
        return Err(ApiError::validation(
            "Content-Range 总长度不一致",
            request_id.as_str(),
        ));
    }
    let offset =
        u64::try_from(lease.upload_offset).map_err(|_| ApiError::internal(request_id.as_str()))?;
    let upload_path = store
        .upload_path(&lease.artifact_id)
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let mut file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(upload_path)
        .await
        .map_err(|_| {
            ApiError::conflict(
                "artifact_upload_not_initiated",
                "上传会话文件不存在",
                request_id.as_str(),
            )
        })?;
    let file_len = file
        .metadata()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?
        .len();
    if file_len < offset {
        return Err(ApiError::conflict(
            "artifact_upload_corrupt",
            "上传会话文件短于已确认 offset",
            request_id.as_str(),
        ));
    }
    if file_len > offset {
        file.set_len(offset)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    }
    if start < offset {
        if end >= offset {
            return Err(ApiError::conflict(
                "artifact_upload_offset_conflict",
                "上传分块与已确认 offset 重叠",
                request_id.as_str(),
            ));
        }
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        let mut existing = vec![0_u8; bytes.len()];
        file.read_exact(&mut existing)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        if existing.as_slice() != bytes.as_ref() {
            return Err(ApiError::conflict(
                "artifact_chunk_mismatch",
                "重放分块内容不一致",
                request_id.as_str(),
            ));
        }
        return Ok(Json(upload_response(lease_id, lease, request_id.as_str())?));
    }
    if start != offset {
        return Err(ApiError::conflict(
            "artifact_upload_offset_conflict",
            "上传分块必须从当前 offset 开始",
            request_id.as_str(),
        ));
    }
    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    file.write_all(&bytes)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    file.sync_data()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let next_offset = end + 1;
    let updated = sqlx::query("UPDATE deployment_artifacts SET upload_offset=?,updated_at=?,version=version+1 WHERE id=? AND upload_offset=? AND status='uploading'")
        .bind(i64::try_from(next_offset).map_err(|_| ApiError::internal(request_id.as_str()))?)
        .bind(Utc::now().to_rfc3339()).bind(&lease.artifact_id).bind(lease.upload_offset)
        .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    if updated.rows_affected() != 1 {
        return Err(ApiError::conflict(
            "artifact_upload_offset_conflict",
            "上传 offset 已被其他请求推进",
            request_id.as_str(),
        ));
    }
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(UploadStatusResponse {
        lease_id,
        artifact_id: lease.artifact_id,
        offset: next_offset,
        upload_size,
        status: lease.artifact_status,
    }))
}

#[utoipa::path(operation_id = "artifact_upload_finalize", post, path = "/api/v1/agent/artifact-leases/{id}/upload/finalize", params(("id" = String, Path), ("Authorization" = String, Header)), responses((status = 200, body = UploadStatusResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn finalize_upload(
    State(state): State<AppState>,
    AxumPath(lease_id): AxumPath<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> ApiResult<Json<UploadStatusResponse>> {
    let identity = authenticate_access(state.pool(), &headers, request_id.as_str()).await?;
    let store = require_store(&state, request_id.as_str())?;
    let artifact_id: String = sqlx::query_scalar(
        "SELECT artifact_id FROM artifact_leases WHERE id=? AND agent_id=? AND purpose='artifact_upload'",
    )
    .bind(&lease_id)
    .bind(&identity.agent_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?
    .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let upload_lock = store.upload_lock(&artifact_id);
    let _upload_guard = upload_lock.lock().await;
    let lease = load_upload_lease(
        state.pool(),
        &lease_id,
        &identity.agent_id,
        request_id.as_str(),
    )
    .await?;
    validate_active_upload(&lease, request_id.as_str())?;
    let upload_size = lease
        .upload_size
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| {
            ApiError::conflict(
                "artifact_upload_not_initiated",
                "上传会话尚未初始化",
                request_id.as_str(),
            )
        })?;
    if u64::try_from(lease.upload_offset).ok() != Some(upload_size) {
        return Err(ApiError::conflict(
            "artifact_upload_incomplete",
            "制品上传尚未完成",
            request_id.as_str(),
        ));
    }
    let digest = lease
        .archive_digest
        .clone()
        .ok_or_else(|| ApiError::internal(request_id.as_str()))?;
    let object_lock = store.upload_lock(&format!("object:{digest}"));
    let _object_guard = object_lock.lock().await;
    let upload_path = store
        .upload_path(&lease.artifact_id)
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let manifest = lease.manifest_json.clone();
    let config = store.config().clone();
    let upload_path_for_verify = upload_path.clone();
    let digest_for_verify = digest.clone();
    let verification_permit = store.verification_permit().await;
    let verification = tokio::task::spawn_blocking(move || {
        super::verify::verify_archive(
            &upload_path_for_verify,
            &manifest,
            &digest_for_verify,
            &config,
            lease.total_size,
            lease.file_count,
        )
    })
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    drop(verification_permit);
    if let Err(code) = verification {
        fail_upload(state.pool(), &lease_id, &lease.artifact_id)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        return Err(ApiError::conflict(
            code,
            "制品归档校验失败",
            request_id.as_str(),
        ));
    }
    let object_path = store
        .object_path(&digest)
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let object_pin = object_path.exists().then(|| store.pin_download(&digest));
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let current = load_upload_lease(
        &mut *transaction,
        &lease_id,
        &identity.agent_id,
        request_id.as_str(),
    )
    .await?;
    validate_active_upload(&current, request_id.as_str())?;
    let reused_object = object_path.exists();
    if reused_object {
        verify_existing_object(&object_path, upload_size, &digest)
            .await
            .map_err(|code| {
                ApiError::conflict(code, "已有制品对象校验失败", request_id.as_str())
            })?;
    } else {
        tokio::fs::rename(&upload_path, &object_path)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    }
    let now = Utc::now().to_rfc3339();
    let consumed = sqlx::query("UPDATE artifact_leases SET status='consumed',consumed_at=? WHERE id=? AND agent_id=? AND status='active' AND expires_at>?")
        .bind(&now).bind(&lease_id).bind(&identity.agent_id).bind(&now)
        .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    if consumed.rows_affected() != 1 {
        return Err(ApiError::conflict(
            "artifact_lease_consumed",
            "制品上传 lease 已失效",
            request_id.as_str(),
        ));
    }
    sqlx::query("UPDATE deployment_artifacts SET status='verified',storage_key=?,verified_at=?,updated_at=?,version=version+1 WHERE id=? AND status='uploading' AND upload_offset=upload_size")
        .bind(&digest).bind(&now).bind(&now).bind(&lease.artifact_id)
        .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    if transaction.commit().await.is_err() {
        if !reused_object {
            let _ = tokio::fs::rename(&object_path, &upload_path).await;
        }
        return Err(ApiError::internal(request_id.as_str()));
    }
    if reused_object
        && let Err(error) = tokio::fs::remove_file(&upload_path).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(artifact_id = %lease.artifact_id, "已复用制品完成，但隔离文件延迟清理");
    }
    drop(object_pin);
    Ok(Json(UploadStatusResponse {
        lease_id,
        artifact_id: lease.artifact_id,
        offset: upload_size,
        upload_size,
        status: "verified".to_owned(),
    }))
}

async fn fail_upload(
    pool: &sqlx::SqlitePool,
    lease_id: &str,
    artifact_id: &str,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE artifact_leases SET status='failed' WHERE id=? AND artifact_id=? AND status='active'")
        .bind(lease_id)
        .bind(artifact_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("UPDATE deployment_artifacts SET status='failed',expires_at=?,updated_at=?,version=version+1 WHERE id=? AND status='uploading'")
        .bind(&now)
        .bind(&now)
        .bind(artifact_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await
}

async fn verify_existing_object(
    path: &Path,
    expected_size: u64,
    expected_digest: &str,
) -> Result<(), &'static str> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|_| "artifact_object_missing")?;
    if file
        .metadata()
        .await
        .map_err(|_| "artifact_object_read_failed")?
        .len()
        != expected_size
    {
        return Err("artifact_object_size_mismatch");
    }
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .await
            .map_err(|_| "artifact_object_read_failed")?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    if format!("{:x}", hasher.finalize()) != expected_digest {
        return Err("artifact_object_digest_mismatch");
    }
    Ok(())
}

async fn load_upload_lease<'e, E>(
    executor: E,
    lease_id: &str,
    agent_id: &str,
    request_id: &str,
) -> ApiResult<UploadLeaseRow>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query_as("SELECT lease.artifact_id,lease.status AS lease_status,lease.expires_at AS lease_expires_at,artifact.status AS artifact_status,artifact.upload_offset,artifact.upload_size,artifact.archive_digest,artifact.manifest_json,lease.manifest_digest AS lease_manifest_digest,artifact.manifest_digest AS artifact_manifest_digest,artifact.total_size,artifact.file_count FROM artifact_leases lease JOIN deployment_artifacts artifact ON artifact.id=lease.artifact_id WHERE lease.id=? AND lease.agent_id=? AND lease.purpose='artifact_upload'")
        .bind(lease_id).bind(agent_id).fetch_optional(executor).await
        .map_err(|_| ApiError::internal(request_id))?.ok_or_else(|| ApiError::not_found(request_id))
}

fn validate_active_upload(lease: &UploadLeaseRow, request_id: &str) -> ApiResult<()> {
    if lease.lease_status != "active"
        || lease.artifact_status != "uploading"
        || lease.lease_expires_at <= Utc::now().to_rfc3339()
    {
        return Err(ApiError::conflict(
            "artifact_lease_inactive",
            "制品上传 lease 已失效",
            request_id,
        ));
    }
    let actual_manifest_digest = format!("{:x}", Sha256::digest(lease.manifest_json.as_bytes()));
    if lease.lease_manifest_digest != actual_manifest_digest
        || lease.artifact_manifest_digest != actual_manifest_digest
    {
        return Err(ApiError::conflict(
            "artifact_manifest_mismatch",
            "制品 manifest 摘要不一致",
            request_id,
        ));
    }
    Ok(())
}

fn upload_response(
    lease_id: String,
    lease: UploadLeaseRow,
    request_id: &str,
) -> ApiResult<UploadStatusResponse> {
    Ok(UploadStatusResponse {
        lease_id,
        artifact_id: lease.artifact_id,
        offset: u64::try_from(lease.upload_offset).map_err(|_| ApiError::internal(request_id))?,
        upload_size: lease
            .upload_size
            .and_then(|value| u64::try_from(value).ok())
            .ok_or_else(|| {
                ApiError::conflict(
                    "artifact_upload_not_initiated",
                    "上传会话尚未初始化",
                    request_id,
                )
            })?,
        status: lease.artifact_status,
    })
}

fn require_store<'a>(state: &'a AppState, request_id: &str) -> ApiResult<&'a ArtifactStore> {
    state
        .artifact_store()
        .ok_or_else(|| ApiError::service_not_ready(request_id))
}

fn validate_digest(value: &str, request_id: &str) -> ApiResult<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(ApiError::validation("制品归档摘要格式不正确", request_id))
    }
}

fn parse_content_range(headers: &HeaderMap, request_id: &str) -> ApiResult<(u64, u64, u64)> {
    let value = headers
        .get("content-range")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("bytes "))
        .ok_or_else(|| ApiError::validation("缺少合法 Content-Range", request_id))?;
    let (range, total) = value
        .split_once('/')
        .ok_or_else(|| ApiError::validation("Content-Range 格式不正确", request_id))?;
    let (start, end) = range
        .split_once('-')
        .ok_or_else(|| ApiError::validation("Content-Range 格式不正确", request_id))?;
    let start = start
        .parse()
        .map_err(|_| ApiError::validation("Content-Range 格式不正确", request_id))?;
    let end = end
        .parse()
        .map_err(|_| ApiError::validation("Content-Range 格式不正确", request_id))?;
    let total = total
        .parse()
        .map_err(|_| ApiError::validation("Content-Range 格式不正确", request_id))?;
    Ok((start, end, total))
}

fn parse_range(
    header: Option<&HeaderValue>,
    total: u64,
    request_id: &str,
) -> ApiResult<(u64, u64, bool)> {
    if total == 0 {
        return Err(ApiError::conflict(
            "artifact_object_empty",
            "制品归档不能为空",
            request_id,
        ));
    }
    let Some(value) = header else {
        return Ok((0, total - 1, false));
    };
    let value = value
        .to_str()
        .ok()
        .and_then(|value| value.strip_prefix("bytes="))
        .ok_or_else(|| ApiError::validation("Range 格式不正确", request_id))?;
    if value.contains(',') {
        return Err(ApiError::validation("不支持多段 Range", request_id));
    }
    let (start, end) = value
        .split_once('-')
        .ok_or_else(|| ApiError::validation("Range 格式不正确", request_id))?;
    if start.is_empty() {
        let suffix = end
            .parse::<u64>()
            .map_err(|_| ApiError::validation("Range 格式不正确", request_id))?;
        if suffix == 0 {
            return Err(ApiError::validation("Range 格式不正确", request_id));
        }
        return Ok((total.saturating_sub(suffix), total - 1, true));
    }
    let start = start
        .parse::<u64>()
        .map_err(|_| ApiError::validation("Range 格式不正确", request_id))?;
    let end = if end.is_empty() {
        total - 1
    } else {
        end.parse::<u64>()
            .map_err(|_| ApiError::validation("Range 格式不正确", request_id))?
    };
    if start > end || end >= total {
        return Err(ApiError::validation("Range 超出制品范围", request_id));
    }
    Ok((start, end, true))
}

#[cfg(test)]
mod tests {
    use super::parse_range;
    use axum::http::HeaderValue;

    #[test]
    fn suffix_range_returns_the_requested_tail() {
        let header = HeaderValue::from_static("bytes=-10");
        assert_eq!(
            parse_range(Some(&header), 100, "test").unwrap(),
            (90, 99, true)
        );
        let oversized = HeaderValue::from_static("bytes=-200");
        assert_eq!(
            parse_range(Some(&oversized), 100, "test").unwrap(),
            (0, 99, true)
        );
        let empty = HeaderValue::from_static("bytes=-0");
        assert!(parse_range(Some(&empty), 100, "test").is_err());
    }
}
