# 对外部署 OpenAPI 与 deploy-go-deployer

## 用途

Deploy Go 提供独立对外部署 API，供外部系统、Agent 或 Codex skill 使用：

- 列出 Key 可部署的应用
- 查看应用详情与可用部署目标
- 编辑非正式环境应用（元数据、标签、参数 Schema、部署后验证配置）
- 登记、更新、删除非正式环境应用的 Env 文件（只写不读明文）
- 配置非正式环境应用的部署目标（部署契约）与固定工作区来源
- 发起部署（支持单目标或应用全部启用目标，仅限非正式环境）
- 查询部署状态
- 取消部署

对外 API 只暴露部署所需数据，不提供 Env 读取、管理面操作或任意命令执行。

## API 地址

- 对外 API：`https://deploy.quanxinfu.com/external/v1/`
- 对外 OpenAPI：`https://deploy.quanxinfu.com/external/v1/openapi.json`
- deployer 二进制下载：
  - manifest：`https://deploy.quanxinfu.com/api/v1/deployer/download/0_3_1/manifest.json`
  - 二进制：`https://deploy.quanxinfu.com/api/v1/deployer/download/0_3_1/deployer/{x86_64|aarch64}`

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
deploy-go-deployer update-app app_01KZBSS1TEGH6R2XZZVH9VT6MS \
  --description "测试环境卡券系统" --tag voucher --tag test
deploy-go-deployer list-env-files app_01KZBSS1TEGH6R2XZZVH9VT6MS
deploy-go-deployer register-env-file app_01KZBSS1TEGH6R2XZZVH9VT6MS \
  --file-name api.env --module api --content-file ./api.env
deploy-go-deployer list-targets app_01KZBSS1TEGH6R2XZZVH9VT6MS
deploy-go-deployer create-target app_01KZBSS1TEGH6R2XZZVH9VT6MS \
  --node-id node_01KZBSS1TEGH6R2XZZVH9VT6MS --script-path /srv/apps/deploy.sh \
  --timeout-seconds 600
deploy-go-deployer set-workspace-source app_01KZBSS1TEGH6R2XZZVH9VT6MS \
  --build-agent-id agent_01KZBSS1TEGH6R2XZZVH9VT6MS --workspace-path /srv/workspace
deploy-go-deployer deploy app_01KZBSS1TEGH6R2XZZVH9VT6MS \
  --target-id target_01KZBSS1TEGH6R2XZZVH9VT6MS \
  --release-version 1.2.0 \
  --parameter release=stable
deploy-go-deployer status dep_01KZBSS1TEGH6R2XZZVH9VT6MS
deploy-go-deployer cancel dep_01KZBSS1TEGH6R2XZZVH9VT6MS
```

## 安装 deployer

Linux 环境直接使用 API 发布物（服务器已安装 0.3.1 双架构）：

```bash
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  'https://deploy.quanxinfu.com/api/v1/deployer/download/0_3_1/deployer/x86_64' \
  -o /usr/local/bin/deploy-go-deployer
chmod 0755 /usr/local/bin/deploy-go-deployer
```

macOS 本机未发布官方二进制，从源码构建并安装：

```bash
cargo build -p deploy-go-deployer --release
cp target/release/deploy-go-deployer ~/.local/bin/
```

### 安装 Codex Skill

仓库内提供自包含 Skill 包 `skills/deploy-go-deployer/`，包含 `SKILL.md`、references、
`agents/openai.yaml` 与安装脚本。macOS/Linux 在仓库 checkout 中执行：

```bash
make deployer-skill-install
```

脚本会构建 `deploy-go-deployer`、安装到
`${CODEX_HOME:-$HOME/.codex}/skills/deploy-go-deployer/`，并执行 `--version` 自检。
Linux 无 Rust 工具链时可设置 `DEPLOY_GO_DEPLOYER_VERSION`，脚本会从 Deploy Go API
下载对应架构二进制并校验 manifest 中的 SHA-256。也可直接运行：

```bash
bash skills/deploy-go-deployer/scripts/install.sh
```

## 直接调用示例

```bash
curl -X POST 'https://deploy.quanxinfu.com/external/v1/applications/app_.../deployments' \
  -H 'Authorization: Bearer dgx_...' \
  -H 'Content-Type: application/json' \
  -H 'Idempotency-Key: my-deploy-001' \
  -d '{"target_id":"target_...","parameters":{},"release_strategy":"automatic"}'
```

编辑非正式环境应用：

```bash
curl -X PATCH 'https://deploy.quanxinfu.com/external/v1/applications/app_...' \
  -H 'Authorization: Bearer dgx_...' \
  -H 'Content-Type: application/json' \
  -d '{"version":3,"description":"测试环境卡券系统","tags":["voucher","test"]}'
```

登记 Env 文件：

```bash
curl -X POST 'https://deploy.quanxinfu.com/external/v1/applications/app_.../env-files' \
  -H 'Authorization: Bearer dgx_...' \
  -H 'Content-Type: application/json' \
  -d '{"files":[{"file_name":"api.env","module":"api","format":"dotenv-v1","content":"API_MODE=fast\n"}]}'
```

配置部署目标（部署契约）与固定工作区来源：

```bash
curl -X POST 'https://deploy.quanxinfu.com/external/v1/applications/app_.../targets' \
  -H 'Authorization: Bearer dgx_...' \
  -H 'Content-Type: application/json' \
  -d '{"node_id":"node_...","script_path":"/srv/apps/deploy.sh","timeout_seconds":600}'

curl -X PUT 'https://deploy.quanxinfu.com/external/v1/applications/app_.../workspace-source' \
  -H 'Authorization: Bearer dgx_...' \
  -H 'Content-Type: application/json' \
  -d '{"build_agent_id":"agent_...","workspace_path":"/srv/workspace"}'
```

## 安全说明

- API Key 服务端只保存 SHA-256 hash，明文只在创建时返回一次。
- 部署记录 `external_api_key_id`，审计可追溯；对外 DTO 不暴露内部字段。
- 对外部署继续执行现有 preview、参数 schema、Env gate、目标状态和 release 策略校验。
- 对外部署调用方不需要先“刷新分支”：应用固定分支后，每次部署创建时由服务端
  自动解析该分支最新 commit；不带 `snapshot_hash` 的直接部署等价于服务端生成
  最新预览后立即确认。
- 部署创建必须带 `Idempotency-Key`，作用域为单个 API Key。
- 对外部署 API 仅允许对非正式环境（`dev` / `test` / `staging`）发起部署。应用或
  指定目标的环境为 `prod` 时返回 403 `external_production_deployment_forbidden`，
  正式环境部署仍须通过管理面执行。
- 对外 API 仅允许编辑非正式环境应用。正式环境应用返回 403
  `external_production_application_forbidden`；把非正式环境应用环境改为 `prod`
  返回 403 `external_production_environment_forbidden`。编辑使用 `version`
  乐观锁，冲突返回 409 `resource_version_conflict`。
- Env、部署目标与固定工作区来源的写操作同样只允许非正式环境应用，正式环境应用
  统一返回 403 `external_production_application_forbidden`。
- Env 只写不读：对外接口提供登记、更新、删除与元数据查询，任何响应都不包含明文；
  也不提供重新认证授权或明文回显入口。
- 部署目标 `script_path` 对外可配置，但服务端强制校验它位于目标节点工作根目录内，
  对外响应不会返回节点工作根目录或密钥根目录。
- 不向外部调用方暴露节点连接、Git 凭证或管理面接口。

## 发布与更新

`deploy-go-deployer` 二进制与 manifest 由正式环境部署脚本构建并安装到
`/var/lib/deploy-go/deployer-releases/`，API 运行态直接提供服务。

- `make external-deploy-check`：CLI、OpenAPI、发布链路契约检查。
- `make deploy-production`：正式环境构建与安装（需要用户另行授权执行）。

## 故障排查

- `manifest.json` 404：服务器尚未安装对应版本 release，检查
  `systemctl status deploy-go-api` 与 `/var/lib/deploy-go/deployer-releases/`。
- 二进制下载 404：确认版本号使用下划线形式（`0_3_1`）且架构为
  `x86_64` 或 `aarch64`。
- API Key 401：Key 已吊销、过期或未绑定目标应用，联系管理员重新创建。
- 部署 403 `external_production_deployment_forbidden`：目标应用或指定目标属于正式
  环境，对外 API 不允许发起正式环境部署，请改走管理面部署流程。
- 编辑 403 `external_production_application_forbidden`：目标是正式环境应用，对外
  API 不允许编辑，请改走管理面。
- 编辑 403 `external_production_environment_forbidden`：请求把环境改为 `prod`，
  对外 API 不允许，请确认目标环境。
- 编辑 409 `resource_version_conflict`：应用已被其他请求修改，重新执行 `show-app`
  获取最新 `version` 后再提交。
- Env 409 `env_file_already_registered`：同名 Env 文件已登记，改用
  `PUT /external/v1/applications/{id}/env-files/{env_file_id}` 更新。
- Env 409 `env_file_referenced_by_image_target`：该 Env 文件被镜像部署目标的
  `image_spec.env_files` 引用，先从目标移除引用再删除。
- 配置类写操作 409 `node_not_deployable` / `agent_offline` /
  `agent_protocol_unsupported`：目标节点或构建 Agent 当前不可用，恢复节点后重试。
- 部署目标 422：通常是 `script_path` 不在节点工作根目录内，或 `execution_mode`
  与 `image_spec`、`secret_file_references` 组合不合法。
- 部署 422：查看错误 `code` 与 `message`，通常来自参数 schema、Env gate
  或目标节点不可用。
