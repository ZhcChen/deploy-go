---
name: deploy-go-deployer
description: 通过 Deploy Go 对外部署 API 列出可部署应用、查看应用与目标、发起部署、查询部署状态和取消部署。用户要求对 Deploy Go 应用发起部署、查看部署状态或取消部署时使用；不用于读取 Env 或执行管理面操作。
---

# Deploy Go 对外部署

使用 `deploy-go-deployer` 二进制封装调用 Deploy Go 对外部署 API。

## 前置条件

- 设置 `DEPLOY_GO_API_BASE_URL`（默认 `https://deploy.quanxinfu.com`）。
- 设置 `DEPLOY_GO_API_KEY`，管理端创建的外部 API Key，格式 `dgx_...`。
- 确保 `deploy-go-deployer` 已安装；未安装时从 API 下载：

  ```bash
  curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    "$DEPLOY_GO_API_BASE_URL/api/v1/deployer/download/0_2_0/deployer/x86_64" \
    -o /usr/local/bin/deploy-go-deployer
  chmod 0755 /usr/local/bin/deploy-go-deployer
  ```

  架构名支持 `x86_64` 与 `aarch64`；版本使用 `0_2_0` 下划线形式。

## 支持命令

- `deploy-go-deployer list-apps`：列出当前 Key 可部署的应用。
- `deploy-go-deployer show-app <application_id>`：查看应用详情与可用目标。
- `deploy-go-deployer deploy <application_id> [--target-id <id>] [--release-version <version>] [--parameter KEY=VALUE]...`：发起部署。
- `deploy-go-deployer status <deployment_id>`：查询部署状态。
- `deploy-go-deployer cancel <deployment_id>`：取消部署。
- `deploy-go-deployer openapi [--output <path>]`：输出或导出对外 OpenAPI 契约。

所有命令都支持 `--api-base`、`--api-key` 与 `--json`；默认输出易读表格/摘要。

## 执行原则

1. 部署前先执行 `list-apps`，需要目标信息时再执行 `show-app`。
2. 发起部署时优先复用已知 `application_id`；需要单目标部署时传入 `--target-id`。
3. 写操作（`deploy`、`cancel`）先向用户确认应用、目标、版本和关键参数。
4. 部署后立即执行 `status`，将服务端返回的实际状态、阶段和错误码报告给用户。
5. 幂等键默认自动生成；需要可重放时由用户显式传入 `--idempotency-key`。
6. 服务端返回 4xx/5xx 时停止，不通过猜测参数重试写操作。

## 禁止事项

- 不读取、回显或猜测 Env、密钥、SSH 凭证或应用参数以外的敏感数据。
- 不使用管理端 `/api/v1` 接口，不创建/修改应用、节点、用户、Agent 或 API Key。
- 不执行任意 shell、Make target、部署脚本或容器命令。
- 不直接构造未包含在对外 OpenAPI 中的 HTTP 请求。
- 不在未经用户明确授权的情况下对真实生产节点发起部署或取消。

完整调用与 Key 管理说明见 `docs/runbooks/external-deploy-api.md`。
