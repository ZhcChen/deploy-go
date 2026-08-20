use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
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
pub const MAX_TEMPLATE_FILES: usize = 64;
pub const MAX_TEMPLATE_FILE_BYTES: usize = 1024 * 1024;
pub const MAX_TEMPLATE_TOTAL_BYTES: usize = 4 * 1024 * 1024;

const REDIS_COMPOSE: &str = include_str!("../../examples/templates/redis/compose.yaml");
const REDIS_CONFIG: &str = include_str!("../../examples/templates/redis/config/redis.conf");
const VALKEY_COMPOSE: &str = include_str!("../../examples/templates/valkey/compose.yaml");
const VALKEY_CONFIG: &str = include_str!("../../examples/templates/valkey/config/valkey.conf");
const POSTGRES_COMPOSE: &str = include_str!("../../examples/templates/postgres/compose.yaml");
const POSTGRES_CONFIG: &str =
    include_str!("../../examples/templates/postgres/config/postgresql.conf");
const ETCD_COMPOSE: &str = include_str!("../../examples/templates/etcd/compose.yaml");
const REDIS_MANIFEST: &str = include_str!("../../examples/templates/redis/deploy-go.yaml");
const VALKEY_MANIFEST: &str = include_str!("../../examples/templates/valkey/deploy-go.yaml");
const POSTGRES_MANIFEST: &str = include_str!("../../examples/templates/postgres/deploy-go.yaml");
const ETCD_MANIFEST: &str = include_str!("../../examples/templates/etcd/deploy-go.yaml");
const REDIS_MAKEFILE: &str = include_str!("../../examples/templates/redis/Makefile");
const REDIS_RELEASE_SCRIPT: &str =
    include_str!("../../examples/templates/redis/scripts/release.sh");
const VALKEY_MAKEFILE: &str = include_str!("../../examples/templates/valkey/Makefile");
const VALKEY_RELEASE_SCRIPT: &str =
    include_str!("../../examples/templates/valkey/scripts/release.sh");
const POSTGRES_MAKEFILE: &str = include_str!("../../examples/templates/postgres/Makefile");
const POSTGRES_RELEASE_SCRIPT: &str =
    include_str!("../../examples/templates/postgres/scripts/release.sh");
const ETCD_MAKEFILE: &str = include_str!("../../examples/templates/etcd/Makefile");
const ETCD_RELEASE_SCRIPT: &str = include_str!("../../examples/templates/etcd/scripts/release.sh");

const REDIS_COMPOSE_ENV: &str = include_str!("../../examples/templates/redis/compose.env.example");
const REDIS_SERVICE_ENV: &str = include_str!("../../examples/templates/redis/redis.env.example");
const REDIS_README: &str = include_str!("../../examples/templates/redis/README.md");
const REDIS_SCHEMA: &str = include_str!("../../examples/templates/redis/parameter-schema.json");
const VALKEY_COMPOSE_ENV: &str =
    include_str!("../../examples/templates/valkey/compose.env.example");
const VALKEY_SERVICE_ENV: &str = include_str!("../../examples/templates/valkey/valkey.env.example");
const VALKEY_README: &str = include_str!("../../examples/templates/valkey/README.md");
const VALKEY_SCHEMA: &str = include_str!("../../examples/templates/valkey/parameter-schema.json");
const POSTGRES_COMPOSE_ENV: &str =
    include_str!("../../examples/templates/postgres/compose.env.example");
const POSTGRES_SERVICE_ENV: &str =
    include_str!("../../examples/templates/postgres/postgres.env.example");
const POSTGRES_README: &str = include_str!("../../examples/templates/postgres/README.md");
const POSTGRES_SCHEMA: &str =
    include_str!("../../examples/templates/postgres/parameter-schema.json");
const ETCD_COMPOSE_ENV: &str = include_str!("../../examples/templates/etcd/compose.env.example");
const ETCD_SERVICE_ENV: &str = include_str!("../../examples/templates/etcd/etcd.env.example");
const ETCD_README: &str = include_str!("../../examples/templates/etcd/README.md");
const ETCD_SCHEMA: &str = include_str!("../../examples/templates/etcd/parameter-schema.json");

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageTemplate {
    Redis,
    Valkey,
    Postgres,
    Etcd,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateFileFormat {
    Yaml,
    Dotenv,
    Ini,
    Json,
    Markdown,
    Shell,
    Makefile,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateFileRole {
    Configuration,
    Reference,
    PlatformManaged,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateFileDelivery {
    Artifact,
    EnvLease,
    SecretFileLease,
    Reference,
    PlatformManaged,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateDeploymentMechanism {
    Image,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateFileDescriptor {
    pub path: String,
    pub deploy_path: Option<String>,
    pub label: String,
    pub format: TemplateFileFormat,
    pub language: String,
    pub role: TemplateFileRole,
    pub delivery: TemplateFileDelivery,
    pub editable: bool,
    pub sensitive: bool,
    pub description: String,
    pub recommended_changes: String,
    pub digest: String,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateDescriptor {
    pub id: String,
    pub version: String,
    pub name: String,
    pub summary: String,
    pub deployment_mechanism: TemplateDeploymentMechanism,
    pub default_image: String,
    pub default_port: u16,
    pub digest: String,
    pub files: Vec<TemplateFileDescriptor>,
}

impl std::fmt::Display for ImageTemplate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Redis => "redis",
            Self::Valkey => "valkey",
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
        ImageTemplate::Valkey => "valkey",
        ImageTemplate::Postgres => "postgres",
        ImageTemplate::Etcd => "etcd",
    }
}

pub fn module_name(template: ImageTemplate) -> &'static str {
    match template {
        ImageTemplate::Redis => "Redis",
        ImageTemplate::Valkey => "Valkey",
        ImageTemplate::Postgres => "PostgreSQL",
        ImageTemplate::Etcd => "etcd",
    }
}

pub fn template_version(template: ImageTemplate) -> &'static str {
    match template {
        ImageTemplate::Redis => "7",
        ImageTemplate::Valkey => "9",
        ImageTemplate::Postgres => "18",
        ImageTemplate::Etcd => "3.6",
    }
}

pub fn default_image(template: ImageTemplate) -> &'static str {
    match template {
        ImageTemplate::Redis => "redis:7-alpine",
        ImageTemplate::Valkey => "valkey/valkey:9-alpine",
        ImageTemplate::Postgres => "postgres:18-alpine",
        ImageTemplate::Etcd => "gcr.io/etcd-development/etcd:v3.6.14",
    }
}

pub fn default_port(template: ImageTemplate) -> u16 {
    match template {
        ImageTemplate::Redis => 6379,
        ImageTemplate::Valkey => 6379,
        ImageTemplate::Postgres => 5432,
        ImageTemplate::Etcd => 2379,
    }
}

pub fn template_descriptor(template: ImageTemplate) -> TemplateDescriptor {
    let (id, name, summary, files) = match template {
        ImageTemplate::Redis => (
            "redis",
            "Redis 7",
            "Docker Compose 部署 Redis，AOF 持久化、健康检查与应用配置只读挂载。",
            vec![
                descriptor_file(
                    "README.md",
                    None,
                    "说明",
                    TemplateFileFormat::Markdown,
                    "markdown",
                    TemplateFileRole::Reference,
                    false,
                    false,
                    REDIS_README,
                    "查看部署边界、目录结构和发布前检查项。",
                    "只读参考，不作为部署输入修改。",
                ),
                descriptor_file(
                    "compose.yaml",
                    Some("compose.yaml"),
                    "Compose 编排",
                    TemplateFileFormat::Yaml,
                    "yaml",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    REDIS_COMPOSE,
                    "定义 Redis 服务、端口、健康检查和数据卷。",
                    "可调整端口和服务参数，但不能启用特权、宿主命名空间或越界挂载。",
                ),
                descriptor_file(
                    "compose.env.example",
                    Some("compose.env"),
                    "Compose Env 字段",
                    TemplateFileFormat::Dotenv,
                    "dotenv",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    REDIS_COMPOSE_ENV,
                    "提供 Compose 端口和时区等非敏感变量示例。",
                    "按目标节点需求调整端口；正式值通过应用 Env 版本保存。",
                ),
                descriptor_file(
                    "redis.env.example",
                    Some("redis.env"),
                    "服务 Env 字段",
                    TemplateFileFormat::Dotenv,
                    "dotenv",
                    TemplateFileRole::Configuration,
                    true,
                    true,
                    REDIS_SERVICE_ENV,
                    "提供 Redis 服务级变量和密码占位符。",
                    "必须替换密码占位符；敏感值使用受保护的 Env 版本保存。",
                ),
                descriptor_file(
                    "config/redis.conf",
                    Some("config/redis.conf"),
                    "Redis 服务配置",
                    TemplateFileFormat::Ini,
                    "redis",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    REDIS_CONFIG,
                    "提供 Redis 持久化、内存和运行参数。",
                    "只调整模板允许的服务参数，不覆盖平台发布入口或安全边界。",
                ),
                descriptor_file(
                    "parameter-schema.json",
                    None,
                    "参数 Schema",
                    TemplateFileFormat::Json,
                    "json",
                    TemplateFileRole::Reference,
                    false,
                    false,
                    REDIS_SCHEMA,
                    "描述模板参数和默认值关系。",
                    "只读参考，部署校验以控制面契约为准。",
                ),
                descriptor_file(
                    "deploy-go.yaml",
                    None,
                    "Deploy Go 清单",
                    TemplateFileFormat::Yaml,
                    "yaml",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    REDIS_MANIFEST,
                    "声明模板类型、版本和 Env 文件清单。",
                    "由平台维护，应用配置副本不能覆盖。",
                ),
                descriptor_file(
                    "Makefile",
                    None,
                    "发布入口",
                    TemplateFileFormat::Makefile,
                    "makefile",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    REDIS_MAKEFILE,
                    "提供平台托管的发布目标。",
                    "由平台维护，不能修改执行入口。",
                ),
                descriptor_file(
                    "scripts/release.sh",
                    None,
                    "发布脚本",
                    TemplateFileFormat::Shell,
                    "shell",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    REDIS_RELEASE_SCRIPT,
                    "执行平台签名链路下的 Compose 发布。",
                    "由平台维护，不能上传或覆盖任意脚本。",
                ),
            ],
        ),
        ImageTemplate::Valkey => (
            "valkey",
            "Valkey 9",
            "Docker Compose 部署 Valkey 9，AOF 持久化、健康检查与应用配置只读挂载。",
            vec![
                descriptor_file(
                    "README.md",
                    None,
                    "说明",
                    TemplateFileFormat::Markdown,
                    "markdown",
                    TemplateFileRole::Reference,
                    false,
                    false,
                    VALKEY_README,
                    "查看部署边界、目录结构和发布前检查项。",
                    "只读参考，不作为部署输入修改。",
                ),
                descriptor_file(
                    "compose.yaml",
                    Some("compose.yaml"),
                    "Compose 编排",
                    TemplateFileFormat::Yaml,
                    "yaml",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    VALKEY_COMPOSE,
                    "定义 Valkey 服务、端口、健康检查和数据卷。",
                    "可调整端口和服务参数，但不能启用特权、宿主命名空间或越界挂载。",
                ),
                descriptor_file(
                    "compose.env.example",
                    Some("compose.env"),
                    "Compose Env 字段",
                    TemplateFileFormat::Dotenv,
                    "dotenv",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    VALKEY_COMPOSE_ENV,
                    "提供 Compose 端口和时区等非敏感变量示例。",
                    "按目标节点需求调整端口；正式值通过应用 Env 版本保存。",
                ),
                descriptor_file(
                    "valkey.env.example",
                    Some("valkey.env"),
                    "服务 Env 字段",
                    TemplateFileFormat::Dotenv,
                    "dotenv",
                    TemplateFileRole::Configuration,
                    true,
                    true,
                    VALKEY_SERVICE_ENV,
                    "提供 Valkey 服务级变量和密码占位符。",
                    "必须替换密码占位符；敏感值使用受保护的 Env 版本保存。",
                ),
                descriptor_file(
                    "config/valkey.conf",
                    Some("config/valkey.conf"),
                    "Valkey 服务配置",
                    TemplateFileFormat::Ini,
                    "valkey",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    VALKEY_CONFIG,
                    "提供 Valkey 持久化、内存和运行参数。",
                    "只调整模板允许的服务参数，不覆盖平台发布入口或安全边界。",
                ),
                descriptor_file(
                    "parameter-schema.json",
                    None,
                    "参数 Schema",
                    TemplateFileFormat::Json,
                    "json",
                    TemplateFileRole::Reference,
                    false,
                    false,
                    VALKEY_SCHEMA,
                    "描述模板参数和默认值关系。",
                    "只读参考，部署校验以控制面契约为准。",
                ),
                descriptor_file(
                    "deploy-go.yaml",
                    None,
                    "Deploy Go 清单",
                    TemplateFileFormat::Yaml,
                    "yaml",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    VALKEY_MANIFEST,
                    "声明模板类型、版本和 Env 文件清单。",
                    "由平台维护，应用配置副本不能覆盖。",
                ),
                descriptor_file(
                    "Makefile",
                    None,
                    "发布入口",
                    TemplateFileFormat::Makefile,
                    "makefile",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    VALKEY_MAKEFILE,
                    "提供平台托管的发布目标。",
                    "由平台维护，不能修改执行入口。",
                ),
                descriptor_file(
                    "scripts/release.sh",
                    None,
                    "发布脚本",
                    TemplateFileFormat::Shell,
                    "shell",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    VALKEY_RELEASE_SCRIPT,
                    "执行平台签名链路下的 Compose 发布。",
                    "由平台维护，不能上传或覆盖任意脚本。",
                ),
            ],
        ),
        ImageTemplate::Postgres => (
            "postgres",
            "PostgreSQL 18",
            "Docker Compose 部署 PostgreSQL，数据卷持久化、健康检查与应用配置只读挂载。",
            vec![
                descriptor_file(
                    "README.md",
                    None,
                    "说明",
                    TemplateFileFormat::Markdown,
                    "markdown",
                    TemplateFileRole::Reference,
                    false,
                    false,
                    POSTGRES_README,
                    "查看部署边界、目录结构和发布前检查项。",
                    "只读参考，不作为部署输入修改。",
                ),
                descriptor_file(
                    "compose.yaml",
                    Some("compose.yaml"),
                    "Compose 编排",
                    TemplateFileFormat::Yaml,
                    "yaml",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    POSTGRES_COMPOSE,
                    "定义 PostgreSQL 服务、端口、健康检查和数据卷。",
                    "可调整端口和服务参数，但不能启用特权、宿主命名空间或越界挂载。",
                ),
                descriptor_file(
                    "compose.env.example",
                    Some("compose.env"),
                    "Compose Env 字段",
                    TemplateFileFormat::Dotenv,
                    "dotenv",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    POSTGRES_COMPOSE_ENV,
                    "提供 Compose 端口和时区等非敏感变量示例。",
                    "按目标节点需求调整端口；正式值通过应用 Env 版本保存。",
                ),
                descriptor_file(
                    "postgres.env.example",
                    Some("postgres.env"),
                    "服务 Env 字段",
                    TemplateFileFormat::Dotenv,
                    "dotenv",
                    TemplateFileRole::Configuration,
                    true,
                    true,
                    POSTGRES_SERVICE_ENV,
                    "提供 PostgreSQL 数据库、用户和密码占位符。",
                    "必须替换密码占位符；敏感值使用受保护的 Env 版本保存。",
                ),
                descriptor_file(
                    "config/postgresql.conf",
                    Some("config/postgresql.conf"),
                    "PostgreSQL 服务配置",
                    TemplateFileFormat::Ini,
                    "postgresql",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    POSTGRES_CONFIG,
                    "提供 PostgreSQL 连接、日志和运行参数。",
                    "只调整模板允许的服务参数，不覆盖平台发布入口或安全边界。",
                ),
                descriptor_file(
                    "parameter-schema.json",
                    None,
                    "参数 Schema",
                    TemplateFileFormat::Json,
                    "json",
                    TemplateFileRole::Reference,
                    false,
                    false,
                    POSTGRES_SCHEMA,
                    "描述模板参数和默认值关系。",
                    "只读参考，部署校验以控制面契约为准。",
                ),
                descriptor_file(
                    "deploy-go.yaml",
                    None,
                    "Deploy Go 清单",
                    TemplateFileFormat::Yaml,
                    "yaml",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    POSTGRES_MANIFEST,
                    "声明模板类型、版本和 Env 文件清单。",
                    "由平台维护，应用配置副本不能覆盖。",
                ),
                descriptor_file(
                    "Makefile",
                    None,
                    "发布入口",
                    TemplateFileFormat::Makefile,
                    "makefile",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    POSTGRES_MAKEFILE,
                    "提供平台托管的发布目标。",
                    "由平台维护，不能修改执行入口。",
                ),
                descriptor_file(
                    "scripts/release.sh",
                    None,
                    "发布脚本",
                    TemplateFileFormat::Shell,
                    "shell",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    POSTGRES_RELEASE_SCRIPT,
                    "执行平台签名链路下的 Compose 发布。",
                    "由平台维护，不能上传或覆盖任意脚本。",
                ),
            ],
        ),
        ImageTemplate::Etcd => (
            "etcd",
            "etcd 3.6（单节点）",
            "Docker Compose 部署单节点 etcd，仅绑定本机回环地址，适用于受控单机部署场景。",
            vec![
                descriptor_file(
                    "README.md",
                    None,
                    "说明",
                    TemplateFileFormat::Markdown,
                    "markdown",
                    TemplateFileRole::Reference,
                    false,
                    false,
                    ETCD_README,
                    "查看单节点拓扑、认证初始化和发布前检查项。",
                    "只读参考，不作为部署输入修改。",
                ),
                descriptor_file(
                    "compose.yaml",
                    Some("compose.yaml"),
                    "Compose 编排",
                    TemplateFileFormat::Yaml,
                    "yaml",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    ETCD_COMPOSE,
                    "定义单节点 etcd 服务、回环端口、健康检查和数据卷。",
                    "可调整受控端口，但不能去掉回环绑定或启用危险容器权限。",
                ),
                descriptor_file(
                    "compose.env.example",
                    Some("compose.env"),
                    "Compose Env 字段",
                    TemplateFileFormat::Dotenv,
                    "dotenv",
                    TemplateFileRole::Configuration,
                    true,
                    false,
                    ETCD_COMPOSE_ENV,
                    "提供 etcd client 端口和时区等非敏感变量示例。",
                    "单节点模板必须保持本机回环绑定；正式值通过应用 Env 版本保存。",
                ),
                descriptor_file(
                    "etcd.env.example",
                    Some("etcd.env"),
                    "服务 Env 字段",
                    TemplateFileFormat::Dotenv,
                    "dotenv",
                    TemplateFileRole::Configuration,
                    true,
                    true,
                    ETCD_SERVICE_ENV,
                    "提供 etcd 成员、数据目录和初始化参数示例。",
                    "修改成员拓扑前必须使用独立方案；认证值通过受保护的 Env 版本保存。",
                ),
                descriptor_file(
                    "parameter-schema.json",
                    None,
                    "参数 Schema",
                    TemplateFileFormat::Json,
                    "json",
                    TemplateFileRole::Reference,
                    false,
                    false,
                    ETCD_SCHEMA,
                    "描述单节点 etcd 模板参数和默认值关系。",
                    "只读参考，部署校验以控制面契约为准。",
                ),
                descriptor_file(
                    "deploy-go.yaml",
                    None,
                    "Deploy Go 清单",
                    TemplateFileFormat::Yaml,
                    "yaml",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    ETCD_MANIFEST,
                    "声明模板类型、版本和 Env 文件清单。",
                    "由平台维护，应用配置副本不能覆盖。",
                ),
                descriptor_file(
                    "Makefile",
                    None,
                    "发布入口",
                    TemplateFileFormat::Makefile,
                    "makefile",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    ETCD_MAKEFILE,
                    "提供平台托管的发布目标。",
                    "由平台维护，不能修改执行入口。",
                ),
                descriptor_file(
                    "scripts/release.sh",
                    None,
                    "发布脚本",
                    TemplateFileFormat::Shell,
                    "shell",
                    TemplateFileRole::PlatformManaged,
                    false,
                    false,
                    ETCD_RELEASE_SCRIPT,
                    "执行平台签名链路下的 Compose 发布。",
                    "由平台维护，不能上传或覆盖任意脚本。",
                ),
            ],
        ),
    };

    let deployment_mechanism = TemplateDeploymentMechanism::Image;
    let default_image = default_image(template).to_owned();
    let default_port = default_port(template);
    let digest = descriptor_digest(
        id,
        template_version(template),
        name,
        summary,
        deployment_mechanism,
        &default_image,
        default_port,
        &files,
    );
    let descriptor = TemplateDescriptor {
        id: id.to_owned(),
        version: template_version(template).to_owned(),
        name: name.to_owned(),
        summary: summary.to_owned(),
        deployment_mechanism,
        default_image,
        default_port,
        digest,
        files,
    };
    validate_template_descriptor(&descriptor).expect("内建应用模板描述必须有效");
    descriptor
}

pub fn all_template_descriptors() -> Vec<TemplateDescriptor> {
    [
        ImageTemplate::Postgres,
        ImageTemplate::Redis,
        ImageTemplate::Valkey,
        ImageTemplate::Etcd,
    ]
    .into_iter()
    .map(template_descriptor)
    .collect()
}

pub fn template_from_id(id: &str) -> Option<ImageTemplate> {
    match id {
        "redis" => Some(ImageTemplate::Redis),
        "valkey" => Some(ImageTemplate::Valkey),
        "postgres" => Some(ImageTemplate::Postgres),
        "etcd" => Some(ImageTemplate::Etcd),
        _ => None,
    }
}

// 静态模板声明保持逐字段可见，完整性由 validate_template_descriptor 统一校验。
#[allow(clippy::too_many_arguments)]
fn descriptor_file(
    path: &str,
    deploy_path: Option<&str>,
    label: &str,
    format: TemplateFileFormat,
    language: &str,
    role: TemplateFileRole,
    editable: bool,
    sensitive: bool,
    content: &str,
    description: &str,
    recommended_changes: &str,
) -> TemplateFileDescriptor {
    let delivery = match (role, format, sensitive) {
        (TemplateFileRole::Configuration, TemplateFileFormat::Dotenv, _) => {
            TemplateFileDelivery::EnvLease
        }
        (TemplateFileRole::Configuration, _, true) => TemplateFileDelivery::SecretFileLease,
        (TemplateFileRole::Configuration, _, false) => TemplateFileDelivery::Artifact,
        (TemplateFileRole::Reference, _, _) => TemplateFileDelivery::Reference,
        (TemplateFileRole::PlatformManaged, _, _) => TemplateFileDelivery::PlatformManaged,
    };
    TemplateFileDescriptor {
        path: path.to_owned(),
        deploy_path: deploy_path.map(str::to_owned),
        label: label.to_owned(),
        format,
        language: language.to_owned(),
        role,
        delivery,
        editable,
        sensitive,
        description: description.to_owned(),
        recommended_changes: recommended_changes.to_owned(),
        digest: sha256_bytes(content.as_bytes()),
        content: content.to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn descriptor_digest(
    id: &str,
    version: &str,
    name: &str,
    summary: &str,
    deployment_mechanism: TemplateDeploymentMechanism,
    default_image: &str,
    default_port: u16,
    files: &[TemplateFileDescriptor],
) -> String {
    let mut hasher = Sha256::new();
    let canonical = serde_json::to_vec(&(
        id,
        version,
        name,
        summary,
        deployment_mechanism,
        default_image,
        default_port,
        files,
    ))
    .expect("模板描述只包含可序列化的稳定字段");
    hasher.update(canonical);
    format!("{:x}", hasher.finalize())
}

pub fn validate_template_descriptor(template: &TemplateDescriptor) -> Result<(), TemplateError> {
    if template.id.is_empty()
        || template.version.is_empty()
        || template.name.is_empty()
        || template.summary.is_empty()
        || template.default_image.is_empty()
        || template.default_port == 0
    {
        return Err(TemplateError::InvalidTemplate(
            "模板身份和说明不能为空".into(),
        ));
    }
    if template.files.is_empty() || template.files.len() > MAX_TEMPLATE_FILES {
        return Err(TemplateError::InvalidTemplate(
            "模板文件数量超出允许范围".into(),
        ));
    }

    let mut paths = BTreeSet::new();
    let mut deploy_paths = BTreeSet::new();
    let mut total_bytes = 0usize;
    for file in &template.files {
        validate_template_relative_path(&file.path)?;
        if !paths.insert(file.path.as_str()) {
            return Err(TemplateError::InvalidTemplate(format!(
                "模板文件路径重复: {}",
                file.path
            )));
        }
        if let Some(path) = &file.deploy_path {
            validate_template_relative_path(path)?;
            if !deploy_paths.insert(path.as_str()) {
                return Err(TemplateError::InvalidTemplate(format!(
                    "部署文件路径重复: {path}"
                )));
            }
        }
        if file.label.is_empty()
            || file.language.is_empty()
            || file.description.is_empty()
            || file.recommended_changes.is_empty()
        {
            return Err(TemplateError::InvalidTemplate(format!(
                "模板文件 metadata 不完整: {}",
                file.path
            )));
        }
        if file.content.len() > MAX_TEMPLATE_FILE_BYTES {
            return Err(TemplateError::InvalidTemplate(format!(
                "模板文件过大: {}",
                file.path
            )));
        }
        total_bytes = total_bytes
            .checked_add(file.content.len())
            .ok_or_else(|| TemplateError::InvalidTemplate("模板文件总大小溢出".into()))?;
        if total_bytes > MAX_TEMPLATE_TOTAL_BYTES {
            return Err(TemplateError::InvalidTemplate(
                "模板文件总大小超出允许范围".into(),
            ));
        }
        if file.digest != sha256_bytes(file.content.as_bytes()) {
            return Err(TemplateError::InvalidTemplate(format!(
                "模板文件 digest 不匹配: {}",
                file.path
            )));
        }

        let expected_delivery = match (file.role, file.format, file.sensitive) {
            (TemplateFileRole::Configuration, TemplateFileFormat::Dotenv, _) => {
                TemplateFileDelivery::EnvLease
            }
            (TemplateFileRole::Configuration, _, true) => TemplateFileDelivery::SecretFileLease,
            (TemplateFileRole::Configuration, _, false) => TemplateFileDelivery::Artifact,
            (TemplateFileRole::Reference, _, _) => TemplateFileDelivery::Reference,
            (TemplateFileRole::PlatformManaged, _, _) => TemplateFileDelivery::PlatformManaged,
        };
        if file.delivery != expected_delivery
            || file.editable != matches!(file.role, TemplateFileRole::Configuration)
            || (file.editable && file.deploy_path.is_none())
            || (!file.editable && file.deploy_path.is_some())
            || (file.sensitive && !matches!(file.role, TemplateFileRole::Configuration))
        {
            return Err(TemplateError::InvalidTemplate(format!(
                "模板文件角色或交付方式不一致: {}",
                file.path
            )));
        }
    }

    if template.digest
        != descriptor_digest(
            &template.id,
            &template.version,
            &template.name,
            &template.summary,
            template.deployment_mechanism,
            &template.default_image,
            template.default_port,
            &template.files,
        )
    {
        return Err(TemplateError::InvalidTemplate(
            "模板 descriptor digest 不匹配".into(),
        ));
    }
    Ok(())
}

fn validate_template_relative_path(path: &str) -> Result<(), TemplateError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(TemplateError::InvalidTemplate(format!(
            "模板文件路径不安全: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn required_env_files(template: ImageTemplate) -> Vec<&'static str> {
    match template {
        ImageTemplate::Redis => vec!["compose.env", "redis.env"],
        ImageTemplate::Valkey => vec!["compose.env", "valkey.env"],
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
        ImageTemplate::Valkey => "valkey",
        ImageTemplate::Postgres => "postgres",
        ImageTemplate::Etcd => "etcd",
    };
    let expected_version = match template {
        ImageTemplate::Redis => "7",
        ImageTemplate::Valkey => "9",
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
    build_platform_artifact_with_overrides(
        spec,
        release_version,
        commit_sha,
        work_dir,
        &HashMap::new(),
    )
}

/// 使用应用配置副本生成平台发布物；overrides 以 deploy_path 为键，
/// 只允许覆盖模板声明的可编辑 artifact 文件，平台托管文件仍由注册表生成。
pub fn build_platform_artifact_with_overrides(
    spec: &ImageDeploySpec,
    release_version: &str,
    commit_sha: &str,
    work_dir: &Path,
    overrides: &HashMap<String, Vec<u8>>,
) -> Result<PlatformArtifact, TemplateError> {
    validate_image_spec(spec)?;
    validate_override_files(spec, overrides)?;
    validate_release_identity(release_version, commit_sha)?;
    fs::create_dir_all(work_dir)?;
    let artifact_dir = work_dir.join("artifact");
    if artifact_dir.exists() {
        fs::remove_dir_all(&artifact_dir)?;
    }
    fs::create_dir_all(&artifact_dir)?;

    let template_size = write_template_archive(spec, &artifact_dir, overrides)?;
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

fn validate_override_files(
    spec: &ImageDeploySpec,
    overrides: &HashMap<String, Vec<u8>>,
) -> Result<(), TemplateError> {
    let allowed: BTreeMap<&str, &str> = template_files(spec.template)
        .into_iter()
        .filter(|(name, _)| *name != "deploy-go.yaml")
        .collect();
    let mut seen = BTreeSet::new();
    for (path, content) in overrides {
        if !allowed.contains_key(path.as_str()) {
            return Err(TemplateError::InvalidSpec(format!(
                "overrides 只能覆盖可编辑模板文件: {}",
                allowed.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        if !seen.insert(path.as_str()) {
            return Err(TemplateError::InvalidSpec(format!(
                "overrides 路径重复: {path}"
            )));
        }
        if content.is_empty()
            || content.len() > MAX_TEMPLATE_FILE_BYTES
            || std::str::from_utf8(content).is_err()
        {
            return Err(TemplateError::InvalidSpec(format!(
                "overrides 文件 {path} 必须是 1B-1MiB 的 UTF-8 文本"
            )));
        }
    }
    Ok(())
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
        ImageTemplate::Valkey => vec![
            ("Makefile", VALKEY_MAKEFILE),
            ("scripts/release.sh", VALKEY_RELEASE_SCRIPT),
            ("deploy-go.yaml", VALKEY_MANIFEST),
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
        ImageTemplate::Valkey => vec![
            ("compose.yaml", VALKEY_COMPOSE),
            ("config/valkey.conf", VALKEY_CONFIG),
            ("deploy-go.yaml", VALKEY_MANIFEST),
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

fn compose_with_spec(
    spec: &ImageDeploySpec,
    overrides: &HashMap<String, Vec<u8>>,
) -> Result<String, TemplateError> {
    let (image_marker, port_marker, container_port) = match spec.template {
        ImageTemplate::Redis => (
            "image: redis:7-alpine",
            "- \"${REDIS_PORT:-6379}:6379\"",
            "6379",
        ),
        ImageTemplate::Valkey => (
            "image: valkey/valkey:9-alpine",
            "- \"${VALKEY_PORT:-6379}:6379\"",
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
    let source = overrides
        .get("compose.yaml")
        .and_then(|value| std::str::from_utf8(value).ok())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            template_files(spec.template)
                .into_iter()
                .find(|(name, _)| *name == "compose.yaml")
                .map(|(_, content)| content.to_owned())
                .expect("compose.yaml 必须存在")
        });
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
        ImageTemplate::Redis | ImageTemplate::Valkey | ImageTemplate::Postgres => {
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
    overrides: &HashMap<String, Vec<u8>>,
) -> Result<u64, TemplateError> {
    let module_dir = artifact_dir.join(template_module(spec.template));
    fs::create_dir_all(&module_dir)?;
    let archive_path = module_dir.join(TEMPLATE_ARCHIVE);
    let file = File::create(&archive_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);
    let compose = compose_with_spec(spec, overrides)?;
    let mut rendered = BTreeMap::new();
    for (name, content) in template_files(spec.template) {
        let content = if name == "compose.yaml" {
            compose.clone()
        } else {
            overrides
                .get(name)
                .and_then(|value| std::str::from_utf8(value).ok())
                .map(str::to_owned)
                .unwrap_or_else(|| content.to_owned())
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

    fn valkey_spec() -> ImageDeploySpec {
        ImageDeploySpec {
            template: ImageTemplate::Valkey,
            image: "docker.io/valkey/valkey:9-alpine".into(),
            host_port: 6379,
            env_files: vec!["compose.env".into(), "valkey.env".into()],
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
        validate_image_spec(&valkey_spec()).unwrap();
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
            (ImageTemplate::Valkey, VALKEY_MANIFEST),
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
    fn template_descriptors_are_complete_and_deterministic() {
        let templates = all_template_descriptors();
        assert_eq!(
            templates
                .iter()
                .map(|template| template.id.as_str())
                .collect::<Vec<_>>(),
            vec!["postgres", "redis", "valkey", "etcd"]
        );
        for template in &templates {
            assert!(!template.digest.is_empty());
            let mut paths = BTreeSet::new();
            for file in &template.files {
                assert!(paths.insert(file.path.as_str()));
                assert!(!file.description.is_empty(), "{}", file.path);
                assert!(!file.recommended_changes.is_empty(), "{}", file.path);
                assert_eq!(file.digest, sha256_bytes(file.content.as_bytes()));
                if file.editable {
                    assert!(matches!(file.role, TemplateFileRole::Configuration));
                    assert!(file.deploy_path.is_some());
                    for heading in ["# 用途:", "# 推荐调整:", "# 默认关系:", "# 安全边界:"]
                    {
                        assert!(
                            file.content.contains(heading),
                            "editable template file {} is missing {heading}",
                            file.path
                        );
                    }
                } else if matches!(file.role, TemplateFileRole::Configuration) {
                    panic!("non-editable configuration file: {}", file.path);
                }
            }
            let env_deploy_paths = template
                .files
                .iter()
                .filter(|file| file.delivery == TemplateFileDelivery::EnvLease)
                .filter_map(|file| file.deploy_path.as_deref())
                .collect::<BTreeSet<_>>();
            let expected_env_paths =
                required_env_files(template_from_id(&template.id).expect("内建模板 ID 必须可解析"))
                    .into_iter()
                    .collect::<BTreeSet<_>>();
            assert_eq!(env_deploy_paths, expected_env_paths);
            assert!(
                template
                    .files
                    .iter()
                    .any(|file| file.path == "compose.yaml")
            );
            assert!(template.files.iter().any(|file| file.sensitive));
            assert_eq!(
                template.digest,
                template_descriptor(template_from_id(&template.id).unwrap()).digest
            );
        }
    }

    #[test]
    fn template_descriptor_validation_rejects_unsafe_paths_and_metadata_drift() {
        let mut unsafe_path = template_descriptor(ImageTemplate::Postgres);
        unsafe_path.files[0].path = "../README.md".into();
        assert!(validate_template_descriptor(&unsafe_path).is_err());

        let mut absolute_deploy_path = template_descriptor(ImageTemplate::Redis);
        let editable = absolute_deploy_path
            .files
            .iter_mut()
            .find(|file| file.editable)
            .unwrap();
        editable.deploy_path = Some("/etc/redis.conf".into());
        assert!(validate_template_descriptor(&absolute_deploy_path).is_err());

        let mut metadata_drift = template_descriptor(ImageTemplate::Etcd);
        metadata_drift.files[0].description.push_str(" changed");
        assert!(validate_template_descriptor(&metadata_drift).is_err());

        let mut summary_drift = template_descriptor(ImageTemplate::Postgres);
        summary_drift.summary.push_str(" changed");
        assert!(validate_template_descriptor(&summary_drift).is_err());
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
    fn rendered_valkey_compose_uses_fixed_image_and_host_port() {
        let directory = tempfile::tempdir().unwrap();
        build_platform_artifact(
            &valkey_spec(),
            "202608200001",
            "0123456789abcdef0123456789abcdef01234567",
            directory.path(),
        )
        .unwrap();
        let inner = GzDecoder::new(
            File::open(directory.path().join("artifact/valkey/template.tar.gz")).unwrap(),
        );
        let mut archive = tar::Archive::new(inner);
        let mut compose = String::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap().to_str().unwrap() == "compose.yaml" {
                entry.read_to_string(&mut compose).unwrap();
            }
        }
        assert!(compose.contains("image: docker.io/valkey/valkey:9-alpine"));
        assert!(compose.contains("- \"6379:6379\""));
        assert!(!compose.contains("${VALKEY_PORT"));
    }

    #[test]
    fn artifact_uses_application_config_overrides_for_editable_files() {
        let directory = tempfile::tempdir().unwrap();
        let mut overrides = HashMap::new();
        overrides.insert(
            "config/redis.conf".to_owned(),
            b"# app-config-override-test\nmaxmemory 64mb\n".to_vec(),
        );
        overrides.insert(
            "compose.yaml".to_owned(),
            b"# app-compose-override-test\nservices:\n  redis:\n    image: redis:7-alpine\n    restart: unless-stopped\n    ports:\n      - \"${REDIS_PORT:-6379}:6379\"\n".to_vec(),
        );
        build_platform_artifact_with_overrides(
            &redis_spec(),
            "202608200001",
            "0123456789abcdef0123456789abcdef01234567",
            directory.path(),
            &overrides,
        )
        .unwrap();

        let inner = GzDecoder::new(
            File::open(directory.path().join("artifact/redis/template.tar.gz")).unwrap(),
        );
        let mut archive = tar::Archive::new(inner);
        let mut files = BTreeMap::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let name = entry.path().unwrap().to_str().unwrap().to_owned();
            let mut content = String::new();
            entry.read_to_string(&mut content).unwrap();
            files.insert(name, content);
        }
        assert!(
            files["config/redis.conf"].contains("app-config-override-test"),
            "artifact 必须使用应用配置副本内容"
        );
        assert!(
            files["compose.yaml"].contains("app-compose-override-test"),
            "artifact compose 必须基于应用配置副本渲染"
        );
        assert!(files["compose.yaml"].contains("image: docker.io/library/redis:7-alpine"));
        assert!(files["compose.yaml"].contains("- \"6379:6379\""));
    }

    #[test]
    fn override_validation_rejects_unknown_or_platform_managed_files() {
        let mut overrides = HashMap::new();
        overrides.insert("deploy-go.yaml".to_owned(), b"schema_version: 1".to_vec());
        assert!(
            validate_override_files(&redis_spec(), &overrides).is_err(),
            "deploy-go.yaml 不能由应用配置覆盖"
        );

        let mut overrides = HashMap::new();
        overrides.insert("../redis.env".to_owned(), b"REDIS_PASSWORD=x".to_vec());
        assert!(
            validate_override_files(&redis_spec(), &overrides).is_err(),
            "未登记路径不能覆盖"
        );
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
