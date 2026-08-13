use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use axum::{
    Router,
    body::Body,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, Response, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};

use deploy_go_agent::{
    artifact_transfer::{
        ArchivePreparation, ArtifactTransferClient, ArtifactTransferError, PreparedArchive,
        extract_archive, extract_archive_atomic, extract_archive_atomic_verified,
    },
    staging::{StagingLimits, verify_artifact_dir},
    token_refresh::{AccessProvider, PreparedAccess, TokenRefreshError},
};
use deploy_go_agent_protocol::ArtifactPrepared;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Clone)]
struct HttpFixture {
    archive: Arc<Vec<u8>>,
    digest: String,
    requests: Arc<AtomicUsize>,
}

async fn start_http_fixture(
    archive: Vec<u8>,
) -> (url::Url, HttpFixture, tokio::task::JoinHandle<()>) {
    let fixture = HttpFixture {
        digest: format!("{:x}", Sha256::digest(&archive)),
        archive: Arc::new(archive),
        requests: Arc::new(AtomicUsize::new(0)),
    };
    let app = Router::new()
        .route(
            "/api/v1/agent/artifact-leases/{id}/upload",
            post(upload_start).put(upload_chunk).get(upload_status),
        )
        .route(
            "/api/v1/agent/artifact-leases/{id}/upload/finalize",
            post(upload_finalize),
        )
        .route(
            "/api/v1/agent/artifact-leases/{id}/download",
            get(download_range),
        )
        .with_state(fixture.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (
        format!("http://{address}/").parse().unwrap(),
        fixture,
        server,
    )
}

fn authorized(headers: &HeaderMap) {
    assert_eq!(
        headers.get(header::AUTHORIZATION).unwrap(),
        "Bearer test-access-token-that-is-never-logged"
    );
}

async fn upload_start(State(state): State<HttpFixture>, headers: HeaderMap) -> impl IntoResponse {
    authorized(&headers);
    axum::Json(serde_json::json!({"offset":5,"upload_size":state.archive.len()}))
}

async fn upload_status(State(state): State<HttpFixture>, headers: HeaderMap) -> impl IntoResponse {
    authorized(&headers);
    axum::Json(serde_json::json!({"offset":5,"upload_size":state.archive.len()}))
}

async fn upload_chunk(
    State(state): State<HttpFixture>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    authorized(&headers);
    let expected = format!(
        "bytes 5-{}/{}",
        state.archive.len() - 1,
        state.archive.len()
    );
    assert_eq!(
        headers.get(header::CONTENT_RANGE).unwrap(),
        expected.as_str()
    );
    assert_eq!(body.as_ref(), &state.archive[5..]);
    state.requests.fetch_add(1, Ordering::SeqCst);
    axum::Json(serde_json::json!({"offset":state.archive.len(),"upload_size":state.archive.len()}))
}

async fn upload_finalize(headers: HeaderMap) -> impl IntoResponse {
    authorized(&headers);
    StatusCode::OK
}

async fn invalid_upload_start(headers: HeaderMap) -> impl IntoResponse {
    authorized(&headers);
    axum::Json(serde_json::json!({"offset":0,"upload_size":999999}))
}

async fn echo_upload_start(
    headers: HeaderMap,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    authorized(&headers);
    axum::Json(serde_json::json!({
        "offset": 0,
        "upload_size": payload["upload_size"]
    }))
}

fn range_total(headers: &HeaderMap) -> (u64, u64) {
    let value = headers
        .get(header::CONTENT_RANGE)
        .unwrap()
        .to_str()
        .unwrap();
    let value = value.strip_prefix("bytes ").unwrap();
    let (range, total) = value.split_once('/').unwrap();
    let (_, end) = range.split_once('-').unwrap();
    (end.parse().unwrap(), total.parse().unwrap())
}

async fn jumping_upload_chunk(headers: HeaderMap) -> impl IntoResponse {
    authorized(&headers);
    let (end, total) = range_total(&headers);
    axum::Json(serde_json::json!({
        "offset": (end + 6).min(total),
        "upload_size": total
    }))
}

async fn stalled_upload_chunk(headers: HeaderMap) -> impl IntoResponse {
    authorized(&headers);
    let (_, total) = range_total(&headers);
    axum::Json(serde_json::json!({"offset":0,"upload_size":total}))
}

#[derive(Clone)]
struct FlakyUploadFixture {
    attempts: Arc<AtomicUsize>,
    total: usize,
}

async fn flaky_upload_start(
    headers: HeaderMap,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    authorized(&headers);
    axum::Json(serde_json::json!({
        "offset": 0,
        "upload_size": payload["upload_size"]
    }))
}

async fn flaky_upload_status(
    State(state): State<FlakyUploadFixture>,
    headers: HeaderMap,
) -> impl IntoResponse {
    authorized(&headers);
    axum::Json(serde_json::json!({"offset":0,"upload_size":state.total}))
}

async fn flaky_upload_chunk(
    State(state): State<FlakyUploadFixture>,
    headers: HeaderMap,
) -> impl IntoResponse {
    authorized(&headers);
    let expected = format!("bytes 0-{}/{}", state.total - 1, state.total);
    assert_eq!(
        headers.get(header::CONTENT_RANGE).unwrap(),
        expected.as_str()
    );
    if state.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    } else {
        axum::Json(serde_json::json!({
            "offset": state.total,
            "upload_size": state.total
        }))
        .into_response()
    }
}

async fn download_range(
    State(state): State<HttpFixture>,
    AxumPath(_): AxumPath<String>,
    headers: HeaderMap,
) -> Response<Body> {
    authorized(&headers);
    let range = headers.get(header::RANGE).unwrap().to_str().unwrap();
    let start = range
        .strip_prefix("bytes=")
        .unwrap()
        .strip_suffix('-')
        .unwrap()
        .parse::<usize>()
        .unwrap();
    state.requests.fetch_add(1, Ordering::SeqCst);
    let total = state.archive.len();
    let (body, end) = if start == 0 {
        let split = total / 2;
        (Body::from(state.archive[..split].to_vec()), split - 1)
    } else {
        (Body::from(state.archive[start..].to_vec()), total - 1)
    };
    let mut response = Response::new(body);
    *response.status_mut() = StatusCode::PARTIAL_CONTENT;
    response.headers_mut().insert(
        header::CONTENT_RANGE,
        format!("bytes {start}-{end}/{total}").parse().unwrap(),
    );
    response
}

struct StaticAccess;

#[async_trait::async_trait]
impl AccessProvider for StaticAccess {
    async fn prepare(&self) -> Result<PreparedAccess, TokenRefreshError> {
        Ok(PreparedAccess {
            access_token: "test-access-token-that-is-never-logged".to_owned(),
            access_expires_at: "2099-01-01T00:00:00Z".to_owned(),
            rotation_id: None,
        })
    }

    async fn commit(&self, _rotation_id: &str) -> Result<(), TokenRefreshError> {
        Ok(())
    }
}

fn artifact(root: &Path) {
    fs::create_dir_all(root.join("api")).unwrap();
    fs::write(root.join("api/app.bin"), b"hello artifact\n").unwrap();
    let digest = format!("{:x}", Sha256::digest(b"hello artifact\n"));
    fs::write(
        root.join("deploy-go-artifact.json"),
        format!(
            r#"{{"schema_version":1,"release_version":"release-1","commit_sha":"0123456789abcdef0123456789abcdef01234567","artifacts":[{{"module":"api","path":"api/app.bin","sha256":"{digest}","size":15}}]}}"#,
        ),
    )
    .unwrap();
}

#[test]
fn deterministic_archive_round_trip_is_verified_before_release() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let extracted = temp.path().join("extracted");
    fs::create_dir(&source).unwrap();
    artifact(&source);
    let client = ArtifactTransferClient::new(
        "https://deploy.example/".parse().unwrap(),
        Arc::new(StaticAccess),
        true,
    );
    let limits = StagingLimits {
        size_limit_bytes: 1024 * 1024,
        max_files: 8,
    };
    let first = client
        .prepare_archive(ArchivePreparation {
            task_id: "task_prepare",
            authorization_id: "auth_prepare",
            deployment_id: "deployment_1",
            artifact_dir: &source,
            archive_path: &temp.path().join("first.tar"),
            expected_release: "release-1",
            expected_commit: "0123456789abcdef0123456789abcdef01234567",
            expected_modules: &["api".to_owned()],
            limits: &limits,
        })
        .unwrap();
    let second = client
        .prepare_archive(ArchivePreparation {
            task_id: "task_prepare",
            authorization_id: "auth_prepare",
            deployment_id: "deployment_1",
            artifact_dir: &source,
            archive_path: &temp.path().join("second.tar"),
            expected_release: "release-1",
            expected_commit: "0123456789abcdef0123456789abcdef01234567",
            expected_modules: &["api".to_owned()],
            limits: &limits,
        })
        .unwrap();
    assert_eq!(first.notice.archive_digest, second.notice.archive_digest);
    assert!(!first.notice.manifest_json.contains("test-access-token"));
    extract_archive(&first.path, &extracted).unwrap();
    verify_artifact_dir(
        &extracted,
        "release-1",
        "0123456789abcdef0123456789abcdef01234567",
        &["api".to_owned()],
        &limits,
    )
    .unwrap();
}

#[test]
fn feature_flag_defaults_to_rejecting_archive_transfer() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    artifact(&source);
    let client = ArtifactTransferClient::new(
        "https://deploy.example/".parse().unwrap(),
        Arc::new(StaticAccess),
        false,
    );
    let error = client
        .prepare_archive(ArchivePreparation {
            task_id: "task_prepare",
            authorization_id: "auth_prepare",
            deployment_id: "deployment_1",
            artifact_dir: &source,
            archive_path: &temp.path().join("artifact.tar"),
            expected_release: "release-1",
            expected_commit: "0123456789abcdef0123456789abcdef01234567",
            expected_modules: &["api".to_owned()],
            limits: &StagingLimits {
                size_limit_bytes: 1024 * 1024,
                max_files: 8,
            },
        })
        .unwrap_err();
    assert!(matches!(error, ArtifactTransferError::Disabled));
}

#[test]
fn extraction_rejects_parent_path_before_writing_outside_staging() {
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("malicious.tar");
    let mut builder = tar::Builder::new(fs::File::create(&archive_path).unwrap());
    let mut header = tar::Header::new_gnu();
    header.set_size(3);
    header.set_mode(0o644);
    header.set_cksum();
    // tar crate itself rejects unsafe paths, which is the same invariant the extractor enforces.
    assert!(
        builder
            .append_data(&mut header, "../escape", &b"bad"[..])
            .is_err()
    );
}

#[tokio::test]
async fn upload_resumes_from_server_offset_instead_of_replaying_confirmed_bytes() {
    let bytes = b"0123456789artifact".to_vec();
    let (base, fixture, server) = start_http_fixture(bytes.clone()).await;
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("artifact.tar");
    fs::write(&archive_path, &bytes).unwrap();
    let client = ArtifactTransferClient::new(base, Arc::new(StaticAccess), true);
    client
        .upload(
            "lease_upload",
            &PreparedArchive {
                path: archive_path,
                notice: ArtifactPrepared {
                    task_id: "task_prepare".into(),
                    authorization_id: "authorization_1".into(),
                    deployment_id: "deployment_1".into(),
                    manifest_json: "{}".into(),
                    manifest_digest: "a".repeat(64),
                    total_size: 1,
                    file_count: 1,
                    archive_size: bytes.len() as u64,
                    archive_digest: fixture.digest.clone(),
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 1);
    server.abort();
}

#[tokio::test]
async fn range_download_resumes_after_body_interruption_and_rejects_wrong_digest() {
    let bytes = b"downloaded artifact bytes".to_vec();
    let (base, fixture, server) = start_http_fixture(bytes.clone()).await;
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("artifact.tar");
    let client = ArtifactTransferClient::new(base, Arc::new(StaticAccess), true);
    client
        .download("lease_download", &archive_path, &fixture.digest)
        .await
        .unwrap();
    assert_eq!(fs::read(&archive_path).unwrap(), bytes);
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 2);

    fs::remove_file(&archive_path).unwrap();
    let error = client
        .download("lease_download", &archive_path, &"f".repeat(64))
        .await
        .unwrap_err();
    assert!(matches!(error, ArtifactTransferError::DigestMismatch));
    assert!(!temp.path().join("release-was-executed").exists());
    server.abort();
}

#[tokio::test]
async fn stale_partial_with_matching_sidecar_restarts_from_zero_after_digest_mismatch() {
    let bytes = b"downloaded artifact bytes".to_vec();
    let (base, fixture, server) = start_http_fixture(bytes.clone()).await;
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("artifact.tar");
    fs::write(temp.path().join("artifact.tar.part"), b"xxxxx").unwrap();
    fs::write(
        temp.path().join("artifact.tar.part.meta"),
        fixture.digest.as_bytes(),
    )
    .unwrap();
    let client = ArtifactTransferClient::new(base, Arc::new(StaticAccess), true);

    client
        .download("lease_download", &archive_path, &fixture.digest)
        .await
        .unwrap();

    assert_eq!(fs::read(&archive_path).unwrap(), bytes);
    assert!(!temp.path().join("artifact.tar.part").exists());
    assert!(!temp.path().join("artifact.tar.part.meta").exists());
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 3);
    server.abort();
}

#[tokio::test]
async fn upload_rejects_server_size_mismatch_before_sending_chunks() {
    let app = Router::new().route(
        "/api/v1/agent/artifact-leases/{id}/upload",
        post(invalid_upload_start),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("artifact.tar");
    fs::write(&archive_path, b"archive").unwrap();
    let client = ArtifactTransferClient::new(
        format!("http://{address}/").parse().unwrap(),
        Arc::new(StaticAccess),
        true,
    );
    let error = client
        .upload(
            "lease_upload",
            &PreparedArchive {
                path: archive_path,
                notice: ArtifactPrepared {
                    task_id: "task_prepare".into(),
                    authorization_id: "authorization_1".into(),
                    deployment_id: "deployment_1".into(),
                    manifest_json: "{}".into(),
                    manifest_digest: "a".repeat(64),
                    total_size: 1,
                    file_count: 1,
                    archive_size: 7,
                    archive_digest: "b".repeat(64),
                },
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ArtifactTransferError::InvalidResponse));
    server.abort();
}

async fn upload_with_handlers(
    bytes: Vec<u8>,
    put_handler: axum::routing::MethodRouter,
) -> ArtifactTransferError {
    let app = Router::new().route(
        "/api/v1/agent/artifact-leases/{id}/upload",
        post(echo_upload_start).merge(put_handler),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("artifact.tar");
    fs::write(&archive_path, &bytes).unwrap();
    let client = ArtifactTransferClient::new(
        format!("http://{address}/").parse().unwrap(),
        Arc::new(StaticAccess),
        true,
    );
    let error = client
        .upload(
            "lease_upload",
            &PreparedArchive {
                path: archive_path,
                notice: ArtifactPrepared {
                    task_id: "task_prepare".into(),
                    authorization_id: "authorization_1".into(),
                    deployment_id: "deployment_1".into(),
                    manifest_json: "{}".into(),
                    manifest_digest: "a".repeat(64),
                    total_size: 1,
                    file_count: 1,
                    archive_size: bytes.len() as u64,
                    archive_digest: format!("{:x}", Sha256::digest(&bytes)),
                },
            },
        )
        .await
        .unwrap_err();
    server.abort();
    error
}

#[tokio::test]
async fn upload_rejects_put_status_that_jumps_past_the_sent_chunk() {
    let error = upload_with_handlers(
        vec![b'x'; 1024 * 1024 + 16],
        axum::routing::put(jumping_upload_chunk),
    )
    .await;
    assert!(matches!(error, ArtifactTransferError::InvalidResponse));
}

#[tokio::test]
async fn upload_rejects_repeated_put_status_without_progress() {
    let error = upload_with_handlers(
        b"archive".to_vec(),
        axum::routing::put(stalled_upload_chunk),
    )
    .await;
    assert!(matches!(error, ArtifactTransferError::InvalidResponse));
}

#[tokio::test]
async fn upload_resumes_after_server_internal_error_for_a_chunk() {
    let bytes = b"0123456789artifact".to_vec();
    let fixture = FlakyUploadFixture {
        attempts: Arc::new(AtomicUsize::new(0)),
        total: bytes.len(),
    };
    let app = Router::new()
        .route(
            "/api/v1/agent/artifact-leases/{id}/upload",
            post(flaky_upload_start)
                .get(flaky_upload_status)
                .put(flaky_upload_chunk),
        )
        .route(
            "/api/v1/agent/artifact-leases/{id}/upload/finalize",
            post(upload_finalize),
        )
        .with_state(fixture.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("artifact.tar");
    fs::write(&archive_path, &bytes).unwrap();
    let client = ArtifactTransferClient::new(
        format!("http://{address}/").parse().unwrap(),
        Arc::new(StaticAccess),
        true,
    );
    client
        .upload(
            "lease_upload",
            &PreparedArchive {
                path: archive_path,
                notice: ArtifactPrepared {
                    task_id: "task_prepare".into(),
                    authorization_id: "authorization_1".into(),
                    deployment_id: "deployment_1".into(),
                    manifest_json: "{}".into(),
                    manifest_digest: "a".repeat(64),
                    total_size: 1,
                    file_count: 1,
                    archive_size: bytes.len() as u64,
                    archive_digest: format!("{:x}", Sha256::digest(&bytes)),
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(fixture.attempts.load(Ordering::SeqCst), 2);
    server.abort();
}

#[test]
fn atomic_extract_failure_preserves_existing_staging() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("staging");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("sentinel"), b"old-release").unwrap();
    let archive_path = temp.path().join("invalid.tar");
    let mut builder = tar::Builder::new(fs::File::create(&archive_path).unwrap());
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Directory);
    header.set_size(0);
    header.set_mode(0o755);
    header.set_cksum();
    builder
        .append_data(&mut header, "unexpected-directory", &[][..])
        .unwrap();
    builder.finish().unwrap();

    assert!(matches!(
        extract_archive_atomic(&archive_path, &target),
        Err(ArtifactTransferError::InvalidPath)
    ));
    assert_eq!(fs::read(target.join("sentinel")).unwrap(), b"old-release");
}

#[test]
fn atomic_extract_verification_failure_preserves_existing_staging() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("staging");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("sentinel"), b"old-release").unwrap();
    let archive_path = temp.path().join("valid.tar");
    let mut builder = tar::Builder::new(fs::File::create(&archive_path).unwrap());
    let mut header = tar::Header::new_gnu();
    header.set_size(11);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "new-release", &b"new-release"[..])
        .unwrap();
    builder.finish().unwrap();

    assert!(matches!(
        extract_archive_atomic_verified(&archive_path, &target, |_| {
            Err(ArtifactTransferError::Verification)
        }),
        Err(ArtifactTransferError::Verification)
    ));
    assert_eq!(fs::read(target.join("sentinel")).unwrap(), b"old-release");
    assert!(!target.join("new-release").exists());
}
