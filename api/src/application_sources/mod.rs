use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
};
use chrono::{Duration, Utc};
use deploy_go_agent_protocol::{GitRefsQueryTask, TaskPayload};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId,
    agents::dispatcher,
    audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    grants,
};

pub(crate) const REFS_QUERY_TIMEOUT_SECONDS: u32 = 30;
const REFS_DISCOVERY_WAIT_SECONDS: i64 = 45;
const MAX_REFS: usize = 1024;

#[derive(Clone, Serialize, ToSchema)]
pub struct ApplicationSourceResponse {
    pub id: String,
    pub application_id: String,
    pub repository_url: String,
    pub git_credential_id: Option<String>,
    pub git_credential_name: Option<String>,
    pub build_agent_id: String,
    pub build_agent_name: Option<String>,
    pub source_policy: String,
    pub deployment_branch: Option<String>,
    pub branch_verified_at: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct GitRefResponse {
    pub name: String,
    #[serde(rename = "ref")]
    pub reference: String,
    pub sha: String,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct GitRefDiscoveryResponse {
    pub id: String,
    pub application_source_id: String,
    pub source_version: i64,
    pub task_id: String,
    pub status: String,
    pub refs: Vec<GitRefResponse>,
    pub error_code: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveSourceRequest {
    repository_url: String,
    git_credential_id: Option<String>,
    build_agent_id: String,
    #[serde(default = "default_source_policy")]
    source_policy: String,
    version: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetBranchRequest {
    branch: String,
    version: i64,
}

#[derive(sqlx::FromRow)]
struct SourceViewRow {
    id: String,
    application_id: String,
    repository_url: String,
    git_credential_id: Option<String>,
    git_credential_name: Option<String>,
    build_agent_id: String,
    build_agent_name: Option<String>,
    source_policy: String,
    deployment_branch: Option<String>,
    branch_verified_at: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
    version: i64,
}

#[derive(sqlx::FromRow)]
struct SourceRow {
    id: String,
    repository_url: String,
    git_credential_id: Option<String>,
    build_agent_id: String,
    source_version: i64,
    status: String,
    version: i64,
}

#[derive(sqlx::FromRow)]
struct DiscoveryRow {
    id: String,
    application_source_id: String,
    source_version: i64,
    task_id: String,
    status: String,
    refs_json: String,
    error_code: Option<String>,
    expires_at: Option<String>,
    created_at: String,
    finished_at: Option<String>,
}

#[derive(sqlx::FromRow)]
struct BuildAgentPolicy {
    node_status: String,
    protocol_version: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/applications/{application_id}/source", get(show).put(save))
        .route(
            "/applications/{application_id}/source/refreshes",
            post(refresh),
        )
        .route(
            "/applications/{application_id}/source/refreshes/{refs_query_id}",
            get(show_discovery),
        )
        .route(
            "/applications/{application_id}/source/branch",
            put(set_branch),
        )
}

#[utoipa::path(operation_id = "application_source_show", get, path = "/api/v1/applications/{application_id}/source", params(("application_id" = String, Path)), responses((status = 200, body = ApplicationSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<ApplicationSourceResponse>> {
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
    Ok(Json(
        find_source_view(state.pool(), &application_id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "application_source_save", put, path = "/api/v1/applications/{application_id}/source", params(("application_id" = String, Path)), request_body = SaveSourceRequest, responses((status = 200, body = ApplicationSourceResponse), (status = 201, body = ApplicationSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn save(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<SaveSourceRequest>,
) -> ApiResult<(StatusCode, Json<ApplicationSourceResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    ensure_application_active(state.pool(), &application_id, request_id.as_str()).await?;
    let repository_url = validate_repository_url(&payload.repository_url, request_id.as_str())?;
    if payload.source_policy != "branch" {
        return Err(ApiError::validation(
            "当前仅支持 branch 来源策略",
            request_id.as_str(),
        ));
    }
    if let Some(credential_id) = payload.git_credential_id.as_deref() {
        ensure_git_credential_active(state.pool(), credential_id, request_id.as_str()).await?;
    }
    let _ = build_agent_policy(state.pool(), &payload.build_agent_id, request_id.as_str()).await?;
    let existing = find_source_row(state.pool(), &application_id, request_id.as_str()).await?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let now = Utc::now().to_rfc3339();
    let created = existing.is_none();
    let source_id = existing
        .as_ref()
        .map(|source| source.id.clone())
        .unwrap_or_else(|| format!("source_{}", Ulid::new()));
    if let Some(existing) = existing {
        let version = payload.version.ok_or_else(|| {
            ApiError::validation("编辑应用来源必须提供 version", request_id.as_str())
        })?;
        let updated = sqlx::query(
            "UPDATE application_sources SET repository_url=?,git_credential_id=?,build_agent_id=?,source_policy='branch',deployment_branch=NULL,branch_verified_at=NULL,status='draft',source_version=source_version+1,updated_at=?,version=version+1 WHERE id=? AND version=?",
        )
        .bind(repository_url)
        .bind(payload.git_credential_id.as_deref())
        .bind(&payload.build_agent_id)
        .bind(&now)
        .bind(&existing.id)
        .bind(version)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
        if updated.rows_affected() != 1 {
            return Err(ApiError::conflict(
                "resource_version_conflict",
                "应用来源已经被其他请求修改",
                request_id.as_str(),
            ));
        }
    } else {
        sqlx::query("INSERT INTO application_sources (id,application_id,repository_url,git_credential_id,build_agent_id,source_policy,status,created_by) VALUES (?,?,?,?,?,'branch','draft',?)")
            .bind(&source_id)
            .bind(&application_id)
            .bind(repository_url)
            .bind(payload.git_credential_id.as_deref())
            .bind(&payload.build_agent_id)
            .bind(&actor.id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                if error.to_string().contains("UNIQUE constraint failed") {
                    ApiError::conflict(
                        "application_source_exists",
                        "应用来源已存在，请刷新后重试",
                        request_id.as_str(),
                    )
                } else {
                    ApiError::internal(request_id.as_str())
                }
            })?;
    }
    invalidate_active_previews(&mut transaction, &application_id)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        if created {
            "application_source.create"
        } else {
            "application_source.update"
        },
        "application_source",
        &source_id,
        request_id.as_str(),
        json!({
            "application_id": application_id,
            "repository_url": sanitized_repository_url(repository_url),
            "build_agent_id": payload.build_agent_id,
            "git_credential_id": payload.git_credential_id.as_deref(),
            "source_policy": "branch"
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(find_source_view(state.pool(), &application_id, request_id.as_str()).await?),
    ))
}

async fn invalidate_active_previews(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    application_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM deployment_previews WHERE application_id=? AND status='active'")
        .bind(application_id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

#[utoipa::path(operation_id = "application_source_refresh_refs", post, path = "/api/v1/applications/{application_id}/source/refreshes", params(("application_id" = String, Path)), responses((status = 202, body = GitRefDiscoveryResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn refresh(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<(StatusCode, Json<GitRefDiscoveryResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let discovery_id =
        enqueue_refs_discovery(&state, &actor.id, &application_id, request_id.as_str()).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(
            discovery_response(
                state.pool(),
                &application_id,
                &discovery_id,
                request_id.as_str(),
            )
            .await?,
        ),
    ))
}

pub(crate) async fn resolve_latest_refs(
    state: &AppState,
    actor_id: &str,
    application_id: &str,
    request_id: &str,
) -> ApiResult<GitRefDiscoveryResponse> {
    let discovery_id = enqueue_refs_discovery(state, actor_id, application_id, request_id).await?;
    wait_for_refs_discovery(state.pool(), application_id, &discovery_id, request_id).await
}

async fn enqueue_refs_discovery(
    state: &AppState,
    actor_id: &str,
    application_id: &str,
    request_id: &str,
) -> ApiResult<String> {
    ensure_application_active(state.pool(), application_id, request_id).await?;
    let source = find_source_row(state.pool(), application_id, request_id).await?;
    let Some(source) = source else {
        return Err(ApiError::conflict(
            "source_not_configured",
            "应用尚未配置 Git 来源",
            request_id,
        ));
    };
    if source.status == "archived" {
        return Err(ApiError::conflict(
            "source_archived",
            "应用来源已归档",
            request_id,
        ));
    }
    let _ = build_agent_policy(state.pool(), &source.build_agent_id, request_id).await?;
    if let Some(credential_id) = source.git_credential_id.as_deref() {
        ensure_git_credential_active(state.pool(), credential_id, request_id).await?;
    }
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT d.id FROM git_ref_discoveries d JOIN agent_tasks t ON t.id=d.task_id WHERE d.application_source_id=? AND d.source_version=? AND d.status IN ('queued','running') AND t.status IN ('queued','delivered','accepted','running','canceling') ORDER BY d.created_at,d.id LIMIT 1",
    )
    .bind(&source.id)
    .bind(source.source_version)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if let Some(discovery_id) = existing {
        return Ok(discovery_id);
    }

    let discovery_id = format!("refs_{}", Ulid::new());
    let task_id = format!("task_{}", Ulid::new());
    let lease_id = source
        .git_credential_id
        .as_deref()
        .map(|_| format!("lease_{}", Ulid::new()));
    let payload = TaskPayload::GitRefsQuery(GitRefsQueryTask {
        refs_query_id: discovery_id.clone(),
        repository_url: source.repository_url.clone(),
        git_credential_lease_id: lease_id.clone(),
        timeout_seconds: REFS_QUERY_TIMEOUT_SECONDS,
    });
    let payload_json =
        serde_json::to_string(&payload).map_err(|_| ApiError::internal(request_id))?;
    let payload_digest = format!("sha256:{:x}", Sha256::digest(payload_json.as_bytes()));
    let deadline_at =
        (Utc::now() + Duration::seconds(i64::from(REFS_QUERY_TIMEOUT_SECONDS) + 60)).to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES(?,?,'git_refs_query',?,?,?,'queued',?)")
        .bind(&task_id)
        .bind(&source.build_agent_id)
        .bind(format!("git-refs:{}:{}", source.id, discovery_id))
        .bind(&payload_digest)
        .bind(&payload_json)
        .bind(&deadline_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    if let (Some(lease_id), Some(credential_id)) =
        (lease_id.as_deref(), source.git_credential_id.as_deref())
    {
        sqlx::query("INSERT INTO git_secret_leases(id,task_id,git_credential_id,payload_digest,purpose,status,expires_at) VALUES(?,?,?,?,'git_credential','issued',?)")
            .bind(lease_id)
            .bind(&task_id)
            .bind(credential_id)
            .bind(&payload_digest)
            .bind((Utc::now() + Duration::seconds(60)).to_rfc3339())
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    }
    sqlx::query("INSERT INTO git_ref_discoveries(id,application_source_id,source_version,task_id,status) VALUES(?,?,?,?,'queued')")
        .bind(&discovery_id)
        .bind(&source.id)
        .bind(source.source_version)
        .bind(&task_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    audit::record(
        &mut transaction,
        Some(actor_id),
        "application_source.refresh",
        "application_source",
        &source.id,
        request_id,
        json!({
            "discovery_id": discovery_id,
            "source_version": source.source_version,
            "build_agent_id": source.build_agent_id
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    dispatcher::try_dispatch(state, &task_id).await?;
    Ok(discovery_id)
}

async fn wait_for_refs_discovery(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    discovery_id: &str,
    request_id: &str,
) -> ApiResult<GitRefDiscoveryResponse> {
    let deadline = Utc::now() + Duration::seconds(REFS_DISCOVERY_WAIT_SECONDS);
    loop {
        let response = discovery_response(pool, application_id, discovery_id, request_id).await?;
        match response.status.as_str() {
            "succeeded" => return Ok(response),
            "failed" | "expired" => {
                let code = response
                    .error_code
                    .clone()
                    .unwrap_or_else(|| "git_ref_discovery_failed".to_owned());
                return Err(ApiError::conflict(
                    &code,
                    "自动解析远程分支失败，请确认仓库、凭证与构建 Agent 后重试",
                    request_id,
                ));
            }
            _ if Utc::now() >= deadline => {
                return Err(ApiError::conflict(
                    "git_ref_discovery_timeout",
                    "自动解析远程分支超时，请确认构建 Agent 在线后重试",
                    request_id,
                ));
            }
            _ => {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }
}

#[utoipa::path(operation_id = "application_source_refresh_show", get, path = "/api/v1/applications/{application_id}/source/refreshes/{refs_query_id}", params(("application_id" = String, Path), ("refs_query_id" = String, Path)), responses((status = 200, body = GitRefDiscoveryResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show_discovery(
    State(state): State<AppState>,
    Path((application_id, refs_query_id)): Path<(String, String)>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<GitRefDiscoveryResponse>> {
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
    Ok(Json(
        discovery_response(
            state.pool(),
            &application_id,
            &refs_query_id,
            request_id.as_str(),
        )
        .await?,
    ))
}

#[utoipa::path(operation_id = "application_source_set_branch", put, path = "/api/v1/applications/{application_id}/source/branch", params(("application_id" = String, Path)), request_body = SetBranchRequest, responses((status = 200, body = ApplicationSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn set_branch(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<SetBranchRequest>,
) -> ApiResult<Json<ApplicationSourceResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let source = find_source_row(state.pool(), &application_id, request_id.as_str()).await?;
    let Some(source) = source else {
        return Err(ApiError::not_found(request_id.as_str()));
    };
    if source.status == "archived" {
        return Err(ApiError::conflict(
            "source_archived",
            "应用来源已归档",
            request_id.as_str(),
        ));
    }
    if source.version != payload.version {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "应用来源已经被其他请求修改",
            request_id.as_str(),
        ));
    }
    let branch = validate_branch_name(&payload.branch, request_id.as_str())?;
    let discovery: Option<DiscoveryRow> = sqlx::query_as::<_, DiscoveryRow>(
        "SELECT id,application_source_id,source_version,task_id,status,refs_json,error_code,expires_at,created_at,finished_at FROM git_ref_discoveries WHERE application_source_id=? AND source_version=? AND status='succeeded' ORDER BY created_at DESC,id DESC LIMIT 1",
    )
    .bind(&source.id)
    .bind(source.source_version)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let Some(discovery) = discovery else {
        return Err(ApiError::conflict(
            "git_ref_discovery_missing",
            "当前来源版本没有可用的分支发现结果",
            request_id.as_str(),
        ));
    };
    if !discovery_expired(&discovery) {
        let refs = parse_refs(&discovery.refs_json, request_id.as_str())?;
        if !refs.iter().any(|reference| reference.name == branch) {
            return Err(ApiError::conflict(
                "git_branch_not_found",
                "分支不在最近的发现结果中",
                request_id.as_str(),
            ));
        }
    } else {
        return Err(ApiError::conflict(
            "git_ref_discovery_expired",
            "分支发现结果已过期，请重新刷新",
            request_id.as_str(),
        ));
    }

    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let updated = sqlx::query("UPDATE application_sources SET deployment_branch=?,branch_verified_at=?,status='verified',updated_at=?,version=version+1 WHERE id=? AND version=?")
        .bind(branch)
        .bind(&now)
        .bind(&now)
        .bind(&source.id)
        .bind(payload.version)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if updated.rows_affected() != 1 {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "应用来源已经被其他请求修改",
            request_id.as_str(),
        ));
    }
    invalidate_active_previews(&mut transaction, &application_id)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application_source.branch.verify",
        "application_source",
        &source.id,
        request_id.as_str(),
        json!({
            "deployment_branch": branch,
            "source_version": source.source_version,
            "discovery_id": discovery.id
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(
        find_source_view(state.pool(), &application_id, request_id.as_str()).await?,
    ))
}

async fn find_source_view(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    request_id: &str,
) -> ApiResult<ApplicationSourceResponse> {
    let row = sqlx::query_as::<_, SourceViewRow>(
        "SELECT s.id,s.application_id,s.repository_url,s.git_credential_id,g.name AS git_credential_name,s.build_agent_id,n.name AS build_agent_name,s.source_policy,s.deployment_branch,s.branch_verified_at,s.status,s.created_at,s.updated_at,s.version FROM application_sources s LEFT JOIN git_credentials g ON g.id=s.git_credential_id JOIN agents a ON a.id=s.build_agent_id JOIN nodes n ON n.id=a.node_id WHERE s.application_id=?",
    )
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| ApiError::not_found(request_id))?;
    Ok(ApplicationSourceResponse {
        id: row.id,
        application_id: row.application_id,
        repository_url: sanitized_repository_url(&row.repository_url),
        git_credential_id: row.git_credential_id,
        git_credential_name: row.git_credential_name,
        build_agent_id: row.build_agent_id,
        build_agent_name: row.build_agent_name,
        source_policy: row.source_policy,
        deployment_branch: row.deployment_branch,
        branch_verified_at: row.branch_verified_at,
        status: row.status,
        created_at: row.created_at,
        updated_at: row.updated_at,
        version: row.version,
    })
}

async fn find_source_row(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    request_id: &str,
) -> ApiResult<Option<SourceRow>> {
    sqlx::query_as::<_, SourceRow>(
        "SELECT id,repository_url,git_credential_id,build_agent_id,source_version,status,version FROM application_sources WHERE application_id=?",
    )
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))
}

async fn discovery_response(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    refs_query_id: &str,
    request_id: &str,
) -> ApiResult<GitRefDiscoveryResponse> {
    let row = sqlx::query_as::<_, DiscoveryRow>(
        "SELECT d.id,d.application_source_id,d.source_version,d.task_id,d.status,d.refs_json,d.error_code,d.expires_at,d.created_at,d.finished_at FROM git_ref_discoveries d JOIN application_sources s ON s.id=d.application_source_id WHERE d.id=? AND s.application_id=?",
    )
    .bind(refs_query_id)
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| ApiError::not_found(request_id))?;
    let refs = parse_refs(&row.refs_json, request_id)?;
    Ok(GitRefDiscoveryResponse {
        id: row.id,
        application_source_id: row.application_source_id,
        source_version: row.source_version,
        task_id: row.task_id,
        status: row.status,
        refs,
        error_code: row.error_code,
        expires_at: row.expires_at,
        created_at: row.created_at,
        finished_at: row.finished_at,
    })
}

fn parse_refs(refs_json: &str, request_id: &str) -> ApiResult<Vec<GitRefResponse>> {
    let value: Value =
        serde_json::from_str(refs_json).map_err(|_| ApiError::internal(request_id))?;
    let Some(items) = value.as_array() else {
        return Err(ApiError::internal(request_id));
    };
    if items.len() > MAX_REFS {
        return Err(ApiError::internal(request_id));
    }
    let mut refs = Vec::with_capacity(items.len());
    for item in items {
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| validate_branch_name(name, request_id).is_ok())
            .ok_or_else(|| ApiError::internal(request_id))?;
        let reference = item
            .get("ref")
            .and_then(Value::as_str)
            .filter(|reference| *reference == format!("refs/heads/{name}"))
            .ok_or_else(|| ApiError::internal(request_id))?;
        let sha = item
            .get("sha")
            .and_then(Value::as_str)
            .filter(|sha| valid_sha(sha))
            .ok_or_else(|| ApiError::internal(request_id))?;
        refs.push(GitRefResponse {
            name: name.to_owned(),
            reference: reference.to_owned(),
            sha: sha.to_owned(),
        });
    }
    Ok(refs)
}

fn discovery_expired(discovery: &DiscoveryRow) -> bool {
    discovery
        .expires_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|expires_at| expires_at <= Utc::now())
        .unwrap_or(true)
}

async fn ensure_application_active(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM applications WHERE id=?")
        .bind(application_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    match status.as_deref() {
        Some("active") => Ok(()),
        Some(_) => Err(ApiError::conflict(
            "application_not_active",
            "应用未处于活动状态",
            request_id,
        )),
        None => Err(ApiError::not_found(request_id)),
    }
}

async fn ensure_git_credential_active(
    pool: &sqlx::SqlitePool,
    credential_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM git_credentials WHERE id=?")
            .bind(credential_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    match status.as_deref() {
        Some("active") => Ok(()),
        Some(_) => Err(ApiError::conflict(
            "git_credential_unavailable",
            "Git 凭证不可用",
            request_id,
        )),
        None => Err(ApiError::not_found(request_id)),
    }
}

async fn build_agent_policy(
    pool: &sqlx::SqlitePool,
    agent_id: &str,
    request_id: &str,
) -> ApiResult<BuildAgentPolicy> {
    let agent = sqlx::query_as::<_, BuildAgentPolicy>(
        "SELECT n.status AS node_status,a.protocol_version FROM agents a JOIN nodes n ON n.id=a.node_id WHERE a.id=? AND a.revoked_at IS NULL AND a.archived_at IS NULL",
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| ApiError::not_found(request_id))?;
    if agent.node_status != "online" {
        return Err(ApiError::conflict(
            "agent_offline",
            "构建 Agent 当前不可用",
            request_id,
        ));
    }
    if agent.protocol_version.unwrap_or_default() < 2 {
        return Err(ApiError::conflict(
            "agent_protocol_unsupported",
            "构建 Agent 协议版本不支持两阶段部署",
            request_id,
        ));
    }
    Ok(agent)
}

fn validate_repository_url<'a>(url: &'a str, request_id: &str) -> ApiResult<&'a str> {
    let url = url.trim();
    if url.is_empty()
        || url.len() > 2048
        || url.starts_with('-')
        || url.starts_with("file://")
        || url.chars().any(char::is_control)
        || url.chars().any(char::is_whitespace)
        || contains_credential_userinfo(url)
    {
        return Err(ApiError::validation("Git 仓库地址格式不正确", request_id));
    }
    Ok(url)
}

fn contains_credential_userinfo(url: &str) -> bool {
    if let Some(scheme_end) = url.find("://") {
        let authority = &url[scheme_end + 3..];
        let before_path = authority.split('/').next().unwrap_or(authority);
        if let Some(at) = before_path.rfind('@') {
            return before_path[..at].contains(':');
        }
    } else if let Some(at) = url.find('@') {
        return url[..at].contains(':');
    }
    false
}

fn validate_branch_name<'a>(branch: &'a str, request_id: &str) -> ApiResult<&'a str> {
    let branch = branch.trim();
    let valid = !branch.is_empty()
        && branch.len() <= 255
        && !branch.starts_with('-')
        && !branch.starts_with('/')
        && !branch.starts_with("refs/")
        && !branch.ends_with('/')
        && !branch.ends_with('.')
        && !branch.ends_with(".lock")
        && !branch.contains("..")
        && !branch.contains("@{")
        && !branch.contains("//")
        && !branch.contains('\\')
        && !branch.contains(':')
        && !branch.contains('?')
        && !branch.contains('*')
        && !branch.contains('[')
        && !branch.chars().any(char::is_control)
        && !branch.chars().any(char::is_whitespace);
    if valid {
        Ok(branch)
    } else {
        Err(ApiError::validation("Git 分支名称格式不正确", request_id))
    }
}

fn valid_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sanitized_repository_url(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let authority_start = scheme_end + 3;
        let rest = &url[authority_start..];
        if let Some(at) = rest.find('@') {
            return format!("{}://[REDACTED]@{}", &url[..scheme_end], &rest[at + 1..]);
        }
    }
    url.to_owned()
}

fn default_source_policy() -> String {
    "branch".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_url_rejects_credentials_and_options() {
        assert!(validate_repository_url("git@github.com:example/app.git", "req").is_ok());
        assert!(validate_repository_url("https://github.com/example/app.git", "req").is_ok());
        assert!(validate_repository_url("ssh://git@github.com:22/example/app.git", "req").is_ok());
        assert!(
            validate_repository_url("ssh://user:pass@github.com/example/app.git", "req").is_err()
        );
        assert!(
            validate_repository_url("https://user:pass@github.com/example/app.git", "req").is_err()
        );
        assert!(validate_repository_url("--upload-pack=id", "req").is_err());
        assert!(validate_repository_url("file:///srv/app", "req").is_err());
        assert!(validate_repository_url("git@github.com:example/app.git --foo", "req").is_err());
    }

    #[test]
    fn branch_name_follows_git_ref_rules() {
        assert!(validate_branch_name("main", "req").is_ok());
        assert!(validate_branch_name("release/1.x", "req").is_ok());
        assert!(validate_branch_name("feature/user-login", "req").is_ok());
        assert!(validate_branch_name("..", "req").is_err());
        assert!(validate_branch_name("feature@{1}", "req").is_err());
        assert!(validate_branch_name("feature\\x", "req").is_err());
        assert!(validate_branch_name("feature lock", "req").is_err());
        assert!(validate_branch_name("refs/heads/main", "req").is_err());
    }

    #[test]
    fn sanitized_repository_url_redacts_userinfo() {
        assert_eq!(
            sanitized_repository_url("https://user:pass@github.com/example/app.git"),
            "https://[REDACTED]@github.com/example/app.git"
        );
        assert_eq!(
            sanitized_repository_url("git@github.com:example/app.git"),
            "git@github.com:example/app.git"
        );
    }
}
