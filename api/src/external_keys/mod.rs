use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    pagination,
};

pub(crate) const SERVICE_USER_ID: &str = "usr_external_api_service";
const TOKEN_PREFIX: &str = "dgx_";

#[derive(Serialize, ToSchema)]
pub struct ExternalApiKeySummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub application_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalApiKeyListResponse {
    items: Vec<ExternalApiKeySummary>,
    next_cursor: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalApiKeyCreatedResponse {
    id: String,
    name: String,
    token: String,
    status: String,
    expires_at: Option<String>,
    application_ids: Vec<String>,
    created_at: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateExternalApiKeyRequest {
    name: String,
    application_ids: Vec<String>,
    #[serde(default)]
    expires_at: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateExternalApiKeyApplicationsRequest {
    application_ids: Vec<String>,
}

#[derive(sqlx::FromRow)]
struct ExternalApiKeyRow {
    id: String,
    name: String,
    status: String,
    expires_at: Option<String>,
    last_used_at: Option<String>,
    created_at: String,
    updated_at: String,
    version: i64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/external-api-keys", get(list).post(create))
        .route("/external-api-keys/{id}", get(show))
        .route("/external-api-keys/{id}/revoke", post(revoke))
        .route(
            "/external-api-keys/{id}/applications",
            put(update_applications),
        )
}

#[utoipa::path(operation_id = "external_api_keys_list", get, path = "/api/v1/external-api-keys", params(("limit" = Option<u32>, Query), ("after" = Option<String>, Query)), responses((status = 200, body = ExternalApiKeyListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(query): Query<pagination::ListQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<ExternalApiKeyListResponse>> {
    actor.require_administrator(request_id.as_str())?;
    let limit = pagination::limit(&query, request_id.as_str())?;
    let (created_at, id) = pagination::decode_after(&query, request_id.as_str())?
        .unwrap_or_else(|| ("0000".to_owned(), "".to_owned()));
    let rows = sqlx::query_as::<_, ExternalApiKeyRow>(
        "SELECT id,name,status,expires_at,last_used_at,created_at,updated_at,version FROM external_api_keys WHERE (created_at>? OR (created_at=? AND id>?)) ORDER BY created_at,id LIMIT ?",
    )
    .bind(&created_at)
    .bind(&created_at)
    .bind(&id)
    .bind((limit + 1) as i64)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let has_more = rows.len() > limit as usize;
    let rows = rows.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor = has_more
        .then(|| rows.last().map(|row| encode_cursor(&row.created_at, &row.id)))
        .flatten();
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(
            with_applications(state.pool(), row, request_id.as_str()).await?,
        );
    }
    Ok(Json(ExternalApiKeyListResponse { items, next_cursor }))
}

#[utoipa::path(operation_id = "external_api_keys_create", post, path = "/api/v1/external-api-keys", params(("X-CSRF-Token" = String, Header)), request_body = CreateExternalApiKeyRequest, responses((status = 201, body = ExternalApiKeyCreatedResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn create(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: axum::http::HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<CreateExternalApiKeyRequest>,
) -> ApiResult<(StatusCode, Json<ExternalApiKeyCreatedResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let name = validate_name(&payload.name, request_id.as_str())?;
    let application_ids = validate_applications(
        state.pool(),
        &payload.application_ids,
        request_id.as_str(),
    )
    .await?;
    let expires_at = validate_expires_at(payload.expires_at.as_deref(), request_id.as_str())?;
    let token = generate_token();
    let key_id = format!("ekey_{}", Ulid::new());
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO external_api_keys(id,name,token_hash,status,expires_at,created_by) VALUES(?,?,?, 'active', ?, ?)")
        .bind(&key_id)
        .bind(&name)
        .bind(token_hash(&token))
        .bind(&expires_at)
        .bind(&actor.id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    for application_id in &application_ids {
        sqlx::query("INSERT INTO external_api_key_applications(api_key_id,application_id,granted_by) VALUES(?,?,?)")
            .bind(&key_id)
            .bind(application_id)
            .bind(&actor.id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        sync_service_grant(&mut transaction, application_id, &actor.id, request_id.as_str())
            .await?;
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "external_api_key.create",
        "external_api_key",
        &key_id,
        request_id.as_str(),
        json!({"name": name, "application_count": application_ids.len()}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let created_at = Utc::now().to_rfc3339();
    Ok((
        StatusCode::CREATED,
        Json(ExternalApiKeyCreatedResponse {
            id: key_id,
            name,
            token,
            status: "active".to_owned(),
            expires_at,
            application_ids,
            created_at,
        }),
    ))
}

#[utoipa::path(operation_id = "external_api_keys_show", get, path = "/api/v1/external-api-keys/{id}", params(("id" = String, Path)), responses((status = 200, body = ExternalApiKeySummary), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<ExternalApiKeySummary>> {
    actor.require_administrator(request_id.as_str())?;
    let row = find_row(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(
        with_applications(state.pool(), row, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "external_api_keys_revoke", post, path = "/api/v1/external-api-keys/{id}/revoke", params(("id" = String, Path), ("X-CSRF-Token" = String, Header)), responses((status = 200, body = ExternalApiKeySummary), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn revoke(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: axum::http::HeaderMap,
    actor: AuthUser,
) -> ApiResult<Json<ExternalApiKeySummary>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let updated = sqlx::query("UPDATE external_api_keys SET status='disabled',updated_at=?,version=version+1 WHERE id=? AND status='active'")
        .bind(Utc::now().to_rfc3339())
        .bind(&id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::not_found(request_id.as_str()));
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "external_api_key.revoke",
        "external_api_key",
        &id,
        request_id.as_str(),
        json!({}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let row = find_row(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(
        with_applications(state.pool(), row, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "external_api_keys_update_applications", put, path = "/api/v1/external-api-keys/{id}/applications", params(("id" = String, Path), ("X-CSRF-Token" = String, Header)), request_body = UpdateExternalApiKeyApplicationsRequest, responses((status = 200, body = ExternalApiKeySummary), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_applications(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: axum::http::HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<UpdateExternalApiKeyApplicationsRequest>,
) -> ApiResult<Json<ExternalApiKeySummary>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let application_ids = validate_applications(
        state.pool(),
        &payload.application_ids,
        request_id.as_str(),
    )
    .await?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM external_api_keys WHERE id=?)")
        .bind(&id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if !exists {
        return Err(ApiError::not_found(request_id.as_str()));
    }
    sqlx::query("DELETE FROM external_api_key_applications WHERE api_key_id=?")
        .bind(&id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    for application_id in &application_ids {
        sqlx::query("INSERT INTO external_api_key_applications(api_key_id,application_id,granted_by) VALUES(?,?,?)")
            .bind(&id)
            .bind(application_id)
            .bind(&actor.id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        sync_service_grant(&mut transaction, application_id, &actor.id, request_id.as_str())
            .await?;
    }
    sqlx::query("UPDATE external_api_keys SET updated_at=?,version=version+1 WHERE id=?")
        .bind(Utc::now().to_rfc3339())
        .bind(&id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "external_api_key.applications.update",
        "external_api_key",
        &id,
        request_id.as_str(),
        json!({"application_count": application_ids.len()}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let row = find_row(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(
        with_applications(state.pool(), row, request_id.as_str()).await?,
    ))
}

async fn find_row(
    pool: &SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<ExternalApiKeyRow> {
    sqlx::query_as::<_, ExternalApiKeyRow>(
        "SELECT id,name,status,expires_at,last_used_at,created_at,updated_at,version FROM external_api_keys WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| ApiError::not_found(request_id))
}

async fn with_applications(
    pool: &SqlitePool,
    row: ExternalApiKeyRow,
    request_id: &str,
) -> ApiResult<ExternalApiKeySummary> {
    let application_ids: Vec<String> = sqlx::query_scalar(
        "SELECT application_id FROM external_api_key_applications WHERE api_key_id=? ORDER BY application_id",
    )
    .bind(&row.id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    Ok(ExternalApiKeySummary {
        id: row.id,
        name: row.name,
        status: row.status,
        expires_at: row.expires_at,
        last_used_at: row.last_used_at,
        application_ids,
        created_at: row.created_at,
        updated_at: row.updated_at,
        version: row.version,
    })
}

async fn sync_service_grant(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    application_id: &str,
    granted_by: &str,
    request_id: &str,
) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO user_application_grants(user_id,application_id,granted_by) VALUES(?,?,?) ON CONFLICT(user_id,application_id) DO NOTHING",
    )
    .bind(SERVICE_USER_ID)
    .bind(application_id)
    .bind(granted_by)
    .execute(&mut **transaction)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    Ok(())
}

async fn validate_applications(
    pool: &SqlitePool,
    application_ids: &[String],
    request_id: &str,
) -> ApiResult<Vec<String>> {
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::with_capacity(application_ids.len());
    for application_id in application_ids {
        if application_id.is_empty()
            || application_id.len() > 128
            || application_id.chars().any(char::is_control)
            || !seen.insert(application_id.clone())
        {
            return Err(ApiError::validation(
                "应用 ID 列表格式不正确",
                request_id,
            ));
        }
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM applications WHERE id=? AND status='active')")
            .bind(application_id)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
        if !exists {
            return Err(ApiError::not_found(request_id));
        }
        normalized.push(application_id.clone());
    }
    Ok(normalized)
}

fn validate_name(value: &str, request_id: &str) -> ApiResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 128 || value.chars().any(char::is_control) {
        return Err(ApiError::validation("Key 名称格式不正确", request_id));
    }
    Ok(value.to_owned())
}

fn validate_expires_at(value: Option<&str>, request_id: &str) -> ApiResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let expires_at = DateTime::parse_from_rfc3339(value)
        .map_err(|_| ApiError::validation("expires_at 必须是 RFC3339 时间", request_id))?;
    if expires_at.with_timezone(&Utc) <= Utc::now() {
        return Err(ApiError::validation(
            "expires_at 必须晚于当前时间",
            request_id,
        ));
    }
    Ok(Some(expires_at.with_timezone(&Utc).to_rfc3339()))
}

fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    format!("{TOKEN_PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes))
}

pub(crate) fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn encode_cursor(created_at: &str, id: &str) -> String {
    URL_SAFE_NO_PAD.encode(format!("{created_at}\0{id}"))
}

#[cfg(test)]
mod tests {
    use super::{TOKEN_PREFIX, token_hash};

    #[test]
    fn token_hash_is_deterministic_and_prefix_is_stable() {
        let token = format!("{TOKEN_PREFIX}abc");
        assert_eq!(token_hash(&token), token_hash(&token));
        assert_ne!(token_hash(&token), token_hash("other"));
    }
}
