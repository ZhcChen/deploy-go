mod common;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use deploy_go_api::{
    AppState, agents::auth::token_hash, app, artifacts::ArtifactStore, config::ArtifactConfig,
    crypto::MasterKeyRing, db,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tower::ServiceExt;

async fn artifact_app() -> (Router, SqlitePool, tempfile::TempDir, ArtifactStore) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let temp = tempfile::tempdir().unwrap();
    let store = ArtifactStore::initialize(ArtifactConfig {
        root: temp.path().to_path_buf(),
        max_file_bytes: 1024 * 1024,
        max_total_bytes: 2 * 1024 * 1024,
        max_files: 16,
        max_chunk_bytes: 1024 * 1024,
        upload_ttl_seconds: 1800,
        retention_ttl_seconds: 86400,
    })
    .unwrap();
    let state = AppState::new(pool.clone())
        .with_master_key_ring(MasterKeyRing::from_raw(1, [7_u8; 32], None).unwrap())
        .with_artifact_store(store.clone());
    (app(state), pool, temp, store)
}

async fn fixture(pool: &SqlitePool) -> (String, Vec<u8>, Value) {
    sqlx::query("INSERT INTO users(id,username,password_hash,identity,status) VALUES('artifact_user','artifact-user','hash','administrator','active')").execute(pool).await.unwrap();
    for suffix in ["build", "other"] {
        sqlx::query("INSERT INTO nodes(id,name,work_root,secrets_root,status) VALUES(?,?, '/srv/apps','/srv/secrets','online')")
            .bind(format!("node_{suffix}")).bind(format!("Node {suffix}")).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO agents(id,node_id,environment) VALUES(?,?,'prod')")
            .bind(format!("agent_{suffix}"))
            .bind(format!("node_{suffix}"))
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agent_credential_families(id,agent_id) VALUES(?,?)")
            .bind(format!("family_{suffix}"))
            .bind(format!("agent_{suffix}"))
            .execute(pool)
            .await
            .unwrap();
        let token = format!("access-token-{suffix}");
        sqlx::query("INSERT INTO agent_access_sessions(id,family_id,agent_id,refresh_credential_id,token_hash,token_key_version,expires_at) VALUES(?,?,?,?,?,1,'2099-01-01T00:00:00Z')")
            .bind(format!("access_{suffix}")).bind(format!("family_{suffix}")).bind(format!("agent_{suffix}"))
            .bind(Option::<String>::None).bind(token_hash("access", &token)).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('artifact_app','Artifact App','artifact-app','active')").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO deployment_targets(id,application_id,node_id,environment,script_path,timeout_seconds,status) VALUES('artifact_target','artifact_app','node_build','prod','/srv/apps/deploy.sh',900,'active')").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO deployments(id,application_id,target_id,requested_by,status,phase,idempotency_key,request_hash,snapshot_hash) VALUES('artifact_deployment','artifact_app','artifact_target','artifact_user','running','preparing','artifact-key','request','snapshot')").execute(pool).await.unwrap();

    let content = b"artifact-content\n";
    let file_digest = format!("{:x}", Sha256::digest(content));
    let manifest = json!({
        "schema_version": 1,
        "release_version": "1.0.0",
        "commit_sha": "0123456789abcdef0123456789abcdef01234567",
        "artifacts": [{"module":"api","path":"api/app.bin","sha256":file_digest,"size":content.len()}]
    });
    let manifest_json = manifest.to_string();
    let manifest_digest = format!("{:x}", Sha256::digest(manifest_json.as_bytes()));
    sqlx::query("INSERT INTO deployment_artifacts(id,deployment_id,manifest_json,manifest_digest,total_size,file_count,status,expires_at) VALUES('artifact_upload','artifact_deployment',?,?,?,1,'uploading','2099-01-01T00:00:00Z')")
        .bind(&manifest_json).bind(&manifest_digest).bind(content.len() as i64).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO artifact_leases(id,artifact_id,agent_id,purpose,manifest_digest,status,expires_at) VALUES('lease_upload','artifact_upload','agent_build','artifact_upload',?,'active','2099-01-01T00:00:00Z')")
        .bind(&manifest_digest).execute(pool).await.unwrap();

    let mut archive = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut archive);
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "api/app.bin", content.as_slice())
            .unwrap();
        builder.finish().unwrap();
    }
    ("access-token-build".to_owned(), archive, manifest)
}

async fn request(
    app: Router,
    method: &str,
    path: &str,
    token: &str,
    body: Body,
    extra: &[(&str, String)],
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {token}"));
    for (name, value) in extra {
        builder = builder.header(*name, value);
    }
    app.oneshot(builder.body(body).unwrap()).await.unwrap()
}

async fn upload_all(app: Router, token: &str, archive: &[u8], digest: &str) {
    let initiated = request(
        app.clone(),
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        token,
        Body::from(json!({"upload_size":archive.len(),"archive_digest":digest}).to_string()),
        &[("content-type", "application/json".to_owned())],
    )
    .await;
    assert_eq!(initiated.status(), StatusCode::OK);
    let uploaded = request(
        app,
        "PUT",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        token,
        Body::from(archive.to_vec()),
        &[(
            "content-range",
            format!("bytes 0-{}/{}", archive.len() - 1, archive.len()),
        )],
    )
    .await;
    assert_eq!(uploaded.status(), StatusCode::OK);
}

fn archive_with_entry(path: &str, content: &[u8], entry_type: tar::EntryType) -> Vec<u8> {
    let mut archive = Vec::new();
    let mut builder = tar::Builder::new(&mut archive);
    let mut header = tar::Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_entry_type(entry_type);
    if path == "../escape" {
        header.as_mut_bytes()[..path.len()].copy_from_slice(path.as_bytes());
    } else {
        header.set_path(path).unwrap();
    }
    header.set_cksum();
    builder.append(&header, content).unwrap();
    builder.finish().unwrap();
    drop(builder);
    archive
}

#[tokio::test]
async fn upload_is_resumable_idempotent_and_finalizes_only_after_full_validation() {
    let (app, pool, temp, store) = artifact_app().await;
    let (token, archive, _) = fixture(&pool).await;
    let archive_digest = format!("{:x}", Sha256::digest(&archive));
    let initiated = request(
        app.clone(),
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::from(
            json!({"upload_size":archive.len(),"archive_digest":archive_digest}).to_string(),
        ),
        &[("content-type", "application/json".to_owned())],
    )
    .await;
    assert_eq!(initiated.status(), StatusCode::OK);
    let split = archive.len() / 2;
    let first = &archive[..split];
    let uploaded = request(
        app.clone(),
        "PUT",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::from(first.to_vec()),
        &[(
            "content-range",
            format!("bytes 0-{}/{}", split - 1, archive.len()),
        )],
    )
    .await;
    assert_eq!(uploaded.status(), StatusCode::OK);
    use tokio::io::AsyncWriteExt;
    let mut upload_file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(temp.path().join("quarantine/artifact_upload.upload"))
        .await
        .unwrap();
    upload_file.write_all(b"uncommitted-tail").await.unwrap();
    upload_file.sync_data().await.unwrap();

    let replayed = request(
        app.clone(),
        "PUT",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::from(first.to_vec()),
        &[(
            "content-range",
            format!("bytes 0-{}/{}", split - 1, archive.len()),
        )],
    )
    .await;
    assert_eq!(replayed.status(), StatusCode::OK);
    let resumed = request(
        app.clone(),
        "GET",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::empty(),
        &[],
    )
    .await;
    assert_eq!(resumed.status(), StatusCode::OK);
    let resumed: Value =
        serde_json::from_slice(&to_bytes(resumed.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(resumed["offset"], split);

    let second = &archive[split..];
    let uploaded = request(
        app.clone(),
        "PUT",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::from(second.to_vec()),
        &[(
            "content-range",
            format!("bytes {}-{}/{}", split, archive.len() - 1, archive.len()),
        )],
    )
    .await;
    assert_eq!(uploaded.status(), StatusCode::OK);
    let finalized = request(
        app.clone(),
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload/finalize",
        &token,
        Body::empty(),
        &[],
    )
    .await;
    assert_eq!(finalized.status(), StatusCode::OK);
    let fact: (String, String, i64) = sqlx::query_as("SELECT status,storage_key,upload_offset FROM deployment_artifacts WHERE id='artifact_upload'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(fact.0, "verified");
    assert_eq!(fact.1, archive_digest);
    assert_eq!(fact.2, archive.len() as i64);

    sqlx::query("INSERT INTO deployment_target_runs(id,deployment_id,target_id,node_id,agent_id,artifact_id,status) VALUES('download_run','artifact_deployment','artifact_target','node_build','agent_build','artifact_upload','downloading')")
        .execute(&pool).await.unwrap();
    let manifest_digest: String = sqlx::query_scalar(
        "SELECT manifest_digest FROM deployment_artifacts WHERE id='artifact_upload'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO artifact_leases(id,artifact_id,agent_id,target_run_id,purpose,manifest_digest,status,expires_at) VALUES('wrong_download_lease','artifact_upload','agent_other','download_run','artifact_download',?,'active','2099-01-01T00:00:00Z')")
        .bind(&manifest_digest).execute(&pool).await.unwrap();
    let wrong_binding = request(
        app.clone(),
        "GET",
        "/api/v1/agent/artifact-leases/wrong_download_lease/download",
        "access-token-other",
        Body::empty(),
        &[],
    )
    .await;
    assert_eq!(wrong_binding.status(), StatusCode::NOT_FOUND);
    sqlx::query("UPDATE artifact_leases SET status='revoked' WHERE id='wrong_download_lease'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO artifact_leases(id,artifact_id,agent_id,target_run_id,purpose,manifest_digest,status,expires_at) VALUES('lease_download','artifact_upload','agent_build','download_run','artifact_download',?,'active','2099-01-01T00:00:00Z')")
        .bind(manifest_digest).execute(&pool).await.unwrap();
    let downloaded = request(
        app,
        "GET",
        "/api/v1/agent/artifact-leases/lease_download/download",
        &token,
        Body::empty(),
        &[("range", "bytes=3-11".to_owned())],
    )
    .await;
    assert_eq!(downloaded.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        downloaded.headers()["content-range"],
        format!("bytes 3-11/{}", archive.len())
    );
    sqlx::query("UPDATE deployment_target_runs SET status='succeeded' WHERE id='download_run'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployment_artifacts SET expires_at='2000-01-01T00:00:00Z' WHERE id='artifact_upload'")
        .execute(&pool).await.unwrap();
    sqlx::query("UPDATE artifact_leases SET status='expired' WHERE artifact_id='artifact_upload'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        deploy_go_api::artifacts::reconcile_and_cleanup(&pool, &store)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        to_bytes(downloaded.into_body(), usize::MAX)
            .await
            .unwrap()
            .as_ref(),
        &archive[3..=11]
    );
    assert_eq!(
        deploy_go_api::artifacts::reconcile_and_cleanup(&pool, &store)
            .await
            .unwrap(),
        1
    );
    let cleaned: (String, Option<String>) = sqlx::query_as(
        "SELECT status,storage_key FROM deployment_artifacts WHERE id='artifact_upload'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(cleaned, ("expired".to_owned(), None));
}

#[tokio::test]
async fn cleanup_does_not_report_success_when_object_deletion_fails() {
    let (app, pool, temp, store) = artifact_app().await;
    let (token, archive, _) = fixture(&pool).await;
    let digest = format!("{:x}", Sha256::digest(&archive));
    upload_all(app.clone(), &token, &archive, &digest).await;
    let finalized = request(
        app,
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload/finalize",
        &token,
        Body::empty(),
        &[],
    )
    .await;
    assert_eq!(finalized.status(), StatusCode::OK);
    let object = temp.path().join("objects").join(&digest);
    tokio::fs::remove_file(&object).await.unwrap();
    tokio::fs::create_dir(&object).await.unwrap();
    sqlx::query("UPDATE artifact_leases SET status='expired' WHERE id='lease_upload'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE deployment_artifacts SET expires_at='2000-01-01T00:00:00Z' WHERE id='artifact_upload'")
        .execute(&pool).await.unwrap();
    sqlx::query("UPDATE deployment_artifacts SET status='deleting' WHERE id='artifact_upload'")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        deploy_go_api::artifacts::reconcile_and_cleanup(&pool, &store)
            .await
            .unwrap(),
        0
    );
    let status: String =
        sqlx::query_scalar("SELECT status FROM deployment_artifacts WHERE id='artifact_upload'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
}

#[tokio::test]
async fn reconciliation_removes_orphan_files_inside_managed_roots_only() {
    let (_app, pool, temp, store) = artifact_app().await;
    let quarantine = temp.path().join("quarantine/orphan.upload");
    let object = temp.path().join("objects").join("a".repeat(64));
    tokio::fs::write(&quarantine, b"orphan").await.unwrap();
    tokio::fs::write(&object, b"orphan").await.unwrap();
    let outside = temp.path().join("outside");
    tokio::fs::write(&outside, b"keep").await.unwrap();

    deploy_go_api::artifacts::reconcile_and_cleanup(&pool, &store)
        .await
        .unwrap();

    assert!(!quarantine.exists());
    assert!(!object.exists());
    assert!(outside.exists());
}

#[tokio::test]
async fn upload_rejects_wrong_agent_and_non_sequential_chunks() {
    let (app, pool, _temp, _store) = artifact_app().await;
    let (token, archive, _) = fixture(&pool).await;
    let digest = format!("{:x}", Sha256::digest(&archive));
    let wrong_agent = request(
        app.clone(),
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        "access-token-other",
        Body::from(json!({"upload_size":archive.len(),"archive_digest":digest}).to_string()),
        &[("content-type", "application/json".to_owned())],
    )
    .await;
    assert_eq!(wrong_agent.status(), StatusCode::NOT_FOUND);
    let initiated = request(
        app.clone(),
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::from(json!({"upload_size":archive.len(),"archive_digest":digest}).to_string()),
        &[("content-type", "application/json".to_owned())],
    )
    .await;
    assert_eq!(initiated.status(), StatusCode::OK);
    let skipped = request(
        app,
        "PUT",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::from(vec![0_u8; 4]),
        &[("content-range", format!("bytes 4-7/{}", archive.len()))],
    )
    .await;
    assert_eq!(skipped.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn expired_revoked_and_manifest_mismatched_upload_leases_are_rejected() {
    for status in ["expired", "revoked"] {
        let (app, pool, _temp, _store) = artifact_app().await;
        let (token, archive, _) = fixture(&pool).await;
        sqlx::query("UPDATE artifact_leases SET status=? WHERE id='lease_upload'")
            .bind(status)
            .execute(&pool)
            .await
            .unwrap();
        let digest = format!("{:x}", Sha256::digest(&archive));
        let response = request(
            app,
            "POST",
            "/api/v1/agent/artifact-leases/lease_upload/upload",
            &token,
            Body::from(json!({"upload_size":archive.len(),"archive_digest":digest}).to_string()),
            &[("content-type", "application/json".to_owned())],
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT, "status={status}");
    }

    let (app, pool, _temp, _store) = artifact_app().await;
    let (token, archive, _) = fixture(&pool).await;
    sqlx::query("UPDATE artifact_leases SET manifest_digest=? WHERE id='lease_upload'")
        .bind("0".repeat(64))
        .execute(&pool)
        .await
        .unwrap();
    let digest = format!("{:x}", Sha256::digest(&archive));
    let response = request(
        app,
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::from(json!({"upload_size":archive.len(),"archive_digest":digest}).to_string()),
        &[("content-type", "application/json".to_owned())],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn finalize_rejects_digest_and_unsafe_archive_entries() {
    let cases = [
        ("wrong-digest", None, Some("0".repeat(64))),
        (
            "path-traversal",
            Some(archive_with_entry(
                "../escape",
                b"artifact-content\n",
                tar::EntryType::Regular,
            )),
            None,
        ),
        (
            "symlink",
            Some(archive_with_entry(
                "api/app.bin",
                b"artifact-content\n",
                tar::EntryType::Symlink,
            )),
            None,
        ),
        (
            "hardlink",
            Some(archive_with_entry(
                "api/app.bin",
                b"artifact-content\n",
                tar::EntryType::Link,
            )),
            None,
        ),
        (
            "unknown-file",
            Some(archive_with_entry(
                "api/other.bin",
                b"artifact-content\n",
                tar::EntryType::Regular,
            )),
            None,
        ),
        ("missing-file", Some(vec![0_u8; 1024]), None),
    ];
    for (name, replacement, claimed_digest) in cases {
        let (app, pool, _temp, _store) = artifact_app().await;
        let (token, original, _) = fixture(&pool).await;
        let archive = replacement.unwrap_or(original);
        let digest = claimed_digest.unwrap_or_else(|| format!("{:x}", Sha256::digest(&archive)));
        upload_all(app.clone(), &token, &archive, &digest).await;
        let response = request(
            app,
            "POST",
            "/api/v1/agent/artifact-leases/lease_upload/upload/finalize",
            &token,
            Body::empty(),
            &[],
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT, "case={name}");
        let status: String = sqlx::query_scalar(
            "SELECT status FROM deployment_artifacts WHERE id='artifact_upload'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status, "failed", "case={name}");
        let lease_status: String =
            sqlx::query_scalar("SELECT status FROM artifact_leases WHERE id='lease_upload'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(lease_status, "failed", "case={name}");
    }
}

#[tokio::test]
async fn finalize_rejects_duplicate_modules() {
    let (app, pool, _temp, _store) = artifact_app().await;
    let (token, _archive, _) = fixture(&pool).await;
    let first = b"first";
    let second = b"second";
    let manifest = json!({
        "schema_version": 1,
        "release_version": "1.0.0",
        "commit_sha": "0123456789abcdef0123456789abcdef01234567",
        "artifacts": [
            {"module":"api","path":"api/first.bin","sha256":format!("{:x}", Sha256::digest(first)),"size":first.len()},
            {"module":"api","path":"api/second.bin","sha256":format!("{:x}", Sha256::digest(second)),"size":second.len()}
        ]
    });
    let manifest_json = manifest.to_string();
    let manifest_digest = format!("{:x}", Sha256::digest(manifest_json.as_bytes()));
    sqlx::query("UPDATE deployment_artifacts SET manifest_json=?,manifest_digest=?,total_size=?,file_count=2 WHERE id='artifact_upload'")
        .bind(&manifest_json).bind(&manifest_digest).bind((first.len() + second.len()) as i64)
        .execute(&pool).await.unwrap();
    sqlx::query("UPDATE artifact_leases SET manifest_digest=? WHERE id='lease_upload'")
        .bind(&manifest_digest)
        .execute(&pool)
        .await
        .unwrap();
    let mut archive = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut archive);
        for (path, content) in [
            ("api/first.bin", first.as_slice()),
            ("api/second.bin", second.as_slice()),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, content).unwrap();
        }
        builder.finish().unwrap();
    }
    let digest = format!("{:x}", Sha256::digest(&archive));
    upload_all(app.clone(), &token, &archive, &digest).await;
    let response = request(
        app,
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload/finalize",
        &token,
        Body::empty(),
        &[],
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn concurrent_chunks_with_the_same_offset_cannot_overwrite_the_winner() {
    let (app, pool, _temp, _store) = artifact_app().await;
    let (token, archive, _) = fixture(&pool).await;
    let digest = format!("{:x}", Sha256::digest(&archive));
    let initiated = request(
        app.clone(),
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::from(json!({"upload_size":archive.len(),"archive_digest":digest}).to_string()),
        &[("content-type", "application/json".to_owned())],
    )
    .await;
    assert_eq!(initiated.status(), StatusCode::OK);
    let length = 64_usize;
    let first_headers = [(
        "content-range",
        format!("bytes 0-{}/{}", length - 1, archive.len()),
    )];
    let second_headers = first_headers.clone();
    let first = request(
        app.clone(),
        "PUT",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::from(archive[..length].to_vec()),
        &first_headers,
    );
    let second = request(
        app,
        "PUT",
        "/api/v1/agent/artifact-leases/lease_upload/upload",
        &token,
        Body::from(vec![0xff; length]),
        &second_headers,
    );
    let (first, second) = tokio::join!(first, second);
    let statuses = [first.status(), second.status()];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CONFLICT)
            .count(),
        1
    );
}

#[tokio::test]
async fn concurrent_finalize_consumes_the_upload_lease_once() {
    let (app, pool, _temp, _store) = artifact_app().await;
    let (token, archive, _) = fixture(&pool).await;
    let digest = format!("{:x}", Sha256::digest(&archive));
    upload_all(app.clone(), &token, &archive, &digest).await;
    let left = request(
        app.clone(),
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload/finalize",
        &token,
        Body::empty(),
        &[],
    );
    let right = request(
        app,
        "POST",
        "/api/v1/agent/artifact-leases/lease_upload/upload/finalize",
        &token,
        Body::empty(),
        &[],
    );
    let (left, right) = tokio::join!(left, right);
    let statuses = [left.status(), right.status()];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CONFLICT)
            .count(),
        1
    );
}
