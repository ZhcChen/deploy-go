use deploy_go_release_authorization::{
    AUDIENCE, AuthorizationError, Claims, ExpectedBinding, ExpectedSecretEnvironmentBinding,
    FileDigest, ReleaseSigner, ReleaseVerifier, SCHEMA_VERSION, SecretEnvironmentClaims,
};

fn claims() -> Claims {
    Claims {
        schema_version: SCHEMA_VERSION,
        audience: AUDIENCE.into(),
        authorization_id: "release_auth_01TEST".into(),
        nonce: "release_nonce_01TEST".into(),
        deployment_id: "deployment_01TEST".into(),
        target_run_id: "run_01TEST".into(),
        target_id: "target_01TEST".into(),
        node_id: "node_01TEST".into(),
        agent_id: "agent_01TEST".into(),
        snapshot_hash: format!("sha256:{}", "a".repeat(64)),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
        checkout_tree_digest: format!("sha256:{}", "b".repeat(64)),
        artifact_manifest_digest: format!("sha256:{}", "c".repeat(64)),
        artifacts: vec![FileDigest {
            relative_path: "api/app.tar.gz".into(),
            digest: format!("sha256:{}", "d".repeat(64)),
        }],
        env_files: vec![FileDigest {
            relative_path: "api.env".into(),
            digest: format!("sha256:{}", "e".repeat(64)),
        }],
        environment: "test".into(),
        release_version: "20260810000000".into(),
        modules: vec!["api".into()],
        task_payload_digest: format!("sha256:{}", "f".repeat(64)),
        cancel_file: "/run/deploy-go/release-01/cancel".into(),
        issued_at: 100,
        expires_at: 200,
        deadline_at: 200,
        secret_environment: None,
    }
}

fn binding(claims: &Claims) -> ExpectedBinding<'_> {
    ExpectedBinding {
        deployment_id: &claims.deployment_id,
        target_run_id: &claims.target_run_id,
        target_id: &claims.target_id,
        node_id: &claims.node_id,
        agent_id: &claims.agent_id,
        snapshot_hash: &claims.snapshot_hash,
        commit_sha: &claims.commit_sha,
        task_payload_digest: &claims.task_payload_digest,
        deadline_at: claims.deadline_at,
        secret_environment: None,
    }
}

#[test]
fn signs_and_verifies_release_specific_binding() {
    let claims = claims();
    let signer = ReleaseSigner::from_seed([9; 32]);
    let verifier = ReleaseVerifier::from_base64(&signer.public_key_base64()).unwrap();
    let token = signer.sign(&claims).unwrap();
    assert_eq!(
        verifier.verify(&token, &binding(&claims), 150).unwrap(),
        claims
    );
}

#[test]
fn rejects_tampering_expiry_and_wrong_binding() {
    let claims = claims();
    let signer = ReleaseSigner::from_seed([9; 32]);
    let verifier = signer.verifier();
    let token = signer.sign(&claims).unwrap();

    let mut tampered = token.into_bytes();
    tampered[10] ^= 1;
    assert!(matches!(
        verifier.verify(
            &String::from_utf8(tampered).unwrap(),
            &binding(&claims),
            150
        ),
        Err(AuthorizationError::InvalidSignature | AuthorizationError::InvalidFormat)
    ));

    let mut wrong = binding(&claims);
    wrong.agent_id = "agent_OTHER";
    let token = signer.sign(&claims).unwrap();
    assert_eq!(
        verifier.verify(&token, &wrong, 150),
        Err(AuthorizationError::BindingMismatch)
    );
    assert_eq!(
        verifier.verify(&token, &binding(&claims), 200),
        Err(AuthorizationError::InvalidTime)
    );
}

#[test]
fn rejects_terminal_shaped_or_unsafe_claims() {
    let signer = ReleaseSigner::from_seed([9; 32]);
    let mut unsafe_claims = claims();
    unsafe_claims.artifacts[0].relative_path = "../escape".into();
    assert_eq!(
        signer.sign(&unsafe_claims),
        Err(AuthorizationError::InvalidClaims)
    );

    let terminal_claims = serde_json::json!({
        "schema_version": 1,
        "capability_id": "cap_01TEST",
        "node_id": "node_01TEST",
        "agent_id": "agent_01TEST",
        "session_id": "term_01TEST",
        "connection_generation": 1,
        "issued_at": 100,
        "expires_at": 115
    });
    assert!(serde_json::from_value::<Claims>(terminal_claims).is_err());
}

#[test]
fn secret_environment_claims_are_bound_to_the_executor_request() {
    let mut claims = claims();
    claims.secret_environment = Some(SecretEnvironmentClaims {
        purpose: "config-center-connection".into(),
        variable_names: vec![
            "DEPLOY_CONFIG_CENTER_ENDPOINTS".into(),
            "DEPLOY_CONFIG_CENTER_PASSWORD".into(),
            "DEPLOY_CONFIG_CENTER_PREFIX".into(),
            "DEPLOY_CONFIG_CENTER_TYPE".into(),
            "DEPLOY_CONFIG_CENTER_USERNAME".into(),
        ],
        descriptor_digest: format!("sha256:{}", "1".repeat(64)),
        value_digest: format!("sha256:{}", "2".repeat(64)),
        credential_version: 7,
        template_id: "etcd".into(),
        template_version: "3.6".into(),
        template_digest: format!("sha256:{}", "3".repeat(64)),
        release_stage: "release".into(),
        executor_audience: "release_executor".into(),
        target_process: "deploy-release".into(),
    });
    let signer = ReleaseSigner::from_seed([13; 32]);
    let token = signer.sign(&claims).unwrap();
    let mut expected = binding(&claims);
    expected.secret_environment = Some(ExpectedSecretEnvironmentBinding {
        purpose: "config-center-connection",
        variable_names: &claims.secret_environment.as_ref().unwrap().variable_names,
        descriptor_digest: &claims
            .secret_environment
            .as_ref()
            .unwrap()
            .descriptor_digest,
        value_digest: &claims.secret_environment.as_ref().unwrap().value_digest,
        credential_version: 7,
        template_id: "etcd",
        template_version: "3.6",
        template_digest: &claims.secret_environment.as_ref().unwrap().template_digest,
        release_stage: "release",
        executor_audience: "release_executor",
        target_process: "deploy-release",
    });
    signer.verifier().verify(&token, &expected, 150).unwrap();

    let mut wrong = binding(&claims);
    wrong.secret_environment = Some(ExpectedSecretEnvironmentBinding {
        purpose: "config-center-connection",
        variable_names: &claims.secret_environment.as_ref().unwrap().variable_names,
        descriptor_digest: &claims
            .secret_environment
            .as_ref()
            .unwrap()
            .descriptor_digest,
        value_digest: "sha256:bad",
        credential_version: 7,
        template_id: "etcd",
        template_version: "3.6",
        template_digest: &claims.secret_environment.as_ref().unwrap().template_digest,
        release_stage: "release",
        executor_audience: "release_executor",
        target_process: "deploy-release",
    });
    assert_eq!(
        signer.verifier().verify(&token, &wrong, 150),
        Err(AuthorizationError::BindingMismatch)
    );
}
