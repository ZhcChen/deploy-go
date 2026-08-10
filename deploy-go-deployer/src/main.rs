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
    long_about = "通过外部 API Key 列出应用、查看目标、发起部署、查询状态与取消部署。\n\
        该工具只能调用对外部署 API，不读取 Env，也不执行任意命令。"
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

#[cfg(test)]
mod tests {
    use super::{EMBEDDED_EXTERNAL_OPENAPI, parse_parameter};

    #[test]
    fn embedded_openapi_has_external_deployment_paths() {
        let document: serde_json::Value = serde_json::from_str(EMBEDDED_EXTERNAL_OPENAPI).unwrap();
        assert!(
            document["paths"]
                .get("/external/v1/applications/{id}/deployments")
                .is_some()
        );
    }

    #[test]
    fn parameter_parser_requires_key_value() {
        let parsed = parse_parameter("release-version=1.0.0").unwrap();
        assert_eq!(parsed, ("release-version".to_owned(), "1.0.0".to_owned()));
        assert!(parse_parameter("missing-separator").is_err());
    }
}
