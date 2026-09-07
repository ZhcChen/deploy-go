use axum::{
    Json, Router,
    extract::{Extension, State},
    http::HeaderMap,
    routing::post,
};
use chrono::Utc;
use deploy_go_agent_protocol::{RuntimeProbeTask, RuntimeProbeType};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId,
    agents::dispatcher,
    audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    execution_spec,
};

const MAX_PROBE_APPLICATIONS: usize = 20;

const SKIP_ARCHIVED_CODE: &str = "runtime_probe_archived";
const SKIP_ARCHIVED_MESSAGE: &str = "应用已归档，不发起运行探测";
const SKIP_NO_TARGET_CODE: &str = "runtime_probe_no_active_target";
const SKIP_NO_TARGET_MESSAGE: &str = "应用没有可用的启用目标，无法发起平台运行探测";
const SKIP_MULTI_TARGET_CODE: &str = "runtime_probe_multi_target";
const SKIP_MULTI_TARGET_MESSAGE: &str =
    "应用存在多个部署目标（多模块），平台运行探测只支持单目标应用";
const SKIP_UNSUPPORTED_TYPE_CODE: &str = "runtime_probe_config_unsupported";
const SKIP_UNSUPPORTED_TYPE_MESSAGE: &str = "部署契约未配置 HTTP/TCP 运行探测，沿用部署验证结果";
const SKIP_MISSING_PORT_CODE: &str = "runtime_probe_config_missing_port";
const SKIP_MISSING_PORT_MESSAGE: &str = "部署契约未配置探测端口，沿用部署验证结果";
const FAIL_TARGET_UNAVAILABLE_CODE: &str = "runtime_probe_target_unavailable";
const FAIL_TARGET_UNAVAILABLE_MESSAGE: &str = "目标节点或 Agent 当前不可用";

#[derive(Clone, Serialize, ToSchema)]
pub struct RuntimeProbeBatchResponse {
    items: Vec<RuntimeProbeItemResponse>,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct RuntimeProbeItemResponse {
    application_id: String,
    /// queued | in_progress | failed | skipped
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime_status_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeProbeBatchRequest {
    application_ids: Vec<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/applications/runtime-probes", post(probe))
}

#[utoipa::path(operation_id = "applications_runtime_probes_request", post, path = "/api/v1/applications/runtime-probes", request_body = RuntimeProbeBatchRequest, responses((status = 200, body = RuntimeProbeBatchResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn probe(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<RuntimeProbeBatchRequest>,
) -> ApiResult<Json<RuntimeProbeBatchResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    if payload.application_ids.is_empty()
        || payload.application_ids.len() > MAX_PROBE_APPLICATIONS
        || payload
            .application_ids
            .iter()
            .any(|id| !valid_application_id(id))
    {
        return Err(ApiError::validation(
            "一次最多为 20 个有效应用发起运行探测",
            request_id.as_str(),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::with_capacity(payload.application_ids.len());
    for application_id in &payload.application_ids {
        if !seen.insert(application_id.clone()) {
            continue;
        }
        items.push(probe_one(&state, application_id, &actor, request_id.as_str()).await?);
    }
    Ok(Json(RuntimeProbeBatchResponse { items }))
}

fn valid_application_id(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

async fn probe_one(
    state: &AppState,
    application_id: &str,
    actor: &AuthUser,
    request_id: &str,
) -> ApiResult<RuntimeProbeItemResponse> {
    let application: Option<(String, String)> =
        sqlx::query_as("SELECT status,verification_config FROM applications WHERE id=?")
            .bind(application_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    let Some((application_status, verification_config_json)) = application else {
        return Ok(skipped(
            application_id,
            "runtime_probe_application_not_found",
            "应用不存在",
        ));
    };
    if application_status == "archived" {
        return Ok(skipped(
            application_id,
            SKIP_ARCHIVED_CODE,
            SKIP_ARCHIVED_MESSAGE,
        ));
    }

    let active_target_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment_targets target JOIN nodes node ON node.id=target.node_id WHERE target.application_id=? AND target.status='active' AND node.archived_at IS NULL",
    )
    .bind(application_id)
    .fetch_one(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if active_target_count == 0 {
        return Ok(skipped(
            application_id,
            SKIP_NO_TARGET_CODE,
            SKIP_NO_TARGET_MESSAGE,
        ));
    }
    if active_target_count > 1 {
        return Ok(skipped(
            application_id,
            SKIP_MULTI_TARGET_CODE,
            SKIP_MULTI_TARGET_MESSAGE,
        ));
    }

    let row: Option<RuntimeProbeTargetRow> = sqlx::query_as(
        "SELECT target.id AS target_id,target.image_spec_json,node.status AS node_status,agent.id AS agent_id,agent.protocol_version,agent.capabilities_json FROM deployment_targets target JOIN nodes node ON node.id=target.node_id LEFT JOIN agents agent ON agent.node_id=target.node_id AND agent.revoked_at IS NULL AND agent.archived_at IS NULL WHERE target.application_id=? AND target.status='active' AND node.archived_at IS NULL",
    )
    .bind(application_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let Some(target) = row else {
        return Ok(skipped(
            application_id,
            SKIP_NO_TARGET_CODE,
            SKIP_NO_TARGET_MESSAGE,
        ));
    };

    let verification_config: Value = serde_json::from_str(&verification_config_json)
        .map_err(|_| ApiError::internal(request_id))?;
    let image_spec = target
        .image_spec_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<Value>(value).ok());
    let Some(plan) = probe_plan(&verification_config, image_spec.as_ref()) else {
        let (error_code, error_message) = probe_plan_skip_reason(&verification_config);
        return Ok(skipped(application_id, error_code, error_message));
    };

    if let Some((runtime_status_id, status)) =
        in_flight_status(state.pool(), &target.target_id, request_id).await?
        && matches!(status.as_str(), "pending" | "running")
    {
        return Ok(RuntimeProbeItemResponse {
            application_id: application_id.to_owned(),
            status: "in_progress".to_owned(),
            runtime_status_id: Some(runtime_status_id),
            error_code: None,
            error_message: None,
        });
    }

    let agent_ready = target.agent_id.as_deref().is_some_and(|_| {
        target.node_status == "online"
            && dispatcher::runtime_probe_compatibility(
                target.protocol_version,
                target.capabilities_json.as_deref(),
            )
            .is_ok()
    });
    if agent_ready {
        let runtime_status_id = insert_status(
            state,
            application_id,
            &target.target_id,
            actor,
            "pending",
            None,
            request_id,
        )
        .await?;
        let enqueue = dispatcher::enqueue_runtime_probe_task(
            state,
            target
                .agent_id
                .as_deref()
                .expect("agent_ready 已保证 Agent 存在"),
            RuntimeProbeTask {
                runtime_status_id: runtime_status_id.clone(),
                probe_type: plan.probe_type,
                port: plan.port,
                path: plan.path.clone(),
                expected_status: plan.expected_status,
                timeout_ms: plan.timeout_ms,
            },
        )
        .await;
        if let Err(error) = enqueue {
            dispatcher::mark_runtime_probe_failed(
                state,
                &runtime_status_id,
                "runtime_probe_dispatch_failed",
                "运行探测任务下发失败",
            )
            .await?;
            return Err(error);
        }
        return Ok(RuntimeProbeItemResponse {
            application_id: application_id.to_owned(),
            status: "queued".to_owned(),
            runtime_status_id: Some(runtime_status_id),
            error_code: None,
            error_message: None,
        });
    }

    let (error_code, error_message) = if target.agent_id.is_none() || target.node_status != "online"
    {
        (
            FAIL_TARGET_UNAVAILABLE_CODE.to_owned(),
            FAIL_TARGET_UNAVAILABLE_MESSAGE.to_owned(),
        )
    } else {
        let (code, message) = dispatcher::runtime_probe_compatibility(
            target.protocol_version,
            target.capabilities_json.as_deref(),
        )
        .expect_err("agent_ready=false 且 Agent 存在时应返回兼容性原因");
        (code.to_owned(), message.to_owned())
    };
    let runtime_status_id = insert_status(
        state,
        application_id,
        &target.target_id,
        actor,
        "failed",
        Some((&error_code, &error_message)),
        request_id,
    )
    .await?;
    Ok(RuntimeProbeItemResponse {
        application_id: application_id.to_owned(),
        status: "failed".to_owned(),
        runtime_status_id: Some(runtime_status_id),
        error_code: Some(error_code),
        error_message: Some(error_message),
    })
}

#[derive(sqlx::FromRow)]
struct RuntimeProbeTargetRow {
    target_id: String,
    image_spec_json: Option<String>,
    node_status: String,
    agent_id: Option<String>,
    protocol_version: Option<i64>,
    capabilities_json: Option<String>,
}

#[derive(Clone)]
struct ProbePlan {
    probe_type: RuntimeProbeType,
    port: u16,
    path: Option<String>,
    expected_status: Option<u16>,
    timeout_ms: u32,
}

fn probe_plan(config: &Value, image_spec: Option<&Value>) -> Option<ProbePlan> {
    let object = config.as_object()?;
    match object.get("type").and_then(Value::as_str) {
        Some("http") => {
            let port = execution_spec::resolve_http_probe_port(config, image_spec)?;
            let path = object.get("path").and_then(Value::as_str)?;
            let expected_status = object
                .get("expected_status")
                .and_then(Value::as_u64)
                .and_then(|status| u16::try_from(status).ok())?;
            let timeout_ms = object
                .get("timeout_ms")
                .and_then(Value::as_u64)
                .and_then(|timeout| u32::try_from(timeout).ok())?;
            if !path.starts_with('/') || path.is_empty() || path.contains(char::is_control) {
                return None;
            }
            Some(ProbePlan {
                probe_type: RuntimeProbeType::Http,
                port,
                path: Some(path.to_owned()),
                expected_status: Some(expected_status),
                timeout_ms,
            })
        }
        Some("tcp") => {
            let port = object
                .get("port")
                .and_then(Value::as_u64)
                .and_then(|port| u16::try_from(port).ok())?;
            let timeout_ms = object
                .get("timeout_ms")
                .and_then(Value::as_u64)
                .and_then(|timeout| u32::try_from(timeout).ok())?;
            Some(ProbePlan {
                probe_type: RuntimeProbeType::Tcp,
                port,
                path: None,
                expected_status: None,
                timeout_ms,
            })
        }
        _ => None,
    }
}

fn probe_plan_skip_reason(config: &Value) -> (&'static str, &'static str) {
    match config.get("type").and_then(Value::as_str) {
        Some("http") => (SKIP_MISSING_PORT_CODE, SKIP_MISSING_PORT_MESSAGE),
        Some(_) | None => (SKIP_UNSUPPORTED_TYPE_CODE, SKIP_UNSUPPORTED_TYPE_MESSAGE),
    }
}

fn skipped(
    application_id: &str,
    error_code: &str,
    error_message: &str,
) -> RuntimeProbeItemResponse {
    RuntimeProbeItemResponse {
        application_id: application_id.to_owned(),
        status: "skipped".to_owned(),
        runtime_status_id: None,
        error_code: Some(error_code.to_owned()),
        error_message: Some(error_message.to_owned()),
    }
}

async fn in_flight_status(
    pool: &SqlitePool,
    target_id: &str,
    request_id: &str,
) -> ApiResult<Option<(String, String)>> {
    sqlx::query_as(
        "SELECT runtime_status.runtime_status_id,runtime_status.status FROM application_runtime_statuses runtime_status WHERE runtime_status.target_id=? AND (runtime_status.status IN ('succeeded','failed') OR EXISTS (SELECT 1 FROM agent_tasks task WHERE task.runtime_status_id=runtime_status.runtime_status_id AND task.status IN ('queued','delivered','accepted','running','canceling'))) ORDER BY runtime_status.created_at DESC,runtime_status.runtime_status_id DESC LIMIT 1",
    )
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))
}

async fn insert_status(
    state: &AppState,
    application_id: &str,
    target_id: &str,
    actor: &AuthUser,
    status: &str,
    error: Option<(&str, &str)>,
    request_id: &str,
) -> ApiResult<String> {
    let runtime_status_id = format!("runtime_status_{}", Ulid::new());
    let now = Utc::now().to_rfc3339();
    let (error_code, error_message) = match error {
        Some((code, message)) => (Some(code), Some(message)),
        None => (None, None),
    };
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    sqlx::query(
        "INSERT INTO application_runtime_statuses (runtime_status_id,application_id,target_id,status,error_code,error_message,observed_at,requested_by,requested_at,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&runtime_status_id)
    .bind(application_id)
    .bind(target_id)
    .bind(status)
    .bind(error_code)
    .bind(error_message)
    .bind((status == "failed").then_some(now.as_str()))
    .bind(&actor.id)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application.runtime_probe.request",
        "application",
        application_id,
        request_id,
        json!({
            "target_id": target_id,
            "runtime_status_id": runtime_status_id,
            "status": status,
            "error_code": error_code,
            "error_message": error_message,
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok(runtime_status_id)
}
