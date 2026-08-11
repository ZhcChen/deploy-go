# 对外部署 OpenAPI 与 deploy-go-deployer

## 用途

Deploy Go 提供独立对外部署 API，供外部系统、Agent 或 Codex skill 使用：

- 列出 Key 可部署的应用
- 查看应用详情与可用部署目标
- 发起部署（支持单目标或应用全部启用目标）
- 查询部署状态
- 取消部署

对外 API 只暴露部署所需数据，不提供 Env 读取、管理面操作或任意命令执行。

## API 地址

- 对外 API：`https://deploy.quanxinfu.com/external/v1/`
- 对外 OpenAPI：`https://deploy.quanxinfu.com/external/v1/openapi.json`
- deployer 二进制下载：
  - manifest：`https://deploy.quanxinfu.com/api/v1/deployer/download/0_2_0/manifest.json`
  - 二进制：`https://deploy.quanxinfu.com/api/v1/deployer/download/0_2_0/deployer/{x86_64|aarch64}`

## 创建 API Key（管理员）

也可以在 Web 管理端操作：`设置 > 对外 API Key`，可创建 Key、绑定/调整应用、
吊销并复制创建时返回的一次性明文 token。

创建 Key 并绑定应用：

```bash
curl -X POST 'https://deploy.quanxinfu.com/api/v1/external-api-keys' \
  -H 'Cookie: deploy_go_session=...' -H 'X-CSRF-Token: ...' \
  -H 'Content-Type: application/json' \
  -d '{"name":"外部 CI"}'
```

创建响应中的 `token`（`dgx_...`）只返回一次，请立即保存。

绑定应用：

```bash
curl -X PUT 'https://deploy.quanxinfu.com/api/v1/external-api-keys/{key_id}/applications' \
  -H 'Cookie: deploy_go_session=...' -H 'X-CSRF-Token: ...' \
  -H 'Content-Type: application/json' \
  -d '{"application_ids":["app_..."]}'
```

吊销 Key：

```bash
curl -X POST 'https://deploy.quanxinfu.com/api/v1/external-api-keys/{key_id}/revoke' \
  -H 'Cookie: deploy_go_session=...' -H 'X-CSRF-Token: ...'
```

## 使用 CLI

```bash
export DEPLOY_GO_API_BASE_URL='https://deploy.quanxinfu.com'
export DEPLOY_GO_API_KEY='dgx_...'

deploy-go-deployer list-apps
deploy-go-deployer show-app app_01KZBSS1TEGH6R2XZZVH9VT6MS
deploy-go-deployer deploy app_01KZBSS1TEGH6R2XZZVH9VT6MS \
  --target-id target_01KZBSS1TEGH6R2XZZVH9VT6MS \
  --release-version 1.2.0 \
  --parameter release=stable
deploy-go-deployer status dep_01KZBSS1TEGH6R2XZZVH9VT6MS
deploy-go-deployer cancel dep_01KZBSS1TEGH6R2XZZVH9VT6MS
```

## 安装 deployer

Linux 环境直接使用 API 发布物（服务器已安装 0.2.0 双架构）：

```bash
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  'https://deploy.quanxinfu.com/api/v1/deployer/download/0_2_0/deployer/x86_64' \
  -o /usr/local/bin/deploy-go-deployer
chmod 0755 /usr/local/bin/deploy-go-deployer
```

macOS 本机未发布官方二进制，从源码构建并安装：

```bash
cargo build -p deploy-go-deployer --release
cp target/release/deploy-go-deployer ~/.local/bin/
```

## 直接调用示例

```bash
curl -X POST 'https://deploy.quanxinfu.com/external/v1/applications/app_.../deployments' \
  -H 'Authorization: Bearer dgx_...' \
  -H 'Content-Type: application/json' \
  -H 'Idempotency-Key: my-deploy-001' \
  -d '{"target_id":"target_...","parameters":{},"release_strategy":"automatic"}'
```

## 安全说明

- API Key 服务端只保存 SHA-256 hash，明文只在创建时返回一次。
- 部署记录 `external_api_key_id`，审计可追溯；对外 DTO 不暴露内部字段。
- 对外部署继续执行现有 preview、参数 schema、Env gate、目标状态和 release 策略校验。
- 部署创建必须带 `Idempotency-Key`，作用域为单个 API Key。
- 不向外部调用方暴露 Env 读取、应用配置、节点连接或管理面接口。

## 发布与更新

`deploy-go-deployer` 二进制与 manifest 由正式环境部署脚本构建并安装到
`/var/lib/deploy-go/deployer-releases/`，API 运行态直接提供服务。

- `make external-deploy-check`：CLI、OpenAPI、发布链路契约检查。
- `make deploy-production`：正式环境构建与安装（需要用户另行授权执行）。

## 故障排查

- `manifest.json` 404：服务器尚未安装对应版本 release，检查
  `systemctl status deploy-go-api` 与 `/var/lib/deploy-go/deployer-releases/`。
- 二进制下载 404：确认版本号使用下划线形式（`0_2_0`）且架构为
  `x86_64` 或 `aarch64`。
- API Key 401：Key 已吊销、过期或未绑定目标应用，联系管理员重新创建。
- 部署 422：查看错误 `code` 与 `message`，通常来自参数 schema、Env gate
  或目标节点不可用。
