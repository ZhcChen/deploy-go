mod common;

use axum::http::StatusCode;
use common::{admin_session, json_request, response_json, test_app};
use deploy_go_agent_protocol::{
    DeployEvent, DeployEventName, DeployEventStatus, DeploymentStage, Environment, Message,
    OutputStream, TaskLifecycleState, TaskOutput, TaskProgress, TaskResult, TaskState,
    TaskTerminalStatus,
};
use deploy_go_api::{
    AppState,
    agents::dispatcher::{handle_agent_message, request_deployment_cancel},
    db,
    deployments::process_one,
};
use serde_json::json;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

const SHA_MAIN: &str = "0123456789abcdef0123456789abcdef01234567";

async fn seed(pool: &SqlitePool) {
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_two','Two Stage App','two-stage-app','active')").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_two','Two Stage Node','/srv/apps','/srv/secrets','online')").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,connection_generation) VALUES('agent_two','node_two','2026-08-03T00:00:00Z','2026-08-03T00:00:00Z','0.2.0',2,2)").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO application_sources(id,application_id,repository_url,build_agent_id,source_policy,deployment_branch,source_version,status,version) VALUES('source_two','app_two','git@git.example.test:deploy-go/example.git','agent_two','branch','main',1,'verified',1)").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task_refs','agent_two','git_refs_query','git-refs:source_two:refs_1','sha256:refs','{}','succeeded','2099-08-06T00:00:00Z')").execute(pool).await.unwrap();
    let refs = json!([{"name":"main","ref":"refs/heads/main","sha":SHA_MAIN},{"name":"develop","ref":"refs/heads/develop","sha":"abcdefabcdefabcdefabcdefabcdefabcdefabcd"}]);
    sqlx::query("INSERT INTO git_ref_discoveries(id,application_source_id,source_version,task_id,status,refs_json,expires_at,finished_at) VALUES('refs_two','source_two',1,'task_refs','succeeded',?,'2099-08-06T00:00:00Z','2026-08-06T00:00:00Z')")
        .bind(refs.to_string())
        .execute(pool)
        .await
        .unwrap();
    let schema = json!({"type":"object","properties":{"release-version":{"type":"string","maxLength":32},"modules":{"type":"string","maxLength":512}},"required":["release-version","modules"],"additionalProperties":false});
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,parameter_schema,timeout_seconds,verification_config,status) VALUES('target_two','app_two','node_two','test','two_stage','/srv/apps/deploy.sh',?,?,?,'active')")
        .bind(schema.to_string())
        .bind(900)
        .bind("{}".to_owned())
        .execute(pool)
        .await
        .unwrap();
}

async fn fixture() -> (AppState, SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    seed(&pool).await;
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
    (AppState::new(pool.clone()), pool)
}

fn two_stage_parameters() -> serde_json::Value {
    json!({"release-version":"20260806183000","modules":"api,admin"})
}

fn deployment_snapshot() -> serde_json::Value {
    json!({
        "target": {
            "application_id": "app_two",
            "node_id": "node_two",
            "environment": "test",
            "script_path": "/srv/apps/deploy.sh",
            "parameter_schema": {"type":"object","additionalProperties":false},
            "timeout_seconds": 900,
            "verification_config": {},
            "secret_file_references": [],
            "version": 1
        },
        "target_id": "target_two",
        "application_name": "Two Stage App",
        "node_name": "Two Stage Node",
        "execution_mode": "two_stage",
        "source": {
            "source_id": "source_two",
            "source_version": 1,
            "source_policy": "branch",
            "repository_url": "git@git.example.test:deploy-go/example.git",
            "git_credential_id": null,
            "build_agent_id": "agent_two",
            "deployment_branch": "main",
            "requested_ref": "refs/heads/main",
            "resolved_commit_sha": SHA_MAIN,
            "refs_discovery_id": "refs_two"
        },
        "two_stage": {
            "release_version": "20260806183000",
            "modules": ["api", "admin"]
        },
        "parameters": two_stage_parameters()
    })
}

async fn insert_deployment(pool: &SqlitePool, id: &str) {
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES(?,?,'admin','queued','queued',?,?,?,?)")
        .bind(id)
        .bind("target_two")
        .bind(format!("request-{id}"))
        .bind(id)
        .bind(id)
        .bind(deployment_snapshot().to_string())
        .execute(pool)
        .await
        .unwrap();
}

async fn complete_task(
    state: &AppState,
    task_id: &str,
    terminal: TaskTerminalStatus,
    summary: &str,
) {
    let exit_code = Some(if terminal == TaskTerminalStatus::Succeeded {
        0
    } else {
        1
    });
    handle_agent_message(
        state,
        "agent_two",
        2,
        &Message::TaskState(TaskState {
            task_id: task_id.to_owned(),
            sequence: 1,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        state,
        "agent_two",
        2,
        &Message::TaskResult(TaskResult {
            task_id: task_id.to_owned(),
            sequence: 2,
            status: terminal,
            exit_code,
            error_code: None,
            summary: Some(summary.to_owned()),
            data: None,
        }),
    )
    .await
    .unwrap();
}

async fn run_stage_to(
    state: &AppState,
    pool: &SqlitePool,
    deployment_id: &str,
    stage: &str,
    terminal: TaskTerminalStatus,
) -> String {
    sqlx::query("UPDATE agent_tasks SET status='running' WHERE deployment_id=? AND stage=?")
        .bind(deployment_id)
        .bind(stage)
        .execute(pool)
        .await
        .unwrap();
    let task_id: String =
        sqlx::query_scalar("SELECT id FROM agent_tasks WHERE deployment_id=? AND stage=?")
            .bind(deployment_id)
            .bind(stage)
            .fetch_one(pool)
            .await
            .unwrap();
    complete_task(state, &task_id, terminal, stage).await;
    task_id
}

async fn run_stage_with_output(
    state: &AppState,
    pool: &SqlitePool,
    deployment_id: &str,
    stage: &str,
    output: &str,
) {
    sqlx::query("UPDATE agent_tasks SET status='running' WHERE deployment_id=? AND stage=?")
        .bind(deployment_id)
        .bind(stage)
        .execute(pool)
        .await
        .unwrap();
    let task_id: String =
        sqlx::query_scalar("SELECT id FROM agent_tasks WHERE deployment_id=? AND stage=?")
            .bind(deployment_id)
            .bind(stage)
            .fetch_one(pool)
            .await
            .unwrap();
    handle_agent_message(
        state,
        "agent_two",
        2,
        &Message::TaskState(TaskState {
            task_id: task_id.clone(),
            sequence: 1,
            state: TaskLifecycleState::Running,
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        state,
        "agent_two",
        2,
        &Message::TaskOutput(TaskOutput {
            task_id: task_id.clone(),
            sequence: 2,
            stream: OutputStream::Stdout,
            text: output.to_owned(),
        }),
    )
    .await
    .unwrap();
    handle_agent_message(
        state,
        "agent_two",
        2,
        &Message::TaskResult(TaskResult {
            task_id,
            sequence: 3,
            status: TaskTerminalStatus::Succeeded,
            exit_code: Some(0),
            error_code: None,
            summary: Some(stage.to_owned()),
            data: None,
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn two_stage_preview_resolves_fixed_branch_and_commit() {
    let (app, pool) = test_app().await;
    seed(&pool).await;
    let (cookie, _csrf) = admin_session(app.clone()).await;
    let preview = json_request(
        app,
        "POST",
        "/api/v1/deployment-targets/target_two/deployment-preview",
        json!({"parameters": two_stage_parameters()}),
        &[("cookie", &cookie)],
    )
    .await;
    if preview.status() != StatusCode::OK {
        println!("preview error: {}", response_json(preview).await);
        panic!("preview failed");
    }
    let body = response_json(preview).await;
    assert_eq!(body["execution_mode"], "two_stage");
    assert_eq!(body["deployment_branch"], "main");
    assert_eq!(body["resolved_commit_sha"], SHA_MAIN);
    assert_eq!(body["modules"], json!(["api", "admin"]));
    assert!(!body["snapshot_hash"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn confirm_and_worker_run_prepare_then_release_to_success() {
    let (app, pool) = test_app().await;
    seed(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let preview = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/deployment-targets/target_two/deployment-preview",
            json!({"parameters": two_stage_parameters()}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    let created = json_request(
        app,
        "POST",
        "/api/v1/deployment-targets/target_two/deployments",
        json!({
            "parameters": two_stage_parameters(),
            "snapshot_hash": preview["snapshot_hash"]
        }),
        &[
            ("cookie", &cookie),
            ("x-csrf-token", &csrf),
            ("idempotency-key", "two-stage-request-0001"),
        ],
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let deployment_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let state = AppState::new(pool.clone());

    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id.as_str())
    );
    let prepare: (String, String, String) = sqlx::query_as(
        "SELECT kind,stage,status FROM agent_tasks WHERE deployment_id=? AND stage='prepare'",
    )
    .bind(&deployment_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        prepare,
        (
            "deployment_prepare".to_owned(),
            "prepare".to_owned(),
            "queued".to_owned()
        )
    );
    run_stage_to(
        &state,
        &pool,
        &deployment_id,
        "prepare",
        TaskTerminalStatus::Succeeded,
    )
    .await;
    let phase: String = sqlx::query_scalar("SELECT phase FROM deployments WHERE id=?")
        .bind(&deployment_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(phase, "deploying");

    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id.as_str())
    );
    let release: (String, String, String) = sqlx::query_as(
        "SELECT kind,stage,status FROM agent_tasks WHERE deployment_id=? AND stage='release'",
    )
    .bind(&deployment_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        release,
        (
            "deployment_release".to_owned(),
            "release".to_owned(),
            "queued".to_owned()
        )
    );
    run_stage_to(
        &state,
        &pool,
        &deployment_id,
        "release",
        TaskTerminalStatus::Succeeded,
    )
    .await;
    let deployment: (String, String, bool) =
        sqlx::query_as("SELECT status,phase,protocol_complete FROM deployments WHERE id=?")
            .bind(&deployment_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        deployment,
        ("succeeded".to_owned(), "succeeded".to_owned(), true)
    );
}

#[tokio::test]
async fn prepare_failure_never_creates_release() {
    let (state, pool) = fixture().await;
    insert_deployment(&pool, "deployment_fail").await;
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some("deployment_fail")
    );
    run_stage_to(
        &state,
        &pool,
        "deployment_fail",
        "prepare",
        TaskTerminalStatus::Failed,
    )
    .await;
    assert_eq!(process_one(&state).await.unwrap(), None);
    let deployment: (String, String) =
        sqlx::query_as("SELECT status,phase FROM deployments WHERE id='deployment_fail'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(deployment, ("failed".to_owned(), "failed".to_owned()));
    let release_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='deployment_fail' AND stage='release'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(release_count, 0);
}

#[tokio::test]
async fn cancel_between_stages_blocks_release_creation() {
    let (state, pool) = fixture().await;
    insert_deployment(&pool, "deployment_cancel").await;
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some("deployment_cancel")
    );
    run_stage_to(
        &state,
        &pool,
        "deployment_cancel",
        "prepare",
        TaskTerminalStatus::Succeeded,
    )
    .await;
    request_deployment_cancel(&state, "deployment_cancel")
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM deployments WHERE id='deployment_cancel'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "canceled"
    );
    assert_eq!(process_one(&state).await.unwrap(), None);
    let release_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_tasks WHERE deployment_id='deployment_cancel' AND stage='release'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(release_count, 0);
}

#[tokio::test]
async fn retry_reuses_original_commit_after_branch_moves() {
    let (app, pool) = test_app().await;
    seed(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let preview = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/deployment-targets/target_two/deployment-preview",
            json!({"parameters": two_stage_parameters()}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    let created = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/deployment-targets/target_two/deployments",
            json!({
                "parameters": two_stage_parameters(),
                "snapshot_hash": preview["snapshot_hash"]
            }),
            &[
                ("cookie", &cookie),
                ("x-csrf-token", &csrf),
                ("idempotency-key", "two-stage-retry-0001"),
            ],
        )
        .await,
    )
    .await;
    let original_id = created["id"].as_str().unwrap().to_owned();
    sqlx::query("UPDATE deployments SET status='failed',phase='failed' WHERE id=?")
        .bind(&original_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE git_ref_discoveries SET refs_json=? WHERE id='refs_two'")
        .bind(
            json!([{"name":"main","ref":"refs/heads/main","sha":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}])
                .to_string(),
        )
        .execute(&pool)
        .await
        .unwrap();
    let retried = response_json(
        json_request(
            app,
            "POST",
            &format!("/api/v1/deployments/{original_id}/retry"),
            json!({}),
            &[
                ("cookie", &cookie),
                ("x-csrf-token", &csrf),
                ("idempotency-key", "two-stage-retry-0002"),
            ],
        )
        .await,
    )
    .await;
    assert_eq!(retried["retry_of_id"], original_id);
    let snapshot: serde_json::Value =
        sqlx::query_scalar("SELECT snapshot_json FROM deployments WHERE id=?")
            .bind(retried["id"].as_str().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(snapshot["source"]["resolved_commit_sha"], SHA_MAIN);
}

#[tokio::test]
async fn task_progress_persists_events_and_advances_verifying_phase() {
    let (state, pool) = fixture().await;
    insert_deployment(&pool, "deployment_progress").await;
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some("deployment_progress")
    );
    run_stage_to(
        &state,
        &pool,
        "deployment_progress",
        "prepare",
        TaskTerminalStatus::Succeeded,
    )
    .await;
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some("deployment_progress")
    );
    let release_id: String = sqlx::query_scalar(
        "SELECT id FROM agent_tasks WHERE deployment_id='deployment_progress' AND stage='release'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE agent_tasks SET status='running' WHERE id=?")
        .bind(&release_id)
        .execute(&pool)
        .await
        .unwrap();
    handle_agent_message(
        &state,
        "agent_two",
        2,
        &Message::TaskProgress(TaskProgress {
            task_id: release_id.clone(),
            sequence: 1,
            event: DeployEvent {
                deploy_id: "deployment_progress".to_owned(),
                stage: DeploymentStage::Release,
                event: DeployEventName::VerificationStarted,
                timestamp: "2026-08-06T12:00:00Z".to_owned(),
                status: DeployEventStatus::Started,
                environment: Environment::Test,
                release_version: "20260806183000".to_owned(),
                target: None,
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
            },
        }),
    )
    .await
    .unwrap();
    let (event_name, status, phase): (String, String, String) = sqlx::query_as(
        "SELECT e.event_name,e.status,d.phase FROM deployment_events e JOIN deployments d ON d.id=e.deployment_id WHERE e.deployment_id='deployment_progress' AND e.event_name='deploy.verification.started'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(event_name, "deploy.verification.started");
    assert_eq!(status, "started");
    assert_eq!(phase, "verifying");
}

#[tokio::test]
async fn deployment_detail_exposes_two_stage_snapshot_and_stage_tasks() {
    let (app, pool) = test_app().await;
    seed(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let preview = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/deployment-targets/target_two/deployment-preview",
            json!({"parameters": two_stage_parameters()}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    let created = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/deployment-targets/target_two/deployments",
            json!({
                "parameters": two_stage_parameters(),
                "snapshot_hash": preview["snapshot_hash"]
            }),
            &[
                ("cookie", &cookie),
                ("x-csrf-token", &csrf),
                ("idempotency-key", "two-stage-detail-0001"),
            ],
        )
        .await,
    )
    .await;
    let deployment_id = created["id"].as_str().unwrap();
    let state = AppState::new(pool.clone());
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id)
    );
    run_stage_to(
        &state,
        &pool,
        deployment_id,
        "prepare",
        TaskTerminalStatus::Succeeded,
    )
    .await;
    let detail = response_json(
        json_request(
            app.clone(),
            "GET",
            &format!("/api/v1/deployments/{deployment_id}"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(detail["execution_mode"], "two_stage");
    assert_eq!(detail["deployment_branch"], "main");
    assert_eq!(detail["resolved_commit_sha"], SHA_MAIN);
    assert_eq!(detail["release_version"], "20260806183000");
    assert_eq!(detail["modules"], json!(["api", "admin"]));
    let prepare = &detail["stage_tasks"][0];
    assert_eq!(prepare["stage"], "prepare");
    assert_eq!(prepare["status"], "succeeded");

    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id)
    );
    run_stage_to(
        &state,
        &pool,
        deployment_id,
        "release",
        TaskTerminalStatus::Succeeded,
    )
    .await;
    let finished = response_json(
        json_request(
            app,
            "GET",
            &format!("/api/v1/deployments/{deployment_id}"),
            json!({}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    assert_eq!(finished["status"], "succeeded");
    let stages: Vec<String> = finished["stage_tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|task| task["stage"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(stages, vec!["prepare".to_owned(), "release".to_owned()]);
}

#[tokio::test]
async fn two_stage_logs_from_prepare_and_release_are_both_kept_with_stage() {
    let (app, pool) = test_app().await;
    seed(&pool).await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let preview = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/deployment-targets/target_two/deployment-preview",
            json!({"parameters": two_stage_parameters()}),
            &[("cookie", &cookie)],
        )
        .await,
    )
    .await;
    let created = response_json(
        json_request(
            app.clone(),
            "POST",
            "/api/v1/deployment-targets/target_two/deployments",
            json!({
                "parameters": two_stage_parameters(),
                "snapshot_hash": preview["snapshot_hash"]
            }),
            &[
                ("cookie", &cookie),
                ("x-csrf-token", &csrf),
                ("idempotency-key", "two-stage-logs-0001"),
            ],
        )
        .await,
    )
    .await;
    let deployment_id = created["id"].as_str().unwrap().to_owned();
    let state = AppState::new(pool.clone());
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id.as_str())
    );
    run_stage_with_output(
        &state,
        &pool,
        &deployment_id,
        "prepare",
        "prepare output\n",
    )
    .await;
    assert_eq!(
        process_one(&state).await.unwrap().as_deref(),
        Some(deployment_id.as_str())
    );
    run_stage_with_output(
        &state,
        &pool,
        &deployment_id,
        "release",
        "release output\n",
    )
    .await;
    let body = axum::body::to_bytes(
        json_request(
            app,
            "GET",
            &format!("/api/v1/deployments/{deployment_id}/logs"),
            json!({}),
            &[("cookie", &cookie), ("last-event-id", "0")],
        )
        .await
        .into_body(),
        usize::MAX,
    )
    .await
    .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("prepare output"));
    assert!(body.contains("release output"));
    assert!(body.contains(r#""stage":"prepare""#));
    assert!(body.contains(r#""stage":"release""#));
}
