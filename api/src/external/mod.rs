use axum::{
    Json, Router,
    extract::{Extension, FromRequestParts, Path, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION, request::Parts},
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId,
    auth::service_actor,
    deployments,
    error::{ApiError, ApiResult},
    external_keys,
};

#[derive(Clone)]
pub(crate) struct ExternalApiKey {
    pub id: String,
}

impl FromRequestParts<AppState> for ExternalApiKey {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let request_id = parts
            .extensions
            .get::<RequestId>()
            .map(RequestId::as_str)
            .unwrap_or("req_unknown");
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|value| !value.is_empty() && value.len() <= 256)
            .ok_or_else(|| ApiError::unauthorized(request_id))?;
        let row: Option<(String, String, Option<String>)> =
            sqlx::query_as("SELECT id,status,expires_at FROM external_api_keys WHERE token_hash=?")
                .bind(external_keys::token_hash(token))
                .fetch_optional(state.pool())
                .await
                .map_err(|_| ApiError::internal(request_id))?;
        let Some((id, status, expires_at)) = row else {
            return Err(ApiError::unauthorized(request_id));
        };
        let now = Utc::now().to_rfc3339();
        if status != "active"
            || expires_at
                .as_deref()
                .is_some_and(|value| value <= now.as_str())
        {
            return Err(ApiError::unauthorized(request_id));
        }
        sqlx::query(
            "UPDATE external_api_keys SET last_used_at=?,updated_at=?,version=version+1 WHERE id=?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(&id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id))?;
        Ok(ExternalApiKey { id })
    }
}

#[derive(Serialize, ToSchema)]
pub struct ExternalApplicationSummary {
    id: String,
    name: String,
    slug: String,
    description: String,
    status: String,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalApplicationListResponse {
    items: Vec<ExternalApplicationSummary>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalDeploymentTarget {
    id: String,
    environment: String,
    node_id: String,
    node_name: String,
    status: String,
    execution_mode: String,
    privileged_release: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalApplicationDetail {
    id: String,
    name: String,
    slug: String,
    description: String,
    status: String,
    targets: Vec<ExternalDeploymentTarget>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalDeploymentRequest {
    #[serde(default)]
    target_id: Option<String>,
    parameters: serde_json::Value,
    #[serde(default = "deployments::default_release_strategy")]
    release_strategy: String,
    #[serde(default)]
    release_version: Option<String>,
    #[serde(default)]
    snapshot_hash: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalDeploymentTargetRun {
    id: String,
    target_id: String,
    node_id: String,
    node_name: String,
    status: String,
    phase: String,
    result_summary: Option<String>,
    error_code: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalDeployment {
    id: String,
    application_id: String,
    application_name: String,
    target_id: String,
    environment: String,
    node_name: String,
    status: String,
    phase: String,
    snapshot_hash: String,
    result_summary: Option<String>,
    exit_code: Option<i64>,
    queued_at: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    cancel_requested_at: Option<String>,
    created_at: String,
    updated_at: String,
    target_runs: Vec<ExternalDeploymentTargetRun>,
}

#[derive(sqlx::FromRow)]
struct ExternalDeploymentRow {
    id: String,
    application_id: String,
    application_name: String,
    target_id: String,
    environment: String,
    node_name: String,
    status: String,
    phase: String,
    snapshot_hash: String,
    result_summary: Option<String>,
    exit_code: Option<i64>,
    queued_at: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    cancel_requested_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct ExternalDeploymentRunRow {
    id: String,
    target_id: String,
    node_id: String,
    node_name: String,
    status: String,
    phase: String,
    result_summary: Option<String>,
    error_code: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    created_at: String,
    updated_at: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/applications", get(list_applications))
        .route("/applications/{id}", get(show_application))
        .route("/applications/{id}/deployments", post(create_deployment))
        .route("/deployments/{id}", get(show_deployment))
        .route("/deployments/{id}/cancel", post(cancel_deployment))
}

#[utoipa::path(operation_id = "external_applications_list", get, path = "/external/v1/applications", responses((status = 200, body = ExternalApplicationListResponse), (status = 401, body = crate::error::ErrorResponse)))]
pub(crate) async fn list_applications(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalApplicationListResponse>> {
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT a.id,a.name,a.slug,a.description FROM applications a JOIN external_api_key_applications key_app ON key_app.application_id=a.id WHERE key_app.api_key_id=? AND a.status='active' ORDER BY a.name COLLATE NOCASE,a.id",
    )
    .bind(&key.id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(ExternalApplicationListResponse {
        items: rows
            .into_iter()
            .map(|(id, name, slug, description)| ExternalApplicationSummary {
                id,
                name,
                slug,
                description,
                status: "active".to_owned(),
            })
            .collect(),
    }))
}

#[utoipa::path(operation_id = "external_applications_show", get, path = "/external/v1/applications/{id}", params(("id" = String, Path)), responses((status = 200, body = ExternalApplicationDetail), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show_application(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalApplicationDetail>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    let application: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT name,slug,description,status FROM applications WHERE id=? AND status='active'",
    )
    .bind(&id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (name, slug, description, status) =
        application.ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
    let targets = sqlx::query_as::<_, (String, String, String, String, String, String, bool)>(
        "SELECT t.id,t.environment,t.node_id,n.name,t.status,t.execution_mode,t.privileged_release FROM deployment_targets t JOIN nodes n ON n.id=t.node_id WHERE t.application_id=? AND t.status='active' ORDER BY t.id",
    )
    .bind(&id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(ExternalApplicationDetail {
        id,
        name,
        slug,
        description,
        status,
        targets: targets
            .into_iter()
            .map(
                |(
                    id,
                    environment,
                    node_id,
                    node_name,
                    status,
                    execution_mode,
                    privileged_release,
                )| {
                    ExternalDeploymentTarget {
                        id,
                        environment,
                        node_id,
                        node_name,
                        status,
                        execution_mode,
                        privileged_release,
                    }
                },
            )
            .collect(),
    }))
}

#[utoipa::path(operation_id = "external_deployments_create", post, path = "/external/v1/applications/{id}/deployments", params(("id" = String, Path), ("Idempotency-Key" = String, Header)), request_body = ExternalDeploymentRequest, responses((status = 200, body = ExternalDeployment), (status = 201, body = ExternalDeployment), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn create_deployment(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<ExternalDeploymentRequest>,
) -> ApiResult<(StatusCode, Json<ExternalDeployment>)> {
    require_key_application_access(state.pool(), &key, &application_id, request_id.as_str())
        .await?;
    let idempotency_key = deployments::validate_idempotency_key(&headers, request_id.as_str())?;
    let actor = service_actor();
    let (status, response) = match payload.target_id.as_deref() {
        Some(target_id) => {
            let target: Option<(String, String)> =
                sqlx::query_as("SELECT application_id,status FROM deployment_targets WHERE id=?")
                    .bind(target_id)
                    .fetch_optional(state.pool())
                    .await
                    .map_err(|_| ApiError::internal(request_id.as_str()))?;
            let Some((target_application_id, target_status)) = target else {
                return Err(ApiError::not_found(request_id.as_str()));
            };
            if target_application_id != application_id || target_status != "active" {
                return Err(ApiError::not_found(request_id.as_str()));
            }
            deployments::create_target_deployment(
                &state,
                &actor,
                Some(&key.id),
                target_id,
                &payload.parameters,
                payload.snapshot_hash.as_deref(),
                &payload.release_strategy,
                payload.release_version.as_deref(),
                &format!("external-target-confirm:{}:{idempotency_key}", key.id),
                request_id.as_str(),
            )
            .await?
        }
        None => {
            deployments::create_application_deployment(
                &state,
                &actor,
                Some(&key.id),
                &application_id,
                &payload.parameters,
                payload.snapshot_hash.as_deref(),
                &payload.release_strategy,
                payload.release_version.as_deref(),
                &format!("external-app-confirm:{}:{idempotency_key}", key.id),
                request_id.as_str(),
            )
            .await?
        }
    };
    let deployment =
        load_external_deployment(state.pool(), &response.id, request_id.as_str()).await?;
    Ok((status, Json(deployment)))
}

#[utoipa::path(operation_id = "external_deployments_show", get, path = "/external/v1/deployments/{id}", params(("id" = String, Path)), responses((status = 200, body = ExternalDeployment), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show_deployment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalDeployment>> {
    let application_id = deployment_application_id(state.pool(), &id, request_id.as_str()).await?;
    require_key_application_access(state.pool(), &key, &application_id, request_id.as_str())
        .await?;
    Ok(Json(
        load_external_deployment(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "external_deployments_cancel", post, path = "/external/v1/deployments/{id}/cancel", params(("id" = String, Path)), responses((status = 200, body = ExternalDeployment), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn cancel_deployment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalDeployment>> {
    let application_id = deployment_application_id(state.pool(), &id, request_id.as_str()).await?;
    require_key_application_access(state.pool(), &key, &application_id, request_id.as_str())
        .await?;
    deployments::cancel_deployment(&state, &service_actor(), &id, request_id.as_str()).await?;
    Ok(Json(
        load_external_deployment(state.pool(), &id, request_id.as_str()).await?,
    ))
}

async fn deployment_application_id(
    pool: &SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<String> {
    sqlx::query_scalar("SELECT application_id FROM deployments WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))
}

async fn load_external_deployment(
    pool: &SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<ExternalDeployment> {
    let row: Option<ExternalDeploymentRow> = sqlx::query_as(
        "SELECT d.id,d.application_id,a.name AS application_name,d.target_id,t.environment,n.name AS node_name,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at FROM deployments d JOIN applications a ON a.id=d.application_id JOIN deployment_targets t ON t.id=d.target_id JOIN nodes n ON n.id=t.node_id WHERE d.id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let Some(row) = row else {
        return Err(ApiError::not_found(request_id));
    };
    let runs: Vec<ExternalDeploymentRunRow> =
        sqlx::query_as(
            "SELECT run.id,run.target_id,run.node_id,n.name AS node_name,run.status,run.phase,run.result_summary,run.error_code,run.started_at,run.finished_at,run.created_at,run.updated_at FROM deployment_target_runs run JOIN nodes n ON n.id=run.node_id WHERE run.deployment_id=? ORDER BY run.target_id,run.id",
        )
        .bind(&row.id)
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok(ExternalDeployment {
        id: row.id,
        application_id: row.application_id,
        application_name: row.application_name,
        target_id: row.target_id,
        environment: row.environment,
        node_name: row.node_name,
        status: row.status,
        phase: row.phase,
        snapshot_hash: row.snapshot_hash,
        result_summary: row.result_summary,
        exit_code: row.exit_code,
        queued_at: row.queued_at,
        started_at: row.started_at,
        finished_at: row.finished_at,
        cancel_requested_at: row.cancel_requested_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
        target_runs: runs
            .into_iter()
            .map(|run| ExternalDeploymentTargetRun {
                id: run.id,
                target_id: run.target_id,
                node_id: run.node_id,
                node_name: run.node_name,
                status: run.status,
                phase: run.phase,
                result_summary: run.result_summary,
                error_code: run.error_code,
                started_at: run.started_at,
                finished_at: run.finished_at,
                created_at: run.created_at,
                updated_at: run.updated_at,
            })
            .collect(),
    })
}

pub(crate) async fn require_key_application_access(
    pool: &SqlitePool,
    key: &ExternalApiKey,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let visible: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM external_api_key_applications key_app JOIN applications a ON a.id=key_app.application_id WHERE key_app.api_key_id=? AND key_app.application_id=? AND a.status='active')",
    )
    .bind(&key.id)
    .bind(application_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if visible {
        Ok(())
    } else {
        Err(ApiError::not_found(request_id))
    }
}
