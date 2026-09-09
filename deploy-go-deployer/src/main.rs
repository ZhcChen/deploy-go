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
    long_about = "通过外部 API Key 列出应用、查看目标、编辑非正式环境应用、发起部署、\n\
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
    #[arg(long, env = "DEPLOY_GO_API_KEY")]
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
    /// 查看应用详情与可用部署目标
    ShowApp { application_id: String },
    /// 编辑非正式环境应用
    UpdateApp(UpdateAppArgs),
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
                Command::ShowApp { application_id } => client.show_app(&application_id).await?,
                Command::UpdateApp(args) => client.update_app(args).await?,
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
    use super::{EMBEDDED_EXTERNAL_OPENAPI, parse_json_arg, parse_parameter};

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
