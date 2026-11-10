use deploy_go_api::{
    application_configs,
    crypto::{EncryptedSecret, MasterKeyRing},
};
use deploy_go_api::{configuration_centers, db, git_credentials, ssh_credentials};
use sqlx::sqlite::SqlitePoolOptions;

const PLAINTEXT: &[u8] = b"private-key-material-that-must-stay-secret";

#[test]
fn encryption_uses_random_nonce_and_authenticated_context() {
    let ring = MasterKeyRing::from_raw(2, [7_u8; 32], Some((1, [3_u8; 32]))).unwrap();
    let first = ring.encrypt("cred_1", "ed25519", PLAINTEXT).unwrap();
    let second = ring.encrypt("cred_1", "ed25519", PLAINTEXT).unwrap();

    assert_ne!(first.nonce, second.nonce);
    assert_ne!(first.ciphertext, second.ciphertext);
    let decrypted = ring.decrypt("cred_1", "ed25519", &first).unwrap();
    assert_eq!(decrypted.as_slice(), PLAINTEXT);
    assert!(ring.decrypt("cred_other", "ed25519", &first).is_err());
}

#[test]
fn decryption_selects_version_and_rejects_wrong_master_key() {
    let old_ring = MasterKeyRing::from_raw(1, [3_u8; 32], None).unwrap();
    let encrypted = old_ring.encrypt("cred_1", "ed25519", PLAINTEXT).unwrap();
    let rotating = MasterKeyRing::from_raw(2, [7_u8; 32], Some((1, [3_u8; 32]))).unwrap();
    let decrypted = rotating.decrypt("cred_1", "ed25519", &encrypted).unwrap();
    assert_eq!(decrypted.as_slice(), PLAINTEXT);

    let wrong = MasterKeyRing::from_raw(2, [8_u8; 32], Some((1, [4_u8; 32]))).unwrap();
    assert!(wrong.decrypt("cred_1", "ed25519", &encrypted).is_err());
    let unknown = EncryptedSecret {
        key_version: 99,
        ..encrypted
    };
    assert!(rotating.decrypt("cred_1", "ed25519", &unknown).is_err());
}

#[test]
fn application_config_encryption_binds_application_file_version_and_previous_key() {
    let old_ring = MasterKeyRing::from_raw(1, [3_u8; 32], None).unwrap();
    let encrypted = old_ring
        .encrypt_application_config("app-1", "file-1", "version-1", PLAINTEXT)
        .unwrap();
    let rotating = MasterKeyRing::from_raw(2, [7_u8; 32], Some((1, [3_u8; 32]))).unwrap();
    assert_eq!(
        rotating
            .decrypt_application_config("app-1", "file-1", "version-1", &encrypted)
            .unwrap()
            .as_slice(),
        PLAINTEXT
    );
    assert!(
        rotating
            .decrypt_application_config("app-2", "file-1", "version-1", &encrypted)
            .is_err()
    );
    assert!(
        rotating
            .decrypt_application_config("app-1", "file-2", "version-1", &encrypted)
            .is_err()
    );
    assert!(
        rotating
            .decrypt_application_config("app-1", "file-1", "version-2", &encrypted)
            .is_err()
    );
    assert!(
        rotating
            .decrypt_application_env("app-1", "file-1", "version-1", &encrypted)
            .is_err()
    );
}

#[tokio::test]
async fn application_config_reencrypt_migrates_previous_key_without_changing_version_identity() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO applications(id,name,slug,status) VALUES('app-config-reencrypt','config','config-reencrypt','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_template_bindings(id,application_id,template_id,template_version,template_digest,source,status) VALUES('binding-config-reencrypt','app-config-reencrypt','redis','7','template-digest','template_creation','active')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_config_files(id,binding_id,application_id,path,deploy_path,format,language,role,delivery,template_source_digest,current_digest) VALUES('file-config-reencrypt','binding-config-reencrypt','app-config-reencrypt','redis.env.example','redis.env','dotenv','dotenv','configuration','artifact','template-digest','content-digest')")
        .execute(&pool)
        .await
        .unwrap();
    let old_ring = MasterKeyRing::from_raw(1, [3_u8; 32], None).unwrap();
    let encrypted = old_ring
        .encrypt_application_config(
            "app-config-reencrypt",
            "file-config-reencrypt",
            "version-config-reencrypt",
            b"REDIS_PASSWORD=secret\n",
        )
        .unwrap();
    sqlx::query("INSERT INTO application_config_versions(id,application_config_file_id,application_id,config_version,algorithm,ciphertext,nonce,key_version,digest,source) VALUES('version-config-reencrypt','file-config-reencrypt','app-config-reencrypt',1,'chacha20poly1305-application-config-v1',?,?,?,'content-digest','template')")
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce)
        .bind(encrypted.key_version)
        .execute(&pool)
        .await
        .unwrap();

    let rotating = MasterKeyRing::from_raw(2, [7_u8; 32], Some((1, [3_u8; 32]))).unwrap();
    assert_eq!(
        application_configs::reencrypt_all(&pool, &rotating)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        application_configs::reencrypt_all(&pool, &rotating)
            .await
            .unwrap(),
        0
    );
    let row: (Vec<u8>, Vec<u8>, i64, i64, String) = sqlx::query_as(
        "SELECT ciphertext,nonce,key_version,config_version,digest FROM application_config_versions WHERE id='version-config-reencrypt'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.2, 2);
    assert_eq!(row.3, 1);
    assert_eq!(row.4, "content-digest");
    assert_eq!(
        rotating
            .decrypt_application_config(
                "app-config-reencrypt",
                "file-config-reencrypt",
                "version-config-reencrypt",
                &EncryptedSecret {
                    ciphertext: row.0,
                    nonce: row.1,
                    key_version: row.2,
                },
            )
            .unwrap()
            .as_slice(),
        b"REDIS_PASSWORD=secret\n"
    );
}

#[test]
fn debug_output_never_contains_master_key_or_plaintext() {
    let ring = MasterKeyRing::from_raw(5, [9_u8; 32], None).unwrap();
    let encrypted = ring.encrypt("cred_1", "ed25519", PLAINTEXT).unwrap();
    let output = format!("{ring:?} {encrypted:?}");
    assert!(!output.contains(&base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        [9_u8; 32]
    )));
    assert!(!output.contains(std::str::from_utf8(PLAINTEXT).unwrap()));
}

#[test]
fn agent_tokens_are_stable_per_credential_and_key_version() {
    let ring = MasterKeyRing::from_raw(2, [7_u8; 32], Some((1, [3_u8; 32]))).unwrap();
    let first = ring.derive_agent_token("refresh", "refresh_01", 2).unwrap();
    let retry = ring.derive_agent_token("refresh", "refresh_01", 2).unwrap();
    assert_eq!(first.as_str(), retry.as_str());
    assert_ne!(
        first.as_str(),
        ring.derive_agent_token("access", "refresh_01", 2)
            .unwrap()
            .as_str()
    );
    assert_ne!(
        first.as_str(),
        ring.derive_agent_token("refresh", "refresh_02", 2)
            .unwrap()
            .as_str()
    );
    assert!(
        ring.derive_agent_token("refresh", "refresh_01", 99)
            .is_err()
    );
}

#[test]
fn etcd_credentials_use_separate_scoped_encryption_and_audit_hmac() {
    let old_ring = MasterKeyRing::from_raw(1, [3_u8; 32], None).unwrap();
    let admin = old_ring
        .encrypt_etcd_admin_credential("cc_admin", b"shared-secret")
        .unwrap();
    let business = old_ring
        .encrypt_etcd_business_credential("cc_business", b"shared-secret")
        .unwrap();
    assert_eq!(
        old_ring
            .decrypt_etcd_admin_credential("cc_admin", &admin)
            .unwrap()
            .as_slice(),
        b"shared-secret"
    );
    assert_eq!(
        old_ring
            .decrypt_etcd_business_credential("cc_business", &business)
            .unwrap()
            .as_slice(),
        b"shared-secret"
    );
    assert!(
        old_ring
            .decrypt_etcd_business_credential("cc_admin", &admin)
            .is_err()
    );
    assert!(
        old_ring
            .decrypt_etcd_business_credential("cc_business", &admin)
            .is_err()
    );
    assert!(old_ring.decrypt("cc_admin", "ed25519", &admin).is_err());

    let rotating = MasterKeyRing::from_raw(2, [7_u8; 32], Some((1, [3_u8; 32]))).unwrap();
    assert_eq!(
        rotating
            .decrypt_etcd_admin_credential("cc_admin", &admin)
            .unwrap()
            .as_slice(),
        b"shared-secret"
    );
    let first = rotating
        .audit_value_digest("app-1/prod/key", b"low-entropy")
        .unwrap();
    let second = rotating
        .audit_value_digest("app-1/prod/key", b"low-entropy")
        .unwrap();
    assert_eq!(first, second);
    assert!(first.starts_with("hmac-sha256-audit-v1:k2:"));
    assert!(!first.contains("low-entropy"));
    assert_ne!(
        first,
        rotating
            .audit_value_digest("app-2/prod/key", b"low-entropy")
            .unwrap()
    );
}

#[tokio::test]
async fn configuration_center_reencrypt_migrates_previous_key_version() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let old_ring = MasterKeyRing::from_raw(1, [3_u8; 32], None).unwrap();
    let encrypted = old_ring
        .encrypt_etcd_admin_credential("cc-reencrypt", b"admin-secret")
        .unwrap();
    sqlx::query("INSERT INTO configuration_center_credentials (id,purpose,algorithm,ciphertext,nonce,key_version,status) VALUES ('cc-reencrypt','platform_admin','chacha20poly1305-etcd-admin-v1',?,?,?,'active')")
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce)
        .bind(encrypted.key_version)
        .execute(&pool)
        .await
        .unwrap();

    let rotating = MasterKeyRing::from_raw(2, [7_u8; 32], Some((1, [3_u8; 32]))).unwrap();
    assert_eq!(
        configuration_centers::reencrypt_all(&pool, &rotating)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        configuration_centers::reencrypt_all(&pool, &rotating)
            .await
            .unwrap(),
        0
    );
    let row: (Vec<u8>, Vec<u8>, i64) = sqlx::query_as(
        "SELECT ciphertext, nonce, key_version FROM configuration_center_credentials WHERE id='cc-reencrypt'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let migrated = EncryptedSecret {
        ciphertext: row.0,
        nonce: row.1,
        key_version: row.2,
    };
    assert_eq!(
        rotating
            .decrypt_etcd_admin_credential("cc-reencrypt", &migrated)
            .unwrap()
            .as_slice(),
        b"admin-secret"
    );
}

#[tokio::test]
async fn reencrypt_migrates_previous_version_and_can_resume() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let old_ring = MasterKeyRing::from_raw(1, [3_u8; 32], None).unwrap();
    let encrypted = old_ring.encrypt("cred_1", "ed25519", PLAINTEXT).unwrap();
    sqlx::query("INSERT INTO ssh_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version) VALUES ('cred_1', 'Primary', 'ed25519', 'ssh-ed25519 AAAA', 'SHA256:test', ?, ?, ?)")
        .bind(encrypted.ciphertext).bind(encrypted.nonce).bind(encrypted.key_version)
        .execute(&pool).await.unwrap();

    let rotating = MasterKeyRing::from_raw(2, [7_u8; 32], Some((1, [3_u8; 32]))).unwrap();
    assert_eq!(
        ssh_credentials::reencrypt_all(&pool, &rotating)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        ssh_credentials::reencrypt_all(&pool, &rotating)
            .await
            .unwrap(),
        0
    );
    let row: (Vec<u8>, Vec<u8>, i64) = sqlx::query_as(
        "SELECT encrypted_private_key, nonce, key_version FROM ssh_credentials WHERE id = 'cred_1'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let migrated = EncryptedSecret {
        ciphertext: row.0,
        nonce: row.1,
        key_version: row.2,
    };
    assert_eq!(
        rotating
            .decrypt("cred_1", "ed25519", &migrated)
            .unwrap()
            .as_slice(),
        PLAINTEXT
    );
}

#[tokio::test]
async fn reencrypt_migrates_git_credentials_without_exposing_plaintext_fields() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    let old_ring = MasterKeyRing::from_raw(1, [3_u8; 32], None).unwrap();
    let encrypted = old_ring
        .encrypt("git_cred_1", "ed25519", PLAINTEXT)
        .unwrap();
    sqlx::query("INSERT INTO git_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version, status) VALUES ('git_cred_1', 'Primary', 'ed25519', 'ssh-ed25519 AAAA', 'SHA256:test', ?, ?, ?, 'active')")
        .bind(encrypted.ciphertext).bind(encrypted.nonce).bind(encrypted.key_version)
        .execute(&pool).await.unwrap();

    let rotating = MasterKeyRing::from_raw(2, [7_u8; 32], Some((1, [3_u8; 32]))).unwrap();
    assert_eq!(
        git_credentials::reencrypt_all(&pool, &rotating)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        git_credentials::reencrypt_all(&pool, &rotating)
            .await
            .unwrap(),
        0
    );
    let row: (Vec<u8>, Vec<u8>, i64) = sqlx::query_as(
        "SELECT encrypted_private_key, nonce, key_version FROM git_credentials WHERE id = 'git_cred_1'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let migrated = EncryptedSecret {
        ciphertext: row.0,
        nonce: row.1,
        key_version: row.2,
    };
    assert_eq!(
        rotating
            .decrypt("git_cred_1", "ed25519", &migrated)
            .unwrap()
            .as_slice(),
        PLAINTEXT
    );
}
