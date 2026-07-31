use std::time::Duration;

use deploy_go_api::executor::deployment::{
    DeploymentExecutor, ExecutionContext, OpenSshDeploymentExecutor,
};
use zeroize::Zeroizing;

#[tokio::test]
async fn openssh_executor_streams_through_an_isolated_fixture() {
    let program = format!(
        "{}/tests/fixtures/mock_deployment_ssh.sh",
        env!("CARGO_MANIFEST_DIR")
    );
    let executor = OpenSshDeploymentExecutor::with_program(program);
    let context = ExecutionContext {
        deployment_id: "deployment_fixture".to_owned(),
        host: "fixture.invalid".to_owned(),
        port: 22,
        username: "deploy".to_owned(),
        work_root: "/srv/apps".to_owned(),
        script_path: "/srv/apps/deploy.sh".to_owned(),
        argument_tokens: vec!["--release".to_owned(), "1.0.0".to_owned()],
        environment: vec![],
        trusted_host_key: "fixture.invalid ssh-ed25519 AAAA".to_owned(),
        private_key: Zeroizing::new(b"fixture private key".to_vec()),
        timeout: Duration::from_secs(2),
    };
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    let execution = tokio::spawn(async move { executor.execute(&context, tx).await });
    let first = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.stream, "stdout");
    assert_eq!(first.bytes, b"fixture stdout\n");
    assert!(!execution.is_finished());
    let second = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.stream, "stderr");
    assert_eq!(execution.await.unwrap().unwrap(), 0);
}
