use axum::{
    Json, Router,
    extract::{Extension, FromRequestParts, Path, Query, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION, request::Parts},
    routing::{get, patch, post, put},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashSet;
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::{
    AppState, RequestId, application_envs, application_workspace_sources,
    auth::service_actor,
    deployment_targets, deployments,
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
    environment: String,
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
}

#[derive(Serialize, ToSchema)]
pub struct ExternalApplicationDetail {
    id: String,
    name: String,
    slug: String,
    description: String,
    app_type: String,
    type_version: String,
    environment: String,
    status: String,
    tags: Vec<String>,
    parameter_schema: serde_json::Value,
    verification_config: serde_json::Value,
    version: i64,
    targets: Vec<ExternalDeploymentTarget>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalApplicationCreateRequest {
    name: String,
    slug: String,
    #[serde(default)]
    description: String,
    #[serde(default = "crate::applications::default_app_type")]
    app_type: String,
    #[serde(default = "crate::applications::default_type_version")]
    type_version: String,
    environment: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    parameter_schema: Option<serde_json::Value>,
    #[serde(default)]
    verification_config: Option<serde_json::Value>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalApplicationUpdateRequest {
    version: i64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    environment: Option<String>,
    #[serde(default)]
    app_type: Option<String>,
    #[serde(default)]
    type_version: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    parameter_schema: Option<serde_json::Value>,
    #[serde(default)]
    verification_config: Option<serde_json::Value>,
}

/// 对外 Env 文件元数据。只返回版本、摘要与同步统计，永不返回明文。
#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct ExternalEnvFile {
    id: String,
    application_id: String,
    file_name: String,
    module: String,
    format: String,
    current_version: i64,
    current_digest: String,
    declared_at: String,
    updated_at: String,
    version: i64,
    target_count: i64,
    pending_count: i64,
    syncing_count: i64,
    succeeded_count: i64,
    failed_count: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalEnvFileListResponse {
    items: Vec<ExternalEnvFile>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalEnvRegistrationResponse {
    created: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalEnvInspectionRequest {
    #[schema(schema_with = inspection_keys_schema)]
    keys: Vec<String>,
}

fn inspection_keys_schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
    utoipa::openapi::schema::ArrayBuilder::new()
        .items(
            utoipa::openapi::ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::Type::String)
                .pattern(Some("^[A-Za-z_][A-Za-z0-9_]*$")),
        )
        .min_items(Some(1))
        .max_items(Some(50))
        .unique_items(true)
        .build()
        .into()
}

#[derive(Serialize, ToSchema)]
pub struct ExternalEnvKeyInspection {
    key: String,
    exists: bool,
    #[schema(required = true)]
    value_length_bytes: Option<usize>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalEnvInspectionResponse {
    items: Vec<ExternalEnvKeyInspection>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalDeploymentTargetListResponse {
    items: Vec<deployment_targets::DeploymentTargetResponse>,
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
    exit_code: Option<i64>,
    failure_stage: Option<String>,
    failure_step: Option<String>,
    logs_available: bool,
    last_log_sequence: Option<i64>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic: Option<ExternalDeploymentDiagnostic>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ExternalDeploymentDiagnostic {
    origin: String,
    error_code: Option<String>,
    summary: Option<String>,
    exit_code: Option<i64>,
    execution_phase: Option<String>,
    failure_stage: Option<String>,
    failure_step: Option<String>,
    protocol_detail: Option<String>,
    logs_available: bool,
    last_log_sequence: Option<i64>,
    tasks: Vec<ExternalDeploymentTaskDiagnostic>,
    truncated: bool,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ExternalDeploymentTaskDiagnostic {
    kind: String,
    execution_phase: String,
    target_id: Option<String>,
    stage: Option<String>,
    status: String,
    origin: String,
    last_event_kind: Option<String>,
    delivered_at: Option<String>,
    acknowledged_at: Option<String>,
    task_started_at: Option<String>,
    task_finished_at: Option<String>,
    error_code: Option<String>,
    summary: Option<String>,
    exit_code: Option<i64>,
    failure_stage: Option<String>,
    failure_step: Option<String>,
    protocol_detail: Option<String>,
    logs_available: bool,
    last_log_sequence: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalDeploymentDiagnosticsResponse {
    deployment_id: String,
    status: String,
    phase: String,
    diagnostic: ExternalDeploymentDiagnostic,
    target_runs: Vec<ExternalDeploymentTargetRun>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalDeploymentLogsQuery {
    #[param(minimum = 0)]
    after: Option<i64>,
    #[schema(minimum = 1, maximum = 200)]
    #[param(minimum = 1, maximum = 200)]
    limit: Option<u32>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalDeploymentLog {
    sequence: i64,
    stage: Option<String>,
    stream: String,
    content: String,
    truncated: bool,
    created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalDeploymentLogsResponse {
    items: Vec<ExternalDeploymentLog>,
    next_after: Option<i64>,
    terminal: bool,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_applications,
        create_application,
        show_application,
        update_application,
        list_env_files,
        register_env_files,
        inspect_env_file,
        update_env_file,
        delete_env_file,
        list_targets,
        create_target,
        update_target,
        update_target_status,
        show_workspace_source,
        save_workspace_source,
        create_deployment,
        show_deployment,
        show_deployment_diagnostics,
        list_deployment_logs,
        cancel_deployment
    ),
    components(schemas(
        crate::error::ErrorResponse,
        ExternalApplicationSummary,
        ExternalApplicationListResponse,
        ExternalDeploymentTarget,
        ExternalApplicationDetail,
        ExternalApplicationCreateRequest,
        ExternalApplicationUpdateRequest,
        ExternalEnvFile,
        ExternalEnvFileListResponse,
        ExternalEnvRegistrationResponse,
        ExternalEnvInspectionRequest,
        ExternalEnvKeyInspection,
        ExternalEnvInspectionResponse,
        ExternalDeploymentTargetListResponse,
        deployment_targets::DeploymentTargetResponse,
        deployment_targets::SaveTargetRequest,
        deployment_targets::TargetStatusRequest,
        application_workspace_sources::WorkspaceSourceResponse,
        application_workspace_sources::SaveWorkspaceSourceRequest,
        crate::application_envs::RegisterAdminApplicationEnvsRequest,
        crate::application_envs::UpdateApplicationEnvRequest,
        crate::application_envs::DeleteApplicationEnvRequest,
        ExternalDeploymentRequest,
        ExternalDeployment,
        ExternalDeploymentTargetRun,
        ExternalDeploymentDiagnostic,
        ExternalDeploymentTaskDiagnostic,
        ExternalDeploymentDiagnosticsResponse,
        ExternalDeploymentLogsQuery,
        ExternalDeploymentLog,
        ExternalDeploymentLogsResponse
    ))
)]
struct ExternalApiDoc;

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
struct ExternalApplicationRow {
    id: String,
    name: String,
    slug: String,
    description: String,
    app_type: String,
    type_version: String,
    environment: String,
    status: String,
    parameter_schema: serde_json::Value,
    verification_config: serde_json::Value,
    version: i64,
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
    result_json: Option<String>,
    deployment_exit_code: Option<i64>,
    last_log_sequence: Option<i64>,
    logs_available: i64,
    failure_payload: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct ExternalDeploymentTaskRow {
    kind: String,
    stage: Option<String>,
    status: String,
    deadline_at: String,
    delivered_at: Option<String>,
    acknowledged_at: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    result_json: Option<String>,
    target_run_id: Option<String>,
    target_id: Option<String>,
    last_event_kind: Option<String>,
    failure_payload: Option<String>,
    protocol_payload: Option<String>,
    last_log_sequence: Option<i64>,
    logs_available: i64,
}

#[derive(sqlx::FromRow)]
struct ExternalDeploymentLogRow {
    sequence: i64,
    stage: Option<String>,
    stream: String,
    content: String,
    truncated: bool,
    created_at: String,
}

fn sanitize_external_log_with_state(content: &str, inside_private_key: &mut bool) -> String {
    let mut output = Vec::with_capacity(content.len());
    for line in content.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("begin ") && lower.contains("private key") {
            if !*inside_private_key {
                output.push("[已脱敏：私钥内容]".to_owned());
            }
            *inside_private_key = true;
            continue;
        }
        if *inside_private_key {
            if lower.contains("end ") && lower.contains("private key") {
                *inside_private_key = false;
            }
            continue;
        }
        if lower.contains("authorization:")
            || lower.contains("bearer ")
            || (lower.contains("://") && lower.contains('@'))
            || [
                "password=",
                "password:",
                "password\"",
                "passwd=",
                "passwd:",
                "token=",
                "token:",
                "token\"",
                "secret=",
                "secret:",
                "secret\"",
                "api_key=",
                "api_key:",
                "private_key=",
                "private_key:",
                "private_key\"",
                "access_key=",
                "access_key:",
                "credential=",
                "credential:",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
        {
            output.push("[已脱敏：敏感日志]".to_owned());
            continue;
        }
        let contains_encoded_credential = line
            .split(|character: char| {
                !(character.is_ascii_alphanumeric() || matches!(character, '+' | '/' | '='))
            })
            .any(|fragment| {
                fragment.len() >= 32
                    && fragment.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')
                    })
            });
        if contains_encoded_credential {
            output.push("[已脱敏：疑似编码凭证]".to_owned());
            continue;
        }
        let delimiter = line.find('=').into_iter().chain(line.find(':')).min();
        if let Some(index) = delimiter {
            let key = line[..index]
                .trim()
                .trim_start_matches("export ")
                .trim_matches(['\'', '"'])
                .to_ascii_uppercase()
                .replace('-', "_");
            let sensitive_key = key == "DATABASE_URL"
                || [
                    "PASSWORD",
                    "PASSWD",
                    "TOKEN",
                    "SECRET",
                    "PRIVATE_KEY",
                    "API_KEY",
                    "ACCESS_KEY",
                    "CREDENTIAL",
                    "CREDENTIALS",
                ]
                .iter()
                .any(|suffix| key == *suffix || key.ends_with(&format!("_{suffix}")));
            if sensitive_key {
                output.push(format!("{}[已脱敏]", &line[..=index]));
                continue;
            }
        }
        output.push(line.to_owned());
    }
    output.join("\n")
}

fn sanitize_external_log(content: &str) -> String {
    sanitize_external_log_with_state(content, &mut false)
}

fn failure_detail(payload: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(payload) = payload else {
        return (None, None);
    };
    let Ok(event) = serde_json::from_str::<serde_json::Value>(payload) else {
        return (None, None);
    };
    let stage = event
        .get("failure_stage")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let step = event
        .get("step_id")
        .or_else(|| event.get("step"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    (stage, step)
}

fn task_result_fields(result_json: Option<&str>) -> (Option<i64>, Option<String>) {
    let Some(raw) = result_json else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return (None, None);
    };
    (
        value.get("exit_code").and_then(serde_json::Value::as_i64),
        value
            .get("error_code")
            .and_then(serde_json::Value::as_str)
            .map(sanitize_external_error_code),
    )
}

fn task_result_details(
    result_json: Option<&str>,
) -> (Option<String>, Option<String>, Option<i64>, bool) {
    let Some(raw) = result_json else {
        return (None, None, None, false);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return (None, None, None, false);
    };
    (
        value
            .get("error_code")
            .and_then(serde_json::Value::as_str)
            .map(sanitize_external_error_code),
        value
            .get("summary")
            .and_then(serde_json::Value::as_str)
            .map(sanitize_external_summary),
        value.get("exit_code").and_then(serde_json::Value::as_i64),
        value.get("status").is_some(),
    )
}

fn protocol_detail(payload: Option<&str>, error_code: Option<&str>) -> Option<String> {
    if error_code != Some("deploy_event_protocol_conflict") {
        return None;
    }
    let payload = payload?;
    let value = serde_json::from_str::<serde_json::Value>(payload).ok()?;
    if value.get("event").and_then(serde_json::Value::as_str) != Some("deploy.finished") {
        return None;
    }
    value
        .get("message")
        .and_then(serde_json::Value::as_str)
        .filter(|message| !message.is_empty())
        .map(sanitize_external_summary)
}

fn sanitize_external_error_code(value: &str) -> String {
    const ALLOWED: &[&str] = &[
        "agent_protocol_unsupported",
        "agent_task_rejected",
        "artifact_download_failed",
        "checkout_failed",
        "process_exited",
        "reconcile_mismatch",
        "runtime_probe_deadline_exceeded",
        "deploy_event_protocol_conflict",
        "task_timeout",
        "timeout",
    ];
    if ALLOWED.contains(&value) {
        value.to_owned()
    } else {
        "agent_error".to_owned()
    }
}

fn sanitize_external_summary(value: &str) -> String {
    let compact = value
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let lower = compact.to_ascii_lowercase();
    let sensitive_markers = [
        "password=",
        "passwd=",
        "token=",
        "secret=",
        "api_key=",
        "authorization:",
        "bearer ",
        "password:",
        "passwd:",
        "token:",
        "secret:",
        "--password",
        "--passwd",
        "--token",
        "--secret",
        "--api-key",
        "private key",
    ];
    if sanitize_external_log(&compact) != compact
        || compact.contains("@") && compact.contains("://")
        || sensitive_markers
            .iter()
            .any(|marker| lower.contains(marker))
        || ["/var/", "/tmp/", "/srv/", "/home/", "/opt/", "\\\\"]
            .iter()
            .any(|marker| compact.contains(marker))
    {
        return "部署任务返回失败（详情已脱敏）".to_owned();
    }
    compact.chars().take(512).collect()
}

fn public_task_status(status: &str) -> String {
    match status {
        "delivered" => "dispatching",
        "accepted" => "acknowledged",
        "queued" | "running" | "canceling" | "succeeded" | "failed" | "canceled"
        | "interrupted" | "expired" => status,
        _ => "unknown",
    }
    .to_owned()
}

fn public_event_kind(
    kind: Option<&str>,
    rejected: bool,
    started: bool,
    reconciled: bool,
) -> Option<String> {
    if rejected {
        return Some("ack_rejected".to_owned());
    }
    Some(
        match kind? {
            "output" => "unknown",
            "state" if started => "started",
            "state" => "acknowledged",
            "result" => "result",
            "diagnostic" if reconciled => "reconciled",
            "progress" => "unknown",
            _ => "unknown",
        }
        .to_owned(),
    )
}

fn public_stage(value: Option<&str>) -> Option<String> {
    match value {
        Some("prepare") => Some("prepare".to_owned()),
        Some("release") => Some("release".to_owned()),
        _ => None,
    }
}

fn public_failure_detail(payload: Option<&str>) -> (Option<String>, Option<String>) {
    let (stage, step) = failure_detail(payload);
    let stage = public_stage(stage.as_deref());
    let step = step.filter(|value| {
        value.len() <= 64
            && value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
    });
    (stage, step)
}

fn task_origin(
    row: &ExternalDeploymentTaskRow,
    has_result: bool,
    _error_code: Option<&str>,
    explicit_timeout: bool,
    explicit_reconcile: bool,
) -> String {
    if row.target_run_id.is_some() && row.target_id.is_none() {
        return "unknown".to_owned();
    }
    if has_result {
        return "agent_result".to_owned();
    }
    if explicit_reconcile {
        return "control_plane_reconcile".to_owned();
    }
    if explicit_timeout {
        return "agent_timeout".to_owned();
    }
    if row.status == "failed"
        && row.acknowledged_at.is_some()
        && row.started_at.is_none()
        && !has_result
    {
        return "agent_rejected".to_owned();
    }
    if row.last_event_kind.as_deref() == Some("result") {
        return "agent_result".to_owned();
    }
    if row.status == "interrupted" {
        return "unknown".to_owned();
    }
    if Utc::now().to_rfc3339() > row.deadline_at
        && matches!(
            row.status.as_str(),
            "accepted" | "running" | "canceling" | "delivered"
        )
    {
        return "agent_timeout".to_owned();
    }
    if row.started_at.is_some() || row.status == "running" {
        return "agent_started".to_owned();
    }
    if row.status == "queued" {
        return "queued".to_owned();
    }
    if row.status == "delivered" {
        return "dispatching".to_owned();
    }
    "unknown".to_owned()
}

async fn load_external_deployment_diagnostic(
    pool: &SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<ExternalDeploymentDiagnostic> {
    const MAX_TASKS: usize = 32;
    let rows: Vec<ExternalDeploymentTaskRow> = sqlx::query_as(
        "SELECT task.kind,task.stage,task.status,task.deadline_at,task.delivered_at,task.acknowledged_at,task.started_at,task.finished_at,task.result_json,task.target_run_id,run.target_id,(SELECT event.kind FROM agent_task_events event WHERE event.task_id=task.id ORDER BY event.sequence DESC LIMIT 1) AS last_event_kind,(SELECT event.payload_json FROM agent_task_events event WHERE event.task_id=task.id AND event.kind='progress' AND json_extract(event.payload_json,'$.event')='deploy.step.failed' ORDER BY event.sequence DESC LIMIT 1) AS failure_payload,(SELECT event.payload_json FROM agent_task_events event WHERE event.task_id=task.id AND event.kind='progress' AND json_extract(event.payload_json,'$.event')='deploy.finished' ORDER BY event.sequence DESC LIMIT 1) AS protocol_payload,(SELECT MAX(log.sequence) FROM deployment_logs log WHERE log.task_id=task.id) AS last_log_sequence,(SELECT EXISTS(SELECT 1 FROM deployment_logs log WHERE log.task_id=task.id)) AS logs_available FROM agent_tasks task LEFT JOIN deployment_target_runs run ON run.id=task.target_run_id AND run.deployment_id=task.deployment_id WHERE task.deployment_id=? AND task.kind IN ('deployment_prepare','deployment_execute','deployment_release') ORDER BY task.kind,task.stage,task.created_at,task.id LIMIT ?",
    )
    .bind(id)
    .bind((MAX_TASKS + 1) as i64)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let truncated = rows.len() > MAX_TASKS;
    let mut tasks = Vec::with_capacity(rows.len().min(MAX_TASKS));
    for row in rows.into_iter().take(MAX_TASKS) {
        let (result_error_code, result_summary, exit_code, has_result) =
            task_result_details(row.result_json.as_deref());
        let explicit_timeout = matches!(
            result_error_code.as_deref(),
            Some("runtime_probe_deadline_exceeded" | "task_timeout" | "timeout")
        );
        let explicit_reconcile = result_error_code.as_deref() == Some("reconcile_mismatch");
        let error_code = result_error_code
            .or_else(|| (row.status == "failed").then(|| "agent_error".to_owned()));
        let (failure_stage, failure_step) = public_failure_detail(row.failure_payload.as_deref());
        let protocol_detail =
            protocol_detail(row.protocol_payload.as_deref(), error_code.as_deref());
        let origin = task_origin(
            &row,
            has_result,
            error_code.as_deref(),
            explicit_timeout,
            explicit_reconcile,
        );
        let execution_phase = match row.kind.as_str() {
            "deployment_prepare" => "prepare",
            "deployment_release" => "release",
            "deployment_execute" => "execute",
            _ => "unknown",
        };
        tasks.push(ExternalDeploymentTaskDiagnostic {
            kind: row.kind,
            execution_phase: execution_phase.to_owned(),
            target_id: row.target_id,
            stage: public_stage(row.stage.as_deref()),
            status: public_task_status(&row.status),
            last_event_kind: public_event_kind(
                row.last_event_kind.as_deref(),
                origin == "agent_rejected",
                row.started_at.is_some() || row.status == "running",
                origin == "control_plane_reconcile",
            ),
            origin,
            delivered_at: row.delivered_at,
            acknowledged_at: row.acknowledged_at,
            task_started_at: row.started_at,
            task_finished_at: row.finished_at,
            error_code,
            summary: result_summary,
            exit_code,
            failure_stage,
            failure_step,
            protocol_detail,
            logs_available: row.logs_available != 0,
            last_log_sequence: row.last_log_sequence,
        });
    }
    let selected = tasks
        .iter()
        .find(|task| task.status == "failed")
        .or_else(|| tasks.last());
    let deployment_logs: (i64, Option<i64>) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM deployment_logs WHERE deployment_id=?),(SELECT MAX(sequence) FROM deployment_logs WHERE deployment_id=?)",
    )
    .bind(id)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    Ok(ExternalDeploymentDiagnostic {
        origin: selected
            .map(|task| task.origin.clone())
            .unwrap_or_else(|| "unknown".to_owned()),
        error_code: selected.and_then(|task| task.error_code.clone()),
        summary: selected.and_then(|task| task.summary.clone()),
        exit_code: selected.and_then(|task| task.exit_code),
        execution_phase: selected.map(|task| task.execution_phase.clone()),
        failure_stage: selected.and_then(|task| task.failure_stage.clone()),
        failure_step: selected.and_then(|task| task.failure_step.clone()),
        protocol_detail: selected.and_then(|task| task.protocol_detail.clone()),
        logs_available: deployment_logs.0 != 0,
        last_log_sequence: deployment_logs.1,
        tasks,
        truncated,
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/openapi.json", get(openapi))
        .route(
            "/applications",
            get(list_applications).post(create_application),
        )
        .route(
            "/applications/{id}",
            get(show_application).patch(update_application),
        )
        .route(
            "/applications/{id}/env-files",
            get(list_env_files).post(register_env_files),
        )
        .route(
            "/applications/{id}/env-files/{env_file_id}",
            put(update_env_file).delete(delete_env_file),
        )
        .route(
            "/applications/{id}/env-files/{env_file_id}/inspect",
            post(inspect_env_file),
        )
        .route(
            "/applications/{id}/targets",
            get(list_targets).post(create_target),
        )
        .route("/deployment-targets/{target_id}", patch(update_target))
        .route(
            "/deployment-targets/{target_id}/status",
            put(update_target_status),
        )
        .route(
            "/applications/{id}/workspace-source",
            get(show_workspace_source).put(save_workspace_source),
        )
        .route("/applications/{id}/deployments", post(create_deployment))
        .route("/deployments/{id}", get(show_deployment))
        .route(
            "/deployments/{id}/diagnostics",
            get(show_deployment_diagnostics),
        )
        .route("/deployments/{id}/logs", get(list_deployment_logs))
        .route("/deployments/{id}/cancel", post(cancel_deployment))
}

pub fn external_openapi_document() -> serde_json::Value {
    let mut document =
        serde_json::to_value(ExternalApiDoc::openapi()).expect("外部 OpenAPI 可以序列化");
    document["info"]["title"] = serde_json::json!("Deploy Go 对外部署 API");
    document["info"]["version"] = serde_json::json!(env!("CARGO_PKG_VERSION"));
    document["components"]["securitySchemes"]["externalApiKey"] = serde_json::json!({
        "type": "http",
        "scheme": "bearer",
        "description": "管理端创建的外部部署 API Key，格式为 dgx_..."
    });
    let Some(paths) = document["paths"].as_object_mut() else {
        return document;
    };
    for (path, path_item) in paths {
        if path == "/external/v1/openapi.json" {
            continue;
        }
        let Some(operations) = path_item.as_object_mut() else {
            continue;
        };
        for (_, operation) in operations {
            operation["security"] = serde_json::json!([{ "externalApiKey": [] }]);
        }
    }
    document
}

async fn openapi() -> Json<serde_json::Value> {
    Json(external_openapi_document())
}

#[utoipa::path(operation_id = "external_applications_list", get, path = "/external/v1/applications", responses((status = 200, body = ExternalApplicationListResponse), (status = 401, body = crate::error::ErrorResponse)))]
pub(crate) async fn list_applications(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalApplicationListResponse>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT a.id,a.display_name AS name,a.slug,a.description,a.environment FROM applications a JOIN external_api_key_applications key_app ON key_app.application_id=a.id WHERE key_app.api_key_id=? AND a.status='active' ORDER BY a.display_name COLLATE NOCASE,a.id",
    )
    .bind(&key.id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(ExternalApplicationListResponse {
        items: rows
            .into_iter()
            .map(
                |(id, name, slug, description, environment)| ExternalApplicationSummary {
                    id,
                    name,
                    slug,
                    description,
                    environment,
                    status: "active".to_owned(),
                },
            )
            .collect(),
    }))
}

#[utoipa::path(operation_id = "external_applications_create", post, path = "/external/v1/applications", request_body = ExternalApplicationCreateRequest, responses((status = 201, body = ExternalApplicationDetail), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn create_application(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<ExternalApplicationCreateRequest>,
) -> ApiResult<(StatusCode, Json<ExternalApplicationDetail>)> {
    if payload.environment.trim() == PRODUCTION_ENVIRONMENT {
        return Err(production_environment_forbidden(request_id.as_str()));
    }
    let actor = service_actor();
    let created = crate::applications::create_application(
        &state,
        &actor.id,
        Some(&key.id),
        &crate::applications::SaveApplicationRequest {
            name: payload.name,
            slug: payload.slug,
            description: payload.description,
            app_type: payload.app_type,
            type_version: payload.type_version,
            environment: payload.environment,
            parameter_schema: payload.parameter_schema,
            verification_config: payload.verification_config,
            template_id: None,
            version: None,
            tags: Some(payload.tags),
        },
        request_id.as_str(),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(
            load_external_application_detail(state.pool(), &created.id, request_id.as_str())
                .await?,
        ),
    ))
}

#[utoipa::path(operation_id = "external_applications_show", get, path = "/external/v1/applications/{id}", params(("id" = String, Path)), responses((status = 200, body = ExternalApplicationDetail), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show_application(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalApplicationDetail>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    Ok(Json(
        load_external_application_detail(state.pool(), &id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "external_applications_update", patch, path = "/external/v1/applications/{id}", params(("id" = String, Path)), request_body = ExternalApplicationUpdateRequest, responses((status = 200, body = ExternalApplicationDetail), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_application(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<ExternalApplicationUpdateRequest>,
) -> ApiResult<Json<ExternalApplicationDetail>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    let current = crate::applications::find(state.pool(), &id, request_id.as_str()).await?;
    if current.environment == PRODUCTION_ENVIRONMENT {
        return Err(production_application_forbidden(request_id.as_str()));
    }
    if payload.environment.as_deref() == Some(PRODUCTION_ENVIRONMENT) {
        return Err(production_environment_forbidden(request_id.as_str()));
    }
    let update = crate::applications::SaveApplicationRequest {
        name: payload.name.unwrap_or(current.name),
        slug: payload.slug.unwrap_or(current.slug),
        description: payload.description.unwrap_or(current.description),
        app_type: payload.app_type.unwrap_or(current.app_type),
        type_version: payload.type_version.unwrap_or(current.type_version),
        environment: payload.environment.unwrap_or(current.environment),
        parameter_schema: payload.parameter_schema.or(Some(current.parameter_schema)),
        verification_config: payload
            .verification_config
            .or(Some(current.verification_config)),
        template_id: None,
        version: Some(payload.version),
        tags: payload.tags.or(Some(current.tags)),
    };
    crate::applications::update_application(
        &state,
        &service_actor().id,
        Some(&key.id),
        &id,
        &update,
        request_id.as_str(),
    )
    .await?;
    Ok(Json(
        load_external_application_detail(state.pool(), &id, request_id.as_str()).await?,
    ))
}

const ENV_FILE_SELECT: &str = "SELECT f.id,f.application_id,f.file_name,f.module,f.format,f.current_version,f.current_digest,f.declared_at,f.updated_at,f.version,(SELECT COUNT(*) FROM deployment_targets t WHERE t.application_id=f.application_id AND t.status='active') target_count,(SELECT COUNT(*) FROM application_env_syncs s JOIN application_env_versions v ON v.id=s.env_version_id WHERE v.env_file_id=f.id AND v.env_version=f.current_version AND s.status='pending') pending_count,(SELECT COUNT(*) FROM application_env_syncs s JOIN application_env_versions v ON v.id=s.env_version_id WHERE v.env_file_id=f.id AND v.env_version=f.current_version AND s.status='syncing') syncing_count,(SELECT COUNT(*) FROM application_env_syncs s JOIN application_env_versions v ON v.id=s.env_version_id WHERE v.env_file_id=f.id AND v.env_version=f.current_version AND s.status='succeeded') succeeded_count,(SELECT COUNT(*) FROM application_env_syncs s JOIN application_env_versions v ON v.id=s.env_version_id WHERE v.env_file_id=f.id AND v.env_version=f.current_version AND s.status='failed') failed_count FROM application_env_files f";

#[utoipa::path(operation_id = "external_env_files_list", get, path = "/external/v1/applications/{id}/env-files", params(("id" = String, Path)), responses((status = 200, body = ExternalEnvFileListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn list_env_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalEnvFileListResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    let query = format!(
        "{ENV_FILE_SELECT} WHERE f.application_id=? AND f.deleted_at IS NULL ORDER BY f.file_name COLLATE NOCASE,f.id"
    );
    let items = sqlx::query_as::<_, ExternalEnvFile>(&query)
        .bind(&id)
        .fetch_all(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(ExternalEnvFileListResponse { items }))
}

#[utoipa::path(operation_id = "external_env_files_register", post, path = "/external/v1/applications/{id}/env-files", params(("id" = String, Path)), request_body = crate::application_envs::RegisterAdminApplicationEnvsRequest, responses((status = 200, body = ExternalEnvRegistrationResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn register_env_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<
        application_envs::RegisterAdminApplicationEnvsRequest,
    >,
) -> ApiResult<Json<ExternalEnvRegistrationResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    let created = application_envs::register_files_core(
        &state,
        &id,
        &payload.files,
        Some(&service_actor().id),
        Some(&key.id),
        request_id.as_str(),
    )
    .await?;
    Ok(Json(ExternalEnvRegistrationResponse { created }))
}

#[utoipa::path(operation_id = "external_env_file_inspect", post, path = "/external/v1/applications/{id}/env-files/{env_file_id}/inspect", params(("id" = String, Path), ("env_file_id" = String, Path)), request_body = ExternalEnvInspectionRequest, responses((status = 200, body = ExternalEnvInspectionResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn inspect_env_file(
    State(state): State<AppState>,
    Path((id, env_file_id)): Path<(String, String)>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<ExternalEnvInspectionRequest>,
) -> ApiResult<Json<ExternalEnvInspectionResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    require_env_file_owner(&state, &id, &env_file_id, request_id.as_str()).await?;
    validate_inspection_keys(&payload.keys, request_id.as_str())?;

    let results = application_envs::inspect_env_keys(
        &state,
        &id,
        &env_file_id,
        &payload.keys,
        request_id.as_str(),
    )
    .await?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    crate::audit::record(
        &mut transaction,
        Some(&service_actor().id),
        "application_env.inspect",
        "application_env_file",
        &env_file_id,
        request_id.as_str(),
        serde_json::json!({
            "application_id": id,
            "external_api_key_id": key.id,
            "key_count": payload.keys.len(),
            "exists_count": results.iter().filter(|(_, length)| length.is_some()).count(),
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;

    Ok(Json(ExternalEnvInspectionResponse {
        items: results
            .into_iter()
            .map(|(key, value_length_bytes)| ExternalEnvKeyInspection {
                key,
                exists: value_length_bytes.is_some(),
                value_length_bytes,
            })
            .collect(),
    }))
}

fn validate_inspection_keys(keys: &[String], request_id: &str) -> ApiResult<()> {
    if !(1..=50).contains(&keys.len()) {
        return Err(invalid_env_inspection_request(request_id));
    }
    let mut unique = HashSet::with_capacity(keys.len());
    if keys
        .iter()
        .any(|key| !application_envs::dotenv::valid_key(key) || !unique.insert(key.as_str()))
    {
        return Err(invalid_env_inspection_request(request_id));
    }
    Ok(())
}

fn invalid_env_inspection_request(request_id: &str) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "env_inspection_keys_invalid",
        "键名必须是 1 到 50 个唯一且格式有效的 dotenv 变量名",
        request_id,
    )
}

#[utoipa::path(operation_id = "external_env_files_update", put, path = "/external/v1/applications/{id}/env-files/{env_file_id}", params(("id" = String, Path), ("env_file_id" = String, Path)), request_body = crate::application_envs::UpdateApplicationEnvRequest, responses((status = 200, body = ExternalEnvFile), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_env_file(
    State(state): State<AppState>,
    Path((id, env_file_id)): Path<(String, String)>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<
        application_envs::UpdateApplicationEnvRequest,
    >,
) -> ApiResult<Json<ExternalEnvFile>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    require_env_file_owner(&state, &id, &env_file_id, request_id.as_str()).await?;
    application_envs::update_file_core(
        &state,
        &env_file_id,
        &payload.content,
        payload.expected_version,
        Some(&service_actor().id),
        Some(&key.id),
        request_id.as_str(),
    )
    .await?;
    Ok(Json(
        load_external_env_file(state.pool(), &env_file_id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "external_env_files_delete", delete, path = "/external/v1/applications/{id}/env-files/{env_file_id}", params(("id" = String, Path), ("env_file_id" = String, Path)), request_body = crate::application_envs::DeleteApplicationEnvRequest, responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn delete_env_file(
    State(state): State<AppState>,
    Path((id, env_file_id)): Path<(String, String)>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<
        application_envs::DeleteApplicationEnvRequest,
    >,
) -> ApiResult<StatusCode> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    require_env_file_owner(&state, &id, &env_file_id, request_id.as_str()).await?;
    application_envs::delete_file_core(
        &state,
        &env_file_id,
        &payload.confirm_file_name,
        payload.expected_version,
        Some(&service_actor().id),
        Some(&key.id),
        request_id.as_str(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn require_env_file_owner(
    state: &AppState,
    application_id: &str,
    env_file_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT application_id FROM application_env_files WHERE id=? AND deleted_at IS NULL",
    )
    .bind(env_file_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if owner.as_deref() == Some(application_id) {
        Ok(())
    } else {
        Err(ApiError::not_found(request_id))
    }
}

async fn load_external_env_file(
    pool: &SqlitePool,
    env_file_id: &str,
    request_id: &str,
) -> ApiResult<ExternalEnvFile> {
    let query = format!("{ENV_FILE_SELECT} WHERE f.id=? AND f.deleted_at IS NULL");
    sqlx::query_as::<_, ExternalEnvFile>(&query)
        .bind(env_file_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))
}

#[utoipa::path(operation_id = "external_deployment_targets_list", get, path = "/external/v1/applications/{id}/targets", params(("id" = String, Path)), responses((status = 200, body = ExternalDeploymentTargetListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn list_targets(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalDeploymentTargetListResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    let items = deployment_targets::list_responses(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(ExternalDeploymentTargetListResponse { items }))
}

#[utoipa::path(operation_id = "external_deployment_targets_create", post, path = "/external/v1/applications/{id}/targets", params(("id" = String, Path)), request_body = deployment_targets::SaveTargetRequest, responses((status = 201, body = deployment_targets::DeploymentTargetResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn create_target(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<deployment_targets::SaveTargetRequest>,
) -> ApiResult<(
    StatusCode,
    Json<deployment_targets::DeploymentTargetResponse>,
)> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    let target = deployment_targets::create_target_core(
        &state,
        &id,
        &payload,
        &service_actor().id,
        Some(&key.id),
        request_id.as_str(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(target)))
}

#[utoipa::path(operation_id = "external_deployment_targets_update", patch, path = "/external/v1/deployment-targets/{target_id}", params(("target_id" = String, Path)), request_body = deployment_targets::SaveTargetRequest, responses((status = 200, body = deployment_targets::DeploymentTargetResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_target(
    State(state): State<AppState>,
    Path(target_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<deployment_targets::SaveTargetRequest>,
) -> ApiResult<Json<deployment_targets::DeploymentTargetResponse>> {
    let application_id =
        deployment_targets::find_application_id(state.pool(), &target_id, request_id.as_str())
            .await?;
    require_key_application_access(state.pool(), &key, &application_id, request_id.as_str())
        .await?;
    require_external_configurable_application(state.pool(), &application_id, request_id.as_str())
        .await?;
    let target = deployment_targets::update_target_core(
        &state,
        &target_id,
        &payload,
        &service_actor().id,
        Some(&key.id),
        request_id.as_str(),
    )
    .await?;
    Ok(Json(target))
}

#[utoipa::path(operation_id = "external_deployment_targets_update_status", put, path = "/external/v1/deployment-targets/{target_id}/status", params(("target_id" = String, Path)), request_body = deployment_targets::TargetStatusRequest, responses((status = 200, body = deployment_targets::DeploymentTargetResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_target_status(
    State(state): State<AppState>,
    Path(target_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<deployment_targets::TargetStatusRequest>,
) -> ApiResult<Json<deployment_targets::DeploymentTargetResponse>> {
    let application_id =
        deployment_targets::find_application_id(state.pool(), &target_id, request_id.as_str())
            .await?;
    require_key_application_access(state.pool(), &key, &application_id, request_id.as_str())
        .await?;
    require_external_configurable_application(state.pool(), &application_id, request_id.as_str())
        .await?;
    let target = deployment_targets::update_target_status_core(
        &state,
        &target_id,
        &payload.status,
        payload.version,
        &service_actor().id,
        Some(&key.id),
        request_id.as_str(),
    )
    .await?;
    Ok(Json(target))
}

#[utoipa::path(operation_id = "external_application_workspace_source_show", get, path = "/external/v1/applications/{id}/workspace-source", params(("id" = String, Path)), responses((status = 200, body = application_workspace_sources::WorkspaceSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show_workspace_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<application_workspace_sources::WorkspaceSourceResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    let view = application_workspace_sources::workspace_source_view(
        state.pool(),
        &id,
        request_id.as_str(),
    )
    .await?
    .ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "application_workspace_source_not_configured",
            "应用尚未配置固定工作区来源",
            request_id.as_str(),
        )
    })?;
    Ok(Json(view))
}

#[utoipa::path(operation_id = "external_application_workspace_source_save", put, path = "/external/v1/applications/{id}/workspace-source", params(("id" = String, Path)), request_body = application_workspace_sources::SaveWorkspaceSourceRequest, responses((status = 200, body = application_workspace_sources::WorkspaceSourceResponse), (status = 201, body = application_workspace_sources::WorkspaceSourceResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn save_workspace_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<
        application_workspace_sources::SaveWorkspaceSourceRequest,
    >,
) -> ApiResult<(
    StatusCode,
    Json<application_workspace_sources::WorkspaceSourceResponse>,
)> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    let (created, view) = application_workspace_sources::save_workspace_source_core(
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

async fn load_external_application_detail(
    pool: &SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<ExternalApplicationDetail> {
    let application: Option<ExternalApplicationRow> = sqlx::query_as(
        "SELECT id,display_name AS name,slug,description,app_type,type_version,environment,status,parameter_schema,verification_config,version FROM applications WHERE id=? AND status='active'",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let application = application.ok_or_else(|| ApiError::not_found(request_id))?;
    let tags = sqlx::query_scalar::<_, String>(
        "SELECT t.name FROM application_tag_links link JOIN application_tags t ON t.id=link.tag_id WHERE link.application_id=? ORDER BY t.name COLLATE NOCASE",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let targets = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT t.id,t.environment,t.node_id,n.name,t.status,t.execution_mode FROM deployment_targets t JOIN nodes n ON n.id=t.node_id WHERE t.application_id=? AND t.status='active' ORDER BY t.id",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    Ok(ExternalApplicationDetail {
        id: application.id,
        name: application.name,
        slug: application.slug,
        description: application.description,
        app_type: application.app_type,
        type_version: application.type_version,
        environment: application.environment,
        status: application.status,
        tags,
        parameter_schema: application.parameter_schema,
        verification_config: application.verification_config,
        version: application.version,
        targets: targets
            .into_iter()
            .map(
                |(id, environment, node_id, node_name, status, execution_mode)| {
                    ExternalDeploymentTarget {
                        id,
                        environment,
                        node_id,
                        node_name,
                        status,
                        execution_mode,
                    }
                },
            )
            .collect(),
    })
}

#[utoipa::path(operation_id = "external_deployments_create", post, path = "/external/v1/applications/{id}/deployments", params(("id" = String, Path), ("Idempotency-Key" = String, Header)), request_body = ExternalDeploymentRequest, responses((status = 200, body = ExternalDeployment), (status = 201, body = ExternalDeployment), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
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
    require_external_non_production_environment(state.pool(), &application_id, request_id.as_str())
        .await?;
    let idempotency_key = deployments::validate_idempotency_key(&headers, request_id.as_str())?;
    let actor = service_actor();
    let (status, response) = match payload.target_id.as_deref() {
        Some(target_id) => {
            let target: Option<(String, String, String)> = sqlx::query_as(
                "SELECT application_id,status,environment FROM deployment_targets WHERE id=?",
            )
            .bind(target_id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
            let Some((target_application_id, target_status, target_environment)) = target else {
                return Err(ApiError::not_found(request_id.as_str()));
            };
            if target_application_id != application_id || target_status != "active" {
                return Err(ApiError::not_found(request_id.as_str()));
            }
            reject_production_environment(&target_environment, request_id.as_str())?;
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
            require_no_production_targets(state.pool(), &application_id, request_id.as_str())
                .await?;
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

#[utoipa::path(
    operation_id = "external_deployments_diagnostics",
    get,
    path = "/external/v1/deployments/{id}/diagnostics",
    params(("id" = String, Path)),
    responses(
        (status = 200, body = ExternalDeploymentDiagnosticsResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 404, body = crate::error::ErrorResponse)
    )
)]
pub(crate) async fn show_deployment_diagnostics(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalDeploymentDiagnosticsResponse>> {
    let application_id = deployment_application_id(state.pool(), &id, request_id.as_str()).await?;
    require_key_application_access(state.pool(), &key, &application_id, request_id.as_str())
        .await?;
    let deployment = load_external_deployment(state.pool(), &id, request_id.as_str()).await?;
    let diagnostic =
        deployment
            .diagnostic
            .clone()
            .unwrap_or_else(|| ExternalDeploymentDiagnostic {
                origin: "unknown".to_owned(),
                error_code: None,
                summary: None,
                exit_code: None,
                execution_phase: None,
                failure_stage: None,
                failure_step: None,
                protocol_detail: None,
                logs_available: false,
                last_log_sequence: None,
                tasks: Vec::new(),
                truncated: false,
            });
    Ok(Json(ExternalDeploymentDiagnosticsResponse {
        deployment_id: deployment.id,
        status: deployment.status,
        phase: deployment.phase,
        diagnostic,
        target_runs: deployment.target_runs,
    }))
}

#[utoipa::path(
    operation_id = "external_deployments_logs",
    get,
    path = "/external/v1/deployments/{id}/logs",
    params(("id" = String, Path), ExternalDeploymentLogsQuery),
    responses((status = 200, body = ExternalDeploymentLogsResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse))
)]
pub(crate) async fn list_deployment_logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ExternalDeploymentLogsQuery>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<ExternalDeploymentLogsResponse>> {
    let application_id = deployment_application_id(state.pool(), &id, request_id.as_str()).await?;
    require_key_application_access(state.pool(), &key, &application_id, request_id.as_str())
        .await?;
    let after = query.after.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);
    if after < 0 {
        return Err(ApiError::validation(
            "日志游标不能为负数",
            request_id.as_str(),
        ));
    }
    if !(1..=200).contains(&limit) {
        return Err(ApiError::validation(
            "日志 limit 必须在 1 到 200 之间",
            request_id.as_str(),
        ));
    }
    let bounds: (Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT MIN(sequence),MAX(sequence) FROM deployment_logs WHERE deployment_id=?",
    )
    .bind(&id)
    .fetch_one(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if after > 0
        && (bounds.1.is_none()
            || bounds.1.is_some_and(|maximum| after > maximum)
            || bounds.0.is_some_and(|minimum| after < minimum - 1))
    {
        return Err(ApiError::validation(
            "日志游标无效或已经过期",
            request_id.as_str(),
        ));
    }
    let rows: Vec<ExternalDeploymentLogRow> = sqlx::query_as(
        "SELECT log.sequence,task.stage,log.stream,log.content,log.truncated,log.created_at FROM deployment_logs log LEFT JOIN agent_tasks task ON task.id=log.task_id WHERE log.deployment_id=? AND log.sequence>? ORDER BY log.sequence LIMIT ?",
    )
    .bind(&id)
    .bind(after)
    .bind(i64::from(limit))
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let previous_private_key_boundary: Option<String> = sqlx::query_scalar(
        "SELECT content FROM deployment_logs WHERE deployment_id=? AND sequence<=? AND UPPER(content) LIKE '%PRIVATE KEY%' ORDER BY sequence DESC LIMIT 1",
    )
    .bind(&id)
    .bind(after)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let mut inside_private_key = false;
    if let Some(content) = previous_private_key_boundary {
        let _ = sanitize_external_log_with_state(&content, &mut inside_private_key);
    }
    let page_after = rows.last().map(|row| row.sequence).unwrap_or(after);
    let status: String = sqlx::query_scalar("SELECT status FROM deployments WHERE id=?")
        .bind(&id)
        .fetch_one(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment_logs WHERE deployment_id=? AND sequence>?",
    )
    .bind(&id)
    .bind(page_after)
    .fetch_one(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let terminal = matches!(
        status.as_str(),
        "succeeded" | "failed" | "canceled" | "interrupted"
    ) && pending == 0;
    let next_after = (pending > 0).then_some(page_after);
    Ok(Json(ExternalDeploymentLogsResponse {
        items: rows
            .into_iter()
            .map(|row| ExternalDeploymentLog {
                sequence: row.sequence,
                stage: row.stage,
                stream: row.stream,
                content: sanitize_external_log_with_state(&row.content, &mut inside_private_key),
                truncated: row.truncated,
                created_at: row.created_at,
            })
            .collect(),
        next_after,
        terminal,
    }))
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
        "SELECT d.id,d.application_id,a.display_name AS application_name,d.target_id,t.environment,n.name AS node_name,d.status,d.phase,d.snapshot_hash,d.result_summary,d.exit_code,d.queued_at,d.started_at,d.finished_at,d.cancel_requested_at,d.created_at,d.updated_at FROM deployments d JOIN applications a ON a.id=d.application_id JOIN deployment_targets t ON t.id=d.target_id JOIN nodes n ON n.id=t.node_id WHERE d.id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let Some(row) = row else {
        return Err(ApiError::not_found(request_id));
    };
    let diagnostic = load_external_deployment_diagnostic(pool, &row.id, request_id).await?;
    let runs: Vec<ExternalDeploymentRunRow> = sqlx::query_as(
        "SELECT run.id,run.target_id,run.node_id,n.name AS node_name,run.status,run.phase,run.result_summary,run.error_code,task.result_json,d.exit_code AS deployment_exit_code,(SELECT MAX(log.sequence) FROM deployment_logs log WHERE log.deployment_id=run.deployment_id AND (log.task_id IS NULL OR log.task_id=task.id)) AS last_log_sequence,EXISTS(SELECT 1 FROM deployment_logs log WHERE log.deployment_id=run.deployment_id AND (log.task_id IS NULL OR log.task_id=task.id)) AS logs_available,(SELECT event.payload_json FROM agent_task_events event WHERE event.task_id=task.id AND json_extract(event.payload_json,'$.event')='deploy.step.failed' ORDER BY event.sequence DESC LIMIT 1) AS failure_payload,run.started_at,run.finished_at,run.created_at,run.updated_at FROM deployment_target_runs run JOIN deployments d ON d.id=run.deployment_id JOIN nodes n ON n.id=run.node_id LEFT JOIN agent_tasks task ON task.id=(SELECT candidate.id FROM agent_tasks candidate WHERE candidate.target_run_id=run.id ORDER BY CASE WHEN candidate.status='failed' THEN 0 ELSE 1 END,candidate.created_at DESC,candidate.id DESC LIMIT 1) WHERE run.deployment_id=? ORDER BY run.target_id,run.id",
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
        result_summary: row
            .result_summary
            .map(|summary| sanitize_external_summary(&summary)),
        exit_code: row.exit_code,
        queued_at: row.queued_at,
        started_at: row.started_at,
        finished_at: row.finished_at,
        cancel_requested_at: row.cancel_requested_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
        target_runs: runs
            .into_iter()
            .map(|run| {
                let (task_exit_code, task_error_code) =
                    task_result_fields(run.result_json.as_deref());
                let (failure_stage, failure_step) = failure_detail(run.failure_payload.as_deref());
                ExternalDeploymentTargetRun {
                    id: run.id,
                    target_id: run.target_id,
                    node_id: run.node_id,
                    node_name: run.node_name,
                    status: run.status,
                    phase: run.phase,
                    result_summary: run
                        .result_summary
                        .map(|summary| sanitize_external_summary(&summary)),
                    error_code: run
                        .error_code
                        .or(task_error_code)
                        .map(|error_code| sanitize_external_error_code(&error_code)),
                    exit_code: task_exit_code.or(run.deployment_exit_code),
                    failure_stage: public_stage(failure_stage.as_deref()),
                    failure_step: failure_step.filter(|value| {
                        value.len() <= 64
                            && value.chars().all(|character| {
                                character.is_ascii_alphanumeric() || "._-".contains(character)
                            })
                    }),
                    logs_available: run.logs_available != 0,
                    last_log_sequence: run.last_log_sequence,
                    started_at: run.started_at,
                    finished_at: run.finished_at,
                    created_at: run.created_at,
                    updated_at: run.updated_at,
                }
            })
            .collect(),
        diagnostic: Some(diagnostic),
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

const PRODUCTION_ENVIRONMENT: &str = "prod";

fn production_deployment_forbidden(request_id: &str) -> ApiError {
    ApiError::new(
        StatusCode::FORBIDDEN,
        "external_production_deployment_forbidden",
        "对外部署 API 不允许发起正式环境部署",
        request_id,
    )
}

fn production_application_forbidden(request_id: &str) -> ApiError {
    ApiError::new(
        StatusCode::FORBIDDEN,
        "external_production_application_forbidden",
        "对外 API 不允许编辑正式环境应用",
        request_id,
    )
}

fn production_environment_forbidden(request_id: &str) -> ApiError {
    ApiError::new(
        StatusCode::FORBIDDEN,
        "external_production_environment_forbidden",
        "对外 API 不允许将应用环境设置为正式环境",
        request_id,
    )
}

fn reject_production_environment(environment: &str, request_id: &str) -> ApiResult<()> {
    if environment == PRODUCTION_ENVIRONMENT {
        return Err(production_deployment_forbidden(request_id));
    }
    Ok(())
}

async fn require_external_non_production_environment(
    pool: &SqlitePool,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let environment: Option<String> =
        sqlx::query_scalar("SELECT environment FROM applications WHERE id=? AND status='active'")
            .bind(application_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    let environment = environment.ok_or_else(|| ApiError::not_found(request_id))?;
    reject_production_environment(&environment, request_id)
}

/// 配置类写操作（Env、部署目标、部署来源）对正式环境应用统一返回编辑禁用错误。
async fn require_external_configurable_application(
    pool: &SqlitePool,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let environment: Option<String> =
        sqlx::query_scalar("SELECT environment FROM applications WHERE id=? AND status='active'")
            .bind(application_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    if environment.as_deref() == Some(PRODUCTION_ENVIRONMENT) {
        return Err(production_application_forbidden(request_id));
    }
    Ok(())
}

async fn require_no_production_targets(
    pool: &SqlitePool,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let has_production_target: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM deployment_targets WHERE application_id=? AND environment=? AND status='active')",
    )
    .bind(application_id)
    .bind(PRODUCTION_ENVIRONMENT)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if has_production_target {
        return Err(production_deployment_forbidden(request_id));
    }
    Ok(())
}
