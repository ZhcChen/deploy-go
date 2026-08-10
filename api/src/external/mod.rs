use axum::{
    Json, Router,
    extract::{Extension, FromRequestParts, Path, State},
    http::{header::AUTHORIZATION, request::Parts},
    routing::get,
};
use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId,
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
        let row: Option<(String, String, Option<String>)> = sqlx::query_as(
            "SELECT id,status,expires_at FROM external_api_keys WHERE token_hash=?",
        )
        .bind(external_keys::token_hash(token))
        .fetch_optional(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id))?;
        let Some((id, status, expires_at)) = row else {
            return Err(ApiError::unauthorized(request_id));
        };
        let now = Utc::now().to_rfc3339();
        if status != "active" || expires_at.as_deref().is_some_and(|value| value <= now.as_str())
        {
            return Err(ApiError::unauthorized(request_id));
        }
        sqlx::query("UPDATE external_api_keys SET last_used_at=?,updated_at=?,version=version+1 WHERE id=?")
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

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/applications", get(list_applications))
        .route("/applications/{id}", get(show_application))
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
                |(id, environment, node_id, node_name, status, execution_mode, privileged_release)| {
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
