use axum::http::StatusCode;
use chrono::{Duration, Utc};
use common::{admin_session, json_request, response_json, test_app};
use deploy_go_agent_protocol::{
    AgentUpgradeAck, AgentUpgradeErrorCode, AgentUpgradeReport, AgentUpgradeReportStatus,
};
use deploy_go_api::{AppState, agents::upgrades, db};
use serde_json::json;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

mod common;

async fn pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-upgrade','Upgrade Node','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,last_seen_at,protocol_version,architecture,capabilities_json,connection_generation) VALUES('agent-upgrade','node-upgrade','test',?,17,'x86_64','[\"agent_upgrade_v1\"]',1)")
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    pool
}

#[tokio::test]
async fn enqueue_is_idempotent_and_claim_creates_fenced_lock() {
    let pool = pool().await;
    let digest = format!("sha256:{}", "a".repeat(64));
    let first = upgrades::enqueue(
        &pool,
        "agent-upgrade",
        "node-upgrade",
        Some("0.3.6"),
        "0.3.7",
        &digest,
        Some("x86_64"),
    )
    .await
    .unwrap();
    assert!(first.is_some());
    assert!(
        upgrades::enqueue(
            &pool,
            "agent-upgrade",
            "node-upgrade",
            Some("0.3.6"),
            "0.3.7",
            &digest,
            Some("x86_64"),
        )
        .await
        .unwrap()
        .is_none()
    );

    let now = Utc::now().to_rfc3339();
    upgrades::refresh_waiting_state(&pool, &now).await.unwrap();
    let claim = upgrades::claim_ready_job(&pool, &now, 120)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claim.node_id, "node-upgrade");
    assert_eq!(claim.lock_epoch, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_maintenance_locks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(
        upgrades::claim_ready_job(&pool, &now, 120)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn archived_nodes_are_skipped_until_restored() {
    let pool = pool().await;
    let state =
        AppState::new(pool.clone()).with_agent_installation(common::test_agent_installation());
    let now = Utc::now().to_rfc3339();

    sqlx::query("UPDATE nodes SET archived_at=? WHERE id='node-upgrade'")
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(upgrades::scan(&state).await.unwrap(), 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_upgrade_jobs")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );

    sqlx::query("UPDATE nodes SET archived_at=NULL WHERE id='node-upgrade'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(upgrades::scan(&state).await.unwrap(), 1);
    upgrades::refresh_waiting_state(&pool, &now).await.unwrap();
    let claim = upgrades::claim_ready_job(&pool, &now, 120)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claim.node_id, "node-upgrade");
}

#[tokio::test]
async fn archived_existing_job_is_not_refreshed_or_claimed_until_restored() {
    let pool = pool().await;
    let digest = format!("sha256:{}", "e".repeat(64));
    upgrades::enqueue(
        &pool,
        "agent-upgrade",
        "node-upgrade",
        Some("0.3.6"),
        "0.3.7",
        &digest,
        Some("x86_64"),
    )
    .await
    .unwrap();
    let now = Utc::now().to_rfc3339();

    sqlx::query("UPDATE nodes SET archived_at=? WHERE id='node-upgrade'")
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
    upgrades::refresh_waiting_state(&pool, &now).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM agent_upgrade_jobs WHERE agent_id='agent-upgrade'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "queued"
    );
    assert!(
        upgrades::claim_ready_job(&pool, &now, 120)
            .await
            .unwrap()
            .is_none()
    );

    sqlx::query("UPDATE nodes SET archived_at=NULL WHERE id='node-upgrade'")
        .execute(&pool)
        .await
        .unwrap();
    upgrades::refresh_waiting_state(&pool, &now).await.unwrap();
    assert!(
        upgrades::claim_ready_job(&pool, &now, 120)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn unsupported_architecture_is_blocked_without_a_claim() {
    let pool = pool().await;
    let digest = format!("sha256:{}", "b".repeat(64));
    let job = upgrades::enqueue(
        &pool,
        "agent-upgrade",
        "node-upgrade",
        Some("0.3.6"),
        "0.3.7-arm",
        &digest,
        Some("aarch64"),
    )
    .await
    .unwrap()
    .unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM agent_upgrade_jobs WHERE id=?")
        .bind(job)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "blocked_unsupported_architecture");
    assert!(
        upgrades::claim_ready_job(&pool, &Utc::now().to_rfc3339(), 120)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn admin_upgrade_api_lists_and_retries_failed_jobs_with_csrf() {
    let (app, pool) = test_app().await;
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-api','API Node','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,agent_version,architecture) VALUES('agent-api','node-api','test','0.3.6','x86_64')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_upgrade_jobs(id,agent_id,node_id,target_version,manifest_digest,target_architecture,status,error_code,error_summary) VALUES('upgrade_api','agent-api','node-api','0.3.7',?, 'x86_64','failed','upgrade_install_failed','安装失败')")
        .bind(format!("sha256:{}", "c".repeat(64)))
        .execute(&pool)
        .await
        .unwrap();

    let (cookie, csrf) = admin_session(app.clone()).await;
    let listed = json_request(
        app.clone(),
        "GET",
        "/api/v1/agent-upgrades",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    assert_eq!(response_json(listed).await[0]["id"], "upgrade_api");

    let missing_csrf = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent-upgrades/upgrade_api/retry",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);

    let retried = json_request(
        app,
        "POST",
        "/api/v1/agent-upgrades/upgrade_api/retry",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(retried.status(), StatusCode::OK);
    assert_eq!(response_json(retried).await["status"], "queued");
}

#[tokio::test]
async fn retry_rejects_failed_job_when_target_is_already_installed() {
    let (app, pool) = test_app().await;
    sqlx::query(
        "INSERT INTO nodes(id,name,status) VALUES('node-installed','Installed Node','online')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,agent_version,architecture) VALUES('agent-installed','node-installed','test','0.3.7','x86_64')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_upgrade_jobs(id,agent_id,node_id,target_version,manifest_digest,target_architecture,status,error_code,error_summary) VALUES('upgrade-installed','agent-installed','node-installed','0.3.7',?, 'x86_64','failed','upgrade_install_failed','安装失败')")
        .bind(format!("sha256:{}", "f".repeat(64)))
        .execute(&pool)
        .await
        .unwrap();

    let (cookie, csrf) = admin_session(app.clone()).await;
    let response = json_request(
        app,
        "POST",
        "/api/v1/agent-upgrades/upgrade-installed/retry",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(response).await["code"],
        "upgrade_target_already_installed"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM agent_upgrade_jobs WHERE id='upgrade-installed'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "failed"
    );
}

#[tokio::test]
async fn agent_api_reports_latest_when_installed_version_has_failed_history() {
    let (app, pool) = test_app().await;
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-latest','Latest Node','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,architecture,capabilities_json) VALUES('agent-latest','node-latest','test','0.3.11',17,'x86_64','[\"agent_upgrade_v1\"]')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_upgrade_jobs(id,agent_id,node_id,target_version,manifest_digest,target_architecture,status,error_code) VALUES('upgrade-latest','agent-latest','node-latest','0.3.11',?,'x86_64','failed','upgrade_manifest_invalid')")
        .bind(format!("sha256:{}", "3".repeat(64)))
        .execute(&pool)
        .await
        .unwrap();

    let (cookie, _) = admin_session(app.clone()).await;
    let response = json_request(
        app,
        "GET",
        "/api/v1/agents",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let agent = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "agent-latest")
        .unwrap();
    assert_eq!(agent["agent_version"], "0.3.11");
    assert_eq!(agent["agent_upgrade"]["state"], "latest");
    assert!(agent["agent_upgrade"]["phase"].is_null());
    assert!(agent["agent_upgrade"]["error_code"].is_null());
    assert!(agent["agent_upgrade"]["error_summary"].is_null());
}

#[tokio::test]
async fn agent_api_reports_latest_immediately_when_waiting_target_is_installed() {
    let (app, pool) = test_app().await;
    sqlx::query("INSERT INTO nodes(id,name,status) VALUES('node-waiting-latest','Waiting Latest Node','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,agent_version,protocol_version,architecture,capabilities_json) VALUES('agent-waiting-latest','node-waiting-latest','test','0.3.11',17,'x86_64','[\"agent_upgrade_v1\"]')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_upgrade_jobs(id,agent_id,node_id,target_version,manifest_digest,target_architecture,status,phase) VALUES('upgrade-waiting-latest','agent-waiting-latest','node-waiting-latest','0.3.11',?,'x86_64','waiting_for_online','validating')")
        .bind(format!("sha256:{}", "5".repeat(64)))
        .execute(&pool)
        .await
        .unwrap();

    let (cookie, _) = admin_session(app.clone()).await;
    let response = json_request(
        app,
        "GET",
        "/api/v1/agents/agent-waiting-latest",
        json!({}),
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let agent = response_json(response).await;
    assert_eq!(agent["agent_upgrade"]["state"], "latest");
    assert!(agent["agent_upgrade"]["phase"].is_null());
}

#[tokio::test]
async fn scan_reconciles_waiting_job_when_target_was_installed_manually() {
    let pool = pool().await;
    sqlx::query("UPDATE agents SET agent_version='0.3.11' WHERE id='agent-upgrade'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_upgrade_jobs(id,agent_id,node_id,target_version,manifest_digest,target_architecture,status,current_version) VALUES('upgrade-manual','agent-upgrade','node-upgrade','0.3.11',?,'x86_64','waiting_for_online','0.3.7')")
        .bind(format!("sha256:{}", "4".repeat(64)))
        .execute(&pool)
        .await
        .unwrap();
    let state =
        AppState::new(pool.clone()).with_agent_installation(common::test_agent_installation());

    assert_eq!(upgrades::scan(&state).await.unwrap(), 0);
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM agent_upgrade_jobs WHERE id='upgrade-manual'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "succeeded"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT summary FROM agent_upgrade_events WHERE job_id='upgrade-manual'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "Agent 已通过外部安装达到目标版本"
    );
}

#[tokio::test]
async fn claim_skips_job_when_target_was_installed_after_waiting_refresh() {
    let pool = pool().await;
    let digest = format!("sha256:{}", "6".repeat(64));
    let job_id = upgrades::enqueue(
        &pool,
        "agent-upgrade",
        "node-upgrade",
        Some("0.3.6"),
        "0.3.7",
        &digest,
        Some("x86_64"),
    )
    .await
    .unwrap()
    .unwrap();
    let now = Utc::now().to_rfc3339();
    upgrades::refresh_waiting_state(&pool, &now).await.unwrap();
    sqlx::query("UPDATE agents SET agent_version='0.3.7' WHERE id='agent-upgrade'")
        .execute(&pool)
        .await
        .unwrap();

    assert!(
        upgrades::claim_ready_job(&pool, &now, 120)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_upgrade_jobs WHERE id=?")
            .bind(&job_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "queued"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_maintenance_locks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_upgrade_leases")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn rejected_ack_fails_job_and_releases_lease_and_node_lock() {
    let pool = pool().await;
    let digest = format!("sha256:{}", "1".repeat(64));
    let job_id = upgrades::enqueue(
        &pool,
        "agent-upgrade",
        "node-upgrade",
        Some("0.3.6"),
        "0.3.7",
        &digest,
        Some("x86_64"),
    )
    .await
    .unwrap()
    .unwrap();
    let now = Utc::now().to_rfc3339();
    upgrades::refresh_waiting_state(&pool, &now).await.unwrap();
    let claim = upgrades::claim_ready_job(&pool, &now, 120)
        .await
        .unwrap()
        .unwrap();

    upgrades::handle_ack(
        &pool,
        "agent-upgrade",
        claim.connection_generation,
        &AgentUpgradeAck {
            job_id: job_id.clone(),
            accepted: false,
            error_code: Some(AgentUpgradeErrorCode::UpgradeExecutorRejected),
        },
    )
    .await
    .unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_upgrade_jobs WHERE id=?")
            .bind(&job_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "failed"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_upgrade_leases WHERE lease_key='global' AND job_id=?"
        )
        .bind(&job_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_maintenance_locks WHERE node_id='node-upgrade' AND job_id=?"
        )
        .bind(&job_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn report_from_current_generation_completes_previous_generation_job() {
    let pool = pool().await;
    let digest = format!("sha256:{}", "2".repeat(64));
    let job_id = upgrades::enqueue(
        &pool,
        "agent-upgrade",
        "node-upgrade",
        Some("0.3.6"),
        "0.3.7",
        &digest,
        Some("x86_64"),
    )
    .await
    .unwrap()
    .unwrap();
    let now = Utc::now().to_rfc3339();
    upgrades::refresh_waiting_state(&pool, &now).await.unwrap();
    let claim = upgrades::claim_ready_job(&pool, &now, 120)
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE agents SET connection_generation=? WHERE id='agent-upgrade'")
        .bind(claim.connection_generation + 1)
        .execute(&pool)
        .await
        .unwrap();
    let report = AgentUpgradeReport {
        job_id: job_id.clone(),
        status: AgentUpgradeReportStatus::Succeeded,
        target_version: "0.3.7".to_owned(),
        manifest_digest: digest,
        error_code: None,
        error_summary: None,
    };

    assert!(
        !upgrades::handle_report(&pool, "agent-upgrade", claim.connection_generation, &report)
            .await
            .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_upgrade_jobs WHERE id=?")
            .bind(&job_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "downloading"
    );

    assert!(
        upgrades::handle_report(
            &pool,
            "agent-upgrade",
            claim.connection_generation + 1,
            &report,
        )
        .await
        .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM agent_upgrade_jobs WHERE id=?")
            .bind(&job_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "succeeded"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_upgrade_leases WHERE lease_key='global' AND job_id=?"
        )
        .bind(&job_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_maintenance_locks WHERE node_id='node-upgrade' AND job_id=?"
        )
        .bind(&job_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn admin_can_recover_installing_job_without_reusing_install_request() {
    let (app, pool) = test_app().await;
    sqlx::query(
        "INSERT INTO nodes(id,name,status) VALUES('node-recover','Recovery Node','online')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,environment,agent_version,architecture) VALUES('agent-recover','node-recover','test','0.3.6','x86_64')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_upgrade_jobs(id,agent_id,node_id,target_version,manifest_digest,target_architecture,status,lease_token) VALUES('upgrade_recover','agent-recover','node-recover','0.3.7',?, 'x86_64','installing','lease-recover')")
        .bind(format!("sha256:{}", "d".repeat(64))).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO agent_maintenance_locks(node_id,agent_id,job_id,lease_token,lock_epoch,reason) VALUES('node-recover','agent-recover','upgrade_recover','lease-recover',1,'agent_upgrade')")
        .execute(&pool).await.unwrap();

    let (cookie, csrf) = admin_session(app.clone()).await;
    let recovered = json_request(
        app,
        "POST",
        "/api/v1/agent-upgrades/upgrade_recover/recover",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(recovered.status(), StatusCode::OK);
    assert_eq!(
        response_json(recovered).await["error_code"],
        "upgrade_manual_recovery_required"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_maintenance_locks WHERE job_id='upgrade_recover'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn expired_installing_job_is_failed_and_unlocks_node() {
    let pool = pool().await;
    let expired_at = (Utc::now() - Duration::seconds(1)).to_rfc3339();
    sqlx::query("INSERT INTO agent_upgrade_jobs(id,agent_id,node_id,target_version,manifest_digest,target_architecture,status,lease_token,lease_expires_at) VALUES('upgrade_expired_install','agent-upgrade','node-upgrade','0.3.11',?,'x86_64','installing','lease-expired',?)")
        .bind(format!("sha256:{}", "f".repeat(64)))
        .bind(&expired_at)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_maintenance_locks(node_id,agent_id,job_id,lease_token,lock_epoch,reason) VALUES('node-upgrade','agent-upgrade','upgrade_expired_install','lease-expired',1,'agent_upgrade')")
        .execute(&pool)
        .await
        .unwrap();

    upgrades::recover_expired_upgrades(&pool).await.unwrap();

    let job = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT status,error_code FROM agent_upgrade_jobs WHERE id='upgrade_expired_install'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        job,
        (
            "failed".to_owned(),
            Some("upgrade_install_lease_expired".to_owned())
        )
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_maintenance_locks WHERE job_id='upgrade_expired_install'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn scan_supersedes_pending_jobs_for_an_older_release() {
    let pool = pool().await;
    sqlx::query("INSERT INTO agent_upgrade_jobs(id,agent_id,node_id,target_version,manifest_digest,target_architecture,status) VALUES('upgrade_old_target','agent-upgrade','node-upgrade','0.3.9',?,'x86_64','queued')")
        .bind(format!("sha256:{}", "9".repeat(64)))
        .execute(&pool)
        .await
        .unwrap();
    let state =
        AppState::new(pool.clone()).with_agent_installation(common::test_agent_installation());

    assert_eq!(upgrades::scan(&state).await.unwrap(), 1);

    let old_job = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT status,error_code FROM agent_upgrade_jobs WHERE id='upgrade_old_target'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        old_job,
        (
            "failed".to_owned(),
            Some("upgrade_target_superseded".to_owned())
        )
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_upgrade_jobs WHERE target_version='0.3.11' AND status='queued'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}
