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

fn default_max_total_log_bytes() -> u64 {
    2 * 1024 * 1024 * 1024
}

/// 会话空闲期：用户持续活跃时滑动续期，超过这么久没有任何请求就过期。
pub(crate) const DEFAULT_SESSION_IDLE_DAYS: u32 = 7;
/// 会话绝对期：无论是否活跃，登录满这么久必须重新登录。
pub(crate) const DEFAULT_SESSION_ABSOLUTE_DAYS: u32 = 30;

fn default_session_idle_days() -> u32 {
    DEFAULT_SESSION_IDLE_DAYS
}

fn default_session_absolute_days() -> u32 {
    DEFAULT_SESSION_ABSOLUTE_DAYS
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSettings {
    pub max_concurrent_deployments: u32,
    pub max_log_bytes: u64,
    #[serde(default = "default_max_total_log_bytes")]
    pub max_total_log_bytes: u64,
    pub log_retention_days: u32,
    /// 登录会话的空闲期（天）。旧版本保存的设置没有该字段，读取时回落到默认值。
    #[serde(default = "default_session_idle_days")]
    pub session_idle_days: u32,
    /// 登录会话的绝对期（天），必须不小于空闲期。
    #[serde(default = "default_session_absolute_days")]
    pub session_absolute_days: u32,
    pub version: i64,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            max_concurrent_deployments: 2,
            max_log_bytes: 50 * 1024 * 1024,
            max_total_log_bytes: default_max_total_log_bytes(),
            log_retention_days: 30,
            session_idle_days: DEFAULT_SESSION_IDLE_DAYS,
            session_absolute_days: DEFAULT_SESSION_ABSOLUTE_DAYS,
            version: 1,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSettingsUpdate {
    pub max_concurrent_deployments: u32,
    pub max_log_bytes: u64,
    pub max_total_log_bytes: Option<u64>,
    pub log_retention_days: u32,
    /// 缺省时保持当前值，旧客户端不传这两个字段也不会改掉它们。
    pub session_idle_days: Option<u32>,
    pub session_absolute_days: Option<u32>,
    pub version: i64,
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

#[utoipa::path(operation_id = "settings_update", patch, path = "/api/v1/settings", request_body = RuntimeSettingsUpdate, responses((status = 200, body = RuntimeSettings), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<RuntimeSettingsUpdate>,
) -> ApiResult<Json<RuntimeSettings>> {
    user.require_administrator(request_id.as_str())?;
    user.verify_csrf(&headers, request_id.as_str())?;
    let current = load(state.pool(), request_id.as_str()).await?;
    if payload.version != current.version {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "设置已经被其他请求修改",
            request_id.as_str(),
        ));
    }
    let mut next = RuntimeSettings {
        max_concurrent_deployments: payload.max_concurrent_deployments,
        max_log_bytes: payload.max_log_bytes,
        max_total_log_bytes: payload
            .max_total_log_bytes
            .unwrap_or(current.max_total_log_bytes),
        log_retention_days: payload.log_retention_days,
        session_idle_days: payload
            .session_idle_days
            .unwrap_or(current.session_idle_days),
        session_absolute_days: payload
            .session_absolute_days
            .unwrap_or(current.session_absolute_days),
        version: payload.version,
    };
    validate(&next, request_id.as_str())?;
    next.version += 1;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let changed = sqlx::query("INSERT INTO system_settings (key, value_json, updated_by, updated_at, version) VALUES (?, ?, ?, ?, ?) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_by = excluded.updated_by, updated_at = excluded.updated_at, version = excluded.version WHERE system_settings.version = ?")
        .bind(SETTINGS_KEY).bind(serde_json::to_string(&next).map_err(|_| ApiError::internal(request_id.as_str()))?)
        .bind(&user.id).bind(Utc::now().to_rfc3339()).bind(next.version).bind(current.version).execute(&mut *transaction).await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if changed.rows_affected() != 1 {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "设置已经被其他请求修改",
            request_id.as_str(),
        ));
    }
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
        || !(1024 * 1024..=i64::MAX as u64).contains(&settings.max_total_log_bytes)
        || !(1..=30).contains(&settings.session_idle_days)
        || !(1..=365).contains(&settings.session_absolute_days)
        || settings.session_absolute_days < settings.session_idle_days
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
