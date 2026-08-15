use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::{Path, PathBuf},
};

use flate2::{Compression, write::GzEncoder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tar::{Builder, Header};
use thiserror::Error;

pub const ARTIFACT_MANIFEST: &str = "deploy-go-artifact.json";
pub const TEMPLATE_ARCHIVE: &str = "template.tar.gz";
pub const MAX_IMAGE_BYTES: usize = 512;
pub const MAX_ENV_FILES: usize = 16;
pub const MAX_ENV_FILE_BYTES: usize = 132;

const REDIS_COMPOSE: &str = include_str!("../../examples/templates/redis/compose.yaml");
const REDIS_CONFIG: &str = include_str!("../../examples/templates/redis/config/redis.conf");
const POSTGRES_COMPOSE: &str = include_str!("../../examples/templates/postgres/compose.yaml");
const POSTGRES_CONFIG: &str =
    include_str!("../../examples/templates/postgres/config/postgresql.conf");
const ETCD_COMPOSE: &str = include_str!("../../examples/templates/etcd/compose.yaml");
const REDIS_MANIFEST: &str = include_str!("../../examples/templates/redis/deploy-go.yaml");
const POSTGRES_MANIFEST: &str = include_str!("../../examples/templates/postgres/deploy-go.yaml");
const ETCD_MANIFEST: &str = include_str!("../../examples/templates/etcd/deploy-go.yaml");
const REDIS_MAKEFILE: &str = include_str!("../../examples/templates/redis/Makefile");
const REDIS_RELEASE_SCRIPT: &str =
    include_str!("../../examples/templates/redis/scripts/release.sh");
const POSTGRES_MAKEFILE: &str = include_str!("../../examples/templates/postgres/Makefile");
const POSTGRES_RELEASE_SCRIPT: &str =
    include_str!("../../examples/templates/postgres/scripts/release.sh");
const ETCD_MAKEFILE: &str = include_str!("../../examples/templates/etcd/Makefile");
const ETCD_RELEASE_SCRIPT: &str = include_str!("../../examples/templates/etcd/scripts/release.sh");

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageTemplate {
    Redis,
    Postgres,
    Etcd,
}

impl std::fmt::Display for ImageTemplate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Redis => "redis",
            Self::Postgres => "postgres",
            Self::Etcd => "etcd",
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageDeploySpec {
    pub template: ImageTemplate,
    pub image: String,
    pub host_port: u16,
    pub env_files: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ApplicationManifest {
    schema_version: u32,
    #[serde(rename = "type")]
    app_type: String,
    type_version: String,
    modules: Vec<String>,
    #[serde(default)]
    env_files: Vec<String>,
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("image spec 无效: {0}")]
    InvalidSpec(String),
    #[error("模板内容无效: {0}")]
    InvalidTemplate(String),
    #[error("文件系统操作失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 序列化失败: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug)]
pub struct PlatformArtifact {
    pub archive_path: PathBuf,
    pub archive_digest: String,
    pub manifest_json: String,
    pub manifest_digest: String,
    pub total_size: u64,
    pub file_count: u32,
}

pub fn template_module(template: ImageTemplate) -> &'static str {
    match template {
        ImageTemplate::Redis => "redis",
        ImageTemplate::Postgres => "postgres",
        ImageTemplate::Etcd => "etcd",
    }
}

pub fn module_name(template: ImageTemplate) -> &'static str {
    match template {
        ImageTemplate::Redis => "Redis",
        ImageTemplate::Postgres => "PostgreSQL",
        ImageTemplate::Etcd => "etcd",
    }
}

pub fn required_env_files(template: ImageTemplate) -> Vec<&'static str> {
    match template {
        ImageTemplate::Redis => vec!["compose.env", "redis.env"],
        ImageTemplate::Postgres => vec!["compose.env", "postgres.env"],
        ImageTemplate::Etcd => vec!["compose.env", "etcd.env"],
    }
}

pub fn validate_application_manifest(
    template: ImageTemplate,
    manifest: &str,
) -> Result<(), TemplateError> {
    let parsed: ApplicationManifest = serde_yaml::from_str(manifest)
        .map_err(|error| TemplateError::InvalidTemplate(format!("deploy-go.yaml 无效: {error}")))?;
    let expected_type = match template {
        ImageTemplate::Redis => "redis",
        ImageTemplate::Postgres => "postgres",
        ImageTemplate::Etcd => "etcd",
    };
    let expected_version = match template {
        ImageTemplate::Redis => "7",
        ImageTemplate::Postgres => "18",
        ImageTemplate::Etcd => "3.6",
    };
    let expected_module = template_module(template);
    if parsed.schema_version != 1
        || parsed.app_type != expected_type
        || parsed.type_version != expected_version
        || parsed.modules != vec![expected_module.to_owned()]
        || !valid_env_files(&parsed.env_files)
        || required_env_files(template)
            .iter()
            .any(|required| !parsed.env_files.iter().any(|file| file == required))
    {
        return Err(TemplateError::InvalidTemplate(
            "deploy-go.yaml 与平台模板注册表不一致".into(),
        ));
    }
    Ok(())
}

fn valid_env_files(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    !values.is_empty()
        && values.len() <= MAX_ENV_FILES
        && values.iter().all(|value| valid_env_file_name(value))
        && values.iter().all(|value| seen.insert(value.as_str()))
}

pub fn validate_image_spec(spec: &ImageDeploySpec) -> Result<(), TemplateError> {
    if spec.image.is_empty() || spec.image.len() > MAX_IMAGE_BYTES {
        return Err(TemplateError::InvalidSpec(
            "image 长度必须在 1-512 字节之间".into(),
        ));
    }
    let bytes = spec.image.as_bytes();
    if !bytes[0].is_ascii_alphanumeric()
        || bytes.iter().any(|byte| {
            !(byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'.' | b'_' | b':' | b'/' | b'@' | b'+' | b'-' | b'[' | b']'
                ))
        })
    {
        return Err(TemplateError::InvalidSpec(
            "image 只允许安全字符，且不能以连字符开头".into(),
        ));
    }
    if spec.image.starts_with("http://") || spec.image.starts_with("https://") {
        return Err(TemplateError::InvalidSpec("image 不允许 URL scheme".into()));
    }
    if spec.host_port == 0 {
        return Err(TemplateError::InvalidSpec(
            "host_port 必须在 1-65535 之间".into(),
        ));
    }
    if spec.env_files.is_empty() || spec.env_files.len() > MAX_ENV_FILES {
        return Err(TemplateError::InvalidSpec(
            "env_files 必须包含 1-16 个文件".into(),
        ));
    }
    let required = required_env_files(spec.template);
    if required
        .iter()
        .any(|required| !spec.env_files.iter().any(|file| file == required))
    {
        return Err(TemplateError::InvalidSpec(format!(
            "{} 模板必须包含 Env 文件: {}",
            module_name(spec.template),
            required.join(", ")
        )));
    }
    let mut seen = BTreeSet::new();
    for file_name in &spec.env_files {
        if !valid_env_file_name(file_name) || !seen.insert(file_name.as_str()) {
            return Err(TemplateError::InvalidSpec(
                "env_files 必须是唯一的 *.env 安全文件名".into(),
            ));
        }
    }
    Ok(())
}

pub fn build_platform_artifact(
    spec: &ImageDeploySpec,
    release_version: &str,
    commit_sha: &str,
    work_dir: &Path,
) -> Result<PlatformArtifact, TemplateError> {
    validate_image_spec(spec)?;
    validate_release_identity(release_version, commit_sha)?;
    fs::create_dir_all(work_dir)?;
    let artifact_dir = work_dir.join("artifact");
    if artifact_dir.exists() {
        fs::remove_dir_all(&artifact_dir)?;
    }
    fs::create_dir_all(&artifact_dir)?;

    let template_size = write_template_archive(spec, &artifact_dir)?;
    let manifest_json = write_manifest(
        &artifact_dir,
        spec,
        release_version,
        commit_sha,
        template_size,
    )?;
    let archive_path = work_dir.join("image-deploy.tar");
    create_deterministic_archive(&artifact_dir, &archive_path)?;

    let manifest_digest = sha256_bytes(manifest_json.as_bytes());
    let archive_digest = sha256_file(&archive_path)?;
    Ok(PlatformArtifact {
        archive_path,
        archive_digest,
        manifest_json,
        manifest_digest,
        total_size: template_size,
        file_count: 1,
    })
}

pub fn write_checkout(root: &Path, spec: &ImageDeploySpec) -> Result<String, TemplateError> {
    validate_image_spec(spec)?;
    if root.exists() {
        fs::remove_dir_all(root)?;
    }
    fs::create_dir_all(root.join("scripts"))?;
    for (relative, content) in checkout_files(spec.template) {
        let path = root.join(relative);
        fs::write(&path, content.as_bytes())?;
        let mode = if relative == "scripts/release.sh" {
            0o755
        } else {
            0o644
        };
        set_mode(&path, mode)?;
    }
    checkout_digest(spec)
}

pub fn checkout_digest(spec: &ImageDeploySpec) -> Result<String, TemplateError> {
    validate_image_spec(spec)?;
    let mut hasher = Sha256::new();
    let files = checkout_files(spec.template)
        .into_iter()
        .map(|(relative, content)| (relative.to_owned(), content))
        .collect::<BTreeMap<_, _>>();
    for (relative, content) in files {
        let file_digest = format!("{:x}", Sha256::digest(content.as_bytes()));
        hasher.update((relative.len() as u64).to_be_bytes());
        hasher.update(relative.as_bytes());
        hasher.update(file_digest.as_bytes());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn checkout_files(template: ImageTemplate) -> Vec<(&'static str, &'static str)> {
    match template {
        ImageTemplate::Redis => vec![
            ("Makefile", REDIS_MAKEFILE),
            ("scripts/release.sh", REDIS_RELEASE_SCRIPT),
            ("deploy-go.yaml", REDIS_MANIFEST),
        ],
        ImageTemplate::Postgres => vec![
            ("Makefile", POSTGRES_MAKEFILE),
            ("scripts/release.sh", POSTGRES_RELEASE_SCRIPT),
            ("deploy-go.yaml", POSTGRES_MANIFEST),
        ],
        ImageTemplate::Etcd => vec![
            ("Makefile", ETCD_MAKEFILE),
            ("scripts/release.sh", ETCD_RELEASE_SCRIPT),
            ("deploy-go.yaml", ETCD_MANIFEST),
        ],
    }
}

fn template_files(template: ImageTemplate) -> Vec<(&'static str, &'static str)> {
    match template {
        ImageTemplate::Redis => vec![
            ("compose.yaml", REDIS_COMPOSE),
            ("config/redis.conf", REDIS_CONFIG),
            ("deploy-go.yaml", REDIS_MANIFEST),
        ],
        ImageTemplate::Postgres => {
            vec![
                ("compose.yaml", POSTGRES_COMPOSE),
                ("config/postgresql.conf", POSTGRES_CONFIG),
                ("deploy-go.yaml", POSTGRES_MANIFEST),
            ]
        }
        ImageTemplate::Etcd => vec![
            ("compose.yaml", ETCD_COMPOSE),
            ("deploy-go.yaml", ETCD_MANIFEST),
        ],
    }
}

fn compose_with_spec(spec: &ImageDeploySpec) -> Result<String, TemplateError> {
    let (image_marker, port_marker, container_port) = match spec.template {
        ImageTemplate::Redis => (
            "image: redis:7-alpine",
            "- \"${REDIS_PORT:-6379}:6379\"",
            "6379",
        ),
        ImageTemplate::Postgres => (
            "image: postgres:18-alpine",
            "- \"${POSTGRES_PORT:-5432}:5432\"",
            "5432",
        ),
        ImageTemplate::Etcd => (
            "image: gcr.io/etcd-development/etcd:v3.6.14",
            "- \"127.0.0.1:${ETCD_CLIENT_PORT:-2379}:2379\"",
            "2379",
        ),
    };
    let (_, source) = template_files(spec.template)
        .into_iter()
        .find(|(name, _)| *name == "compose.yaml")
        .ok_or_else(|| TemplateError::InvalidTemplate("compose.yaml 缺失".into()))?;
    let etcd_client_url_marker = "http://127.0.0.1:${ETCD_CLIENT_PORT:-2379}";
    if !source.contains(image_marker)
        || !source.contains(port_marker)
        || (matches!(spec.template, ImageTemplate::Etcd)
            && !source.contains(etcd_client_url_marker))
    {
        return Err(TemplateError::InvalidTemplate(
            "compose.yaml 与镜像模板占位符不一致".into(),
        ));
    }
    let port_mapping = match spec.template {
        ImageTemplate::Etcd => format!("- \"127.0.0.1:{}:{container_port}\"", spec.host_port),
        ImageTemplate::Redis | ImageTemplate::Postgres => {
            format!("- \"{}:{container_port}\"", spec.host_port)
        }
    };
    let rendered = source
        .replace(image_marker, &format!("image: {}", spec.image))
        .replace(port_marker, &port_mapping);
    let rendered = if matches!(spec.template, ImageTemplate::Etcd) {
        rendered.replace(
            etcd_client_url_marker,
            &format!("http://127.0.0.1:{}", spec.host_port),
        )
    } else {
        rendered
    };
    Ok(rendered)
}

fn write_template_archive(
    spec: &ImageDeploySpec,
    artifact_dir: &Path,
) -> Result<u64, TemplateError> {
    let module_dir = artifact_dir.join(template_module(spec.template));
    fs::create_dir_all(&module_dir)?;
    let archive_path = module_dir.join(TEMPLATE_ARCHIVE);
    let file = File::create(&archive_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);
    let compose = compose_with_spec(spec)?;
    let mut rendered = BTreeMap::new();
    for (name, content) in template_files(spec.template) {
        let content = if name == "compose.yaml" {
            compose.clone()
        } else {
            content.to_owned()
        };
        rendered.insert(name.to_owned(), content);
    }
    for (name, content) in checkout_files(spec.template) {
        if let Some(existing) = rendered.insert(name.to_owned(), content.to_owned())
            && existing != content
        {
            return Err(TemplateError::InvalidTemplate(
                "checkout 与发布物文件内容不一致".into(),
            ));
        }
    }
    for (name, content) in rendered {
        let mut header = Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        builder.append_data(&mut header, Path::new(&name), content.as_bytes())?;
    }
    builder.finish()?;
    let encoder = builder.into_inner()?;
    let file = encoder.finish()?;
    file.sync_all()?;
    Ok(file.metadata()?.len())
}

fn write_manifest(
    artifact_dir: &Path,
    spec: &ImageDeploySpec,
    release_version: &str,
    commit_sha: &str,
    template_size: u64,
) -> Result<String, TemplateError> {
    let module = template_module(spec.template);
    let archive_path = artifact_dir.join(module).join(TEMPLATE_ARCHIVE);
    let template_digest = sha256_file(&archive_path)?;
    let manifest = json!({
        "schema_version": 1,
        "release_version": release_version,
        "commit_sha": commit_sha,
        "artifacts": [{
            "module": module,
            "path": format!("{module}/{TEMPLATE_ARCHIVE}"),
            "sha256": template_digest,
            "size": template_size
        }]
    });
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(
        artifact_dir.join(ARTIFACT_MANIFEST),
        manifest_json.as_bytes(),
    )?;
    Ok(manifest_json)
}

fn create_deterministic_archive(root: &Path, archive_path: &Path) -> Result<(), TemplateError> {
    let mut files = Vec::new();
    collect_regular_files(root, root, &mut files)?;
    files.sort();
    let output = File::create(archive_path)?;
    let mut builder = Builder::new(output);
    for relative in files {
        let path = root.join(&relative);
        let mut source = File::open(&path)?;
        let metadata = source.metadata()?;
        let mut header = Header::new_gnu();
        header.set_size(metadata.len());
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        builder.append_data(&mut header, &relative, &mut source)?;
    }
    builder.finish()?;
    builder.into_inner()?.sync_all()?;
    Ok(())
}

fn collect_regular_files(
    root: &Path,
    current: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), TemplateError> {
    for item in fs::read_dir(current)? {
        let item = item?;
        let path = item.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(TemplateError::InvalidTemplate(
                "发布物目录不允许符号链接".into(),
            ));
        }
        if metadata.is_dir() {
            collect_regular_files(root, &path, output)?;
        } else if metadata.is_file() {
            output.push(
                path.strip_prefix(root)
                    .map_err(|_| TemplateError::InvalidTemplate("路径越界".into()))?
                    .to_owned(),
            );
        } else {
            return Err(TemplateError::InvalidTemplate(
                "发布物目录只允许普通文件".into(),
            ));
        }
    }
    Ok(())
}

fn validate_release_identity(release_version: &str, commit_sha: &str) -> Result<(), TemplateError> {
    if release_version.is_empty()
        || release_version.len() > 256
        || !release_version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        || commit_sha.len() < 40
        || commit_sha.len() > 64
        || !commit_sha.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(TemplateError::InvalidSpec(
            "release version 或 commit SHA 无效".into(),
        ));
    }
    Ok(())
}

fn valid_env_file_name(value: &str) -> bool {
    if value.len() < 5 || value.len() > MAX_ENV_FILE_BYTES || !value.ends_with(".env") {
        return false;
    }
    let bytes = value.as_bytes();
    bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> Result<String, TemplateError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn set_mode(path: &Path, mode: u32) -> Result<(), TemplateError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    let _ = mode;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    use flate2::read::GzDecoder;

    fn redis_spec() -> ImageDeploySpec {
        ImageDeploySpec {
            template: ImageTemplate::Redis,
            image: "docker.io/library/redis:7-alpine".into(),
            host_port: 6379,
            env_files: vec!["compose.env".into(), "redis.env".into()],
        }
    }

    fn postgres_spec() -> ImageDeploySpec {
        ImageDeploySpec {
            template: ImageTemplate::Postgres,
            image: "postgres:18-alpine".into(),
            host_port: 5432,
            env_files: vec!["compose.env".into(), "postgres.env".into()],
        }
    }

    fn etcd_spec() -> ImageDeploySpec {
        ImageDeploySpec {
            template: ImageTemplate::Etcd,
            image: "gcr.io/etcd-development/etcd:v3.6.14".into(),
            host_port: 2379,
            env_files: vec!["compose.env".into(), "etcd.env".into()],
        }
    }

    #[test]
    fn validates_image_spec_and_rejects_unsafe_inputs() {
        validate_image_spec(&redis_spec()).unwrap();
        validate_image_spec(&postgres_spec()).unwrap();
        validate_image_spec(&etcd_spec()).unwrap();
        let with_digest = ImageDeploySpec {
            image: "registry.example.test/library/redis@sha256:0123456789abcdef".into(),
            ..redis_spec()
        };
        validate_image_spec(&with_digest).unwrap();

        for image in [
            " redis:7-alpine",
            "--redis:7-alpine",
            "redis:7-alpine; id",
            "https://registry.example.test/redis",
            "redis:7-alpine\n",
            "$(id)",
            "",
        ] {
            let invalid = ImageDeploySpec {
                image: image.into(),
                ..redis_spec()
            };
            assert!(validate_image_spec(&invalid).is_err(), "accepted {image:?}");
        }
        assert!(
            validate_image_spec(&ImageDeploySpec {
                host_port: 0,
                ..redis_spec()
            })
            .is_err()
        );
        assert!(
            validate_image_spec(&ImageDeploySpec {
                env_files: Vec::new(),
                ..redis_spec()
            })
            .is_err()
        );
        assert!(
            validate_image_spec(&ImageDeploySpec {
                env_files: vec!["redis.env".into()],
                ..redis_spec()
            })
            .is_err()
        );
        assert!(
            validate_image_spec(&ImageDeploySpec {
                env_files: vec!["compose.env".into(), "redis.env".into(), "extra.env".into()],
                ..redis_spec()
            })
            .is_ok()
        );
        assert!(
            validate_image_spec(&ImageDeploySpec {
                env_files: vec!["compose.env".into(), "compose.env".into()],
                ..redis_spec()
            })
            .is_err()
        );
        assert!(
            validate_image_spec(&ImageDeploySpec {
                env_files: vec!["compose.env".into(), "../redis.env".into()],
                ..redis_spec()
            })
            .is_err()
        );
    }

    #[test]
    fn template_manifests_match_registry_and_reject_unknown_fields() {
        for (template, manifest) in [
            (ImageTemplate::Redis, REDIS_MANIFEST),
            (ImageTemplate::Postgres, POSTGRES_MANIFEST),
            (ImageTemplate::Etcd, ETCD_MANIFEST),
        ] {
            validate_application_manifest(template, manifest).unwrap();
        }
        assert!(validate_application_manifest(
            ImageTemplate::Redis,
            "schema_version: 1\ntype: redis\ntype_version: \"7\"\nmodules: [redis]\nenv_files: [compose.env, redis.env]\ncommand: id\n",
        )
        .is_err());
        assert!(validate_application_manifest(
            ImageTemplate::Redis,
            "schema_version: 1\ntype: redis\ntype_version: \"7\"\nmodules: [redis]\nenv_files: [compose.env]\n",
        )
        .is_err());
    }

    #[test]
    fn builds_platform_artifact_with_expected_layout() {
        let directory = tempfile::tempdir().unwrap();
        let artifact = build_platform_artifact(
            &redis_spec(),
            "202608110001",
            "0123456789abcdef0123456789abcdef01234567",
            directory.path(),
        )
        .unwrap();
        assert_eq!(artifact.file_count, 1);
        assert_eq!(
            artifact.total_size,
            fs::metadata(directory.path().join("artifact/redis/template.tar.gz"))
                .unwrap()
                .len()
        );
        assert_eq!(
            artifact.archive_digest,
            sha256_file(&artifact.archive_path).unwrap()
        );
        assert_eq!(
            artifact.manifest_digest,
            sha256_bytes(artifact.manifest_json.as_bytes())
        );

        let mut outer = tar::Archive::new(File::open(&artifact.archive_path).unwrap());
        let mut names = Vec::new();
        for entry in outer.entries().unwrap() {
            let entry = entry.unwrap();
            names.push(entry.path().unwrap().to_str().unwrap().to_owned());
        }
        assert_eq!(
            names,
            vec!["deploy-go-artifact.json", "redis/template.tar.gz"]
        );

        let manifest: serde_json::Value = serde_json::from_str(&artifact.manifest_json).unwrap();
        assert_eq!(manifest["artifacts"][0]["module"], "redis");
        assert_eq!(manifest["artifacts"][0]["path"], "redis/template.tar.gz");

        let inner = GzDecoder::new(
            File::open(directory.path().join("artifact/redis/template.tar.gz")).unwrap(),
        );
        let mut inner_archive = tar::Archive::new(inner);
        let mut inner_names = Vec::new();
        for entry in inner_archive.entries().unwrap() {
            let entry = entry.unwrap();
            inner_names.push(entry.path().unwrap().to_str().unwrap().to_owned());
        }
        assert_eq!(
            inner_names,
            vec![
                "Makefile",
                "compose.yaml",
                "config/redis.conf",
                "deploy-go.yaml",
                "scripts/release.sh",
            ]
        );
    }

    #[test]
    fn rendered_compose_uses_fixed_image_and_host_port() {
        let directory = tempfile::tempdir().unwrap();
        build_platform_artifact(
            &redis_spec(),
            "202608110001",
            "0123456789abcdef0123456789abcdef01234567",
            directory.path(),
        )
        .unwrap();
        let inner = GzDecoder::new(
            File::open(directory.path().join("artifact/redis/template.tar.gz")).unwrap(),
        );
        let mut archive = tar::Archive::new(inner);
        let mut compose = String::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap().to_str().unwrap() == "compose.yaml" {
                entry.read_to_string(&mut compose).unwrap();
            }
        }
        assert!(compose.contains("image: docker.io/library/redis:7-alpine"));
        assert!(compose.contains("- \"6379:6379\""));
        assert!(!compose.contains("${REDIS_PORT"));
    }

    #[test]
    fn etcd_artifact_keeps_client_port_on_loopback() {
        let directory = tempfile::tempdir().unwrap();
        let spec = ImageDeploySpec {
            host_port: 12379,
            ..etcd_spec()
        };
        build_platform_artifact(
            &spec,
            "202608150001",
            "0123456789abcdef0123456789abcdef01234567",
            directory.path(),
        )
        .unwrap();
        let inner = GzDecoder::new(
            File::open(directory.path().join("artifact/etcd/template.tar.gz")).unwrap(),
        );
        let mut archive = tar::Archive::new(inner);
        let mut names = Vec::new();
        let mut compose = String::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let name = entry.path().unwrap().to_str().unwrap().to_owned();
            if name == "compose.yaml" {
                entry.read_to_string(&mut compose).unwrap();
            }
            names.push(name);
        }
        assert_eq!(
            names,
            vec![
                "Makefile",
                "compose.yaml",
                "deploy-go.yaml",
                "scripts/release.sh",
            ]
        );
        assert!(compose.contains("image: gcr.io/etcd-development/etcd:v3.6.14"));
        assert!(compose.contains("- \"127.0.0.1:12379:2379\""));
        assert!(compose.contains("ETCD_ADVERTISE_CLIENT_URLS: \"http://127.0.0.1:12379\""));
        assert!(!compose.contains("${ETCD_CLIENT_PORT"));
    }

    #[test]
    fn checkout_digest_matches_written_tree_and_is_stable_per_template() {
        let directory = tempfile::tempdir().unwrap();
        let spec = redis_spec();
        let digest = write_checkout(directory.path(), &spec).unwrap();
        assert_eq!(digest, checkout_digest(&spec).unwrap());
        let mut hasher = Sha256::new();
        let files = checkout_files(spec.template)
            .into_iter()
            .map(|(relative, content)| (relative.to_owned(), content))
            .collect::<BTreeMap<_, _>>();
        for (relative, content) in files {
            let file_digest = format!("{:x}", Sha256::digest(content.as_bytes()));
            hasher.update((relative.len() as u64).to_be_bytes());
            hasher.update(relative.as_bytes());
            hasher.update(file_digest.as_bytes());
        }
        assert_eq!(digest, format!("{:x}", hasher.finalize()));

        let mut changed_image = redis_spec();
        changed_image.image = "redis:7.4-alpine".into();
        assert_eq!(
            checkout_digest(&redis_spec()).unwrap(),
            checkout_digest(&changed_image).unwrap()
        );
    }
}
