use axum::{
    Json, Router,
    extract::{Extension, Query, State},
    routing::get,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Sqlite, Transaction};
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId,
    auth::AuthUser,
    error::{ApiError, ApiResult},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuditListQuery {
    limit: Option<u32>,
    after: Option<String>,
    action: Option<String>,
    resource_type: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AuditLogResponse {
    id: String,
    actor_id: Option<String>,
    action: String,
    resource_type: String,
    resource_id: String,
    request_id: String,
    summary: Value,
    created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct AuditLogListResponse {
    items: Vec<AuditLogResponse>,
    next_cursor: Option<String>,
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    actor_id: Option<String>,
    action: String,
    resource_type: String,
    resource_id: String,
    request_id: String,
    summary_json: String,
    created_at: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/audit-logs", get(list))
}

#[utoipa::path(get, path = "/api/v1/audit-logs", params(("limit" = Option<u32>, Query), ("after" = Option<String>, Query), ("action" = Option<String>, Query), ("resource_type" = Option<String>, Query)), responses((status = 200, body = AuditLogListResponse), (status = 401), (status = 403), (status = 422)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(query): Query<AuditListQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<AuditLogListResponse>> {
    actor.require_administrator(request_id.as_str())?;
    let limit = query.limit.unwrap_or(50);
    if !(1..=200).contains(&limit) {
        return Err(ApiError::validation(
            "limit 必须介于 1 和 200",
            request_id.as_str(),
        ));
    }
    for value in [query.action.as_deref(), query.resource_type.as_deref()]
        .into_iter()
        .flatten()
    {
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(ApiError::validation(
                "审计筛选值格式不正确",
                request_id.as_str(),
            ));
        }
    }
    let cursor = query
        .after
        .as_deref()
        .map(decode_cursor)
        .transpose()
        .map_err(|_| ApiError::validation("审计游标格式不正确", request_id.as_str()))?;
    let (created_at, id) =
        cursor.unwrap_or_else(|| ("9999-12-31T23:59:59Z".to_owned(), "~".to_owned()));
    let rows=sqlx::query_as::<_,AuditRow>("SELECT id,actor_id,action,resource_type,resource_id,request_id,summary_json,created_at FROM audit_logs WHERE (created_at<? OR (created_at=? AND id<?)) AND (? IS NULL OR action=?) AND (? IS NULL OR resource_type=?) ORDER BY created_at DESC,id DESC LIMIT ?")
        .bind(&created_at).bind(&created_at).bind(&id).bind(&query.action).bind(&query.action).bind(&query.resource_type).bind(&query.resource_type).bind(i64::from(limit)+1)
        .fetch_all(state.pool()).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
    let has_more = rows.len() > limit as usize;
    let rows = rows.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor = has_more
        .then(|| {
            rows.last()
                .map(|row| encode_cursor(&row.created_at, &row.id))
        })
        .flatten();
    let items = rows
        .into_iter()
        .map(|row| {
            Ok(AuditLogResponse {
                id: row.id,
                actor_id: row.actor_id,
                action: row.action,
                resource_type: row.resource_type,
                resource_id: row.resource_id,
                request_id: row.request_id,
                summary: serde_json::from_str(&row.summary_json)
                    .map_err(|_| ApiError::internal(request_id.as_str()))?,
                created_at: row.created_at,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(Json(AuditLogListResponse { items, next_cursor }))
}

fn encode_cursor(created_at: &str, id: &str) -> String {
    URL_SAFE_NO_PAD.encode(format!("{created_at}\0{id}"))
}
fn decode_cursor(value: &str) -> Result<(String, String), ()> {
    let value =
        String::from_utf8(URL_SAFE_NO_PAD.decode(value).map_err(|_| ())?).map_err(|_| ())?;
    let (created, id) = value.split_once('\0').ok_or(())?;
    if created.is_empty() || id.is_empty() || id.contains('\0') {
        return Err(());
    }
    Ok((created.to_owned(), id.to_owned()))
}

pub async fn record(
    transaction: &mut Transaction<'_, Sqlite>,
    actor_id: Option<&str>,
    action: &str,
    resource_type: &str,
    resource_id: &str,
    request_id: &str,
    summary: Value,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO audit_logs (id, actor_id, action, resource_type, resource_id, request_id, summary_json) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(format!("aud_{}", Ulid::new()))
    .bind(actor_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(request_id)
    .bind(summary.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
