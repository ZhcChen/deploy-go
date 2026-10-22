# 对外部署 OpenAPI 与 deploy-go-deployer

## 用途

Deploy Go 提供独立对外部署 API，供外部系统、Agent 或 Claude Code Skill 使用：

- 创建非正式环境应用（正式环境应用只能由管理员在管理面创建）
- 列出 Key 可部署的应用
- 查看应用详情与可用部署目标
- 编辑非正式环境应用（元数据、标签、参数 Schema、部署后验证配置）
- 登记、更新、删除非正式环境应用的 Env 文件
- 检查非生产 Env 文件中指定键是否存在及其 UTF-8 字节长度（不返回明文）
- 配置非正式环境应用的部署目标（部署契约）、固定工作区来源与 Git 来源（仓库、凭证、构建 Agent、sparse 检出）
- 发现远程分支并设置非正式环境应用的部署分支
- 查询可用的构建 Agent 与 Git 凭证（仅元数据）
- 启用或归档非正式环境应用、重试失败的 Env 同步、查看配置文件元数据（不含内容）
- 生成部署预览并按预览固定提交部署、触发平台运行探测并查看运行状态（非正式环境）
- 发起部署（支持单目标或应用全部启用目标，仅限非正式环境）
- 查询部署状态
- 查询部署失败详情与脱敏后的分页日志
- 取消部署

对外 API 只暴露部署所需数据，不提供 Env 明文读取、管理面操作或任意命令执行。

## API 地址

- 对外 API：`https://deploy.quanxinfu.com/external/v1/`
- 对外 OpenAPI：`https://deploy.quanxinfu.com/external/v1/openapi.json`
- deployer 二进制下载：
  - 稳定版 manifest：`https://deploy.quanxinfu.com/api/v1/deployer/download/stable/manifest.json`
  - 稳定版二进制：`https://deploy.quanxinfu.com/api/v1/deployer/download/stable/deployer/x86_64`

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

prepare 构建脚本退出 0 而发布物交接失败时，`diagnose` 保留 `artifact_transfer_failed`、`artifact_transfer_timeout`、`artifact_prepare_failed`、`artifact_authorization_failed` 等稳定错误码，任务摘要包含脱敏传输子阶段及可用的请求次数、耗时、确认 offset。未知字段显示未知，旧 Agent 缺详情时明确提示；不开放 `data` 或节点系统日志。现有 CLI 的诊断任务表与 `--json` 均支持现有 summary 字段，无需新增命令。

先查询同一 deployment 的状态和诊断，不能因 exit0 或最后一行“构建完成”认定成功；也不要盲目重复创建部署。恢复与回滚步骤见 `docs/runbooks/deployment-recovery.md`。

```bash
export DEPLOY_GO_API_BASE_URL='https://deploy.quanxinfu.com'
export DEPLOY_GO_API_KEY='dgx_...'

deploy-go-deployer list-apps
deploy-go-deployer create-app --name "Clickhouse 测试" --slug clickhouse-test \
  --environment test --tag clickhouse
deploy-go-deployer show-app app_01KZBSS1TEGH6R2XZZVH9VT6MS
deploy-go-deployer update-app app_01KZBSS1TEGH6R2XZZVH9VT6MS \
  --description "测试环境卡券系统" --tag voucher --tag test
deploy-go-deployer list-env-files app_01KZBSS1TEGH6R2XZZVH9VT6MS
deploy-go-deployer inspect-env-file app_01KZBSS1TEGH6R2XZZVH9VT6MS envf_01KZ... \
  --key BI_SESSION_KEY --key DATABASE_URL
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
deploy-go-deployer diagnose dep_01KZBSS1TEGH6R2XZZVH9VT6MS
deploy-go-deployer logs dep_01KZBSS1TEGH6R2XZZVH9VT6MS --limit 100
deploy-go-deployer cancel dep_01KZBSS1TEGH6R2XZZVH9VT6MS
```

inspect-env-file 仅支持 dev、test、staging 应用。每次请求需提供 1 到 50 个唯一 dotenv 变量名，结果仅包含请求键的存在性和 UTF-8 字节长度；外围成对引号不计入长度，缺失键的长度为 null。键名通过 POST JSON body 传输。API 响应、审计、CLI 输出均不含 Env 值；不得用长度推断密钥内容。

## 部署失败排查顺序

部署前用 `show-app` 查看 `sources.git` / `sources.workspace`：未配置为 null，draft/archived 如实返回。Git 的 `deployment_branch` 为配置分支，`branch_verified_at` 为历史验证时间；普通读取不查询远端 HEAD、不下发任务、不返回凭证。`two_stage` 使用 Git，`two_stage_script` 使用工作区，两份配置可以共存。`build_node_status` 只表示节点状态，不能替代 Agent 在线和能力检查。新 CLI 兼容旧控制面，但缺少 `sources` 时会提示信息不可用，不能据此确认分支。

创建部署与 `status` 返回的 `source_policy`、`deployment_branch`、`resolved_commit_sha`、`release_version` 来自该次持久化快照，不会因后续来源配置变更而改变。仅 branch 来源返回 Git SHA，工作区摘要不冒充 Git SHA；历史未保存来源的记录返回 null。`release_version` 是发布标识，不代表代码提交；镜像模式从 `image` 快照读取，两阶段模式从 `two_stage` 快照读取。

按以下顺序使用同一个部署 ID 排查：

1. `status`：确认部署汇总状态、退出码、目标运行的 `started_at` 和日志游标。
2. `diagnose`：确认失败发生在哪个任务阶段，以及 Agent 是否拒绝、返回结果、超时或由控制面收敛。
3. `logs`：仅在 `logs_available=true` 时读取已持久化的 stdout/stderr，并按 `next_after` 继续分页。

`started_at=null` 只表示该任务没有收到公开的 Running 事实，不等于没有执行过任务；Agent
可能在进入 Running 前直接返回失败，或在 ACK 阶段拒绝任务。`logs_available=false` 也不等于
没有失败：启动前拒绝、TaskResult 失败、超时和控制面恢复都可能没有 stdout/stderr，此时以
`diagnose` 返回的 `origin`、`error_code`、`summary`、`last_event_kind` 和时间字段为第一轮
判断依据。

当 `error_code=deploy_event_protocol_conflict` 时，若 Agent 已上报协议结束事件，诊断还会返回
脱敏后的 `protocol_detail`。该字段用于区分事件未闭合、顺序错误或重复事件；缺少该字段时，
仍需到管理面查看完整事件流。

示例：

```bash
deploy-go-deployer status dep_...
deploy-go-deployer diagnose dep_...
deploy-go-deployer logs dep_... --after 128 --limit 100
```

诊断中的 `tasks` 覆盖部署级 `prepare`、`execute`、`release` 任务；`origin` 只使用稳定的
公开分类：`queued`、`dispatching`、`agent_rejected`、`agent_started`、`agent_result`、
`agent_timeout`、`control_plane_reconcile` 和 `unknown`。其中 `agent_result` 也适用于未进入
Running 就收到 TaskResult 的情况，不能仅凭 `started_at` 判断执行是否发生。

## 查询部署日志

部署详情中的 `target_runs` 会返回 `exit_code`、`failure_stage`、`failure_step`、
`logs_available` 和 `last_log_sequence`。需要进一步排查时，使用部署级全局序号分页：

```bash
curl --fail 'https://deploy.quanxinfu.com/external/v1/deployments/dep_.../logs?limit=100' \
  -H 'Authorization: Bearer dgx_...'

curl --fail 'https://deploy.quanxinfu.com/external/v1/deployments/dep_.../logs?after=128&limit=100' \
  -H 'Authorization: Bearer dgx_...'
```

响应包含 `items`、`next_after` 和 `terminal`。`limit` 范围为 1 到 200；调用方应保存
`next_after` 并作为下一次 `after`。日志只读取已持久化内容，不提供 SSE follow 或 Agent
控制能力。返回内容会脱敏凭证、密码、Token、Bearer 认证头和私钥块，并受现有日志保留
策略约束；游标早于保留窗口时应重新从当前可用窗口读取。

`diagnostics` 和 `logs` 都复用 External API Key 的应用归属校验。Key 未绑定该部署所属应用、
应用已停用或部署不存在时，不返回其他应用是否存在的内部信息。诊断不会返回原始任务 ID、
Agent 事件载荷、节点连接信息、工作目录或凭证；`protocol_detail` 仅返回经过 External 安全
投影的协议错误摘要，不返回原始事件名称或完整载荷。需要完整事件和节点日志时仍应使用
Deploy Go 管理面或登录目标节点按权限查看。

## 安装 deployer

Linux amd64 环境直接使用 API 当前稳定发布物：

```bash
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  'https://deploy.quanxinfu.com/api/v1/deployer/download/stable/deployer/x86_64' \
  -o /usr/local/bin/deploy-go-deployer
chmod 0755 /usr/local/bin/deploy-go-deployer
```

macOS 本机未发布官方二进制，从源码构建并安装：

```bash
cargo build -p deploy-go-deployer --release
cp target/release/deploy-go-deployer ~/.local/bin/
```

### 安装 Claude Code Skill

仓库内提供自包含 Skill 包 `skills/deploy-go-deployer/`，包含 `SKILL.md`、references
与安装脚本。macOS/Linux 在仓库 checkout 中执行：

```bash
make deployer-skill-install
```

脚本会构建 `deploy-go-deployer`、安装到
`${CLAUDE_HOME:-$HOME/.claude}/skills/deploy-go-deployer/`，并执行 `--version` 自检。
Linux 无 Rust 工具链时，脚本默认从 Deploy Go API 下载 `stable` 对应架构二进制并校验
manifest 中的 SHA-256；需要固定历史版本时可设置 `DEPLOY_GO_DEPLOYER_VERSION`。也可直接运行：

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

创建非正式环境应用：

```bash
curl -X POST 'https://deploy.quanxinfu.com/external/v1/applications' \
  -H 'Authorization: Bearer dgx_...' \
  -H 'Content-Type: application/json' \
  -d '{"name":"Clickhouse 测试","slug":"clickhouse-test","environment":"test","tags":["clickhouse"]}'
```

创建成功返回 201 与应用详情（`targets` 为空数组），新应用自动绑定到调用方 API Key。

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
- 对外 API 只能创建非正式环境应用。`POST /external/v1/applications` 传
  `environment=prod` 返回 403 `external_production_environment_forbidden`，正式环境
  应用只能由管理员在管理面手动创建；创建成功的新应用自动绑定到调用方 API Key，
  其他 Key 不可见。不支持从模板创建（`template_id` 不在对外请求体中）。
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
- 不向外部调用方暴露节点连接、Git 凭证内容或管理面接口。Git 凭证只暴露 id、名称、
  指纹与状态，外部只能引用，不能创建、轮换、停用或读取。
- Git 来源、分支发现与设置分支只允许非正式环境应用，`prod` 统一返回 403
  `external_production_application_forbidden`；保存来源使用 `version` 乐观锁，保存会清空
  已验证的部署分支并作废活动预览。
- 节点白名单：写 Git 来源、固定工作区来源和部署目标时，所用节点（构建 Agent 所属节点或
  目标节点）必须在管理员为该 Key 授权的节点内，否则 403 `external_node_not_allowed`。
  发起分支发现与设置分支会让来源现用的构建 Agent 参与，同样要求该节点仍在授权内。读取、
  已有目标的部署与启停目标不受限制。

## Git 来源与部署分支

接口（均在 `/external/v1` 下）：

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/build-agents` | 该 Key 被授权节点上的构建 Agent，含 `ready` 与 `unavailable_code`，不含连接信息 |
| GET | `/git-credentials` | 可引用的 Git 凭证：`id`、`name`、`fingerprint`、`status` |
| GET / PUT | `/applications/{id}/git-source` | 读取 / 保存 Git 来源，首次创建 201 |
| POST | `/applications/{id}/git-source/refreshes` | 发起分支发现，返回 202；同一来源版本已有进行中的发现时返回同一条 |
| GET | `/applications/{id}/git-source/refreshes/{refs_query_id}` | 轮询发现结果 |
| PUT | `/applications/{id}/git-source/branch` | 设置部署分支，分支必须在最近一次未过期的发现结果中 |

CLI：`list-build-agents`、`list-git-credentials`、`show-git-source`、`set-git-source`、
`refresh-git-branches --wait`、`show-git-refresh`、`set-git-branch`。完整顺序见 Skill 的
`references/workflows.md`。

## 应用启停、Env 同步重试与配置文件元数据

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| PUT | `/applications/{id}/status` | `active` / `archived`，`version` 乐观锁；响应只含 `id`、`status`、`version` |
| POST | `/applications/{id}/env-files/{env_file_id}/sync-retry` | 把失败的 Env 同步重新排队，可选 `?target_id=`；响应只含 `retried` |
| GET | `/applications/{id}/config-files` | 配置文件元数据，`Cache-Control: no-store` |

- 三个接口都只允许非正式环境应用，`prod` 返回 403 `external_production_application_forbidden`，已归档的生产应用也不能经对外 API 重新启用。
- 状态接口的访问检查只看 Key 绑定、不要求应用处于启用状态，否则归档后无法再启用；其余接口行为不变。归档后的应用不再出现在 `GET /applications`，也不能发起部署（404）。
- 配置文件元数据使用独立响应结构，不含内容、模板摘要、部署路径和说明文本，敏感文件也不返回摘要；外部没有读取配置内容的接口。
- CLI：`set-app-status`、`retry-env-sync`、`list-config-files`。

## 部署预览、运行探测与运行状态

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| POST | `/applications/{id}/deployment-preview` | 应用级预览（全部启用目标），返回 `snapshot_hash` |
| POST | `/deployment-targets/{target_id}/deployment-preview` | 单目标预览 |
| POST | `/applications/{id}/runtime-probe` | 发起平台运行探测，返回 `queued` / `in_progress` / `failed` / `skipped` |
| GET | `/applications/{id}/runtime-status` | 最近一次运行状态摘要（`runtime_state`、`probe_status`、`checked_at`） |

- 预览与探测会写入预览快照或探测任务，所以与其他写接口一致：Key 绑定 + 非生产守卫。`prod` 应用或带正式环境目标的应用返回 403 `external_production_deployment_forbidden`。`runtime-status` 只读，沿用 Key 绑定即可查看（含 `prod`，行为与其他只读接口一致）。
- 对外预览响应是独立的脱敏结构，只含应用与目标摘要、解析出的分支与提交、发布版本、模块、`snapshot_hash` 与有效期；不含 Agent 标识、Env 门禁状态、镜像规格和工作区路径。
- 按预览部署：`POST /applications/{id}/deployments` 带 `snapshot_hash`（CLI：`deploy --snapshot-hash`），部署固定在预览解析出的提交上；预览按“服务账号 + 应用/目标 + hash”匹配，对外调用共用同一服务账号，hash 本身是凭据，不要在日志里暴露。
- 运行探测是异步任务；多目标应用不支持平台探测（返回 `skipped`）。
- CLI：`preview-deploy`、`probe-runtime`、`show-runtime-status`。

## 节点白名单上线步骤

`external_api_key_nodes`（migration `0043`）上线时不回填任何数据，所有已有 Key 的节点集合为空。
因此升级后这些 Key 立刻不能经对外 API 新增或修改 Git 来源、固定工作区来源和部署目标
（返回 403 `external_node_not_allowed`）；查询、已有目标的部署、取消和停用目标不受影响。

1. 部署包含 `0043` 的版本（需要用户另行授权正式环境部署）。
2. 管理员在管理面“对外 API Key”页面，对每个仍需要配置能力的 Key 点击“管理节点”，勾选允许使用的节点并保存。
3. 用 Key 执行 `list-build-agents` 确认能看到被授权节点上的 Agent。

回滚：`0043` 只新增表，回滚程序不需要回滚 schema；旧版本会忽略该表。

## 发布与更新

`deploy-go-deployer` 二进制与 manifest 由正式环境部署脚本构建并安装到
`/var/lib/deploy-go/deployer-releases/`，API 运行态直接提供服务。

- `make external-deploy-check`：CLI、OpenAPI、发布链路契约检查。
- `make deploy-production`：正式环境构建与安装（需要用户另行授权执行）。

## 故障排查

- `manifest.json` 404：服务器尚未安装对应版本 release，检查
  `systemctl status deploy-go-api` 与 `/var/lib/deploy-go/deployer-releases/`。
- 二进制下载 404：优先使用 `stable`，固定版本可使用点号或下划线形式，并确认架构为
  `x86_64`。
- API Key 401：Key 已吊销、过期或未绑定目标应用，联系管理员重新创建。
- 部署 403 `external_production_deployment_forbidden`：目标应用或指定目标属于正式
  环境，对外 API 不允许发起正式环境部署，请改走管理面部署流程。
- 编辑 403 `external_production_application_forbidden`：目标是正式环境应用，对外
  API 不允许编辑，请改走管理面。
- 编辑 403 `external_production_environment_forbidden`：请求把环境改为 `prod`，
  对外 API 不允许，请确认目标环境。
- 创建 403 `external_production_environment_forbidden`：请求创建 `prod` 应用，对外
  API 不允许；正式环境应用改走管理面手动创建。
- 创建 409 `application_slug_exists` / `application_identity_exists`：slug 或应用
  名称冲突，更换 slug 后重试；不要重复提交同一请求。
- 编辑 409 `resource_version_conflict`：应用已被其他请求修改，重新执行 `show-app`
  获取最新 `version` 后再提交。
- Env 409 `env_file_already_registered`：同名 Env 文件已登记，改用
  `PUT /external/v1/applications/{id}/env-files/{env_file_id}` 更新。
- Env 409 `env_file_referenced_by_image_target`：该 Env 文件被镜像部署目标的
  `image_spec.env_files` 引用，先从目标移除引用再删除。
- 403 `external_node_not_allowed`：Key 未被授权使用该节点（或根本没有授权任何节点），联系管理员
  在管理面“对外 API Key”页面授权后重试；不要更换节点绕过。
- Git 来源 409 `git_ref_discovery_missing` / `git_ref_discovery_expired` / `git_branch_not_found`：
  先执行 `refresh-git-branches --wait` 重新发现，再从发现结果中选择分支。
- 配置类写操作 409 `node_not_deployable` / `agent_offline` /
  `agent_protocol_unsupported`：目标节点或构建 Agent 当前不可用，恢复节点后重试。
- 部署目标 422：通常是 `script_path` 不在节点工作根目录内，或 `execution_mode`
  与 `image_spec`、`secret_file_references` 组合不合法。
- 部署 422：查看错误 `code` 与 `message`，通常来自参数 schema、Env gate
  或目标节点不可用。
