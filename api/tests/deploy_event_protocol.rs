use deploy_go_agent_protocol::{
    DeployEvent, DeployEventName, DeployEventStatus, DeploymentStage, Environment, Message,
    TaskProgress,
};
use deploy_go_api::{AppState, agents::dispatcher::handle_agent_message, db};
use sqlx::sqlite::SqlitePoolOptions;

fn progress(task_id: &str, deploy_id: &str, sequence: u64) -> Message {
    Message::TaskProgress(TaskProgress {
        task_id: task_id.to_owned(),
        sequence,
        event: DeployEvent {
            deploy_id: deploy_id.to_owned(),
            stage: DeploymentStage::Release,
            event: DeployEventName::ModuleSucceeded,
            timestamp: "2026-08-06T12:00:00Z".to_owned(),
            status: DeployEventStatus::Succeeded,
            environment: Environment::Test,
            release_version: "20260806183000".to_owned(),
            target: Some("test".to_owned()),
            module: Some("api".to_owned()),
            module_name: Some("API".to_owned()),
            step_id: None,
            step: None,
            message: None,
            failure_stage: None,
            recovery_hint: None,
            candidate_release: None,
            current_release: None,
            current_switched: None,
        },
    })
}

#[tokio::test]
async fn progress_events_are_idempotent_and_bound_to_their_deployment() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app_progress','Progress App','progress-app','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_progress','Progress Node','/srv/apps','/srv/secrets','online')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,registered_at,last_seen_at,agent_version,protocol_version,connection_generation) VALUES('agent_progress','node_progress','2026-08-03T00:00:00Z','2026-08-03T00:00:00Z','0.2.0',2,2)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,parameter_schema,timeout_seconds,verification_config,status) VALUES('target_progress','app_progress','node_progress','test','/srv/apps/deploy.sh','{\"type\":\"object\",\"additionalProperties\":false}',60,'{}','active')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments(id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES('deployment_progress','target_progress','admin','running','deploying','request-progress-0001','hash','snapshot')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,stage,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES('task_progress','agent_progress','deployment_progress','release','deployment_release','deployment:deployment_progress:release','sha256:release','{}','running','2099-08-06T00:00:00Z')").execute(&pool).await.unwrap();
    let state = AppState::new(pool.clone());

    handle_agent_message(
        &state,
        "agent_progress",
        2,
        &progress("task_progress", "deployment_progress", 1),
    )
    .await
    .unwrap();
    handle_agent_message(
        &state,
        "agent_progress",
        2,
        &progress("task_progress", "deployment_progress", 1),
    )
    .await
    .unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment_events WHERE deployment_id='deployment_progress'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);

    let mismatch = handle_agent_message(
        &state,
        "agent_progress",
        2,
        &progress("task_progress", "deployment_other", 2),
    )
    .await;
    assert!(mismatch.is_err());
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM agent_task_events WHERE task_id='task_progress' AND sequence=2",
    )
    .fetch_optional(&pool)
    .await
    .unwrap()
    .unwrap_or_default();
    assert!(payload.is_empty());
}
