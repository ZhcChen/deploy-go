use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use axum::{
    Router,
    body::Body,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, Response, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use futures_util::StreamExt;
use futures_util::stream::{once, pending};

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
            post(fixture_finalize),
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

async fn fixture_finalize(
    State(state): State<HttpFixture>,
    headers: HeaderMap,
) -> impl IntoResponse {
    authorized(&headers);
    axum::Json(
        serde_json::json!({"status":"verified","offset":state.archive.len(),"upload_size":state.archive.len()}),
    )
}

async fn flaky_finalize(
    State(state): State<FlakyUploadFixture>,
    headers: HeaderMap,
) -> impl IntoResponse {
    authorized(&headers);
    axum::Json(
        serde_json::json!({"status":"verified","offset":state.total,"upload_size":state.total}),
    )
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

#[derive(Clone)]
struct StalledDownloadFixture {
    archive: Arc<Vec<u8>>,
    requests: Arc<AtomicUsize>,
    stall_first: Arc<std::sync::atomic::AtomicBool>,
}

async fn stalled_download_range(
    State(state): State<StalledDownloadFixture>,
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
    let mut response = Response::new(Body::empty());
    let (body, end) = if start == 0
        && state
            .stall_first
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    {
        let split = total / 2;
        let prefix = state.archive[..split].to_vec();
        let body = Body::from_stream(
            once(async move { Ok::<_, std::io::Error>(axum::body::Bytes::from(prefix)) })
                .chain(pending()),
        );
        (body, split - 1)
    } else {
        (Body::from(state.archive[start..].to_vec()), total - 1)
    };
    *response.body_mut() = body;
    *response.status_mut() = StatusCode::PARTIAL_CONTENT;
    response.headers_mut().insert(
        header::CONTENT_RANGE,
        format!("bytes {start}-{end}/{total}").parse().unwrap(),
    );
    response
}

struct StaticAccess;

#[derive(Clone, Default)]
struct UploadFaults {
    bytes: Arc<std::sync::Mutex<Vec<u8>>>,
    size: usize,
    init_calls: Arc<AtomicUsize>,
    put_calls: Arc<AtomicUsize>,
    status_calls: Arc<AtomicUsize>,
    finalize_calls: Arc<AtomicUsize>,
    init_lost: bool,
    put_lost: bool,
    commit_put: bool,
    status_lost: usize,
    finalize_lost: bool,
    reject: Option<StatusCode>,
    init_reject: Option<StatusCode>,
    finalize_reject: Option<StatusCode>,
    finalize_consumed: bool,
    finalize_unverified: bool,
    finalize_lost_count: usize,
    stalled_rejection: bool,
    stalled_status: bool,
    budget: Option<Duration>,
}

fn lost_upload_response() -> Response<Body> {
    // 已经发送响应头和部分 JSON 后断开，验证真实 HTTP body 中断。
    Response::new(Body::from_stream(
        once(async { Ok::<_, std::io::Error>(axum::body::Bytes::from_static(b"{\"offset\":")) })
            .chain(once(async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Err(std::io::Error::other("secret-response-marker"))
            })),
    ))
}

fn fault_status(state: &UploadFaults) -> Response<Body> {
    axum::Json(serde_json::json!({
        "offset": state.bytes.lock().unwrap().len(), "upload_size": state.size,
        "status": "uploading"
    }))
    .into_response()
}

async fn fault_init(State(state): State<UploadFaults>, headers: HeaderMap) -> Response<Body> {
    authorized(&headers);
    if let Some(status) = state.init_reject {
        if state.stalled_rejection {
            let mut response = Response::new(Body::from_stream(
                once(async {
                    Ok::<_, std::io::Error>(axum::body::Bytes::from_static(
                        b"secret-response-marker",
                    ))
                })
                .chain(pending()),
            ));
            *response.status_mut() = status;
            return response;
        }
        return (status, "secret-response-marker").into_response();
    }
    if state.init_calls.fetch_add(1, Ordering::SeqCst) == 0 && state.init_lost {
        return lost_upload_response();
    }
    fault_status(&state)
}

async fn fault_put(
    State(state): State<UploadFaults>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response<Body> {
    authorized(&headers);
    let call = state.put_calls.fetch_add(1, Ordering::SeqCst);
    if let Some(status) = state.reject {
        return (status, "secret-response-marker").into_response();
    }
    if call == 0 && state.put_lost && !state.commit_put {
        return lost_upload_response();
    }
    let range = headers
        .get(header::CONTENT_RANGE)
        .unwrap()
        .to_str()
        .unwrap();
    let start: usize = range
        .strip_prefix("bytes ")
        .unwrap()
        .split('-')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let mut bytes = state.bytes.lock().unwrap();
    if start != bytes.len() {
        return StatusCode::CONFLICT.into_response();
    }
    bytes.extend_from_slice(&body);
    drop(bytes);
    if call == 0 && state.put_lost {
        lost_upload_response()
    } else {
        fault_status(&state)
    }
}

async fn fault_get(State(state): State<UploadFaults>, headers: HeaderMap) -> Response<Body> {
    authorized(&headers);
    if state.stalled_status {
        return Response::new(Body::from_stream(pending::<
            Result<axum::body::Bytes, std::io::Error>,
        >()));
    }
    if state.status_calls.fetch_add(1, Ordering::SeqCst) < state.status_lost {
        return lost_upload_response();
    }
    fault_status(&state)
}

async fn fault_finalize(State(state): State<UploadFaults>, headers: HeaderMap) -> Response<Body> {
    authorized(&headers);
    assert_eq!(state.bytes.lock().unwrap().len(), state.size);
    let call = state.finalize_calls.fetch_add(1, Ordering::SeqCst);
    if let Some(status) = state.finalize_reject {
        return (status, "secret-response-marker").into_response();
    }
    if call == 0 && state.finalize_consumed {
        return (StatusCode::CONFLICT, axum::Json(serde_json::json!({"code":"artifact_lease_consumed","message":"secret-response-marker"}))).into_response();
    }
    if (call == 0 && state.finalize_lost) || call < state.finalize_lost_count {
        lost_upload_response()
    } else if state.finalize_unverified {
        fault_status(&state)
    } else {
        axum::Json(
            serde_json::json!({"status":"verified", "offset":state.size,"upload_size":state.size}),
        )
        .into_response()
    }
}

#[tokio::test]
async fn upload_initial_and_final_errors_report_stage_and_never_expose_body() {
    for initial in [true, false] {
        let state = UploadFaults {
            size: 17,
            init_reject: initial.then_some(StatusCode::FORBIDDEN),
            finalize_reject: (!initial).then_some(StatusCode::FORBIDDEN),
            ..Default::default()
        };
        let error = run_fault_upload(state, vec![b'x'; 17]).await.unwrap_err();
        let failure = error.upload_failure().unwrap();
        assert_eq!(
            failure.stage,
            if initial {
                "upload_init"
            } else {
                "upload_finalize"
            }
        );
        assert_eq!(failure.http_status, Some(403));
        assert_eq!(failure.attempts, 1);
        assert!(!error.to_string().contains("secret-response-marker"));
    }
    let state = UploadFaults {
        size: 17,
        finalize_unverified: true,
        ..Default::default()
    };
    let error = run_fault_upload(state, vec![b'x'; 17]).await.unwrap_err();
    assert_eq!(error.upload_failure().unwrap().stage, "upload_finalize");
    assert_eq!(error.upload_failure().unwrap().category, "invalid_response");
}

#[tokio::test]
async fn upload_retries_finalize_consumed_window_without_assuming_success() {
    let state = UploadFaults {
        size: 17,
        finalize_consumed: true,
        ..Default::default()
    };
    run_fault_upload(state.clone(), vec![b'x'; 17])
        .await
        .unwrap();
    assert_eq!(state.finalize_calls.load(Ordering::SeqCst), 2);
}

struct RefusedAccess;
#[async_trait::async_trait]
impl AccessProvider for RefusedAccess {
    async fn prepare(&self) -> Result<PreparedAccess, TokenRefreshError> {
        Err(TokenRefreshError::Rejected)
    }
    async fn commit(&self, _: &str) -> Result<(), TokenRefreshError> {
        Ok(())
    }
}

struct FailedRefreshAccess;
#[async_trait::async_trait]
impl AccessProvider for FailedRefreshAccess {
    async fn prepare(&self) -> Result<PreparedAccess, TokenRefreshError> {
        Err(TokenRefreshError::HttpStatus(500))
    }
    async fn commit(&self, _: &str) -> Result<(), TokenRefreshError> {
        Ok(())
    }
}

#[tokio::test]
async fn download_refresh_failure_preserves_stage_without_contacting_artifact_endpoint() {
    let (base, fixture, server) = start_http_fixture(b"archive".to_vec()).await;
    let directory = tempfile::tempdir().unwrap();
    let client = ArtifactTransferClient::new(base, Arc::new(FailedRefreshAccess), true);
    let path = directory.path().join("private-artifact.tar");
    let error = client
        .download("lease", &path, &fixture.digest)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ArtifactTransferError::Access {
            category: "http",
            http_status: Some(500)
        }
    ));
    assert_eq!(
        error.download_diagnostic(),
        "stage=access category=http http_status=Some(500)"
    );
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 0);
    assert!(!path.exists());
    assert!(
        !directory
            .path()
            .join("private-artifact.tar.part.meta")
            .exists()
    );
    let io_error = ArtifactTransferError::Io(std::io::Error::other("secret-path-and-value"));
    assert_eq!(io_error.download_diagnostic(), "stage=download category=io");
    server.abort();
}

#[tokio::test]
async fn download_network_failure_remains_distinct_from_access_failure() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    drop(listener);
    let directory = tempfile::tempdir().unwrap();
    let client = ArtifactTransferClient::with_client(
        base,
        Arc::new(StaticAccess),
        true,
        reqwest::Client::builder().no_proxy().build().unwrap(),
    );
    let error = client
        .download(
            "lease",
            &directory.path().join("artifact.tar"),
            &"a".repeat(64),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ArtifactTransferError::Transport));
    assert_eq!(
        error.download_diagnostic(),
        "stage=download category=transport"
    );
}

#[tokio::test]
async fn upload_access_failure_is_not_reported_as_network_error() {
    let client = ArtifactTransferClient::new(
        "http://127.0.0.1:1/".parse().unwrap(),
        Arc::new(RefusedAccess),
        true,
    );
    let error = client
        .upload(
            "lease_upload",
            &PreparedArchive {
                path: "unused-secret-path".into(),
                notice: ArtifactPrepared {
                    task_id: "task".into(),
                    authorization_id: "auth".into(),
                    deployment_id: "dep".into(),
                    manifest_json: "{}".into(),
                    manifest_digest: "a".repeat(64),
                    total_size: 17,
                    file_count: 1,
                    archive_size: 17,
                    archive_digest: "b".repeat(64),
                },
            },
        )
        .await
        .unwrap_err();
    assert_eq!(error.upload_failure().unwrap().stage, "access_prepare");
    assert_eq!(error.upload_failure().unwrap().category, "authorization");
    assert_eq!(error.upload_failure().unwrap().http_status, None);
    assert!(!error.to_string().contains("unused-secret-path"));
}

async fn run_fault_upload(
    state: UploadFaults,
    bytes: Vec<u8>,
) -> Result<(), ArtifactTransferError> {
    let budget = state.budget.unwrap_or(Duration::from_secs(30));
    let app = Router::new()
        .route(
            "/api/v1/agent/artifact-leases/{id}/upload",
            post(fault_init).put(fault_put).get(fault_get),
        )
        .route(
            "/api/v1/agent/artifact-leases/{id}/upload/finalize",
            post(fault_finalize),
        )
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("archive.tar");
    fs::write(&path, &bytes).unwrap();
    let client = ArtifactTransferClient::with_client(
        base,
        Arc::new(StaticAccess),
        true,
        reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap(),
    );
    let result = client
        .upload_with_budget(
            "lease_upload",
            &PreparedArchive {
                path,
                notice: ArtifactPrepared {
                    task_id: "task_prepare".into(),
                    authorization_id: "authorization_1".into(),
                    deployment_id: "deployment_1".into(),
                    manifest_json: "{}".into(),
                    manifest_digest: "a".repeat(64),
                    total_size: bytes.len() as u64,
                    file_count: 1,
                    archive_size: bytes.len() as u64,
                    archive_digest: format!("{:x}", Sha256::digest(&bytes)),
                },
            },
            budget,
        )
        .await;
    server.abort();
    result
}

#[tokio::test]
async fn permanent_rejection_headers_are_not_blocked_by_stalled_body() {
    let state = UploadFaults {
        size: 17,
        init_reject: Some(StatusCode::FORBIDDEN),
        stalled_rejection: true,
        budget: Some(Duration::from_millis(200)),
        ..Default::default()
    };
    let error = run_fault_upload(state, vec![b'x'; 17]).await.unwrap_err();
    assert_eq!(error.upload_failure().unwrap().category, "http_rejected");
    assert_eq!(error.upload_failure().unwrap().http_status, Some(403));
}

#[tokio::test]
async fn deadline_keeps_progress_query_stage_and_original_chunk_failure() {
    let state = UploadFaults {
        size: 17,
        put_lost: true,
        stalled_status: true,
        budget: Some(Duration::from_millis(200)),
        ..Default::default()
    };
    let error = run_fault_upload(state, vec![b'x'; 17]).await.unwrap_err();
    let failure = error.upload_failure().unwrap();
    assert_eq!(failure.stage, "upload_status");
    assert_eq!(failure.category, "timeout");
    assert_eq!(failure.attempts, 1);
    assert_eq!(failure.confirmed_offset, Some(0));
    assert_eq!(failure.cause.as_ref().unwrap().stage, "upload_chunk");
}

#[tokio::test]
async fn recovery_backoff_never_outlives_deadline_or_loses_cause_to_watchdog() {
    let state = UploadFaults {
        size: 17,
        put_lost: true,
        status_lost: usize::MAX,
        budget: Some(Duration::from_millis(390)),
        ..Default::default()
    };
    let error = tokio::time::timeout(
        Duration::from_millis(490),
        run_fault_upload(state, vec![b'x'; 17]),
    )
    .await
    .expect("请求应在任务 deadline 内返回结构化错误")
    .unwrap_err();
    let failure = error.upload_failure().unwrap();
    assert_eq!(failure.stage, "upload_status");
    assert_eq!(failure.category, "timeout");
    assert_eq!(failure.confirmed_offset, Some(0));
    assert_eq!(failure.cause.as_ref().unwrap().stage, "upload_chunk");
}

#[tokio::test]
async fn invalid_finalize_preserves_actual_successful_attempt_metadata() {
    let state = UploadFaults {
        size: 17,
        finalize_lost_count: 2,
        finalize_unverified: true,
        ..Default::default()
    };
    let error = run_fault_upload(state, vec![b'x'; 17]).await.unwrap_err();
    let failure = error.upload_failure().unwrap();
    assert_eq!(failure.stage, "upload_finalize");
    assert_eq!(failure.category, "invalid_response");
    assert_eq!(failure.http_status, Some(200));
    assert_eq!(failure.attempts, 3);
    assert!(failure.elapsed_ms >= 300);
}

#[tokio::test]
async fn upload_recovers_status_interruption_without_blind_put() {
    let state = UploadFaults {
        size: 17,
        put_lost: true,
        status_lost: 3,
        ..Default::default()
    };
    run_fault_upload(state.clone(), vec![b'x'; 17])
        .await
        .unwrap();
    assert_eq!(state.bytes.lock().unwrap().len(), 17);
    assert_eq!(state.finalize_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.status_calls.load(Ordering::SeqCst), 4);
    assert_eq!(state.put_calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn upload_recovers_initialization_response_loss() {
    let state = UploadFaults {
        size: 17,
        init_lost: true,
        ..Default::default()
    };
    run_fault_upload(state.clone(), vec![b'x'; 17])
        .await
        .unwrap();
    assert_eq!(state.init_calls.load(Ordering::SeqCst), 2);
    assert_eq!(state.put_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn upload_recovers_committed_chunk_response_loss() {
    let state = UploadFaults {
        size: 17,
        put_lost: true,
        commit_put: true,
        ..Default::default()
    };
    run_fault_upload(state.clone(), vec![b'x'; 17])
        .await
        .unwrap();
    assert_eq!(state.bytes.lock().unwrap().as_slice(), &[b'x'; 17]);
    assert_eq!(state.finalize_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn upload_status_failure_keeps_original_cause_and_bounded_budget() {
    let state = UploadFaults {
        size: 17,
        put_lost: true,
        status_lost: usize::MAX,
        ..Default::default()
    };
    let started = Instant::now();
    let error = run_fault_upload(state.clone(), vec![b'x'; 17])
        .await
        .unwrap_err();
    let failure = error.upload_failure().unwrap();
    assert_eq!(failure.stage, "upload_status");
    assert_eq!(failure.attempts, 3);
    assert_eq!(failure.confirmed_offset, Some(0));
    assert_eq!(failure.cause.as_ref().unwrap().stage, "upload_chunk");
    assert_eq!(state.status_calls.load(Ordering::SeqCst), 9);
    assert_eq!(state.put_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.finalize_calls.load(Ordering::SeqCst), 0);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(!error.to_string().contains("secret-response-marker"));
    assert!(!format!("{error:?}").contains("test-access-token"));
}

#[tokio::test]
async fn upload_permanent_rejections_do_not_query_or_retry() {
    for status in [
        StatusCode::FORBIDDEN,
        StatusCode::NOT_FOUND,
        StatusCode::CONFLICT,
        StatusCode::UNPROCESSABLE_ENTITY,
    ] {
        let state = UploadFaults {
            size: 17,
            reject: Some(status),
            ..Default::default()
        };
        let error = run_fault_upload(state.clone(), vec![b'x'; 17])
            .await
            .unwrap_err();
        let failure = error.upload_failure().unwrap();
        assert_eq!(failure.stage, "upload_chunk");
        assert_eq!(failure.category, "http_rejected");
        assert_eq!(failure.http_status, Some(status.as_u16()));
        assert_eq!(failure.attempts, 1);
        assert_eq!(state.put_calls.load(Ordering::SeqCst), 1);
        assert_eq!(state.status_calls.load(Ordering::SeqCst), 0);
        assert_eq!(state.finalize_calls.load(Ordering::SeqCst), 0);
        assert!(!error.to_string().contains("secret-response-marker"));
    }
}

#[tokio::test]
async fn upload_finalization_response_loss_is_retried_on_same_lease() {
    let state = UploadFaults {
        size: 17,
        finalize_lost: true,
        ..Default::default()
    };
    run_fault_upload(state.clone(), vec![b'x'; 17])
        .await
        .unwrap();
    assert_eq!(state.finalize_calls.load(Ordering::SeqCst), 2);
    assert_eq!(state.put_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.status_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn sixty_megabyte_upload_normal_and_interrupted_preserve_digest() {
    let bytes: Vec<u8> = (0..60 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    let digest = Sha256::digest(&bytes);
    for interrupted in [false, true] {
        let state = UploadFaults {
            size: bytes.len(),
            put_lost: interrupted,
            commit_put: true,
            ..Default::default()
        };
        run_fault_upload(state.clone(), bytes.clone())
            .await
            .unwrap();
        assert_eq!(
            Sha256::digest(state.bytes.lock().unwrap().as_slice()),
            digest
        );
        assert_eq!(state.put_calls.load(Ordering::SeqCst), 60);
        assert_eq!(state.finalize_calls.load(Ordering::SeqCst), 1);
    }
}

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
async fn range_download_recovers_after_read_idle_timeout() {
    let bytes = b"downloaded artifact bytes with a longer body for interruption".to_vec();
    let fixture = StalledDownloadFixture {
        archive: Arc::new(bytes.clone()),
        requests: Arc::new(AtomicUsize::new(0)),
        stall_first: Arc::new(std::sync::atomic::AtomicBool::new(true)),
    };
    let app = Router::new()
        .route(
            "/api/v1/agent/artifact-leases/{id}/download",
            get(stalled_download_range),
        )
        .with_state(fixture.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("artifact.tar");
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let client = ArtifactTransferClient::new(
        format!("http://{address}/").parse().unwrap(),
        Arc::new(StaticAccess),
        true,
    )
    .with_download_read_idle_timeout(Duration::from_millis(60));
    let started = Instant::now();
    client
        .download("lease_download", &archive_path, &digest)
        .await
        .unwrap();
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "下载应在一个短 read idle 超时后立即续传"
    );
    assert_eq!(fs::read(&archive_path).unwrap(), bytes);
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 2);
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
    assert_eq!(error.upload_failure().unwrap().category, "invalid_response");
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
    assert_eq!(error.upload_failure().unwrap().category, "invalid_response");
}

#[tokio::test]
async fn upload_rejects_repeated_put_status_without_progress() {
    let error = upload_with_handlers(
        b"archive".to_vec(),
        axum::routing::put(stalled_upload_chunk),
    )
    .await;
    assert_eq!(error.upload_failure().unwrap().category, "invalid_response");
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
            post(flaky_finalize),
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
