use deploy_go_api::crypto::{EncryptedSecret, MasterKeyRing};
use deploy_go_api::{db, ssh_credentials};
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
