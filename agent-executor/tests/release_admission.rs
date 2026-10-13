use deploy_go_agent_executor::{
    protocol::{PROTOCOL_VERSION, ReleaseStartRequest},
    release::{
        ARTIFACT_MANIFEST, ReleaseAdmission, ReleaseAdmissionError, checkout_tree_digest,
        file_digest,
    },
};
use deploy_go_release_authorization::{
    AUDIENCE, Claims, FileDigest, ReleaseSigner, SCHEMA_VERSION,
};
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

struct Fixture {
    _directory: tempfile::TempDir,
    admission: ReleaseAdmission,
    request: ReleaseStartRequest,
}

fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let source = root.join("source");
    let checkout = source.join("checkout");
    let artifact = source.join("artifacts");
    let env = source.join("env");
    fs::create_dir_all(checkout.join("scripts")).unwrap();
    fs::create_dir_all(checkout.join(".git")).unwrap();
    fs::create_dir_all(artifact.join("api")).unwrap();
    fs::create_dir_all(&env).unwrap();
    fs::write(checkout.join("Makefile"), "deploy-go-release:\n\t@id -u\n").unwrap();
    fs::write(checkout.join("scripts/release.sh"), "#!/bin/sh\nexit 0\n").unwrap();
    fs::write(checkout.join(".git/config"), "untrusted metadata").unwrap();
    // 隐藏目录里的普通文件正常参与摘要；符号链接相关测试会在此基础上改动它。
    fs::create_dir_all(checkout.join(".claude")).unwrap();
    fs::write(checkout.join(".claude/config.json"), "{}").unwrap();
    let artifact_bytes = b"release artifact";
    fs::write(artifact.join("api/app.tar.gz"), artifact_bytes).unwrap();
    let artifact_digest = format!("{:x}", sha2::Sha256::digest(artifact_bytes));
    let manifest = serde_json::json!({
        "schema_version":1,
        "release_version":"20260810000000",
        "commit_sha":"0123456789abcdef0123456789abcdef01234567",
        "artifacts":[{
            "module":"api",
            "path":"api/app.tar.gz",
            "sha256":artifact_digest,
            "size":artifact_bytes.len()
        }]
    });
    fs::write(
        artifact.join(ARTIFACT_MANIFEST),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(env.join("api.env"), "DATABASE_URL=fixture\n").unwrap();
    fs::write(source.join("cancel"), "").unwrap();

    let claims = Claims {
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
        checkout_tree_digest: format!("sha256:{}", checkout_tree_digest(&checkout).unwrap()),
        artifact_manifest_digest: format!(
            "sha256:{}",
            file_digest(&artifact.join(ARTIFACT_MANIFEST)).unwrap()
        ),
        artifacts: vec![FileDigest {
            relative_path: "api/app.tar.gz".into(),
            digest: format!("sha256:{artifact_digest}"),
        }],
        env_files: vec![FileDigest {
            relative_path: "api.env".into(),
            digest: format!("sha256:{}", file_digest(&env.join("api.env")).unwrap()),
        }],
        environment: "test".into(),
        release_version: "20260810000000".into(),
        modules: vec!["api".into()],
        task_payload_digest: format!("sha256:{}", "b".repeat(64)),
        cancel_file: source.join("cancel").display().to_string(),
        issued_at: 100,
        expires_at: 200,
        deadline_at: 200,
        secret_environment: None,
    };
    let signer = ReleaseSigner::from_seed([11; 32]);
    let request = ReleaseStartRequest {
        version: PROTOCOL_VERSION,
        job_id: "release_01TEST".into(),
        authorization: signer.sign(&claims).unwrap(),
        deployment_id: claims.deployment_id.clone(),
        target_run_id: claims.target_run_id.clone(),
        target_id: claims.target_id.clone(),
        node_id: claims.node_id.clone(),
        agent_id: claims.agent_id.clone(),
        snapshot_hash: claims.snapshot_hash.clone(),
        commit_sha: claims.commit_sha.clone(),
        checkout_dir: checkout.display().to_string(),
        artifact_dir: artifact.display().to_string(),
        env_dir: env.display().to_string(),
        cancel_file: claims.cancel_file.clone(),
        environment: claims.environment.clone(),
        release_version: claims.release_version.clone(),
        modules: claims.modules.clone(),
        target_code: "test".into(),
        task_payload_digest: claims.task_payload_digest.clone(),
        deadline_at: claims.deadline_at,
        secret_environment: None,
    };
    let admission = ReleaseAdmission::new(
        signer.verifier(),
        root.join("jobs"),
        claims.node_id,
        claims.agent_id,
    );
    Fixture {
        _directory: directory,
        admission,
        request,
    }
}

#[test]
fn seals_verified_inputs_into_read_only_bundle_and_consumes_nonce() {
    let fixture = fixture();
    let sealed = fixture.admission.admit(&fixture.request, 150).unwrap();
    assert!(sealed.checkout_dir.join("Makefile").is_file());
    assert!(!sealed.checkout_dir.join(".git").exists());
    assert!(sealed.artifact_dir.join("api/app.tar.gz").is_file());
    assert!(sealed.env_dir.join("api.env").is_file());
    assert_eq!(
        fs::metadata(&sealed.checkout_dir)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o500
    );
    assert_eq!(
        fs::metadata(sealed.checkout_dir.join("Makefile"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o400
    );
    assert_eq!(
        fixture.admission.admit(&fixture.request, 150),
        Err(ReleaseAdmissionError::JobConflict)
    );
}

#[test]
fn constructs_only_the_fixed_make_command_and_environment_whitelist() {
    let fixture = fixture();
    let sealed = fixture.admission.admit(&fixture.request, 150).unwrap();
    let command = sealed.command("test").unwrap();
    assert_eq!(command.get_program(), "/usr/bin/make");
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        ["--no-print-directory", "deploy-go-release"]
    );
    assert_eq!(
        command.get_current_dir(),
        Some(sealed.checkout_dir.as_path())
    );
    let environment = command
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.unwrap().to_string_lossy().into_owned(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        environment.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "DEPLOY_ARTIFACT_DIR",
            "DEPLOY_CANCEL_FILE",
            "DEPLOY_COMMIT_SHA",
            "DEPLOY_ENVIRONMENT",
            "DEPLOY_ENV_DIR",
            "DEPLOY_ID",
            "DEPLOY_MODULES",
            "DEPLOY_RELEASE_VERSION",
            "DEPLOY_TARGET",
            "PATH",
        ]
    );
    assert!(!environment.values().any(|value| value.contains("secret")));
}

#[test]
fn rejects_path_escape_symlink_hardlink_and_digest_changes() {
    let mut escaped = fixture();
    escaped.request.env_dir = escaped
        ._directory
        .path()
        .join("outside")
        .display()
        .to_string();
    fs::create_dir_all(&escaped.request.env_dir).unwrap();
    assert_eq!(
        escaped.admission.admit(&escaped.request, 150),
        Err(ReleaseAdmissionError::PathEscape)
    );

    let symlinked = fixture();
    let env_file = Path::new(&symlinked.request.env_dir).join("api.env");
    fs::remove_file(&env_file).unwrap();
    std::os::unix::fs::symlink("/etc/hosts", &env_file).unwrap();
    assert_eq!(
        symlinked.admission.admit(&symlinked.request, 150),
        Err(ReleaseAdmissionError::UnsafeFile)
    );

    let hardlinked = fixture();
    let env_file = Path::new(&hardlinked.request.env_dir).join("api.env");
    fs::hard_link(
        &env_file,
        Path::new(&hardlinked.request.env_dir).join("copy.env"),
    )
    .unwrap();
    assert_eq!(
        hardlinked.admission.admit(&hardlinked.request, 150),
        Err(ReleaseAdmissionError::UnsafeFile)
    );

    let changed = fixture();
    fs::write(
        Path::new(&changed.request.checkout_dir).join("Makefile"),
        "changed",
    )
    .unwrap();
    assert_eq!(
        changed.admission.admit(&changed.request, 150),
        Err(ReleaseAdmissionError::DigestMismatch)
    );
}

#[test]
fn rejects_wrong_binding_expired_authorization_and_nonce_replay() {
    let mut wrong = fixture();
    wrong.request.agent_id = "agent_OTHER".into();
    assert_eq!(
        wrong.admission.admit(&wrong.request, 150),
        Err(ReleaseAdmissionError::InvalidRequest)
    );

    let expired = fixture();
    assert_eq!(
        expired.admission.admit(&expired.request, 200),
        Err(ReleaseAdmissionError::Authorization)
    );

    let replayed = fixture();
    let mut second = replayed.request.clone();
    second.job_id = "release_02TEST".into();
    replayed.admission.admit(&replayed.request, 150).unwrap();
    assert_eq!(
        replayed.admission.admit(&second, 150),
        Err(ReleaseAdmissionError::Replayed)
    );
}

use sha2::Digest as _;

#[test]
fn admits_checkout_with_symlinks_in_hidden_top_level_directories_and_drops_them() {
    let fixture = fixture();
    let checkout = Path::new(&fixture.request.checkout_dir).to_path_buf();
    // 授权摘要是在没有链接时计算的：被忽略的链接不得改变它，否则准入会误报摘要不一致。
    fs::create_dir_all(checkout.join(".agents/skills")).unwrap();
    std::os::unix::fs::symlink(
        "../../scripts",
        checkout.join(".agents/skills/project-test-deploy"),
    )
    .unwrap();
    std::os::unix::fs::symlink("/etc", checkout.join(".outside")).unwrap();

    let sealed = fixture.admission.admit(&fixture.request, 150).unwrap();

    assert!(sealed.checkout_dir.join("Makefile").is_file());
    assert!(sealed.checkout_dir.join(".agents/skills").is_dir());
    for dropped in [".agents/skills/project-test-deploy", ".outside"] {
        assert!(
            fs::symlink_metadata(sealed.checkout_dir.join(dropped)).is_err(),
            "封存 bundle 中不得含符号链接：{dropped}"
        );
    }
}

#[test]
fn still_rejects_symlinks_outside_hidden_top_level_directories() {
    let fixture = fixture();
    let checkout = Path::new(&fixture.request.checkout_dir).to_path_buf();
    std::os::unix::fs::symlink("../Makefile", checkout.join("scripts/link")).unwrap();

    assert_eq!(
        fixture.admission.admit(&fixture.request, 150).unwrap_err(),
        ReleaseAdmissionError::UnsafeFile
    );
    // 被拒绝的请求既不能留下 job 目录，也不能留下 `.seal-*` 临时目录。
    let jobs = Path::new(&fixture.request.checkout_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("jobs");
    let leftovers = fs::read_dir(&jobs)
        .map(|entries| entries.flatten().count())
        .unwrap_or(0);
    assert_eq!(
        leftovers,
        0,
        "被拒绝的请求不得在 {} 留下任何内容",
        jobs.display()
    );
}

#[test]
fn swapping_an_authorized_regular_file_for_a_hidden_symlink_is_rejected() {
    // 授权摘要包含 .claude/config.json 这个普通文件；准入前它被换成指向别处的符号链接。
    // 链接会被忽略、文件从摘要中消失，因此必须与授权摘要不一致，不能借此让内容“消失”后放行。
    let fixture = fixture();
    let config = Path::new(&fixture.request.checkout_dir).join(".claude/config.json");
    assert!(config.is_file(), "fixture 必须把该文件纳入授权摘要");

    fs::remove_file(&config).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", &config).unwrap();

    assert_eq!(
        fixture.admission.admit(&fixture.request, 150).unwrap_err(),
        ReleaseAdmissionError::DigestMismatch
    );
}
