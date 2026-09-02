use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use chrono::Utc;
use deploy_go_agent_protocol::{AgentCapability, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    grants,
};

const MAX_WORKSPACE_PATH_LEN: usize = 4096;

#[derive(Clone, Serialize, ToSchema)]
pub struct WorkspaceSourceResponse {
    pub id: String,
    pub application_id: String,
    pub build_agent_id: String,
    pub build_agent_name: Option<String>,
    pub workspace_path: String,
    pub workspace_version: i64,
    pub status: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveWorkspaceSourceRequest {
    build_agent_id: String,
    workspace_path: String,
    version: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct WorkspaceSourceViewRow {
    id: String,
    application_id: String,
    build_agent_id: String,
    build_agent_name: Option<String>,
    workspace_path: String,
    workspace_version: i64,
    status: String,
    created_by: Option<String>,
    created_at: String,
    updated_at: String,
    version: i64,
}

#[derive(sqlx::FromRow)]
struct WorkspaceSourceRow {
    id: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/applications/{application_id}/workspace-source",
        get(show).put(save),
    )
}

#[utoipa::path(operation_id = "application_workspace_source_show", get, path = "/api/v1/applications/{application_id}/workspace-source", params(("application_id" = String, Path)), responses((status = 200, body = WorkspaceSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<WorkspaceSourceResponse>> {
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
    Ok(Json(
        find_view(state.pool(), &application_id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "application_workspace_source_save", put, path = "/api/v1/applications/{application_id}/workspace-source", params(("application_id" = String, Path)), request_body = SaveWorkspaceSourceRequest, responses((status = 200, body = WorkspaceSourceResponse), (status = 201, body = WorkspaceSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn save(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<SaveWorkspaceSourceRequest>,
) -> ApiResult<(StatusCode, Json<WorkspaceSourceResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    ensure_application_active(state.pool(), &application_id, request_id.as_str()).await?;
    let workspace_path = validate_workspace_path(&payload.workspace_path, request_id.as_str())?;
    ensure_build_agent_ready(state.pool(), &payload.build_agent_id, request_id.as_str()).await?;

    let existing = find_row(state.pool(), &application_id, request_id.as_str()).await?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let created = existing.is_none();
    let source_id = existing
        .as_ref()
        .map(|source| source.id.clone())
        .unwrap_or_else(|| format!("workspace_source_{}", Ulid::new()));
    let now = Utc::now().to_rfc3339();

    if let Some(_existing) = existing {
        let version = payload.version.ok_or_else(|| {
            ApiError::validation("编辑工作区来源必须提供 version", request_id.as_str())
        })?;
        let updated = sqlx::query(
            "UPDATE application_workspace_sources SET build_agent_id=?,workspace_path=?,workspace_version=workspace_version+1,status='verified',updated_at=?,version=version+1 WHERE id=? AND version=?",
        )
        .bind(&payload.build_agent_id)
        .bind(workspace_path)
        .bind(&now)
        .bind(&source_id)
        .bind(version)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
        if updated.rows_affected() != 1 {
            return Err(ApiError::conflict(
                "resource_version_conflict",
                "工作区来源已经被其他请求修改",
                request_id.as_str(),
            ));
        }
    } else {
        sqlx::query("INSERT INTO application_workspace_sources (id,application_id,build_agent_id,workspace_path,status,created_by) VALUES (?,?,?,?,'verified',?)")
            .bind(&source_id)
            .bind(&application_id)
            .bind(&payload.build_agent_id)
            .bind(workspace_path)
            .bind(&actor.id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
    }

    sqlx::query("DELETE FROM deployment_previews WHERE application_id=? AND status='active'")
        .bind(&application_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        if created {
            "application_workspace_source.create"
        } else {
            "application_workspace_source.update"
        },
        "application_workspace_source",
        &source_id,
        request_id.as_str(),
        json!({
            "application_id": application_id,
            "build_agent_id": payload.build_agent_id,
            "workspace_path": workspace_path,
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;

    Ok((
        if created {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(find_view(state.pool(), &application_id, request_id.as_str()).await?),
    ))
}

async fn find_view(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    request_id: &str,
) -> ApiResult<WorkspaceSourceResponse> {
    let row = sqlx::query_as::<_, WorkspaceSourceViewRow>(
        "SELECT s.id,s.application_id,s.build_agent_id,n.name AS build_agent_name,s.workspace_path,s.workspace_version,s.status,s.created_by,s.created_at,s.updated_at,s.version FROM application_workspace_sources s JOIN agents a ON a.id=s.build_agent_id JOIN nodes n ON n.id=a.node_id WHERE s.application_id=?",
    )
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| ApiError::not_found(request_id))?;
    Ok(WorkspaceSourceResponse {
        id: row.id,
        application_id: row.application_id,
        build_agent_id: row.build_agent_id,
        build_agent_name: row.build_agent_name,
        workspace_path: row.workspace_path,
        workspace_version: row.workspace_version,
        status: row.status,
        created_by: row.created_by,
        created_at: row.created_at,
        updated_at: row.updated_at,
        version: row.version,
    })
}

async fn find_row(
    pool: &sqlx::SqlitePool,
    application_id: &str,
    request_id: &str,
) -> ApiResult<Option<WorkspaceSourceRow>> {
    sqlx::query_as::<_, WorkspaceSourceRow>(
        "SELECT id FROM application_workspace_sources WHERE application_id=?",
    )
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))
}

fn validate_workspace_path<'a>(value: &'a str, request_id: &str) -> ApiResult<&'a str> {
    let path = std::path::Path::new(value);
    let normalized = value
        .chars()
        .all(|character| !character.is_control() && character != '\0');
    if !normalized
        || value.len() > MAX_WORKSPACE_PATH_LEN
        || !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(ApiError::validation(
            "工作区路径必须是安全的绝对路径",
            request_id,
        ));
    }
    Ok(value)
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

async fn ensure_build_agent_ready(
    pool: &sqlx::SqlitePool,
    agent_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let agent = sqlx::query_as::<_, (Option<i64>, Option<String>, String)>(
        "SELECT a.protocol_version,a.capabilities_json,n.status FROM agents a JOIN nodes n ON n.id=a.node_id WHERE a.id=? AND a.revoked_at IS NULL AND a.archived_at IS NULL",
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| ApiError::not_found(request_id))?;
    let (protocol_version, capabilities_json, node_status) = agent;
    if node_status != "online" {
        return Err(ApiError::conflict(
            "agent_offline",
            "构建 Agent 当前不可用",
            request_id,
        ));
    }
    let protocol_version = protocol_version.unwrap_or_default();
    if protocol_version < i64::from(PROTOCOL_VERSION)
        || protocol_version > i64::from(PROTOCOL_VERSION)
    {
        return Err(ApiError::conflict(
            "agent_protocol_unsupported",
            "脚本两阶段要求构建 Agent 升级到控制协议 v14",
            request_id,
        ));
    }
    let capabilities = capabilities_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<Vec<AgentCapability>>(value).ok())
        .unwrap_or_default();
    if !capabilities.contains(&AgentCapability::PtyTerminal)
        || !capabilities.contains(&AgentCapability::PrivilegedRelease)
    {
        return Err(ApiError::conflict(
            "agent_capability_unavailable",
            "构建 Agent 未具备 PTY 或特权 release 能力",
            request_id,
        ));
    }
    Ok(())
}
