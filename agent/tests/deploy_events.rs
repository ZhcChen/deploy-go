use deploy_go_agent::deploy_events::{
    DeployEventContext, MarkerState, finished_event, process_line, started_event,
};
use deploy_go_agent_protocol::{DeployEventName, DeployEventStatus, DeploymentStage, Environment};

fn context() -> DeployEventContext {
    DeployEventContext {
        deploy_id: "dep_01".to_owned(),
        stage: DeploymentStage::Prepare,
        environment: Environment::Test,
        release_version: "0.1.0".to_owned(),
        target: None,
    }
}

fn marker(body: &str) -> String {
    format!("DEPLOY_GO_EVENT {body}")
}

#[test]
fn valid_sequence_is_enriched_and_finishes_succeeded() {
    let context = context();
    let mut state = MarkerState::new();
    for line in [
        r#"{"schema_version":1,"event":"deploy.preflight.started"}"#,
        r#"{"schema_version":1,"event":"deploy.preflight.succeeded"}"#,
        r#"{"schema_version":1,"event":"deploy.module.started","module":"api","module_name":"API 服务"}"#,
        r#"{"schema_version":1,"event":"deploy.step.started","module":"api","step_id":"api.build","step":"构建 API"}"#,
        r#"{"schema_version":1,"event":"deploy.step.succeeded","module":"api","step_id":"api.build","step":"构建 API"}"#,
        r#"{"schema_version":1,"event":"deploy.verification.started","module":"api","step_id":"api.verify","step":"验证 API"}"#,
        r#"{"schema_version":1,"event":"deploy.verification.succeeded","module":"api","step_id":"api.verify","step":"验证 API"}"#,
        r#"{"schema_version":1,"event":"deploy.module.succeeded","module":"api","module_name":"API 服务"}"#,
    ] {
        let event = process_line(&marker(line), &context, &mut state)
            .unwrap()
            .expect("marker 必须生成事件");
        assert_eq!(event.deploy_id, "dep_01");
        assert_eq!(event.stage, DeploymentStage::Prepare);
        assert_eq!(event.environment, Environment::Test);
        assert_eq!(event.release_version, "0.1.0");
    }
    assert_eq!(
        started_event(&context).event,
        DeployEventName::DeployStarted
    );
    let (finished, error) = finished_event(&context, &state, true);
    assert_eq!(finished.event, DeployEventName::DeployFinished);
    assert_eq!(finished.status, DeployEventStatus::Succeeded);
    assert!(error.is_none());
}

#[test]
fn malformed_marker_is_reported_without_crashing_the_stream() {
    let context = context();
    let mut state = MarkerState::new();
    let violation = process_line("DEPLOY_GO_EVENT {not-json", &context, &mut state)
        .expect_err("畸形 marker 必须被拒绝");
    assert_eq!(violation.kind, "invalid_marker_json");
    let (_, error) = finished_event(&context, &state, true);
    assert!(error.is_some());
}

#[test]
fn unknown_event_and_duplicate_module_are_rejected() {
    let context = context();
    let mut state = MarkerState::new();
    let violation = process_line(
        &marker(r#"{"schema_version":1,"event":"deploy.unknown"}"#),
        &context,
        &mut state,
    )
    .expect_err("未知事件必须被拒绝");
    assert_eq!(violation.kind, "invalid_marker_json");

    for line in [
        r#"{"schema_version":1,"event":"deploy.preflight.started"}"#,
        r#"{"schema_version":1,"event":"deploy.preflight.succeeded"}"#,
        r#"{"schema_version":1,"event":"deploy.module.started","module":"api"}"#,
        r#"{"schema_version":1,"event":"deploy.module.succeeded","module":"api"}"#,
    ] {
        process_line(&marker(line), &context, &mut state).unwrap();
    }
    let duplicate = process_line(
        &marker(r#"{"schema_version":1,"event":"deploy.module.started","module":"api"}"#),
        &context,
        &mut state,
    )
    .expect_err("重复模块必须被拒绝");
    assert_eq!(duplicate.kind, "module_duplicate");
}

#[test]
fn out_of_order_preflight_and_step_unfinished_are_detected() {
    let context = context();
    let mut state = MarkerState::new();
    process_line(
        &marker(r#"{"schema_version":1,"event":"deploy.module.started","module":"api"}"#),
        &context,
        &mut state,
    )
    .expect_err("preflight 前启动模块必须被拒绝");

    let mut state = MarkerState::new();
    for line in [
        r#"{"schema_version":1,"event":"deploy.preflight.started"}"#,
        r#"{"schema_version":1,"event":"deploy.preflight.succeeded"}"#,
        r#"{"schema_version":1,"event":"deploy.module.started","module":"api"}"#,
        r#"{"schema_version":1,"event":"deploy.step.started","module":"api","step_id":"api.build","step":"构建"}"#,
    ] {
        process_line(&marker(line), &context, &mut state).unwrap();
    }
    let (event, error) = finished_event(&context, &state, true);
    assert_eq!(event.status, DeployEventStatus::Failed);
    assert_eq!(error.as_deref(), Some("deploy_event_protocol_conflict"));
    assert!(state.violations.is_empty());
}

#[test]
fn exit_failure_marks_finished_failed_without_protocol_error() {
    let context = context();
    let mut state = MarkerState::new();
    for line in [
        r#"{"schema_version":1,"event":"deploy.preflight.started"}"#,
        r#"{"schema_version":1,"event":"deploy.preflight.succeeded"}"#,
        r#"{"schema_version":1,"event":"deploy.module.started","module":"api"}"#,
        r#"{"schema_version":1,"event":"deploy.module.failed","module":"api"}"#,
    ] {
        process_line(&marker(line), &context, &mut state).unwrap();
    }
    let (finished, error) = finished_event(&context, &state, false);
    assert_eq!(finished.status, DeployEventStatus::Failed);
    assert!(error.is_none());
}
