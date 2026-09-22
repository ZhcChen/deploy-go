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
fn unknown_event_is_ignored_but_duplicate_module_is_rejected() {
    let context = context();
    let mut state = MarkerState::new();
    let unknown = process_line(
        &marker(r#"{"schema_version":1,"event":"deploy.unknown"}"#),
        &context,
        &mut state,
    )
    .expect("未知事件应降级为普通日志");
    assert!(unknown.is_none());
    assert_eq!(state.diagnostics, vec!["unknown_event"]);
    for line in [
        r#"{"schema_version":1,"event":"deploy.preflight.started"}"#,
        r#"{"schema_version":1,"event":"deploy.preflight.succeeded"}"#,
        r#"{"schema_version":1,"event":"deploy.module.started","module":"api"}"#,
        r#"{"schema_version":1,"event":"deploy.module.succeeded","module":"api"}"#,
    ] {
        process_line(&marker(line), &context, &mut state).unwrap();
    }
    let (finished, error) = finished_event(&context, &state, true);
    assert!(error.is_none());
    assert_eq!(finished.message.as_deref(), Some("unknown_event"));
    let duplicate = process_line(
        &marker(r#"{"schema_version":1,"event":"deploy.module.started","module":"api"}"#),
        &context,
        &mut state,
    )
    .expect_err("重复模块必须被拒绝");
    assert_eq!(duplicate.kind, "module_duplicate");
}

#[test]
fn unknown_marker_fields_are_ignored() {
    let context = context();
    let mut state = MarkerState::new();
    let event = process_line(
        &marker(
            r#"{"schema_version":1,"event":"deploy.preflight.started","future_field":"ignored"}"#,
        ),
        &context,
        &mut state,
    )
    .expect("未知字段不应破坏 marker 解析")
    .expect("已知事件仍应生成标准事件");
    assert_eq!(event.event, DeployEventName::PreflightStarted);
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

#[test]
fn identical_step_failure_replay_is_idempotent() {
    let context = context();
    let mut state = MarkerState::new();
    for line in [
        r#"{"schema_version":1,"event":"deploy.preflight.started"}"#,
        r#"{"schema_version":1,"event":"deploy.preflight.succeeded"}"#,
        r#"{"schema_version":1,"event":"deploy.module.started","module":"api"}"#,
        r#"{"schema_version":1,"event":"deploy.step.started","module":"api","step_id":"api.remote.seed","step":"执行 API seed"}"#,
    ] {
        process_line(&marker(line), &context, &mut state).unwrap();
    }

    let failed = r#"{"schema_version":1,"event":"deploy.step.failed","module":"api","step_id":"api.remote.seed","step":"执行 API seed","failure_stage":"api.remote.seed","message":"远端部署失败"}"#;
    assert!(
        process_line(&marker(failed), &context, &mut state)
            .unwrap()
            .is_some()
    );
    assert!(
        process_line(&marker(failed), &context, &mut state)
            .unwrap()
            .is_none()
    );

    process_line(
        &marker(
            r#"{"schema_version":1,"event":"deploy.module.failed","module":"api","message":"发布阶段失败"}"#,
        ),
        &context,
        &mut state,
    )
    .unwrap();
    let (finished, error) = finished_event(&context, &state, false);
    assert_eq!(finished.status, DeployEventStatus::Failed);
    assert!(error.is_none());
}

#[test]
fn changed_or_conflicting_step_failure_replay_is_rejected() {
    let context = context();
    let mut state = MarkerState::new();
    for line in [
        r#"{"schema_version":1,"event":"deploy.preflight.started"}"#,
        r#"{"schema_version":1,"event":"deploy.preflight.succeeded"}"#,
        r#"{"schema_version":1,"event":"deploy.module.started","module":"api"}"#,
        r#"{"schema_version":1,"event":"deploy.step.started","module":"api","step_id":"api.remote.seed","step":"执行 API seed"}"#,
        r#"{"schema_version":1,"event":"deploy.step.failed","module":"api","step_id":"api.remote.seed","step":"执行 API seed","message":"第一次失败"}"#,
    ] {
        process_line(&marker(line), &context, &mut state).unwrap();
    }

    for conflicting in [
        r#"{"schema_version":1,"event":"deploy.step.failed","module":"api","step_id":"api.remote.seed","step":"执行 API seed","message":"第二次失败"}"#,
        r#"{"schema_version":1,"event":"deploy.step.failed","module":"api","step_id":"api.remote.migrate","step":"执行 migration","message":"第一次失败"}"#,
        r#"{"schema_version":1,"event":"deploy.step.succeeded","module":"api","step_id":"api.remote.seed","step":"执行 API seed"}"#,
    ] {
        let violation = process_line(&marker(conflicting), &context, &mut state)
            .expect_err("冲突终态 marker 必须被拒绝");
        assert_eq!(violation.kind, "step_mismatch");
    }
}

#[test]
fn module_failure_implicitly_closes_the_active_step() {
    let context = context();
    let mut state = MarkerState::new();
    for line in [
        r#"{"schema_version":1,"event":"deploy.preflight.started"}"#,
        r#"{"schema_version":1,"event":"deploy.preflight.succeeded"}"#,
        r#"{"schema_version":1,"event":"deploy.module.started","module":"admin","module_name":"平台后台"}"#,
        r#"{"schema_version":1,"event":"deploy.step.started","module":"admin","step_id":"admin.remote.prepare_candidate","step":"准备候选 release","failure_stage":"admin.remote.prepare_candidate"}"#,
        r#"{"schema_version":1,"event":"deploy.module.failed","module":"admin","module_name":"平台后台","message":"发布阶段失败"}"#,
    ] {
        process_line(&marker(line), &context, &mut state).unwrap();
    }

    let (finished, error) = finished_event(&context, &state, false);
    assert_eq!(finished.status, DeployEventStatus::Failed);
    assert!(error.is_none());
}

#[test]
fn module_success_still_rejects_an_unfinished_step() {
    let context = context();
    let mut state = MarkerState::new();
    for line in [
        r#"{"schema_version":1,"event":"deploy.preflight.started"}"#,
        r#"{"schema_version":1,"event":"deploy.preflight.succeeded"}"#,
        r#"{"schema_version":1,"event":"deploy.module.started","module":"admin"}"#,
        r#"{"schema_version":1,"event":"deploy.step.started","module":"admin","step_id":"admin.build","step":"构建 admin"}"#,
    ] {
        process_line(&marker(line), &context, &mut state).unwrap();
    }

    let violation = process_line(
        &marker(r#"{"schema_version":1,"event":"deploy.module.succeeded","module":"admin"}"#),
        &context,
        &mut state,
    )
    .expect_err("成功 module 不得隐式关闭未完成 step");
    assert_eq!(violation.kind, "step_unfinished");
}

#[test]
fn step_failure_replay_after_another_marker_is_rejected() {
    let context = context();
    let mut state = MarkerState::new();
    for line in [
        r#"{"schema_version":1,"event":"deploy.preflight.started"}"#,
        r#"{"schema_version":1,"event":"deploy.preflight.succeeded"}"#,
        r#"{"schema_version":1,"event":"deploy.module.started","module":"admin"}"#,
        r#"{"schema_version":1,"event":"deploy.step.started","module":"admin","step_id":"admin.verify","step":"验证 admin"}"#,
        r#"{"schema_version":1,"event":"deploy.step.failed","module":"admin","step_id":"admin.verify","step":"验证 admin","message":"验证失败"}"#,
        r#"{"schema_version":1,"event":"deploy.verification.started","module":"admin"}"#,
    ] {
        process_line(&marker(line), &context, &mut state).unwrap();
    }

    let violation = process_line(
        &marker(
            r#"{"schema_version":1,"event":"deploy.step.failed","module":"admin","step_id":"admin.verify","step":"验证 admin","message":"验证失败"}"#,
        ),
        &context,
        &mut state,
    )
    .expect_err("被其他 marker 隔开的失败终态不得视为紧邻重放");
    assert_eq!(violation.kind, "step_mismatch");
}
