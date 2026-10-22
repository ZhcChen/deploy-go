//! 对外 API：部署预览、运行探测与运行状态（非生产应用）。
//!
//! 预览与探测都会写入记录（预览快照、探测任务），因此与其他写接口一致：
//! Key 绑定 → 非生产守卫 → 与管理面共用的核心函数。运行状态只读，沿用 Key 绑定即可。

use axum::{
    Json,
    extract::{Extension, Path, State},
};
use serde::Serialize;
use utoipa::ToSchema;

use super::{
    ExternalApiKey, reject_production_environment, require_external_non_production_environment,
    require_key_application_access, require_no_production_targets,
};
use crate::{
    AppState, RequestId, applications,
    auth::service_actor,
    deployments::{self, ExternalDeploymentPreview, ExternalPreviewRequest},
    error::{ApiError, ApiResult},
    runtime_probe::{self, RuntimeProbeItemResponse},
};

/// 对外运行状态：应用最近一次探测或部署结果的脱敏摘要。
#[derive(Serialize, ToSchema)]
pub struct ExternalRuntimeStatus {
    application_id: String,
    /// `running` / `failed` / `checking` / `unknown` / `archived`
    runtime_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    checked_at: Option<String>,
    /// 最近一次平台探测的状态：`pending` / `running` / `succeeded` / `failed`
    #[serde(skip_serializing_if = "Option::is_none")]
    probe_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    probe_error_code: Option<String>,
}

#[utoipa::path(operation_id = "external_application_deployment_preview", post, path = "/external/v1/applications/{id}/deployment-preview", params(("id" = String, Path)), request_body = ExternalPreviewRequest, responses((status = 200, body = ExternalDeploymentPreview), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn application_preview(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<ExternalPreviewRequest>,
) -> ApiResult<Json<ExternalDeploymentPreview>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_non_production_environment(state.pool(), &id, request_id.as_str()).await?;
    // 与创建部署一致：应用带有正式环境目标时不允许对外预览。
    require_no_production_targets(state.pool(), &id, request_id.as_str()).await?;
    let preview = deployments::application_preview_core(
        &state,
        &service_actor(),
        &id,
        &payload.parameters,
        &payload.release_strategy,
        request_id.as_str(),
    )
    .await?;
    Ok(Json(preview.into()))
}

#[utoipa::path(operation_id = "external_target_deployment_preview", post, path = "/external/v1/deployment-targets/{target_id}/deployment-preview", params(("target_id" = String, Path)), request_body = ExternalPreviewRequest, responses((status = 200, body = ExternalDeploymentPreview), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn target_preview(
    State(state): State<AppState>,
    Path(target_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<ExternalPreviewRequest>,
) -> ApiResult<Json<ExternalDeploymentPreview>> {
    let target: Option<(String, String, String)> = sqlx::query_as(
        "SELECT application_id,status,environment FROM deployment_targets WHERE id=?",
    )
    .bind(&target_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let Some((application_id, status, environment)) = target else {
        return Err(ApiError::not_found(request_id.as_str()));
    };
    require_key_application_access(state.pool(), &key, &application_id, request_id.as_str())
        .await?;
    if status != "active" {
        return Err(ApiError::not_found(request_id.as_str()));
    }
    reject_production_environment(&environment, request_id.as_str())?;
    require_external_non_production_environment(state.pool(), &application_id, request_id.as_str())
        .await?;
    let preview = deployments::target_preview_core(
        &state,
        &service_actor(),
        &target_id,
        &payload.parameters,
        &payload.release_strategy,
        request_id.as_str(),
    )
    .await?;
    Ok(Json(preview.into()))
}

#[utoipa::path(operation_id = "external_application_runtime_probe", post, path = "/external/v1/applications/{id}/runtime-probe", params(("id" = String, Path)), responses((status = 200, body = RuntimeProbeItemResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn probe_application(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<RuntimeProbeItemResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_non_production_environment(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(
        runtime_probe::probe_application_core(&state, &id, &service_actor(), request_id.as_str())
            .await?,
    ))
}

#[utoipa::path(operation_id = "external_application_runtime_status", get, path = "/external/v1/applications/{id}/runtime-status", params(("id" = String, Path)), responses((status = 200, body = ExternalRuntimeStatus), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn runtime_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalRuntimeStatus>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    let application = applications::find(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(ExternalRuntimeStatus {
        application_id: application.id,
        runtime_state: application.runtime_state,
        checked_at: application.runtime_checked_at,
        probe_status: application.runtime_probe_status,
        probe_error_code: application.runtime_probe_error_code,
    }))
}
