use deploy_go_agent::executor_client::ExecutorClient;
use deploy_go_agent_executor::protocol::{
    ExecutorCapability, HealthyResponse, MAX_FRAME_BYTES, PROTOCOL_VERSION, Request, Response,
    read_request, write_message,
};
use tokio::net::UnixListener;

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
