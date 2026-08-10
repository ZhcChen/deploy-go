use std::{
    env, fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use deploy_go_release_authorization::ReleaseSigner;

#[derive(Debug, thiserror::Error)]
pub enum SignerConfigError {
    #[error("DEPLOY_GO_RELEASE_SIGNING_KEY_FILE 缺失")]
    Missing,
    #[error("特权发布私钥文件必须是 root 所有、服务组只读的 0440 普通文件")]
    InsecureFile,
    #[error("特权发布私钥必须是 base64 编码的 32 字节 seed")]
    InvalidKey,
}

pub fn signer_from_env() -> Result<ReleaseSigner, SignerConfigError> {
    let path =
        env::var("DEPLOY_GO_RELEASE_SIGNING_KEY_FILE").map_err(|_| SignerConfigError::Missing)?;
    let metadata = fs::symlink_metadata(&path).map_err(|_| SignerConfigError::InsecureFile)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != 0
        || metadata.permissions().mode() & 0o777 != 0o440
    {
        return Err(SignerConfigError::InsecureFile);
    }
    let value = fs::read_to_string(path).map_err(|_| SignerConfigError::InvalidKey)?;
    let bytes = STANDARD
        .decode(value.trim())
        .map_err(|_| SignerConfigError::InvalidKey)?;
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| SignerConfigError::InvalidKey)?;
    Ok(ReleaseSigner::from_seed(seed))
}
