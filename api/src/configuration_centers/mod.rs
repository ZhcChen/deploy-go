use axum::{
    Json, Router,
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ulid::Ulid;
use url::Url;
use utoipa::ToSchema;
use zeroize::Zeroizing;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    crypto::{ETCD_ADMIN_ALGORITHM, EncryptedSecret, MasterKeyRing},
    error::{ApiError, ApiResult},
};

const PLATFORM_ID: &str = "platform";
const MAX_ENDPOINTS: usize = 8;
const MAX_ENDPOINT_LENGTH: usize = 256;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PlatformConfigurationCenterResponse {
    pub provider: &'static str,
    pub endpoints: Vec<String>,
    pub username: String,
    pub password_configured: bool,
    pub status: String,
    pub checked_at: Option<String>,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavePlatformConfigurationCenterRequest {
    endpoints: Vec<String>,
    username: String,
    password: Option<String>,
    version: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeletePlatformConfigurationCenterRequest {
    version: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct PlatformRow {
    endpoints_json: String,
    username: String,
    status: String,
    last_checked_at: Option<String>,
    updated_at: String,
    version: i64,
    encrypted_password: Vec<u8>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/configuration-centers/platform",
        get(show_platform)
            .put(save_platform)
            .delete(delete_platform),
    )
}

#[utoipa::path(
    operation_id = "configuration_centers_platform_show",
    get,
    path = "/api/v1/configuration-centers/platform",
    responses(
        (status = 200, body = PlatformConfigurationCenterResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse)
    )
)]
pub(crate) async fn show_platform(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<PlatformConfigurationCenterResponse>> {
    actor.require_administrator(request_id.as_str())?;
    let row = sqlx::query_as::<_, PlatformRow>(
        "SELECT configuration_centers.endpoints_json, configuration_centers.username, configuration_centers.status, configuration_centers.last_checked_at, configuration_centers.updated_at, configuration_centers.version, configuration_center_credentials.ciphertext AS encrypted_password FROM configuration_centers JOIN configuration_center_credentials ON configuration_center_credentials.id = configuration_centers.credential_id WHERE configuration_centers.id = ? AND configuration_centers.scope = 'platform'",
    )
    .bind(PLATFORM_ID)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;

    Ok(Json(platform_response(row, request_id.as_str())?))
}

#[utoipa::path(
    operation_id = "configuration_centers_platform_save",
    put,
    path = "/api/v1/configuration-centers/platform",
    request_body = SavePlatformConfigurationCenterRequest,
    responses(
        (status = 200, body = PlatformConfigurationCenterResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 409, body = crate::error::ErrorResponse),
        (status = 422, body = crate::error::ErrorResponse)
    )
)]
pub(crate) async fn save_platform(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<SavePlatformConfigurationCenterRequest>,
) -> ApiResult<Json<PlatformConfigurationCenterResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let endpoints = normalize_endpoints(&payload.endpoints, request_id.as_str())?;
    let username = validate_username(&payload.username, request_id.as_str())?;
    let password = validate_password(payload.password.as_deref(), request_id.as_str())?;
    let password_updated = password.is_some();
    if payload.version < 0 {
        return Err(ApiError::validation(
            "配置中心版本必须为非负整数",
            request_id.as_str(),
        ));
    }

    let existing = sqlx::query_as::<_, PlatformRow>(
        "SELECT configuration_centers.endpoints_json, configuration_centers.username, configuration_centers.status, configuration_centers.last_checked_at, configuration_centers.updated_at, configuration_centers.version, configuration_center_credentials.ciphertext AS encrypted_password FROM configuration_centers JOIN configuration_center_credentials ON configuration_center_credentials.id = configuration_centers.credential_id WHERE configuration_centers.id = ? AND configuration_centers.scope = 'platform'",
    )
    .bind(PLATFORM_ID)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if let Some(row) = existing.as_ref() {
        if payload.version != row.version {
            return Err(ApiError::conflict(
                "resource_version_conflict",
                "平台配置中心已经被其他请求修改",
                request_id.as_str(),
            ));
        }
        if row.status == "retired" && password.is_none() {
            return Err(ApiError::validation(
                "重新启用平台配置中心时必须提供密码",
                request_id.as_str(),
            ));
        }
    } else if payload.version != 0 {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "平台配置中心尚未配置",
            request_id.as_str(),
        ));
    }

    let ring = if password.is_some() {
        Some(state.master_key_ring().ok_or_else(|| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "master_key_not_configured",
                "服务未配置主密钥，无法保存配置中心密码",
                request_id.as_str(),
            )
        })?)
    } else {
        None
    };
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;

    let credential_id = if let Some(password) = password {
        let credential_id = format!("cc_cred_{}", Ulid::new());
        let encrypted = ring
            .ok_or_else(|| ApiError::internal(request_id.as_str()))?
            .encrypt_etcd_admin_credential(&credential_id, password.as_bytes())
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        sqlx::query("INSERT INTO configuration_center_credentials (id, purpose, algorithm, ciphertext, nonce, key_version, status, created_by) VALUES (?, 'platform_admin', ?, ?, ?, ?, 'active', ?)")
            .bind(&credential_id)
            .bind(ETCD_ADMIN_ALGORITHM)
            .bind(&encrypted.ciphertext)
            .bind(&encrypted.nonce)
            .bind(encrypted.key_version)
            .bind(&actor.id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        credential_id
    } else {
        existing
            .as_ref()
            .ok_or_else(|| ApiError::validation("首次配置必须提供密码", request_id.as_str()))?;
        String::new()
    };
    let credential_id = if credential_id.is_empty() {
        sqlx::query_scalar::<_, String>(
            "SELECT credential_id FROM configuration_centers WHERE id = ?",
        )
        .bind(PLATFORM_ID)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?
    } else {
        credential_id
    };
    let now = Utc::now().to_rfc3339();
    let endpoints_json =
        serde_json::to_string(&endpoints).map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = if existing.is_some() {
        sqlx::query("UPDATE configuration_centers SET endpoints_json = ?, username = ?, status = 'unchecked', last_error_code = NULL, last_checked_at = NULL, credential_id = ?, updated_at = ?, version = version + 1 WHERE id = ? AND version = ?")
            .bind(&endpoints_json)
            .bind(username)
            .bind(&credential_id)
            .bind(&now)
            .bind(PLATFORM_ID)
            .bind(payload.version)
            .execute(&mut *transaction)
            .await
    } else {
        sqlx::query("INSERT INTO configuration_centers (id, center_type, scope, endpoints_json, username, credential_id, status, created_by) VALUES (?, 'etcd', 'platform', ?, ?, ?, 'unchecked', ?)")
            .bind(PLATFORM_ID)
            .bind(&endpoints_json)
            .bind(username)
            .bind(&credential_id)
            .bind(&actor.id)
            .execute(&mut *transaction)
            .await
    };
    let result = result.map_err(|error| map_save_error(error, request_id.as_str()))?;
    if result.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "平台配置中心已经被其他请求修改",
            request_id.as_str(),
        ));
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "configuration_center.platform.save",
        "configuration_center",
        PLATFORM_ID,
        request_id.as_str(),
        json!({"provider":"etcd","endpoint_count":endpoints.len(),"username":username,"password_updated":password_updated}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;

    Ok(Json(
        show_platform_value(state.pool(), request_id.as_str()).await?,
    ))
}

#[utoipa::path(
    operation_id = "configuration_centers_platform_delete",
    delete,
    path = "/api/v1/configuration-centers/platform",
    request_body = DeletePlatformConfigurationCenterRequest,
    responses(
        (status = 200, body = PlatformConfigurationCenterResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 409, body = crate::error::ErrorResponse),
        (status = 422, body = crate::error::ErrorResponse)
    )
)]
pub(crate) async fn delete_platform(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<DeletePlatformConfigurationCenterRequest>,
) -> ApiResult<Json<PlatformConfigurationCenterResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    if payload.version < 1 {
        return Err(ApiError::validation(
            "已配置的平台版本必须为正整数",
            request_id.as_str(),
        ));
    }
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE configuration_centers SET status = 'retired', last_error_code = NULL, last_checked_at = NULL, updated_at = ?, version = version + 1 WHERE id = ? AND scope = 'platform' AND status <> 'retired' AND version = ?")
        .bind(&now)
        .bind(PLATFORM_ID)
        .bind(payload.version)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() == 0 {
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM configuration_centers WHERE id = ? AND scope = 'platform'",
        )
        .bind(PLATFORM_ID)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
        return Err(if exists.is_some() {
            ApiError::conflict(
                "resource_version_conflict",
                "平台配置中心已经被其他请求修改",
                request_id.as_str(),
            )
        } else {
            ApiError::not_found(request_id.as_str())
        });
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "configuration_center.platform.delete",
        "configuration_center",
        PLATFORM_ID,
        request_id.as_str(),
        json!({"status":"retired"}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(
        show_platform_value(state.pool(), request_id.as_str()).await?,
    ))
}

async fn show_platform_value(
    pool: &sqlx::SqlitePool,
    request_id: &str,
) -> ApiResult<PlatformConfigurationCenterResponse> {
    let row = sqlx::query_as::<_, PlatformRow>(
        "SELECT configuration_centers.endpoints_json, configuration_centers.username, configuration_centers.status, configuration_centers.last_checked_at, configuration_centers.updated_at, configuration_centers.version, configuration_center_credentials.ciphertext AS encrypted_password FROM configuration_centers JOIN configuration_center_credentials ON configuration_center_credentials.id = configuration_centers.credential_id WHERE configuration_centers.id = ? AND configuration_centers.scope = 'platform'",
    )
    .bind(PLATFORM_ID)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    platform_response(row, request_id)
}

fn platform_response(
    row: Option<PlatformRow>,
    request_id: &str,
) -> ApiResult<PlatformConfigurationCenterResponse> {
    let Some(row) = row else {
        return Ok(PlatformConfigurationCenterResponse {
            provider: "etcd",
            endpoints: Vec::new(),
            username: String::new(),
            password_configured: false,
            status: "unconfigured".to_owned(),
            checked_at: None,
            updated_at: String::new(),
            version: 0,
        });
    };
    let endpoints = serde_json::from_str::<Vec<String>>(&row.endpoints_json)
        .map_err(|_| ApiError::internal(request_id))?;
    Ok(PlatformConfigurationCenterResponse {
        provider: "etcd",
        endpoints,
        username: row.username,
        password_configured: row.status != "retired" && !row.encrypted_password.is_empty(),
        status: if row.status == "retired" {
            "unconfigured".to_owned()
        } else {
            row.status
        },
        checked_at: row.last_checked_at,
        updated_at: row.updated_at,
        version: row.version,
    })
}

fn normalize_endpoints(values: &[String], request_id: &str) -> ApiResult<Vec<String>> {
    if values.is_empty() || values.len() > MAX_ENDPOINTS {
        return Err(ApiError::validation(
            "配置中心 Endpoint 数量必须介于 1 和 8 之间",
            request_id,
        ));
    }
    let mut normalized = Vec::with_capacity(values.len());
    for value in values {
        let value = value.trim();
        if value.is_empty() || value.len() > MAX_ENDPOINT_LENGTH {
            return Err(ApiError::validation(
                "配置中心 Endpoint 格式不正确",
                request_id,
            ));
        }
        let parsed = Url::parse(value)
            .map_err(|_| ApiError::validation("配置中心 Endpoint 格式不正确", request_id))?;
        if parsed.scheme() != "http"
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || (parsed.path() != "" && parsed.path() != "/")
        {
            return Err(ApiError::validation(
                "配置中心 Endpoint 必须是无认证信息的 http://host:port",
                request_id,
            ));
        }
        let host = parsed.host_str().unwrap_or_default();
        let port = parsed.port().unwrap_or(2379);
        let host = if host.contains(':') {
            format!("[{host}]")
        } else {
            host.to_owned()
        };
        normalized.push(format!("http://{host}:{port}"));
    }
    normalized.sort();
    normalized.dedup();
    if normalized.is_empty() {
        return Err(ApiError::validation(
            "配置中心 Endpoint 不能为空",
            request_id,
        ));
    }
    Ok(normalized)
}

fn validate_username<'a>(username: &'a str, request_id: &str) -> ApiResult<&'a str> {
    let username = username.trim();
    if username.is_empty()
        || username.chars().count() > 128
        || username.chars().any(char::is_control)
    {
        return Err(ApiError::validation("配置中心用户名格式不正确", request_id));
    }
    Ok(username)
}

fn validate_password<'a>(
    password: Option<&'a str>,
    request_id: &str,
) -> ApiResult<Option<&'a str>> {
    let Some(password) = password else {
        return Ok(None);
    };
    if password.is_empty() || password.len() > 4096 || password.chars().any(char::is_control) {
        return Err(ApiError::validation("配置中心密码格式不正确", request_id));
    }
    Ok(Some(password))
}

fn map_save_error(error: sqlx::Error, request_id: &str) -> ApiError {
    if error.to_string().contains("UNIQUE constraint failed") {
        ApiError::conflict(
            "platform_etcd_already_bound",
            "平台 etcd 已存在生效配置",
            request_id,
        )
    } else {
        ApiError::internal(request_id)
    }
}

pub async fn reencrypt_all(pool: &sqlx::SqlitePool, ring: &MasterKeyRing) -> anyhow::Result<u64> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        purpose: String,
        algorithm: String,
        ciphertext: Vec<u8>,
        nonce: Vec<u8>,
        key_version: i64,
    }

    let mut migrated = 0_u64;
    loop {
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, purpose, algorithm, ciphertext, nonce, key_version FROM configuration_center_credentials WHERE key_version != ? ORDER BY id LIMIT 100",
        )
        .bind(ring.current_version())
        .fetch_all(pool)
        .await?;
        if rows.is_empty() {
            return Ok(migrated);
        }
        for row in rows {
            let encrypted = EncryptedSecret {
                ciphertext: row.ciphertext,
                nonce: row.nonce,
                key_version: row.key_version,
            };
            let plaintext =
                decrypt_by_purpose(ring, &row.purpose, &row.id, &row.algorithm, &encrypted)?;
            let replacement = encrypt_by_purpose(
                ring,
                &row.purpose,
                &row.id,
                &row.algorithm,
                plaintext.as_slice(),
            )?;
            let result = sqlx::query("UPDATE configuration_center_credentials SET ciphertext = ?, nonce = ?, key_version = ?, updated_at = ?, version = version + 1 WHERE id = ? AND key_version = ?")
                .bind(&replacement.ciphertext)
                .bind(&replacement.nonce)
                .bind(replacement.key_version)
                .bind(Utc::now().to_rfc3339())
                .bind(&row.id)
                .bind(row.key_version)
                .execute(pool)
                .await?;
            migrated += result.rows_affected();
        }
    }
}

fn decrypt_by_purpose(
    ring: &MasterKeyRing,
    purpose: &str,
    id: &str,
    algorithm: &str,
    encrypted: &EncryptedSecret,
) -> anyhow::Result<Zeroizing<Vec<u8>>> {
    match (purpose, algorithm) {
        ("platform_admin", ETCD_ADMIN_ALGORITHM) => {
            Ok(ring.decrypt_etcd_admin_credential(id, encrypted)?)
        }
        ("custom_connection", "chacha20poly1305-etcd-custom-v1") => {
            Ok(ring.decrypt_etcd_custom_credential(id, encrypted)?)
        }
        ("business_identity", "chacha20poly1305-etcd-business-v1") => {
            Ok(ring.decrypt_etcd_business_credential(id, encrypted)?)
        }
        _ => Err(anyhow::anyhow!("配置中心凭据用途或算法无效")),
    }
}

fn encrypt_by_purpose(
    ring: &MasterKeyRing,
    purpose: &str,
    id: &str,
    algorithm: &str,
    plaintext: &[u8],
) -> anyhow::Result<EncryptedSecret> {
    match (purpose, algorithm) {
        ("platform_admin", ETCD_ADMIN_ALGORITHM) => {
            Ok(ring.encrypt_etcd_admin_credential(id, plaintext)?)
        }
        ("custom_connection", "chacha20poly1305-etcd-custom-v1") => {
            Ok(ring.encrypt_etcd_custom_credential(id, plaintext)?)
        }
        ("business_identity", "chacha20poly1305-etcd-business-v1") => {
            Ok(ring.encrypt_etcd_business_credential(id, plaintext)?)
        }
        _ => Err(anyhow::anyhow!("配置中心凭据用途或算法无效")),
    }
}
