use deploy_go_agent::executor_client::ExecutorClient;
use deploy_go_agent_executor::protocol::{
    ExecutorCapability, HealthyResponse, InputRequest, MAX_FRAME_BYTES, PROTOCOL_VERSION, Request,
    Response, read_request, write_message,
};
use std::time::Duration;
use tokio::{io::AsyncWriteExt, net::UnixListener};

#[tokio::test]
async fn terminal_and_release_capabilities_are_reported_independently() {
    for (capabilities, terminal, release) in [
        (vec![ExecutorCapability::PtyTerminal], true, false),
        (vec![ExecutorCapability::DeploymentRelease], false, true),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let expected = capabilities.clone();
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                assert!(matches!(
                    read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap(),
                    Some(Request::Probe(_))
                ));
                write_message(
                    &mut stream,
                    &Response::Healthy(HealthyResponse {
                        version: PROTOCOL_VERSION,
                        capabilities: expected.clone(),
                    }),
                    MAX_FRAME_BYTES,
                )
                .await
                .unwrap();
            }
        });
        let client = ExecutorClient::new(socket);
        assert_eq!(client.probe().await, terminal);
        assert_eq!(
            client
                .probe_capabilities()
                .await
                .unwrap()
                .contains(&ExecutorCapability::DeploymentRelease),
            release
        );
        server.await.unwrap();
    }
}

#[tokio::test]
async fn executor_request_reports_frame_size_without_exposing_payload() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("executor.sock");
    let _listener = UnixListener::bind(&socket).unwrap();
    let client = ExecutorClient::new(socket);
    let payload = "sensitive-value".repeat(MAX_FRAME_BYTES);

    let error = client
        .request(Request::Input(InputRequest {
            version: PROTOCOL_VERSION,
            session_id: "session".into(),
            sequence: 1,
            data: payload.into_bytes(),
        }))
        .await
        .unwrap_err();

    assert_eq!(error.error_code(), "executor_request_too_large");
    assert!(error.request_frame_bytes().unwrap() > MAX_FRAME_BYTES);
    assert_eq!(error.frame_limit_bytes(), Some(MAX_FRAME_BYTES));
    assert!(!error.to_string().contains("sensitive-value"));
}

#[tokio::test]
async fn executor_request_distinguishes_response_timeout_and_eof() {
    for (close, expected_code) in [
        (false, "executor_response_timeout"),
        (true, "executor_response_closed"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("executor.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap();
            if !close {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        let client = ExecutorClient::new(socket);
        let error = client
            .request_with_timeout(
                Request::Probe(deploy_go_agent_executor::protocol::ProbeRequest {
                    version: PROTOCOL_VERSION,
                }),
                Duration::from_millis(20),
            )
            .await
            .unwrap_err();
        assert_eq!(error.error_code(), expected_code);
        server.await.unwrap();
    }
}

#[tokio::test]
async fn executor_request_classifies_malformed_response() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _ = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap();
        let payload = b"not-json";
        stream.write_u32(payload.len() as u32).await.unwrap();
        stream.write_all(payload).await.unwrap();
    });
    let client = ExecutorClient::new(socket);
    let error = client
        .request(Request::Probe(
            deploy_go_agent_executor::protocol::ProbeRequest {
                version: PROTOCOL_VERSION,
            },
        ))
        .await
        .unwrap_err();

    assert_eq!(error.error_code(), "executor_response_invalid");
    server.await.unwrap();
}

#[tokio::test]
async fn executor_request_reports_oversized_response_frame() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _ = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap();
        stream
            .write_u32((MAX_FRAME_BYTES as u32) + 1)
            .await
            .unwrap();
    });
    let client = ExecutorClient::new(socket);
    let error = client
        .request(Request::Probe(
            deploy_go_agent_executor::protocol::ProbeRequest {
                version: PROTOCOL_VERSION,
            },
        ))
        .await
        .unwrap_err();

    assert_eq!(error.error_code(), "executor_response_too_large");
    assert_eq!(error.response_frame_bytes(), Some(MAX_FRAME_BYTES + 1));
    assert_eq!(error.frame_limit_bytes(), Some(MAX_FRAME_BYTES));
    server.await.unwrap();
}

#[tokio::test]
async fn executor_request_classifies_partial_response_header_as_truncated() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _ = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap();
        stream.write_all(&[0, 0]).await.unwrap();
    });
    let client = ExecutorClient::new(socket);
    let error = client
        .request(Request::Probe(
            deploy_go_agent_executor::protocol::ProbeRequest {
                version: PROTOCOL_VERSION,
            },
        ))
        .await
        .unwrap_err();

    assert_eq!(error.error_code(), "executor_response_truncated");
    server.await.unwrap();
}

#[test]
fn executor_client_marks_only_post_connect_failures_as_potentially_processed() {
    use deploy_go_agent::executor_client::ExecutorClientError;
    use std::io;

    assert!(
        !ExecutorClientError::Unavailable {
            io_kind: io::ErrorKind::NotFound,
        }
        .request_may_have_been_processed()
    );
    assert!(
        !ExecutorClientError::RequestTooLarge {
            actual_bytes: MAX_FRAME_BYTES + 1,
            limit_bytes: MAX_FRAME_BYTES,
        }
        .request_may_have_been_processed()
    );
    assert!(
        ExecutorClientError::RequestWriteFailed {
            io_kind: io::ErrorKind::BrokenPipe,
        }
        .request_may_have_been_processed()
    );
    assert!(ExecutorClientError::ResponseTimeout.request_may_have_been_processed());
}
