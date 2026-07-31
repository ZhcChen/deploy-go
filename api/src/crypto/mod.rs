use std::{env, fmt, fs, path::Path};

use base64::{Engine, engine::general_purpose::STANDARD};
use chacha20poly1305::{
    ChaCha20Poly1305, KeyInit,
    aead::{Aead, Payload},
};
use hkdf::Hkdf;
use rand::RngCore;
use serde::Serialize;
use sha2::Sha256;
use thiserror::Error;
use zeroize::Zeroizing;

const NONCE_LENGTH: usize = 12;
const KEY_LENGTH: usize = 32;
const HKDF_CONTEXT: &[u8] = b"deploy-go/ssh-credential/aead/v1";

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("主密钥版本配置无效")]
    InvalidKeyVersion,
    #[error("密钥派生失败")]
    KeyDerivation,
    #[error("凭证加密失败")]
    Encryption,
    #[error("凭证解密失败")]
    Decryption,
    #[error("凭证加密上下文无效")]
    InvalidContext,
    #[error("主密钥配置缺失或不完整")]
    MissingConfiguration,
    #[error("主密钥配置格式无效")]
    InvalidConfiguration,
    #[error("主密钥文件必须是仅限所有者访问的普通文件")]
    InsecureKeyFile,
}

#[derive(Clone)]
struct VersionedKey {
    version: i64,
    material: Zeroizing<[u8; KEY_LENGTH]>,
}

#[derive(Clone)]
pub struct MasterKeyRing {
    current: VersionedKey,
    previous: Option<VersionedKey>,
}

impl fmt::Debug for MasterKeyRing {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MasterKeyRing")
            .field("current_version", &self.current.version)
            .field(
                "previous_version",
                &self.previous.as_ref().map(|key| key.version),
            )
            .finish()
    }
}

#[derive(Clone)]
pub struct EncryptedSecret {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_version: i64,
}

impl fmt::Debug for EncryptedSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncryptedSecret")
            .field("ciphertext_bytes", &self.ciphertext.len())
            .field("nonce_bytes", &self.nonce.len())
            .field("key_version", &self.key_version)
            .finish()
    }
}

#[derive(Serialize)]
struct AssociatedData<'a> {
    credential_id: &'a str,
    algorithm: &'a str,
    key_version: i64,
}

impl MasterKeyRing {
    pub fn from_env() -> Result<Self, CryptoError> {
        let current_version = env_version("DEPLOY_GO_MASTER_KEY_VERSION")?;
        let current = load_key_source("DEPLOY_GO_MASTER_KEY", "DEPLOY_GO_MASTER_KEY_FILE")?;
        let previous_version = env::var("DEPLOY_GO_PREVIOUS_MASTER_KEY_VERSION").ok();
        let previous_value = env::var("DEPLOY_GO_PREVIOUS_MASTER_KEY").ok();
        let previous_file = env::var("DEPLOY_GO_PREVIOUS_MASTER_KEY_FILE").ok();
        let previous = match (previous_version, previous_value, previous_file) {
            (None, None, None) => None,
            (Some(version), value, file) => Some((
                parse_version(&version)?,
                load_key_values(value.as_deref(), file.as_deref())?,
            )),
            _ => return Err(CryptoError::MissingConfiguration),
        };
        Self::from_raw(current_version, current, previous)
    }

    pub fn from_base64(
        current_version: i64,
        current: &str,
        previous: Option<(i64, &str)>,
    ) -> Result<Self, CryptoError> {
        Self::from_raw(
            current_version,
            decode_key(current)?,
            previous
                .map(|(version, value)| decode_key(value).map(|key| (version, key)))
                .transpose()?,
        )
    }

    pub fn from_raw(
        current_version: i64,
        current: [u8; KEY_LENGTH],
        previous: Option<(i64, [u8; KEY_LENGTH])>,
    ) -> Result<Self, CryptoError> {
        if current_version <= 0
            || previous
                .as_ref()
                .is_some_and(|(version, _)| *version <= 0 || *version == current_version)
        {
            return Err(CryptoError::InvalidKeyVersion);
        }
        Ok(Self {
            current: VersionedKey {
                version: current_version,
                material: Zeroizing::new(current),
            },
            previous: previous.map(|(version, material)| VersionedKey {
                version,
                material: Zeroizing::new(material),
            }),
        })
    }

    pub fn current_version(&self) -> i64 {
        self.current.version
    }

    pub fn encrypt(
        &self,
        credential_id: &str,
        algorithm: &str,
        plaintext: &[u8],
    ) -> Result<EncryptedSecret, CryptoError> {
        let mut nonce = [0_u8; NONCE_LENGTH];
        rand::rng().fill_bytes(&mut nonce);
        let key = derive_key(&self.current)?;
        let cipher = ChaCha20Poly1305::new_from_slice(key.as_ref())
            .map_err(|_| CryptoError::KeyDerivation)?;
        let aad = associated_data(credential_id, algorithm, self.current.version)?;
        let ciphertext = cipher
            .encrypt(
                (&nonce).into(),
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| CryptoError::Encryption)?;
        Ok(EncryptedSecret {
            ciphertext,
            nonce: nonce.to_vec(),
            key_version: self.current.version,
        })
    }

    pub fn decrypt(
        &self,
        credential_id: &str,
        algorithm: &str,
        encrypted: &EncryptedSecret,
    ) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
        let versioned_key = self.key_for_version(encrypted.key_version)?;
        let nonce: [u8; NONCE_LENGTH] = encrypted
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::Decryption)?;
        let key = derive_key(versioned_key)?;
        let cipher = ChaCha20Poly1305::new_from_slice(key.as_ref())
            .map_err(|_| CryptoError::KeyDerivation)?;
        let aad = associated_data(credential_id, algorithm, encrypted.key_version)?;
        cipher
            .decrypt(
                (&nonce).into(),
                Payload {
                    msg: &encrypted.ciphertext,
                    aad: &aad,
                },
            )
            .map(Zeroizing::new)
            .map_err(|_| CryptoError::Decryption)
    }

    fn key_for_version(&self, version: i64) -> Result<&VersionedKey, CryptoError> {
        if self.current.version == version {
            return Ok(&self.current);
        }
        self.previous
            .as_ref()
            .filter(|key| key.version == version)
            .ok_or(CryptoError::Decryption)
    }
}

fn env_version(name: &str) -> Result<i64, CryptoError> {
    let value = env::var(name).map_err(|_| CryptoError::MissingConfiguration)?;
    parse_version(&value)
}

fn parse_version(value: &str) -> Result<i64, CryptoError> {
    value
        .parse::<i64>()
        .ok()
        .filter(|version| *version > 0)
        .ok_or(CryptoError::InvalidConfiguration)
}

fn load_key_source(value_name: &str, file_name: &str) -> Result<[u8; KEY_LENGTH], CryptoError> {
    let value = env::var(value_name).ok();
    let file = env::var(file_name).ok();
    load_key_values(value.as_deref(), file.as_deref())
}

fn load_key_values(
    encoded: Option<&str>,
    file: Option<&str>,
) -> Result<[u8; KEY_LENGTH], CryptoError> {
    match (encoded, file) {
        (Some(value), None) => decode_key(value),
        (None, Some(path)) => decode_key_file(Path::new(path)),
        _ => Err(CryptoError::MissingConfiguration),
    }
}

fn decode_key(encoded: &str) -> Result<[u8; KEY_LENGTH], CryptoError> {
    let decoded = Zeroizing::new(
        STANDARD
            .decode(encoded.trim())
            .map_err(|_| CryptoError::InvalidConfiguration)?,
    );
    decoded
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::InvalidConfiguration)
}

fn decode_key_file(path: &Path) -> Result<[u8; KEY_LENGTH], CryptoError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| CryptoError::InvalidConfiguration)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || insecure_permissions(&metadata) {
        return Err(CryptoError::InsecureKeyFile);
    }
    let encoded =
        Zeroizing::new(fs::read_to_string(path).map_err(|_| CryptoError::InvalidConfiguration)?);
    decode_key(&encoded)
}

#[cfg(unix)]
fn insecure_permissions(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o077 != 0
}

#[cfg(not(unix))]
fn insecure_permissions(_metadata: &fs::Metadata) -> bool {
    false
}

fn derive_key(versioned_key: &VersionedKey) -> Result<Zeroizing<[u8; KEY_LENGTH]>, CryptoError> {
    let hkdf = Hkdf::<Sha256>::new(None, versioned_key.material.as_ref());
    let mut output = Zeroizing::new([0_u8; KEY_LENGTH]);
    let mut info = Vec::with_capacity(HKDF_CONTEXT.len() + 8);
    info.extend_from_slice(HKDF_CONTEXT);
    info.extend_from_slice(&versioned_key.version.to_be_bytes());
    hkdf.expand(&info, output.as_mut())
        .map_err(|_| CryptoError::KeyDerivation)?;
    Ok(output)
}

fn associated_data(
    credential_id: &str,
    algorithm: &str,
    key_version: i64,
) -> Result<Vec<u8>, CryptoError> {
    serde_json::to_vec(&AssociatedData {
        credential_id,
        algorithm,
        key_version,
    })
    .map_err(|_| CryptoError::InvalidContext)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use base64::{Engine, engine::general_purpose::STANDARD};
    use tempfile::NamedTempFile;

    use super::{CryptoError, decode_key_file};

    #[cfg(unix)]
    #[test]
    fn key_file_requires_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{}", STANDARD.encode([7_u8; 32])).unwrap();
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            decode_key_file(file.path()),
            Err(CryptoError::InsecureKeyFile)
        ));
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(decode_key_file(file.path()).unwrap(), [7_u8; 32]);
    }
}
