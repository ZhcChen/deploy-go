use std::collections::HashSet;

use chrono::Utc;
use deploy_go_agent_protocol::{
    DeployEvent, DeployEventName, DeployEventStatus, DeploymentStage, Environment,
};
use serde::Deserialize;
use thiserror::Error;

const MARKER_PREFIX: &str = "DEPLOY_GO_EVENT ";

#[derive(Clone, Debug)]
pub struct DeployEventContext {
    pub deploy_id: String,
    pub stage: DeploymentStage,
    pub environment: Environment,
    pub release_version: String,
    pub target: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMarker {
    schema_version: u32,
    event: MarkerEventName,
    #[serde(default)]
    module: Option<String>,
    #[serde(default)]
    module_name: Option<String>,
    #[serde(default)]
    step_id: Option<String>,
    #[serde(default)]
    step: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    failure_stage: Option<String>,
    #[serde(default)]
    recovery_hint: Option<String>,
    #[serde(default)]
    candidate_release: Option<String>,
    #[serde(default)]
    current_release: Option<String>,
    #[serde(default)]
    current_switched: Option<bool>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MarkerEventName {
    #[serde(rename = "deploy.preflight.started")]
    PreflightStarted,
    #[serde(rename = "deploy.preflight.succeeded")]
    PreflightSucceeded,
    #[serde(rename = "deploy.preflight.failed")]
    PreflightFailed,
    #[serde(rename = "deploy.module.started")]
    ModuleStarted,
    #[serde(rename = "deploy.module.succeeded")]
    ModuleSucceeded,
    #[serde(rename = "deploy.module.failed")]
    ModuleFailed,
    #[serde(rename = "deploy.step.started")]
    StepStarted,
    #[serde(rename = "deploy.step.succeeded")]
    StepSucceeded,
    #[serde(rename = "deploy.step.failed")]
    StepFailed,
    #[serde(rename = "deploy.verification.started")]
    VerificationStarted,
    #[serde(rename = "deploy.verification.succeeded")]
    VerificationSucceeded,
    #[serde(rename = "deploy.verification.failed")]
    VerificationFailed,
}

#[derive(Clone, Debug, Error)]
#[error("{kind}: {message}")]
pub struct MarkerViolation {
    pub kind: String,
    pub message: String,
}

#[derive(Default)]
pub struct MarkerState {
    preflight_started: bool,
    preflight_finished: bool,
    active_module: Option<String>,
    active_step: Option<(String, String)>,
    finished_modules: HashSet<String>,
    pub violations: Vec<String>,
}

impl MarkerState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn started_event(ctx: &DeployEventContext) -> DeployEvent {
    base_event(
        ctx,
        DeployEventName::DeployStarted,
        DeployEventStatus::Started,
    )
}

pub fn finished_event(
    ctx: &DeployEventContext,
    state: &MarkerState,
    exit_ok: bool,
) -> (DeployEvent, Option<String>) {
    let protocol_invalid = !state.violations.is_empty()
        || state.active_module.is_some()
        || state.active_step.is_some()
        || !state.preflight_started
        || !state.preflight_finished;
    if protocol_invalid {
        let summary = if state.violations.is_empty() {
            "deployment protocol ended with an unclosed stage".to_owned()
        } else {
            state.violations.join("; ")
        };
        let mut event = base_event(
            ctx,
            DeployEventName::DeployFinished,
            DeployEventStatus::Failed,
        );
        event.message = Some(summary);
        return (event, Some("deploy_event_protocol_conflict".to_owned()));
    }
    (
        base_event(
            ctx,
            DeployEventName::DeployFinished,
            if exit_ok {
                DeployEventStatus::Succeeded
            } else {
                DeployEventStatus::Failed
            },
        ),
        None,
    )
}

/// 处理一行 stdout。非 marker 行返回 Ok(None)；marker 校验失败返回 Err 且不中断日志流。
pub fn process_line(
    line: &str,
    ctx: &DeployEventContext,
    state: &mut MarkerState,
) -> Result<Option<DeployEvent>, MarkerViolation> {
    if !line.starts_with(MARKER_PREFIX) {
        return Ok(None);
    }
    let body = &line[MARKER_PREFIX.len()..];
    let marker: RawMarker = serde_json::from_str(body).map_err(|error| MarkerViolation {
        kind: "invalid_marker_json".to_owned(),
        message: error.to_string(),
    })?;
    if marker.schema_version != 1 {
        return Err(MarkerViolation {
            kind: "unsupported_marker_schema".to_owned(),
            message: format!("schema_version={}", marker.schema_version),
        });
    }
    state.validate_transition(&marker)?;
    Ok(Some(enrich(ctx, &marker)))
}

impl MarkerState {
    fn validate_transition(&mut self, marker: &RawMarker) -> Result<(), MarkerViolation> {
        match marker.event {
            MarkerEventName::PreflightStarted => {
                if self.preflight_started || self.preflight_finished || self.active_module.is_some()
                {
                    return Err(violation("preflight_out_of_order"));
                }
                self.preflight_started = true;
                Ok(())
            }
            MarkerEventName::PreflightSucceeded | MarkerEventName::PreflightFailed => {
                if !self.preflight_started || self.preflight_finished {
                    return Err(violation("preflight_out_of_order"));
                }
                self.preflight_finished = true;
                Ok(())
            }
            MarkerEventName::ModuleStarted => {
                if !self.preflight_finished || self.active_module.is_some() {
                    return Err(violation("module_out_of_order"));
                }
                let module = required_field(marker.module.as_deref(), "module")?;
                if self.finished_modules.contains(module) {
                    return Err(violation("module_duplicate"));
                }
                self.active_module = Some(module.to_owned());
                Ok(())
            }
            MarkerEventName::ModuleSucceeded | MarkerEventName::ModuleFailed => {
                let module = required_field(marker.module.as_deref(), "module")?;
                if self.active_module.as_deref() != Some(module) {
                    return Err(violation("module_mismatch"));
                }
                if self.active_step.is_some() {
                    return Err(violation("step_unfinished"));
                }
                self.active_module = None;
                self.finished_modules.insert(module.to_owned());
                Ok(())
            }
            MarkerEventName::StepStarted => {
                let module = required_field(marker.module.as_deref(), "module")?;
                if self.active_module.as_deref() != Some(module) || self.active_step.is_some() {
                    return Err(violation("step_out_of_order"));
                }
                let step_id = required_field(marker.step_id.as_deref(), "step_id")?;
                let step = required_field(marker.step.as_deref(), "step")?;
                self.active_step = Some((step_id.to_owned(), step.to_owned()));
                Ok(())
            }
            MarkerEventName::StepSucceeded | MarkerEventName::StepFailed => {
                let module = required_field(marker.module.as_deref(), "module")?;
                let step_id = required_field(marker.step_id.as_deref(), "step_id")?;
                let step = required_field(marker.step.as_deref(), "step")?;
                if self.active_module.as_deref() != Some(module)
                    || self.active_step.as_ref() != Some(&(step_id.to_owned(), step.to_owned()))
                {
                    return Err(violation("step_mismatch"));
                }
                self.active_step = None;
                Ok(())
            }
            MarkerEventName::VerificationStarted
            | MarkerEventName::VerificationSucceeded
            | MarkerEventName::VerificationFailed => {
                let module = required_field(marker.module.as_deref(), "module")?;
                if self.active_module.as_deref() != Some(module) || self.active_step.is_some() {
                    return Err(violation("verification_out_of_order"));
                }
                Ok(())
            }
        }
    }
}

fn enrich(ctx: &DeployEventContext, marker: &RawMarker) -> DeployEvent {
    let (event, status) = match marker.event {
        MarkerEventName::PreflightStarted => (
            DeployEventName::PreflightStarted,
            DeployEventStatus::Started,
        ),
        MarkerEventName::PreflightSucceeded => (
            DeployEventName::PreflightSucceeded,
            DeployEventStatus::Succeeded,
        ),
        MarkerEventName::PreflightFailed => {
            (DeployEventName::PreflightFailed, DeployEventStatus::Failed)
        }
        MarkerEventName::ModuleStarted => {
            (DeployEventName::ModuleStarted, DeployEventStatus::Started)
        }
        MarkerEventName::ModuleSucceeded => (
            DeployEventName::ModuleSucceeded,
            DeployEventStatus::Succeeded,
        ),
        MarkerEventName::ModuleFailed => (DeployEventName::ModuleFailed, DeployEventStatus::Failed),
        MarkerEventName::StepStarted => (DeployEventName::StepStarted, DeployEventStatus::Started),
        MarkerEventName::StepSucceeded => {
            (DeployEventName::StepSucceeded, DeployEventStatus::Succeeded)
        }
        MarkerEventName::StepFailed => (DeployEventName::StepFailed, DeployEventStatus::Failed),
        MarkerEventName::VerificationStarted => (
            DeployEventName::VerificationStarted,
            DeployEventStatus::Started,
        ),
        MarkerEventName::VerificationSucceeded => (
            DeployEventName::VerificationSucceeded,
            DeployEventStatus::Succeeded,
        ),
        MarkerEventName::VerificationFailed => (
            DeployEventName::VerificationFailed,
            DeployEventStatus::Failed,
        ),
    };
    base_event(ctx, event, status)
        .with_module(marker.module.clone())
        .with_module_name(marker.module_name.clone())
        .with_step(marker.step_id.clone(), marker.step.clone())
        .with_message(marker.message.clone())
        .with_failure_stage(marker.failure_stage.clone())
        .with_recovery_hint(marker.recovery_hint.clone())
        .with_candidate_release(marker.candidate_release.clone())
        .with_current_release(marker.current_release.clone())
        .with_current_switched(marker.current_switched)
}

fn base_event(
    ctx: &DeployEventContext,
    event: DeployEventName,
    status: DeployEventStatus,
) -> DeployEvent {
    DeployEvent {
        deploy_id: ctx.deploy_id.clone(),
        stage: ctx.stage.clone(),
        event,
        timestamp: Utc::now().to_rfc3339(),
        status,
        environment: ctx.environment.clone(),
        release_version: ctx.release_version.clone(),
        target: ctx.target.clone(),
        module: None,
        module_name: None,
        step_id: None,
        step: None,
        message: None,
        failure_stage: None,
        recovery_hint: None,
        candidate_release: None,
        current_release: None,
        current_switched: None,
    }
}

trait EventBuilder {
    fn with_module(self, value: Option<String>) -> Self;
    fn with_module_name(self, value: Option<String>) -> Self;
    fn with_step(self, step_id: Option<String>, step: Option<String>) -> Self;
    fn with_message(self, value: Option<String>) -> Self;
    fn with_failure_stage(self, value: Option<String>) -> Self;
    fn with_recovery_hint(self, value: Option<String>) -> Self;
    fn with_candidate_release(self, value: Option<String>) -> Self;
    fn with_current_release(self, value: Option<String>) -> Self;
    fn with_current_switched(self, value: Option<bool>) -> Self;
}

impl EventBuilder for DeployEvent {
    fn with_module(mut self, value: Option<String>) -> Self {
        self.module = value;
        self
    }
    fn with_module_name(mut self, value: Option<String>) -> Self {
        self.module_name = value;
        self
    }
    fn with_step(mut self, step_id: Option<String>, step: Option<String>) -> Self {
        self.step_id = step_id;
        self.step = step;
        self
    }
    fn with_message(mut self, value: Option<String>) -> Self {
        self.message = value;
        self
    }
    fn with_failure_stage(mut self, value: Option<String>) -> Self {
        self.failure_stage = value;
        self
    }
    fn with_recovery_hint(mut self, value: Option<String>) -> Self {
        self.recovery_hint = value;
        self
    }
    fn with_candidate_release(mut self, value: Option<String>) -> Self {
        self.candidate_release = value;
        self
    }
    fn with_current_release(mut self, value: Option<String>) -> Self {
        self.current_release = value;
        self
    }
    fn with_current_switched(mut self, value: Option<bool>) -> Self {
        self.current_switched = value;
        self
    }
}

fn required_field<'a>(value: Option<&'a str>, name: &str) -> Result<&'a str, MarkerViolation> {
    value.ok_or_else(|| violation(name))
}

fn violation(kind: &str) -> MarkerViolation {
    MarkerViolation {
        kind: kind.to_owned(),
        message: format!("marker 顺序或字段违反 {kind} 约束"),
    }
}
