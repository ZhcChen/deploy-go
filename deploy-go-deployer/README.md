# deploy-go-deployer

`deploy-go-deployer` 是 Deploy Go 对外部署 API 的 Rust CLI / Codex skill 执行器。

支持：

- `list-apps`：列出当前 Key 可部署的应用。
- `show-app <application_id>`：查看应用详情与可用目标。
- `update-app <application_id>`：编辑非正式环境应用的元数据、标签、参数 Schema
  与部署后验证配置。
- `list-env-files <application_id>` / `register-env-file` / `update-env-file` /
  `delete-env-file`：登记与维护非正式环境应用的 Env 文件（只写不读明文）。
- `list-targets` / `create-target` / `update-target` / `set-target-status`：
  维护非正式环境应用的部署目标契约。
- `show-workspace-source` / `set-workspace-source`：维护非正式环境应用的固定
  工作区来源。
- `deploy <application_id>`：发起部署，可用 `--target-id`、`--release-version`、
  `--parameter KEY=VALUE` 与 `--idempotency-key`。
- `status <deployment_id>`：查询部署状态。
- `cancel <deployment_id>`：取消部署。
- `openapi`：输出或导出内置对外 OpenAPI 契约。

配置：

- `--api-base` / `DEPLOY_GO_API_BASE_URL`
- `--api-key` / `DEPLOY_GO_API_KEY`（外部 API Key，格式 `dgx_...`）

安装：

- Linux：从 Deploy Go API 下载发布物，见 `docs/runbooks/external-deploy-api.md`。
- macOS：本机源码构建 `cargo build -p deploy-go-deployer --release`，并把
  `target/release/deploy-go-deployer` 安装到 PATH。

安全边界：该工具只能调用 `/external/v1` 对外部署 API，不读取 Env，不做其他管理面
操作，不执行任意命令；发起部署和编辑应用仅限非正式环境，正式环境会返回
`external_production_deployment_forbidden`、`external_production_application_forbidden`
或 `external_production_environment_forbidden`。正式发布下载路径由 Deploy Go API 提供。
