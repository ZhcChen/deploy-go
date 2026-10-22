//! 对外 API：Git 来源配置、分支发现与目录（构建 Agent、Git 凭证）。
//!
//! 写入统一经过：Key 绑定 → 非生产守卫 → 节点白名单 → 与管理面共用的核心函数。
//! 目录接口只返回标识与状态，不返回节点连接信息、公钥或私钥。

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};
use serde::Serialize;
use utoipa::ToSchema;

use super::{
    ExternalApiKey, require_external_configurable_application, require_key_application_access,
};
use crate::{
    AppState, RequestId,
    application_sources::{
        self, ApplicationSourceResponse, GitRefDiscoveryResponse, SaveSourceRequest,
        SetBranchRequest,
    },
    auth::service_actor,
    error::{ApiError, ApiResult},
    external_keys,
};

/// 对外构建 Agent 目录项。只包含该 Key 被授权节点上的 Agent，不含主机、端口、用户名与目录。
#[derive(Serialize, ToSchema)]
pub struct ExternalBuildAgent {
    agent_id: String,
    node_id: String,
    node_name: String,
    node_status: String,
    /// 是否满足构建要求（节点在线、协议与能力满足）；与保存 Git 来源时的校验一致。
    ready: bool,
    /// `ready=false` 时的稳定原因码，例如 `agent_offline`。
    unavailable_code: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalBuildAgentListResponse {
    items: Vec<ExternalBuildAgent>,
}

/// 对外 Git 凭证目录项，只含元数据，外部只能引用，不能创建、轮换或停用。
#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct ExternalGitCredential {
    id: String,
    name: String,
    fingerprint: String,
    status: String,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalGitCredentialListResponse {
    items: Vec<ExternalGitCredential>,
}

#[derive(sqlx::FromRow)]
struct BuildAgentRow {
    agent_id: String,
    node_id: String,
    node_name: String,
    node_status: String,
    protocol_version: Option<i64>,
    capabilities_json: Option<String>,
}

const MAX_DIRECTORY_ITEMS: i64 = 500;

#[utoipa::path(operation_id = "external_build_agents_list", get, path = "/external/v1/build-agents", responses((status = 200, body = ExternalBuildAgentListResponse), (status = 401, body = crate::error::ErrorResponse)))]
pub(crate) async fn list_build_agents(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalBuildAgentListResponse>> {
    let rows = sqlx::query_as::<_, BuildAgentRow>(
        "SELECT a.id AS agent_id,n.id AS node_id,n.name AS node_name,n.status AS node_status,a.protocol_version,a.capabilities_json FROM agents a JOIN nodes n ON n.id=a.node_id JOIN external_api_key_nodes key_node ON key_node.node_id=n.id WHERE key_node.api_key_id=? AND a.revoked_at IS NULL AND a.archived_at IS NULL AND n.archived_at IS NULL ORDER BY n.name COLLATE NOCASE,a.id LIMIT ?",
    )
    .bind(&key.id)
    .bind(MAX_DIRECTORY_ITEMS)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let items = rows
        .into_iter()
        .map(|row| {
            let unavailable = application_sources::build_agent_unavailable(
                &row.node_status,
                false,
                row.protocol_version,
                row.capabilities_json.as_deref(),
            );
            ExternalBuildAgent {
                agent_id: row.agent_id,
                node_id: row.node_id,
                node_name: row.node_name,
                node_status: row.node_status,
                ready: unavailable.is_none(),
                unavailable_code: unavailable.map(|(code, _)| code.to_owned()),
            }
        })
        .collect();
    Ok(Json(ExternalBuildAgentListResponse { items }))
}

#[utoipa::path(operation_id = "external_git_credentials_list", get, path = "/external/v1/git-credentials", responses((status = 200, body = ExternalGitCredentialListResponse), (status = 401, body = crate::error::ErrorResponse)))]
pub(crate) async fn list_git_credentials(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    _key: ExternalApiKey,
) -> ApiResult<Json<ExternalGitCredentialListResponse>> {
    let items = sqlx::query_as::<_, ExternalGitCredential>(
        "SELECT id,name,fingerprint,status FROM git_credentials ORDER BY name COLLATE NOCASE,id LIMIT ?",
    )
    .bind(MAX_DIRECTORY_ITEMS)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(ExternalGitCredentialListResponse { items }))
}

#[utoipa::path(operation_id = "external_application_git_source_show", get, path = "/external/v1/applications/{id}/git-source", params(("id" = String, Path)), responses((status = 200, body = ApplicationSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show_git_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ApplicationSourceResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    // 来源详情含仓库地址与凭证名，既有 `sources.git` 摘要不含这些字段：新增读取不对 prod 开放。
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(
        application_sources::find_source_view(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "external_application_git_source_save", put, path = "/external/v1/applications/{id}/git-source", params(("id" = String, Path)), request_body = SaveSourceRequest, responses((status = 200, body = ApplicationSourceResponse), (status = 201, body = ApplicationSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn save_git_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<SaveSourceRequest>,
) -> ApiResult<(StatusCode, Json<ApplicationSourceResponse>)> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    external_keys::require_key_agent_node(
        state.pool(),
        &key.id,
        &payload.build_agent_id,
        request_id.as_str(),
    )
    .await?;
    let (created, view) = application_sources::save_source_core(
        &state,
        &id,
        &payload,
        &service_actor().id,
        Some(&key.id),
        request_id.as_str(),
    )
    .await?;
    Ok((
        if created {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(view),
    ))
}

/// 对已配置的来源做写前校验：应用必须非生产，且来源现用的构建 Agent 仍在 Key 的授权节点内
/// （发起发现和设置分支都会让该 Agent 参与，不能因为授权被收回后仍可驱动该节点）。
async fn require_configured_source_access(
    state: &AppState,
    key: &ExternalApiKey,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    require_key_application_access(state.pool(), key, application_id, request_id).await?;
    require_external_configurable_application(state.pool(), application_id, request_id).await?;
    let source =
        application_sources::find_source_view(state.pool(), application_id, request_id).await?;
    external_keys::require_key_agent_node(state.pool(), &key.id, &source.build_agent_id, request_id)
        .await
}

#[utoipa::path(operation_id = "external_application_git_source_refresh", post, path = "/external/v1/applications/{id}/git-source/refreshes", params(("id" = String, Path)), responses((status = 202, body = GitRefDiscoveryResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn refresh_git_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<(StatusCode, Json<GitRefDiscoveryResponse>)> {
    require_configured_source_access(&state, &key, &id, request_id.as_str()).await?;
    let discovery = application_sources::refresh_core(
        &state,
        &service_actor().id,
        Some(&key.id),
        &id,
        request_id.as_str(),
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(discovery)))
}

#[utoipa::path(operation_id = "external_application_git_source_refresh_show", get, path = "/external/v1/applications/{id}/git-source/refreshes/{refs_query_id}", params(("id" = String, Path), ("refs_query_id" = String, Path)), responses((status = 200, body = GitRefDiscoveryResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show_git_source_refresh(
    State(state): State<AppState>,
    Path((id, refs_query_id)): Path<(String, String)>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<GitRefDiscoveryResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(
        application_sources::discovery_response(
            state.pool(),
            &id,
            &refs_query_id,
            request_id.as_str(),
        )
        .await?,
    ))
}

#[utoipa::path(operation_id = "external_application_git_source_branch_set", put, path = "/external/v1/applications/{id}/git-source/branch", params(("id" = String, Path)), request_body = SetBranchRequest, responses((status = 200, body = ApplicationSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn set_git_branch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<SetBranchRequest>,
) -> ApiResult<Json<ApplicationSourceResponse>> {
    require_configured_source_access(&state, &key, &id, request_id.as_str()).await?;
    Ok(Json(
        application_sources::set_branch_core(
            &state,
            &id,
            &payload,
            &service_actor().id,
            Some(&key.id),
            request_id.as_str(),
        )
        .await?,
    ))
}
