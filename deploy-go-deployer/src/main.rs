use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use reqwest::Method;
use serde_json::{Value, json};

const EMBEDDED_EXTERNAL_OPENAPI: &str = include_str!("../../api/openapi/external.json");

#[derive(Parser)]
#[command(
    name = "deploy-go-deployer",
    version,
    about = "Deploy Go 对外部署 API 的 Agent/CLI 封装",
    long_about = "通过外部 API Key 创建与列出应用、查看目标、编辑非正式环境应用、发起部署、\n\
        查询状态与取消部署。该工具只能调用对外部署 API，不读取 Env，也不执行任意命令。"
)]
struct Cli {
    /// 主控 API 基础地址
    #[arg(
        long,
        env = "DEPLOY_GO_API_BASE_URL",
        default_value = "https://deploy.quanxinfu.com"
    )]
    api_base: String,

    /// 外部部署 API Key（dgx_...）
    #[arg(long, env = "DEPLOY_GO_API_KEY", hide_env_values = true)]
    api_key: Option<String>,

    /// 输出原始 JSON（默认输出易读文本）
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 列出当前 Key 可部署的应用
    ListApps,
    /// 创建非正式环境应用（正式环境只能由管理面手动创建）
    CreateApp(CreateAppArgs),
    /// 查看应用详情与可用部署目标
    ShowApp { application_id: String },
    /// 编辑非正式环境应用
    UpdateApp(UpdateAppArgs),
    /// 列出应用的 Env 文件元数据（不返回明文）
    ListEnvFiles { application_id: String },
    /// 登记 Env 文件（内容从本地文件读取，避免密钥进入命令行）
    RegisterEnvFile(RegisterEnvFileArgs),
    /// 更新 Env 文件内容
    UpdateEnvFile(UpdateEnvFileArgs),
    /// 删除 Env 文件
    DeleteEnvFile(DeleteEnvFileArgs),
    /// 列出应用的部署目标契约
    ListTargets { application_id: String },
    /// 新增部署目标
    CreateTarget(CreateTargetArgs),
    /// 编辑部署目标
    UpdateTarget(UpdateTargetArgs),
    /// 启用或停用部署目标
    SetTargetStatus(SetTargetStatusArgs),
    /// 查看应用固定工作区来源
    ShowWorkspaceSource { application_id: String },
    /// 保存应用固定工作区来源
    SetWorkspaceSource(SetWorkspaceSourceArgs),
    /// 发起部署
    Deploy(DeployArgs),
    /// 查询部署状态
    Status { deployment_id: String },
    /// 取消部署
    Cancel { deployment_id: String },
    /// 输出对外 OpenAPI 契约（默认打印，可 --output 写文件）
    Openapi {
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Args)]
struct CreateAppArgs {
    /// 应用名称
    #[arg(long)]
    name: String,
    /// 应用 slug：3-64 位小写字母、数字或短横线
    #[arg(long)]
    slug: String,
    /// 仅支持非正式环境：dev、test 或 staging
    #[arg(long, value_parser = ["dev", "test", "staging"])]
    environment: String,
    #[arg(long, default_value = "")]
    description: String,
    /// 应用类型，默认 binary
    #[arg(long)]
    app_type: Option<String>,
    /// 应用类型版本，默认 1
    #[arg(long)]
    type_version: Option<String>,
    /// 应用标签，可重复传入
    #[arg(long = "tag")]
    tags: Vec<String>,
    /// 参数 JSON Schema（内联 JSON）
    #[arg(long, conflicts_with = "parameter_schema_file")]
    parameter_schema: Option<String>,
    /// 参数 JSON Schema 文件
    #[arg(long, value_name = "PATH", conflicts_with = "parameter_schema")]
    parameter_schema_file: Option<PathBuf>,
    /// 部署后验证配置（内联 JSON）
    #[arg(long, conflicts_with = "verification_config_file")]
    verification_config: Option<String>,
    /// 部署后验证配置文件
    #[arg(long, value_name = "PATH", conflicts_with = "verification_config")]
    verification_config_file: Option<PathBuf>,
}

#[derive(Args)]
struct UpdateAppArgs {
    application_id: String,
    /// 当前应用版本；省略时先读取应用详情自动获取
    #[arg(long)]
    version: Option<i64>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    slug: Option<String>,
    #[arg(long)]
    description: Option<String>,
    /// 非正式环境：dev、test 或 staging
    #[arg(long, value_parser = ["dev", "test", "staging"])]
    environment: Option<String>,
    #[arg(long)]
    app_type: Option<String>,
    #[arg(long)]
    type_version: Option<String>,
    /// 应用标签，可重复传入
    #[arg(long = "tag", conflicts_with = "clear_tags")]
    tags: Vec<String>,
    /// 清空全部标签
    #[arg(long)]
    clear_tags: bool,
    /// 参数 JSON Schema（内联 JSON）
    #[arg(long, conflicts_with = "parameter_schema_file")]
    parameter_schema: Option<String>,
    /// 参数 JSON Schema 文件
    #[arg(long, value_name = "PATH", conflicts_with = "parameter_schema")]
    parameter_schema_file: Option<PathBuf>,
    /// 部署后验证配置（内联 JSON）
    #[arg(long, conflicts_with = "verification_config_file")]
    verification_config: Option<String>,
    /// 部署后验证配置文件
    #[arg(long, value_name = "PATH", conflicts_with = "verification_config")]
    verification_config_file: Option<PathBuf>,
}

#[derive(Args)]
struct RegisterEnvFileArgs {
    application_id: String,
    /// Env 文件名，必须以 .env 结尾
    #[arg(long)]
    file_name: String,
    /// Env 归属模块
    #[arg(long)]
    module: String,
    /// Env 内容文件（dotenv-v1）
    #[arg(long, value_name = "PATH")]
    content_file: PathBuf,
}

#[derive(Args)]
struct UpdateEnvFileArgs {
    application_id: String,
    env_file_id: String,
    /// Env 内容文件（dotenv-v1）
    #[arg(long, value_name = "PATH")]
    content_file: PathBuf,
    /// 当前 Env 文件版本；省略时从 Env 列表自动获取
    #[arg(long)]
    version: Option<i64>,
}

#[derive(Args)]
struct DeleteEnvFileArgs {
    application_id: String,
    env_file_id: String,
    /// 确认删除的文件名，必须与登记名一致
    #[arg(long)]
    confirm_file_name: String,
    /// 当前 Env 文件版本；省略时从 Env 列表自动获取
    #[arg(long)]
    version: Option<i64>,
}

#[derive(Args)]
struct CreateTargetArgs {
    application_id: String,
    /// 目标节点 ID
    #[arg(long)]
    node_id: String,
    /// 发布脚本路径，必须位于节点工作根目录内
    #[arg(long)]
    script_path: String,
    /// 发布超时秒数（1-86400）
    #[arg(long)]
    timeout_seconds: i64,
    /// 目标稳定标识；省略时使用应用环境
    #[arg(long)]
    target_code: Option<String>,
    /// 执行模式，默认 script
    #[arg(long, default_value = "script")]
    execution_mode: String,
    /// 敏感文件引用，格式 ENVIRONMENT_KEY=FILE_PATH，可重复传入
    #[arg(long = "secret-file", value_parser = parse_secret_reference)]
    secret_file_references: Vec<(String, String)>,
    /// 镜像规格 JSON（仅 image 模式）
    #[arg(long, conflicts_with = "image_spec_file")]
    image_spec: Option<String>,
    /// 镜像规格 JSON 文件（仅 image 模式）
    #[arg(long, value_name = "PATH", conflicts_with = "image_spec")]
    image_spec_file: Option<PathBuf>,
}

#[derive(Args)]
struct UpdateTargetArgs {
    target_id: String,
    /// 目标节点 ID
    #[arg(long)]
    node_id: String,
    /// 发布脚本路径，必须位于节点工作根目录内
    #[arg(long)]
    script_path: String,
    /// 发布超时秒数（1-86400）
    #[arg(long)]
    timeout_seconds: i64,
    /// 当前目标版本；可用 list-targets 查看
    #[arg(long)]
    version: i64,
    #[arg(long)]
    target_code: Option<String>,
    #[arg(long)]
    execution_mode: Option<String>,
    #[arg(long = "secret-file", value_parser = parse_secret_reference)]
    secret_file_references: Vec<(String, String)>,
    #[arg(long, conflicts_with = "image_spec_file")]
    image_spec: Option<String>,
    #[arg(long, value_name = "PATH", conflicts_with = "image_spec")]
    image_spec_file: Option<PathBuf>,
}

#[derive(Args)]
struct SetTargetStatusArgs {
    target_id: String,
    /// active 或 disabled
    #[arg(long)]
    status: String,
    /// 当前目标版本；可用 list-targets 查看
    #[arg(long)]
    version: i64,
}

#[derive(Args)]
struct SetWorkspaceSourceArgs {
    application_id: String,
    /// 构建 Agent ID
    #[arg(long)]
    build_agent_id: String,
    /// 构建节点上的固定绝对路径
    #[arg(long)]
    workspace_path: String,
    /// 当前工作区来源版本；省略时从现状自动获取
    #[arg(long)]
    version: Option<i64>,
}

#[derive(Args)]
struct DeployArgs {
    application_id: String,
    /// 指定单个部署目标；省略时部署应用全部启用目标
    #[arg(long)]
    target_id: Option<String>,
    /// 发布版本（两阶段部署需要）
    #[arg(long)]
    release_version: Option<String>,
    #[arg(long, default_value = "automatic")]
    release_strategy: String,
    /// 部署参数，格式 KEY=VALUE，可重复传入
    #[arg(long = "parameter", value_parser = parse_parameter)]
    parameters: Vec<(String, String)>,
    /// 幂等键（16-128 个可见字符）；省略时自动生成
    #[arg(long)]
    idempotency_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Openapi { output } => {
            let document: Value = serde_json::from_str(EMBEDDED_EXTERNAL_OPENAPI)
                .context("内置对外 OpenAPI 契约解析失败")?;
            match output {
                Some(path) => {
                    let content = serde_json::to_string_pretty(&document)?;
                    std::fs::write(&path, content + "\n")
                        .with_context(|| format!("写入 OpenAPI 产物失败：{}", path.display()))?;
                    println!("已写入 {}", path.display());
                }
                None => println!("{}", serde_json::to_string_pretty(&document)?),
            }
            return Ok(());
        }
        command => {
            let api_key = cli
                .api_key
                .as_deref()
                .filter(|value| value.starts_with("dgx_"))
                .ok_or_else(|| {
                    anyhow::anyhow!("缺少有效的 DEPLOY_GO_API_KEY（外部 API Key，格式 dgx_...）")
                })?;
            let client = ApiClient::new(&cli.api_base, api_key);
            let output = match command {
                Command::ListApps => client.list_apps().await?,
                Command::CreateApp(args) => client.create_app(args).await?,
                Command::ShowApp { application_id } => client.show_app(&application_id).await?,
                Command::UpdateApp(args) => client.update_app(args).await?,
                Command::ListEnvFiles { application_id } => {
                    client.list_env_files(&application_id).await?
                }
                Command::RegisterEnvFile(args) => client.register_env_file(args).await?,
                Command::UpdateEnvFile(args) => client.update_env_file(args).await?,
                Command::DeleteEnvFile(args) => client.delete_env_file(args).await?,
                Command::ListTargets { application_id } => {
                    client.list_targets(&application_id).await?
                }
                Command::CreateTarget(args) => client.create_target(args).await?,
                Command::UpdateTarget(args) => client.update_target(args).await?,
                Command::SetTargetStatus(args) => client.set_target_status(args).await?,
                Command::ShowWorkspaceSource { application_id } => {
                    client.show_workspace_source(&application_id).await?
                }
                Command::SetWorkspaceSource(args) => client.set_workspace_source(args).await?,
                Command::Deploy(args) => client.deploy(args).await?,
                Command::Status { deployment_id } => client.status(&deployment_id).await?,
                Command::Cancel { deployment_id } => client.cancel(&deployment_id).await?,
                Command::Openapi { .. } => unreachable!(),
            };
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                print_human(&output);
            }
        }
    }
    Ok(())
}

struct ApiClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl ApiClient {
    fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key: api_key.to_owned(),
            http: reqwest::Client::new(),
        }
    }

    async fn list_apps(&self) -> Result<Value> {
        self.request(Method::GET, "/external/v1/applications", None, None)
            .await
    }

    async fn show_app(&self, application_id: &str) -> Result<Value> {
        self.request(
            Method::GET,
            &format!("/external/v1/applications/{application_id}"),
            None,
            None,
        )
        .await
    }

    async fn create_app(&self, args: CreateAppArgs) -> Result<Value> {
        let mut body = serde_json::Map::new();
        body.insert("name".to_owned(), json!(args.name));
        body.insert("slug".to_owned(), json!(args.slug));
        body.insert("environment".to_owned(), json!(args.environment));
        if !args.description.is_empty() {
            body.insert("description".to_owned(), json!(args.description));
        }
        if let Some(value) = args.app_type {
            body.insert("app_type".to_owned(), json!(value));
        }
        if let Some(value) = args.type_version {
            body.insert("type_version".to_owned(), json!(value));
        }
        if !args.tags.is_empty() {
            body.insert("tags".to_owned(), json!(args.tags));
        }
        if let Some(value) = parse_json_arg(
            args.parameter_schema.as_deref(),
            args.parameter_schema_file.as_deref(),
            "参数 JSON Schema",
        )? {
            body.insert("parameter_schema".to_owned(), value);
        }
        if let Some(value) = parse_json_arg(
            args.verification_config.as_deref(),
            args.verification_config_file.as_deref(),
            "部署后验证配置",
        )? {
            body.insert("verification_config".to_owned(), value);
        }
        self.request(
            Method::POST,
            "/external/v1/applications",
            Some(Value::Object(body)),
            None,
        )
        .await
    }

    async fn update_app(&self, args: UpdateAppArgs) -> Result<Value> {
        let mut body = serde_json::Map::new();
        let version = match args.version {
            Some(version) => version,
            None => self
                .show_app(&args.application_id)
                .await?
                .get("version")
                .and_then(Value::as_i64)
                .context("应用详情缺少 version")?,
        };
        body.insert("version".to_owned(), json!(version));
        if let Some(value) = args.name {
            body.insert("name".to_owned(), json!(value));
        }
        if let Some(value) = args.slug {
            body.insert("slug".to_owned(), json!(value));
        }
        if let Some(value) = args.description {
            body.insert("description".to_owned(), json!(value));
        }
        if let Some(value) = args.environment {
            body.insert("environment".to_owned(), json!(value));
        }
        if let Some(value) = args.app_type {
            body.insert("app_type".to_owned(), json!(value));
        }
        if let Some(value) = args.type_version {
            body.insert("type_version".to_owned(), json!(value));
        }
        if !args.tags.is_empty() {
            body.insert("tags".to_owned(), json!(args.tags));
        } else if args.clear_tags {
            body.insert("tags".to_owned(), json!([]));
        }
        if let Some(value) = parse_json_arg(
            args.parameter_schema.as_deref(),
            args.parameter_schema_file.as_deref(),
            "参数 JSON Schema",
        )? {
            body.insert("parameter_schema".to_owned(), value);
        }
        if let Some(value) = parse_json_arg(
            args.verification_config.as_deref(),
            args.verification_config_file.as_deref(),
            "部署后验证配置",
        )? {
            body.insert("verification_config".to_owned(), value);
        }
        if body.len() == 1 {
            bail!("至少提供一个要修改的字段");
        }
        self.request(
            Method::PATCH,
            &format!("/external/v1/applications/{}", args.application_id),
            Some(Value::Object(body)),
            None,
        )
        .await
    }

    async fn list_env_files(&self, application_id: &str) -> Result<Value> {
        self.request(
            Method::GET,
            &format!("/external/v1/applications/{application_id}/env-files"),
            None,
            None,
        )
        .await
    }

    async fn register_env_file(&self, args: RegisterEnvFileArgs) -> Result<Value> {
        let content = read_text_file(&args.content_file, "Env 内容")?;
        let body = json!({
            "files": [{
                "file_name": args.file_name,
                "module": args.module,
                "format": "dotenv-v1",
                "content": content,
            }]
        });
        self.request(
            Method::POST,
            &format!(
                "/external/v1/applications/{}/env-files",
                args.application_id
            ),
            Some(body),
            None,
        )
        .await
    }

    async fn update_env_file(&self, args: UpdateEnvFileArgs) -> Result<Value> {
        let content = read_text_file(&args.content_file, "Env 内容")?;
        let version = match args.version {
            Some(version) => version,
            None => env_file_version(
                self.list_env_files(&args.application_id).await?,
                &args.env_file_id,
            )?,
        };
        self.request(
            Method::PUT,
            &format!(
                "/external/v1/applications/{}/env-files/{}",
                args.application_id, args.env_file_id
            ),
            Some(json!({"content": content, "expected_version": version})),
            None,
        )
        .await
    }

    async fn delete_env_file(&self, args: DeleteEnvFileArgs) -> Result<Value> {
        let version = match args.version {
            Some(version) => version,
            None => env_file_version(
                self.list_env_files(&args.application_id).await?,
                &args.env_file_id,
            )?,
        };
        self.request(
            Method::DELETE,
            &format!(
                "/external/v1/applications/{}/env-files/{}",
                args.application_id, args.env_file_id
            ),
            Some(json!({"expected_version": version, "confirm_file_name": args.confirm_file_name})),
            None,
        )
        .await
    }

    async fn list_targets(&self, application_id: &str) -> Result<Value> {
        self.request(
            Method::GET,
            &format!("/external/v1/applications/{application_id}/targets"),
            None,
            None,
        )
        .await
    }

    async fn create_target(&self, args: CreateTargetArgs) -> Result<Value> {
        let mut body = serde_json::Map::new();
        body.insert("node_id".to_owned(), json!(args.node_id));
        body.insert("script_path".to_owned(), json!(args.script_path));
        body.insert("timeout_seconds".to_owned(), json!(args.timeout_seconds));
        body.insert("execution_mode".to_owned(), json!(args.execution_mode));
        if let Some(value) = args.target_code {
            body.insert("target_code".to_owned(), json!(value));
        }
        body.insert(
            "secret_file_references".to_owned(),
            secret_references(&args.secret_file_references),
        );
        if let Some(value) = parse_json_arg(
            args.image_spec.as_deref(),
            args.image_spec_file.as_deref(),
            "镜像规格",
        )? {
            body.insert("image_spec".to_owned(), value);
        }
        self.request(
            Method::POST,
            &format!("/external/v1/applications/{}/targets", args.application_id),
            Some(Value::Object(body)),
            None,
        )
        .await
    }

    async fn update_target(&self, args: UpdateTargetArgs) -> Result<Value> {
        let mut body = serde_json::Map::new();
        body.insert("version".to_owned(), json!(args.version));
        body.insert("node_id".to_owned(), json!(args.node_id));
        body.insert("script_path".to_owned(), json!(args.script_path));
        body.insert("timeout_seconds".to_owned(), json!(args.timeout_seconds));
        if let Some(value) = args.target_code {
            body.insert("target_code".to_owned(), json!(value));
        }
        if let Some(value) = args.execution_mode {
            body.insert("execution_mode".to_owned(), json!(value));
        }
        body.insert(
            "secret_file_references".to_owned(),
            secret_references(&args.secret_file_references),
        );
        if let Some(value) = parse_json_arg(
            args.image_spec.as_deref(),
            args.image_spec_file.as_deref(),
            "镜像规格",
        )? {
            body.insert("image_spec".to_owned(), value);
        }
        self.request(
            Method::PATCH,
            &format!("/external/v1/deployment-targets/{}", args.target_id),
            Some(Value::Object(body)),
            None,
        )
        .await
    }

    async fn set_target_status(&self, args: SetTargetStatusArgs) -> Result<Value> {
        self.request(
            Method::PUT,
            &format!("/external/v1/deployment-targets/{}/status", args.target_id),
            Some(json!({"status": args.status, "version": args.version})),
            None,
        )
        .await
    }

    async fn show_workspace_source(&self, application_id: &str) -> Result<Value> {
        self.request(
            Method::GET,
            &format!("/external/v1/applications/{application_id}/workspace-source"),
            None,
            None,
        )
        .await
    }

    async fn set_workspace_source(&self, args: SetWorkspaceSourceArgs) -> Result<Value> {
        let mut body = serde_json::Map::new();
        body.insert("build_agent_id".to_owned(), json!(args.build_agent_id));
        body.insert("workspace_path".to_owned(), json!(args.workspace_path));
        match args.version {
            Some(version) => {
                body.insert("version".to_owned(), json!(version));
            }
            None => {
                if let Ok(current) = self.show_workspace_source(&args.application_id).await
                    && let Some(version) = current.get("version").and_then(Value::as_i64)
                {
                    body.insert("version".to_owned(), json!(version));
                }
            }
        }
        self.request(
            Method::PUT,
            &format!(
                "/external/v1/applications/{}/workspace-source",
                args.application_id
            ),
            Some(Value::Object(body)),
            None,
        )
        .await
    }

    async fn deploy(&self, args: DeployArgs) -> Result<Value> {
        let parameters = args
            .parameters
            .into_iter()
            .map(|(key, value)| (key, Value::String(value)))
            .collect::<serde_json::Map<_, _>>();
        let body = json!({
            "target_id": args.target_id,
            "parameters": Value::Object(parameters),
            "release_strategy": args.release_strategy,
            "release_version": args.release_version,
        });
        let idempotency_key = args
            .idempotency_key
            .unwrap_or_else(|| format!("dgx-{}", ulid::Ulid::new()));
        self.request(
            Method::POST,
            &format!(
                "/external/v1/applications/{}/deployments",
                args.application_id
            ),
            Some(body),
            Some(&idempotency_key),
        )
        .await
    }

    async fn status(&self, deployment_id: &str) -> Result<Value> {
        self.request(
            Method::GET,
            &format!("/external/v1/deployments/{deployment_id}"),
            None,
            None,
        )
        .await
    }

    async fn cancel(&self, deployment_id: &str) -> Result<Value> {
        self.request(
            Method::POST,
            &format!("/external/v1/deployments/{deployment_id}/cancel"),
            Some(Value::Null),
            None,
        )
        .await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        idempotency_key: Option<&str>,
    ) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.http.request(method, &url).bearer_auth(&self.api_key);
        if let Some(body) = body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body.to_string());
        }
        if let Some(idempotency_key) = idempotency_key {
            request = request.header("Idempotency-Key", idempotency_key);
        }
        let response = request
            .send()
            .await
            .with_context(|| format!("请求失败：{url}"))?;
        let status = response.status();
        let text = response.text().await.context("读取响应失败")?;
        let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
        if !status.is_success() {
            let code = parsed
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("http_error");
            let message = parsed
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("请求失败");
            let request_id = parsed
                .get("request_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            bail!("请求失败 status={status} code={code} message={message} request_id={request_id}");
        }
        serde_json::from_value(parsed).context("解析响应 JSON 失败")
    }
}

fn print_human(value: &Value) {
    if let Some(items) = value.get("items").and_then(Value::as_array) {
        if items
            .first()
            .is_some_and(|item| item.get("file_name").is_some())
        {
            println!(
                "{}",
                format_row(&[
                    "Env 文件 ID",
                    "文件名",
                    "模块",
                    "当前版本",
                    "文件版本",
                    "待同步",
                    "同步失败"
                ])
            );
            for item in items {
                println!(
                    "{}",
                    format_row(&[
                        item["id"].as_str().unwrap_or(""),
                        item["file_name"].as_str().unwrap_or(""),
                        item["module"].as_str().unwrap_or(""),
                        &item["current_version"]
                            .as_i64()
                            .unwrap_or_default()
                            .to_string(),
                        &item["version"].as_i64().unwrap_or_default().to_string(),
                        &item["pending_count"]
                            .as_i64()
                            .unwrap_or_default()
                            .to_string(),
                        &item["failed_count"]
                            .as_i64()
                            .unwrap_or_default()
                            .to_string(),
                    ])
                );
            }
            return;
        }
        if items
            .first()
            .is_some_and(|item| item.get("node_id").is_some())
        {
            println!(
                "{}",
                format_row(&[
                    "目标 ID",
                    "标识",
                    "节点 ID",
                    "环境",
                    "模式",
                    "脚本路径",
                    "超时(秒)",
                    "状态",
                    "版本"
                ])
            );
            for item in items {
                println!(
                    "{}",
                    format_row(&[
                        item["id"].as_str().unwrap_or(""),
                        item["target_code"].as_str().unwrap_or(""),
                        item["node_id"].as_str().unwrap_or(""),
                        item["environment"].as_str().unwrap_or(""),
                        item["execution_mode"].as_str().unwrap_or(""),
                        item["script_path"].as_str().unwrap_or(""),
                        &item["timeout_seconds"]
                            .as_i64()
                            .unwrap_or_default()
                            .to_string(),
                        item["status"].as_str().unwrap_or(""),
                        &item["version"].as_i64().unwrap_or_default().to_string(),
                    ])
                );
            }
            return;
        }
        println!("{}", format_row(&["ID", "名称", "Slug", "状态"]));
        for item in items {
            println!(
                "{}",
                format_row(&[
                    item["id"].as_str().unwrap_or(""),
                    item["name"].as_str().unwrap_or(""),
                    item["slug"].as_str().unwrap_or(""),
                    item["status"].as_str().unwrap_or(""),
                ])
            );
        }
        return;
    }
    if let Some(targets) = value.get("targets").and_then(Value::as_array) {
        println!(
            "应用：{} ({})",
            value["name"].as_str().unwrap_or(""),
            value["id"].as_str().unwrap_or("")
        );
        println!("Slug：{}", value["slug"].as_str().unwrap_or(""));
        println!(
            "环境：{}  类型：{} v{}  版本：{}",
            value["environment"].as_str().unwrap_or(""),
            value["app_type"].as_str().unwrap_or(""),
            value["type_version"].as_str().unwrap_or(""),
            value["version"].as_i64().unwrap_or_default()
        );
        let tags = value["tags"]
            .as_array()
            .map(|tags| {
                tags.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        println!("标签：{}", if tags.is_empty() { "-" } else { &tags });
        println!(
            "{}",
            format_row(&["目标 ID", "环境", "节点", "模式", "状态"])
        );
        for target in targets {
            println!(
                "{}",
                format_row(&[
                    target["id"].as_str().unwrap_or(""),
                    target["environment"].as_str().unwrap_or(""),
                    target["node_name"].as_str().unwrap_or(""),
                    target["execution_mode"].as_str().unwrap_or(""),
                    target["status"].as_str().unwrap_or(""),
                ])
            );
        }
        return;
    }
    if let Some(path) = value.get("workspace_path").and_then(Value::as_str) {
        println!("工作区来源：{}", value["id"].as_str().unwrap_or(""));
        println!(
            "构建 Agent：{} ({})",
            value["build_agent_id"].as_str().unwrap_or(""),
            value["build_agent_name"].as_str().unwrap_or("-")
        );
        println!("工作区路径：{path}");
        println!(
            "工作区版本：{}  状态：{}  记录版本：{}",
            value["workspace_version"].as_i64().unwrap_or_default(),
            value["status"].as_str().unwrap_or(""),
            value["version"].as_i64().unwrap_or_default()
        );
        return;
    }
    if let Some(file_name) = value.get("file_name").and_then(Value::as_str) {
        println!(
            "Env 文件：{} ({})",
            file_name,
            value["id"].as_str().unwrap_or("")
        );
        println!(
            "模块：{}  格式：{}  摘要：{}",
            value["module"].as_str().unwrap_or(""),
            value["format"].as_str().unwrap_or(""),
            value["current_digest"].as_str().unwrap_or("")
        );
        println!(
            "内容版本：{}  文件版本：{}",
            value["current_version"].as_i64().unwrap_or_default(),
            value["version"].as_i64().unwrap_or_default()
        );
        println!(
            "同步：待处理 {} / 同步中 {} / 成功 {} / 失败 {}",
            value["pending_count"].as_i64().unwrap_or_default(),
            value["syncing_count"].as_i64().unwrap_or_default(),
            value["succeeded_count"].as_i64().unwrap_or_default(),
            value["failed_count"].as_i64().unwrap_or_default()
        );
        return;
    }
    if let Some(runs) = value.get("target_runs").and_then(Value::as_array) {
        println!(
            "部署：{} 状态={} phase={}",
            value["id"].as_str().unwrap_or(""),
            value["status"].as_str().unwrap_or(""),
            value["phase"].as_str().unwrap_or("")
        );
        println!(
            "{}",
            format_row(&["运行 ID", "目标", "节点", "状态", "阶段"])
        );
        for run in runs {
            println!(
                "{}",
                format_row(&[
                    run["id"].as_str().unwrap_or(""),
                    run["target_id"].as_str().unwrap_or(""),
                    run["node_name"].as_str().unwrap_or(""),
                    run["status"].as_str().unwrap_or(""),
                    run["phase"].as_str().unwrap_or(""),
                ])
            );
        }
        return;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

fn format_row(fields: &[&str]) -> String {
    fields.join("\t")
}

fn parse_parameter(value: &str) -> Result<(String, String)> {
    value
        .split_once('=')
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .ok_or_else(|| anyhow::anyhow!("参数格式必须是 KEY=VALUE：{value}"))
}

fn parse_secret_reference(value: &str) -> Result<(String, String)> {
    let (key, path) = value.split_once('=').ok_or_else(|| {
        anyhow::anyhow!("敏感文件引用格式必须是 ENVIRONMENT_KEY=FILE_PATH：{value}")
    })?;
    if key.is_empty() || path.is_empty() {
        bail!("敏感文件引用的环境变量名与路径都不能为空：{value}");
    }
    Ok((key.to_owned(), path.to_owned()))
}

fn secret_references(references: &[(String, String)]) -> Value {
    Value::Array(
        references
            .iter()
            .map(|(environment_key, file_path)| {
                json!({"environment_key": environment_key, "file_path": file_path})
            })
            .collect(),
    )
}

fn read_text_file(path: &std::path::Path, label: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("读取{label}文件失败：{}", path.display()))
}

fn env_file_version(list: Value, env_file_id: &str) -> Result<i64> {
    list.get("items")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("id").and_then(Value::as_str) == Some(env_file_id))
        })
        .and_then(|item| item.get("version").and_then(Value::as_i64))
        .ok_or_else(|| anyhow::anyhow!("未找到 Env 文件 {env_file_id} 的当前版本"))
}

fn parse_json_arg(
    inline: Option<&str>,
    file: Option<&std::path::Path>,
    label: &str,
) -> Result<Option<Value>> {
    let raw = match (inline, file) {
        (Some(inline), None) => inline.to_owned(),
        (None, Some(path)) => std::fs::read_to_string(path)
            .with_context(|| format!("读取{label}文件失败：{}", path.display()))?,
        (None, None) => return Ok(None),
        (Some(_), Some(_)) => unreachable!("clap conflicts_with 已保证互斥"),
    };
    let value: Value =
        serde_json::from_str(&raw).with_context(|| format!("{label}不是有效 JSON"))?;
    if !value.is_object() {
        bail!("{label}必须是 JSON object");
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::{
        EMBEDDED_EXTERNAL_OPENAPI, env_file_version, parse_json_arg, parse_parameter,
        parse_secret_reference, secret_references,
    };

    #[test]
    fn embedded_openapi_has_external_deployment_paths() {
        let document: serde_json::Value = serde_json::from_str(EMBEDDED_EXTERNAL_OPENAPI).unwrap();
        assert!(
            document["paths"]
                .get("/external/v1/applications/{id}/deployments")
                .is_some()
        );
        assert!(document["paths"]["/external/v1/applications/{id}"]["patch"].is_object());
        assert!(document["components"]["schemas"]["ExternalApplicationUpdateRequest"].is_object());
        assert!(document["paths"]["/external/v1/applications"]["post"].is_object());
        assert!(document["components"]["schemas"]["ExternalApplicationCreateRequest"].is_object());
        for path in [
            "/external/v1/applications/{id}/env-files",
            "/external/v1/applications/{id}/targets",
            "/external/v1/applications/{id}/workspace-source",
            "/external/v1/deployment-targets/{target_id}",
        ] {
            assert!(document["paths"].get(path).is_some(), "缺少路径 {path}");
        }
    }

    #[test]
    fn env_file_version_reads_the_matching_entry() {
        let list = serde_json::json!({
            "items": [
                {"id": "envf_a", "version": 3},
                {"id": "envf_b", "version": 7},
            ]
        });
        assert_eq!(env_file_version(list.clone(), "envf_b").unwrap(), 7);
        assert!(env_file_version(list, "envf_missing").is_err());
    }

    #[test]
    fn secret_reference_parser_requires_key_and_path() {
        let parsed = parse_secret_reference("TLS_CERT=/srv/secrets/tls.pem").unwrap();
        assert_eq!(
            parsed,
            ("TLS_CERT".to_owned(), "/srv/secrets/tls.pem".to_owned())
        );
        assert!(parse_secret_reference("missing-separator").is_err());
        assert!(parse_secret_reference("=path").is_err());
        assert_eq!(
            secret_references(&[("A".to_owned(), "/p".to_owned())]),
            serde_json::json!([{"environment_key": "A", "file_path": "/p"}])
        );
    }

    #[test]
    fn parameter_parser_requires_key_value() {
        let parsed = parse_parameter("release-version=1.0.0").unwrap();
        assert_eq!(parsed, ("release-version".to_owned(), "1.0.0".to_owned()));
        assert!(parse_parameter("missing-separator").is_err());
    }

    #[test]
    fn json_argument_parser_requires_object() {
        let parsed = parse_json_arg(Some(r#"{"type":"object"}"#), None, "schema").unwrap();
        assert!(parsed.unwrap().is_object());
        assert!(parse_json_arg(Some("[]"), None, "schema").is_err());
        assert!(parse_json_arg(Some("not-json"), None, "schema").is_err());
        assert!(parse_json_arg(None, None, "schema").unwrap().is_none());
    }
}
