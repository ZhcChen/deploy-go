# deploy-go-deployer

`deploy-go-deployer` 是 Deploy Go 对外部署 API 的 Rust CLI / Codex skill 执行器。

支持：

- `list-apps`：列出当前 Key 可部署的应用。
- `show-app <application_id>`：查看应用详情与可用目标。
- `deploy <application_id>`：发起部署，可用 `--target-id`、`--release-version`、
  `--parameter KEY=VALUE` 与 `--idempotency-key`。
- `status <deployment_id>`：查询部署状态。
- `cancel <deployment_id>`：取消部署。
- `openapi`：输出或导出内置对外 OpenAPI 契约。

配置：

- `--api-base` / `DEPLOY_GO_API_BASE_URL`
- `--api-key` / `DEPLOY_GO_API_KEY`（外部 API Key，格式 `dgx_...`）

安全边界：该工具只能调用 `/external/v1` 对外部署 API，不读取 Env，不做管理面操作，
不执行任意命令。正式发布下载路径由 Deploy Go API 提供。
