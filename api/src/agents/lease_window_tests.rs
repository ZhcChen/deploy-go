//! Git 凭证租约的取用窗口从“派发”开始计时，而不是从“创建任务”开始。
//!
//! 线上事故：同一节点的部署名额被占满时，新任务排队；排队超过 60 秒，租约已过期并被清理为 `expired`，
//! 任务一派发、Agent 取租约就被拒绝，任务约 0.3 秒后失败（`secret_lease_lease_not_available`）。
//! 这些测试使用真实的 `try_dispatch`、`expire_secret_leases` 和 `resolve_secret_lease`（含主密钥解密）。
use super::*;
use crate::{crypto::MasterKeyRing, db};
use chrono::DateTime;
use deploy_go_agent_protocol::SecretLeasePurpose;
use sqlx::sqlite::SqlitePoolOptions;

const CAPABILITIES: &str = r#"["pty_terminal","privileged_release"]"#;
const PRIVATE_KEY: &str = "lease-window-test-key-material";
const FAR_DEADLINE: &str = "2099-01-01T00:00:00Z";

fn execute_payload() -> TaskPayload {
    TaskPayload::DeploymentExecute(DeploymentExecuteTask {
        deployment_id: "deployment_lease".to_owned(),
        work_root: "/srv/apps".to_owned(),
        script_path: "/srv/apps/deploy.sh".to_owned(),
        argument_tokens: Vec::new(),
        environment_file_references: Vec::new(),
        timeout_seconds: 60,
        wrapper_version: "1".to_owned(),
    })
}

fn prepare_payload(lease_id: &str) -> TaskPayload {
    TaskPayload::DeploymentPrepare(DeploymentPrepareTask {
        deployment_id: "deployment_lease".into(),
        source_policy: SourcePolicy::Branch,
        source_materialization: None,
        repository_url: "https://git.example.test/app.git".into(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
        workspace_path: None,
        checkout_dir: "/srv/tasks/task/checkout".into(),
        work_root: "/srv/tasks/task".into(),
        output_dir: "/srv/tasks/task/staging".into(),
        environment: Environment::Test,
        release_version: "release-1".into(),
        modules: vec!["api".into()],
        make_target: MakeTarget::DeployGoPrepare,
        git_credential_lease_id: Some(lease_id.into()),
        timeout_seconds: 900,
        artifact_upload: None,
    })
}

fn release_payload(lease_id: &str) -> TaskPayload {
    TaskPayload::DeploymentRelease(DeploymentReleaseTask {
        deployment_id: "deployment_lease".into(),
        target_code: "test".into(),
        work_root: "/srv/deploy-go".into(),
        checkout_dir: "/srv/deploy-go/deployments/deployment/checkout".into(),
        artifact_dir: "/srv/deploy-go/deployments/deployment/staging".into(),
        environment: Environment::Test,
        release_version: "release-1".into(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
        modules: vec!["api".into()],
        make_target: MakeTarget::DeployGoRelease,
        timeout_seconds: 600,
        cancel_file: String::new(),
        privileged: false,
        privileged_context: None,
        artifact_download: None,
        repository_url: Some("https://git.example.test/app.git".into()),
        git_credential_lease_id: Some(lease_id.into()),
        application_slug: None,
        required_env: Vec::new(),
        checkout_mode: ReleaseCheckoutMode::Git,
        source_materialization: None,
        secret_environment: None,
    })
}

/// 返回 (JSON, 摘要)。
fn encode(payload: &TaskPayload) -> (String, String) {
    let json = serde_json::to_string(payload).unwrap();
    let digest = format!("sha256:{:x}", Sha256::digest(json.as_bytes()));
    (json, digest)
}

/// 一个在线 Agent（名额 1）和一把加密保存的 Git 凭证。
async fn fixture() -> (AppState, sqlx::SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node','node','/srv/apps','/srv/secrets','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,capabilities_json,connection_generation) VALUES('agent','node',18,?,1)")
        .bind(CAPABILITIES)
        .execute(&pool)
        .await
        .unwrap();
    // `deployment_prepare` 任务必须关联一条部署记录（表上的 CHECK 约束），按真实数据搭一条最小的链。
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('admin','admin','hash','administrator','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app','App','app','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,execution_mode,script_path,timeout_seconds,status) VALUES('target','app','node','test','two_stage','/unused',60,'active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash,snapshot_json) VALUES('deployment','app','target','admin','running','preparing','idem','request','snapshot','{\"targets\":[]}')")
        .execute(&pool)
        .await
        .unwrap();
    let ring = MasterKeyRing::from_raw(1, [7_u8; 32], None).unwrap();
    let encrypted = ring
        .encrypt("git_cred", "ed25519", PRIVATE_KEY.as_bytes())
        .unwrap();
    sqlx::query("INSERT INTO git_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version, status) VALUES ('git_cred', 'lease-key', 'ed25519', 'ssh-ed25519 AAAA', 'SHA256:lease', ?, ?, ?, 'active')")
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce)
        .bind(encrypted.key_version)
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone()).with_master_key_ring(ring);
    // 让 Agent 在线：`try_dispatch` 在更新前会检查连接，没有它所有断言都会因为“未连接”而假性通过。
    let (outbound, receiver) = tokio::sync::mpsc::channel(64);
    std::mem::forget(receiver);
    let _stop = state.agent_connections().register("agent", 1, outbound);
    (state, pool)
}

async fn insert_task(
    pool: &sqlx::SqlitePool,
    id: &str,
    agent: &str,
    kind: &str,
    payload: &TaskPayload,
    status: &str,
    deadline_at: &str,
) -> String {
    let (json, digest) = encode(payload);
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES(?,?,?,?,?,?,?,?)")
        .bind(id)
        .bind(agent)
        .bind(kind)
        .bind(format!("idem-{id}-0123456789"))
        .bind(&digest)
        .bind(&json)
        .bind(status)
        .bind(deadline_at)
        .execute(pool)
        .await
        .unwrap();
    digest
}

/// 构建任务：关联部署记录、`stage='prepare'`，与线上数据一致。
async fn insert_prepare_task(
    pool: &sqlx::SqlitePool,
    id: &str,
    payload: &TaskPayload,
    status: &str,
    deadline_at: &str,
) -> String {
    let (json, digest) = encode(payload);
    sqlx::query("INSERT INTO agent_tasks(id,agent_id,deployment_id,stage,kind,idempotency_key,payload_digest,payload_json,status,deadline_at) VALUES(?,'agent','deployment','prepare','deployment_prepare',?,?,?,?,?)")
        .bind(id)
        .bind(format!("idem-{id}-0123456789"))
        .bind(&digest)
        .bind(&json)
        .bind(status)
        .bind(deadline_at)
        .execute(pool)
        .await
        .unwrap();
    digest
}

async fn insert_lease(
    pool: &sqlx::SqlitePool,
    id: &str,
    task_id: &str,
    status: &str,
    expires_at: DateTime<Utc>,
    granted_at: Option<DateTime<Utc>>,
) {
    sqlx::query("INSERT INTO git_secret_leases(id,task_id,git_credential_id,payload_digest,purpose,status,expires_at,granted_at) VALUES(?,?,'git_cred','sha256:test','git_credential',?,?,?)")
        .bind(id)
        .bind(task_id)
        .bind(status)
        .bind(expires_at.to_rfc3339())
        .bind(granted_at.map(|value| value.to_rfc3339()))
        .execute(pool)
        .await
        .unwrap();
}

type LeaseRow = (String, DateTime<Utc>, Option<String>);

async fn lease_row(pool: &sqlx::SqlitePool, id: &str) -> LeaseRow {
    let (status, expires_at, granted_at): (String, String, Option<String>) =
        sqlx::query_as("SELECT status,expires_at,granted_at FROM git_secret_leases WHERE id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
    (
        status,
        DateTime::parse_from_rfc3339(&expires_at)
            .unwrap()
            .with_timezone(&Utc),
        granted_at,
    )
}

async fn task_status(pool: &sqlx::SqlitePool, id: &str) -> String {
    sqlx::query_scalar("SELECT status FROM agent_tasks WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn lease_request(task_id: &str, lease_id: &str, digest: &str) -> SecretLeaseRequest {
    SecretLeaseRequest {
        task_id: task_id.to_owned(),
        lease_id: lease_id.to_owned(),
        payload_digest: digest.to_owned(),
        purpose: SecretLeasePurpose::GitCredential,
    }
}

#[tokio::test]
async fn an_expired_unused_lease_is_fetchable_again_once_the_window_restarts() {
    let (state, pool) = fixture().await;
    let digest = insert_prepare_task(
        &pool,
        "task_wait",
        &prepare_payload("lease_wait"),
        "delivered",
        FAR_DEADLINE,
    )
    .await;
    // 任务排队了 90 秒才被派发：租约创建时只有 60 秒有效，30 秒前已经过期。
    insert_lease(
        &pool,
        "lease_wait",
        "task_wait",
        "issued",
        Utc::now() - Duration::seconds(30),
        None,
    )
    .await;
    // 真实的调度循环每个周期都会清理过期租约。
    assert_eq!(expire_secret_leases(&state).await.unwrap(), 1);
    assert_eq!(lease_row(&pool, "lease_wait").await.0, "expired");
    let request = lease_request("task_wait", "lease_wait", &digest);

    // 线上的失败：任务一派发，Agent 取租约就被拒绝。
    let rejected = resolve_secret_lease(&state, "agent", &request)
        .await
        .unwrap();
    assert_eq!(rejected.error_code.as_deref(), Some("lease_not_available"));

    // 派发时重新计时之后，同一个租约能取到私钥……
    let restarted =
        restart_git_lease_window(&state, "task_wait", FAR_DEADLINE, Duration::seconds(60))
            .await
            .unwrap();
    assert_eq!(restarted, 1);
    let granted = resolve_secret_lease(&state, "agent", &request)
        .await
        .unwrap();
    assert_eq!(granted.error_code, None);
    assert_eq!(granted.private_key, PRIVATE_KEY);

    // ……并且仍然是一次性的：取用后不能再取，再次派发也不会把它重新打开。
    let again = resolve_secret_lease(&state, "agent", &request)
        .await
        .unwrap();
    assert_eq!(again.error_code.as_deref(), Some("lease_not_available"));
    let reopened =
        restart_git_lease_window(&state, "task_wait", FAR_DEADLINE, Duration::seconds(60))
            .await
            .unwrap();
    assert_eq!(reopened, 0);
    assert_eq!(lease_row(&pool, "lease_wait").await.0, "granted");
}

#[tokio::test]
async fn dispatch_restarts_the_window_for_a_task_that_waited_in_the_queue() {
    let (state, pool) = fixture().await;
    insert_task(
        &pool,
        "task_q",
        "agent",
        "deployment_execute",
        &execute_payload(),
        "queued",
        FAR_DEADLINE,
    )
    .await;
    insert_lease(
        &pool,
        "lease_q",
        "task_q",
        "expired",
        Utc::now() - Duration::seconds(30),
        None,
    )
    .await;

    let before = Utc::now();
    assert!(try_dispatch(&state, "task_q").await.unwrap());

    assert_eq!(task_status(&pool, "task_q").await, "delivered");
    let (status, expires_at, granted_at) = lease_row(&pool, "lease_q").await;
    assert_eq!(status, "issued");
    assert!(granted_at.is_none());
    assert!(
        expires_at >= before + Duration::seconds(59)
            && expires_at <= Utc::now() + Duration::seconds(60),
        "窗口应从派发时刻起约 60 秒：{expires_at}"
    );
}

#[tokio::test]
async fn a_dispatch_that_does_not_happen_leaves_the_lease_alone() {
    let (state, pool) = fixture().await;
    // 同一 Agent 上的第一个任务占用唯一的名额，第二个被并发门禁拒绝。
    insert_task(
        &pool,
        "task_first",
        "agent",
        "deployment_execute",
        &execute_payload(),
        "queued",
        FAR_DEADLINE,
    )
    .await;
    insert_task(
        &pool,
        "task_blocked",
        "agent",
        "deployment_execute",
        &execute_payload(),
        "queued",
        FAR_DEADLINE,
    )
    .await;
    let stale = Utc::now() - Duration::seconds(30);
    insert_lease(
        &pool,
        "lease_blocked",
        "task_blocked",
        "expired",
        stale,
        None,
    )
    .await;
    assert!(try_dispatch(&state, "task_first").await.unwrap());
    assert!(!try_dispatch(&state, "task_blocked").await.unwrap());
    assert_eq!(task_status(&pool, "task_blocked").await, "queued");
    assert_eq!(
        lease_row(&pool, "lease_blocked").await,
        ("expired".to_owned(), stale, None),
        "被门禁拒绝的任务租约不应被改动"
    );

    // Agent 没有连接：同样不派发、不改动。
    sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES('node_off','node-off','/srv/apps','/srv/secrets','online')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agents(id,node_id,protocol_version,capabilities_json,connection_generation) VALUES('agent_off','node_off',18,?,1)")
        .bind(CAPABILITIES)
        .execute(&pool)
        .await
        .unwrap();
    insert_task(
        &pool,
        "task_off",
        "agent_off",
        "deployment_execute",
        &execute_payload(),
        "queued",
        FAR_DEADLINE,
    )
    .await;
    insert_lease(&pool, "lease_off", "task_off", "expired", stale, None).await;
    assert!(!try_dispatch(&state, "task_off").await.unwrap());
    assert_eq!(
        lease_row(&pool, "lease_off").await,
        ("expired".to_owned(), stale, None),
        "Agent 未连接时不应改动租约"
    );
}

#[tokio::test]
async fn redelivery_restarts_the_window_again() {
    let (state, pool) = fixture().await;
    insert_task(
        &pool,
        "task_again",
        "agent",
        "deployment_execute",
        &execute_payload(),
        "queued",
        FAR_DEADLINE,
    )
    .await;
    insert_lease(
        &pool,
        "lease_again",
        "task_again",
        "expired",
        Utc::now() - Duration::seconds(30),
        None,
    )
    .await;
    assert!(try_dispatch(&state, "task_again").await.unwrap());
    assert_eq!(lease_row(&pool, "lease_again").await.0, "issued");

    // Agent 没有及时确认，任务回到队列，又过了很久，租约再次被清理为 expired。
    let stale = Utc::now() - Duration::seconds(5);
    sqlx::query(
        "UPDATE agent_tasks SET status='queued',lease_expires_at=NULL WHERE id='task_again'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE git_secret_leases SET status='expired',expires_at=? WHERE id='lease_again'",
    )
    .bind(stale.to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    let before = Utc::now();
    assert!(try_dispatch(&state, "task_again").await.unwrap());
    let (status, expires_at, _) = lease_row(&pool, "lease_again").await;
    assert_eq!(status, "issued");
    assert!(expires_at >= before + Duration::seconds(59), "{expires_at}");
}

#[tokio::test]
async fn a_task_that_is_not_currently_delivered_does_not_get_its_lease_revived() {
    let (state, pool) = fixture().await;
    let stale = Utc::now() - Duration::seconds(30);
    // 已终结的任务在终结时租约被主动作废；还没派发的任务、已经开始运行的任务也不该被这里改动。
    for status in ["queued", "running", "failed", "canceled", "succeeded"] {
        let task_id = format!("task_{status}");
        let lease_id = format!("lease_{status}");
        insert_task(
            &pool,
            &task_id,
            "agent",
            "deployment_execute",
            &execute_payload(),
            status,
            FAR_DEADLINE,
        )
        .await;
        insert_lease(&pool, &lease_id, &task_id, "expired", stale, None).await;

        let restarted =
            restart_git_lease_window(&state, &task_id, FAR_DEADLINE, Duration::seconds(60))
                .await
                .unwrap();

        assert_eq!(restarted, 0, "{status}");
        assert_eq!(
            lease_row(&pool, &lease_id).await,
            ("expired".to_owned(), stale, None),
            "{status} 状态的任务租约不应被改动"
        );
    }
}

#[test]
fn the_window_depends_on_the_stage() {
    assert_eq!(
        git_lease_window(&prepare_payload("lease")),
        Duration::seconds(60)
    );
    assert_eq!(git_lease_window(&execute_payload()), Duration::seconds(60));
    // 发布要先下载并校验制品才取租约，窗口更长。
    assert_eq!(
        git_lease_window(&release_payload("lease")),
        Duration::seconds(600)
    );
}

#[tokio::test]
async fn the_new_expiry_never_shortens_a_lease_and_never_outlives_the_task() {
    let (state, pool) = fixture().await;
    insert_task(
        &pool,
        "task_cap",
        "agent",
        "deployment_execute",
        &execute_payload(),
        "delivered",
        FAR_DEADLINE,
    )
    .await;

    // 只延长、不缩短：租约还有 30 分钟，60 秒的窗口不能把它改短。
    let long = Utc::now() + Duration::minutes(30);
    insert_lease(&pool, "lease_long", "task_cap", "issued", long, None).await;
    let restarted =
        restart_git_lease_window(&state, "task_cap", FAR_DEADLINE, Duration::seconds(60))
            .await
            .unwrap();
    assert_eq!(restarted, 0);
    assert_eq!(
        lease_row(&pool, "lease_long").await,
        ("issued".to_owned(), long, None)
    );
    sqlx::query("DELETE FROM git_secret_leases WHERE id='lease_long'")
        .execute(&pool)
        .await
        .unwrap();

    // 不超过任务截止时间：窗口 10 分钟，但任务 20 秒后就到期。
    insert_lease(
        &pool,
        "lease_cap",
        "task_cap",
        "expired",
        Utc::now() - Duration::seconds(30),
        None,
    )
    .await;
    let deadline = Utc::now() + Duration::seconds(20);
    let restarted = restart_git_lease_window(
        &state,
        "task_cap",
        &deadline.to_rfc3339(),
        Duration::seconds(600),
    )
    .await
    .unwrap();
    assert_eq!(restarted, 1);
    let (status, expires_at, _) = lease_row(&pool, "lease_cap").await;
    assert_eq!(status, "issued");
    assert_eq!(expires_at, deadline, "租约不能比任务活得更久");

    // 任务截止时间已经过去，或者无法解析：不改动，沿用现有行为。
    let stale = Utc::now() - Duration::seconds(30);
    sqlx::query("UPDATE git_secret_leases SET status='expired',expires_at=? WHERE id='lease_cap'")
        .bind(stale.to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    let past = (Utc::now() - Duration::seconds(5)).to_rfc3339();
    for deadline in [past.as_str(), "not a timestamp", ""] {
        let restarted =
            restart_git_lease_window(&state, "task_cap", deadline, Duration::seconds(600))
                .await
                .unwrap();
        assert_eq!(restarted, 0, "{deadline:?}");
    }
    assert_eq!(
        lease_row(&pool, "lease_cap").await,
        ("expired".to_owned(), stale, None)
    );
}

#[tokio::test]
async fn only_unused_leases_of_the_delivered_task_are_restarted() {
    let (state, pool) = fixture().await;
    insert_task(
        &pool,
        "task_a",
        "agent",
        "deployment_execute",
        &execute_payload(),
        "delivered",
        FAR_DEADLINE,
    )
    .await;
    insert_task(
        &pool,
        "task_b",
        "agent",
        "deployment_execute",
        &execute_payload(),
        "queued",
        FAR_DEADLINE,
    )
    .await;
    let now = Utc::now();
    let stale = now - Duration::seconds(30);
    let granted_at = now - Duration::seconds(120);
    // 应当被重新计时：从未被取用的租约（已被清理为 expired，或仍是 issued 但快到期）。
    insert_lease(&pool, "a_unused_expired", "task_a", "expired", stale, None).await;
    insert_lease(
        &pool,
        "a_unused_issued",
        "task_a",
        "issued",
        now + Duration::seconds(5),
        None,
    )
    .await;
    // 不应改动：已被取用过（即使后来到期被清理成 expired）、已被拒绝、其他任务的租约。
    insert_lease(
        &pool,
        "a_granted_live",
        "task_a",
        "granted",
        now + Duration::seconds(30),
        Some(granted_at),
    )
    .await;
    insert_lease(
        &pool,
        "a_granted_then_expired",
        "task_a",
        "expired",
        stale,
        Some(granted_at),
    )
    .await;
    insert_lease(&pool, "a_rejected", "task_a", "rejected", stale, None).await;
    insert_lease(&pool, "b_expired", "task_b", "expired", stale, None).await;
    let untouched = [
        "a_granted_live",
        "a_granted_then_expired",
        "a_rejected",
        "b_expired",
    ];
    let mut before = Vec::new();
    for id in untouched {
        before.push(lease_row(&pool, id).await);
    }

    let restarted = restart_git_lease_window(&state, "task_a", FAR_DEADLINE, Duration::seconds(60))
        .await
        .unwrap();

    assert_eq!(restarted, 2);
    for id in ["a_unused_expired", "a_unused_issued"] {
        let (status, expires_at, granted_at) = lease_row(&pool, id).await;
        assert_eq!(status, "issued", "{id}");
        assert!(granted_at.is_none(), "{id}");
        assert!(
            expires_at >= now + Duration::seconds(59),
            "{id}: {expires_at}"
        );
    }
    for (id, previous) in untouched.iter().zip(before) {
        assert_eq!(lease_row(&pool, id).await, previous, "{id} 不应被改动");
    }
}
