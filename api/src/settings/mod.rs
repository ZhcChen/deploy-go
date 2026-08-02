use axum::{
    Json, Router,
    extract::{Extension, State},
    http::HeaderMap,
    routing::get,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
};

const SETTINGS_KEY: &str = "runtime";

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSettings {
    pub max_concurrent_deployments: u32,
    pub max_log_bytes: u64,
    pub log_retention_days: u32,
    pub version: i64,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            max_concurrent_deployments: 2,
            max_log_bytes: 50 * 1024 * 1024,
            log_retention_days: 30,
            version: 1,
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new().route("/settings", get(show).patch(update))
}

#[utoipa::path(operation_id = "settings_show", get, path = "/api/v1/settings", responses((status = 200, body = RuntimeSettings), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    user: AuthUser,
) -> ApiResult<Json<RuntimeSettings>> {
    user.require_administrator(request_id.as_str())?;
    Ok(Json(load(state.pool(), request_id.as_str()).await?))
}

#[utoipa::path(operation_id = "settings_update", patch, path = "/api/v1/settings", request_body = RuntimeSettings, responses((status = 200, body = RuntimeSettings), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<RuntimeSettings>,
) -> ApiResult<Json<RuntimeSettings>> {
    user.require_administrator(request_id.as_str())?;
    user.verify_csrf(&headers, request_id.as_str())?;
    validate(&payload, request_id.as_str())?;
    let current = load(state.pool(), request_id.as_str()).await?;
    if payload.version != current.version {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "设置已经被其他请求修改",
            request_id.as_str(),
        ));
    }
    let mut next = payload;
    next.version += 1;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO system_settings (key, value_json, updated_by, updated_at, version) VALUES (?, ?, ?, ?, ?) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_by = excluded.updated_by, updated_at = excluded.updated_at, version = excluded.version")
        .bind(SETTINGS_KEY).bind(serde_json::to_string(&next).map_err(|_| ApiError::internal(request_id.as_str()))?)
        .bind(&user.id).bind(Utc::now().to_rfc3339()).bind(next.version).execute(&mut *transaction).await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&user.id),
        "settings.update",
        "settings",
        SETTINGS_KEY,
        request_id.as_str(),
        json!({"version":next.version}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(next))
}

fn validate(settings: &RuntimeSettings, request_id: &str) -> ApiResult<()> {
    if !(1..=64).contains(&settings.max_concurrent_deployments)
        || !(1024 * 1024..=1024 * 1024 * 1024).contains(&settings.max_log_bytes)
        || !(1..=3650).contains(&settings.log_retention_days)
    {
        return Err(ApiError::validation("系统设置超出允许范围", request_id));
    }
    Ok(())
}

pub(crate) async fn load(pool: &sqlx::SqlitePool, request_id: &str) -> ApiResult<RuntimeSettings> {
    let row: Option<(String, i64)> =
        sqlx::query_as("SELECT value_json, version FROM system_settings WHERE key = ?")
            .bind(SETTINGS_KEY)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    match row {
        Some((value, version)) => {
            let mut settings: RuntimeSettings =
                serde_json::from_str(&value).map_err(|_| ApiError::internal(request_id))?;
            settings.version = version;
            Ok(settings)
        }
        None => Ok(RuntimeSettings::default()),
    }
}
