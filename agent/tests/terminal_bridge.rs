use std::{path::PathBuf, time::Duration};

use deploy_go_agent::terminal::TerminalBridge;
use deploy_go_agent_executor::protocol::{
    CloseReason, ExitedResponse, HealthyResponse, MAX_FRAME_BYTES, OpenedResponse, OutputResponse,
    PROTOCOL_VERSION, Request, Response, read_request, write_message,
};
use deploy_go_agent_protocol::{Message, TerminalInput, TerminalOpen};
use tokio::{net::UnixListener, sync::mpsc};

#[tokio::test]
async fn missing_executor_is_reported_as_unavailable() {
    let bridge = TerminalBridge::new(PathBuf::from("/tmp/deploy-go-missing-executor.sock"));
    assert!(!bridge.probe().await);
}

#[tokio::test]
async fn open_failure_reports_executor_unavailable_without_blocking_control_loop() {
    let bridge = TerminalBridge::new(PathBuf::from("/tmp/deploy-go-missing-executor.sock"));
    let (outbound, mut messages) = mpsc::channel(4);
    bridge.handle(open("terminal_01"), outbound).await;
    let message = recv(&mut messages).await;
    assert!(matches!(message, Message::TerminalExited(_)));
}

#[tokio::test]
async fn probe_and_terminal_exit_allow_a_second_session_on_the_same_bridge() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (mut probe, _) = listener.accept().await.unwrap();
        assert!(matches!(
            read_request(&mut probe, MAX_FRAME_BYTES).await.unwrap(),
            Some(Request::Probe(_))
        ));
        write_message(
            &mut probe,
            &Response::Healthy(HealthyResponse {
                version: PROTOCOL_VERSION,
            }),
            MAX_FRAME_BYTES,
        )
        .await
        .unwrap();
        for id in ["terminal_01", "terminal_02"] {
            let (mut stream, _) = listener.accept().await.unwrap();
            assert!(matches!(
                read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap(),
                Some(Request::Open(_))
            ));
            write_message(
                &mut stream,
                &Response::Opened(OpenedResponse {
                    version: PROTOCOL_VERSION,
                    session_id: id.into(),
                }),
                MAX_FRAME_BYTES,
            )
            .await
            .unwrap();
            write_message(
                &mut stream,
                &Response::Output(OutputResponse {
                    version: PROTOCOL_VERSION,
                    session_id: id.into(),
                    sequence: 1,
                    data: b"root\n".to_vec(),
                }),
                MAX_FRAME_BYTES,
            )
            .await
            .unwrap();
            write_message(
                &mut stream,
                &Response::Exited(ExitedResponse {
                    version: PROTOCOL_VERSION,
                    session_id: id.into(),
                    reason: "process_exited".into(),
                    exit_code: Some(0),
                }),
                MAX_FRAME_BYTES,
            )
            .await
            .unwrap();
        }
    });
    let bridge = TerminalBridge::new(socket);
    assert!(bridge.probe().await);
    let (outbound, mut messages) = mpsc::channel(8);
    bridge.handle(open("terminal_01"), outbound.clone()).await;
    assert_sequences(&mut messages, "terminal_01").await;
    tokio::task::yield_now().await;
    bridge.handle(open("terminal_02"), outbound).await;
    assert_sequences(&mut messages, "terminal_02").await;
    server.await.unwrap();
}

#[tokio::test]
async fn out_of_order_input_closes_executor_and_reader_emits_the_only_terminal_exit() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let Some(Request::Open(open)) = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap()
        else {
            panic!()
        };
        write_message(
            &mut stream,
            &Response::Opened(OpenedResponse {
                version: PROTOCOL_VERSION,
                session_id: open.session_id.clone(),
            }),
            MAX_FRAME_BYTES,
        )
        .await
        .unwrap();
        let Some(Request::Close(close)) = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap()
        else {
            panic!()
        };
        assert_eq!(close.reason, CloseReason::ProtocolError);
        assert_eq!(close.sequence, 1);
        write_message(
            &mut stream,
            &Response::Exited(ExitedResponse {
                version: PROTOCOL_VERSION,
                session_id: open.session_id,
                reason: "protocol_error".into(),
                exit_code: None,
            }),
            MAX_FRAME_BYTES,
        )
        .await
        .unwrap();
    });
    let bridge = TerminalBridge::new(socket);
    let (outbound, mut messages) = mpsc::channel(8);
    bridge.handle(open("terminal_01"), outbound.clone()).await;
    let Message::TerminalOpened(opened) = recv(&mut messages).await else {
        panic!()
    };
    assert_eq!(opened.sequence, 1);
    bridge
        .handle(
            Message::TerminalInput(TerminalInput {
                session_id: "terminal_01".into(),
                sequence: 2,
                encoding: deploy_go_agent_protocol::TerminalBytesEncoding::Base64,
                data: "YQ==".into(),
            }),
            outbound,
        )
        .await;
    let Message::TerminalExited(exited) = recv(&mut messages).await else {
        panic!()
    };
    assert_eq!(exited.sequence, 2);
    server.await.unwrap();
}

#[tokio::test]
async fn control_connection_close_terminates_the_executor_session() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("executor.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let Some(Request::Open(open)) = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap()
        else {
            panic!()
        };
        write_message(
            &mut stream,
            &Response::Opened(OpenedResponse {
                version: PROTOCOL_VERSION,
                session_id: open.session_id,
            }),
            MAX_FRAME_BYTES,
        )
        .await
        .unwrap();
        let Some(Request::Close(close)) = read_request(&mut stream, MAX_FRAME_BYTES).await.unwrap()
        else {
            panic!()
        };
        assert_eq!(close.reason, CloseReason::PeerDisconnected);
        assert_eq!(close.sequence, 1);
    });
    let bridge = TerminalBridge::new(socket);
    let (outbound, mut messages) = mpsc::channel(8);
    bridge.handle(open("terminal_01"), outbound).await;
    assert!(matches!(
        recv(&mut messages).await,
        Message::TerminalOpened(_)
    ));

    bridge.close().await;

    server.await.unwrap();
}

fn open(session_id: &str) -> Message {
    Message::TerminalOpen(TerminalOpen {
        session_id: session_id.into(),
        sequence: 0,
        columns: 80,
        rows: 24,
        connection_generation: 7,
        capability: "signed-capability".into(),
    })
}

async fn recv(receiver: &mut mpsc::Receiver<Message>) -> Message {
    tokio::time::timeout(Duration::from_secs(1), receiver.recv())
        .await
        .unwrap()
        .unwrap()
}

async fn assert_sequences(receiver: &mut mpsc::Receiver<Message>, session_id: &str) {
    let messages = [
        recv(receiver).await,
        recv(receiver).await,
        recv(receiver).await,
    ];
    assert!(
        matches!(&messages[0], Message::TerminalOpened(value) if value.session_id == session_id && value.sequence == 1)
    );
    assert!(
        matches!(&messages[1], Message::TerminalOutput(value) if value.session_id == session_id && value.sequence == 2)
    );
    assert!(
        matches!(&messages[2], Message::TerminalExited(value) if value.session_id == session_id && value.sequence == 3)
    );
}
