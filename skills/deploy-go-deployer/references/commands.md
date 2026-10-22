# CLI 命令参考

以下命令中的 `<cli>` 表示当前 Skill 目录内的 `scripts/deploy-go-deployer`，Windows 使用 `scripts\deploy-go-deployer.exe`。

所有命令都支持 `--api-base`、`--api-key` 与 `--json`。业务成功结果默认输出易读文本；加 `--json` 输出原始 JSON。

来源摘要与固定 Git 提交的文本输出自 0.3.26 提供。部署前若仍使用旧 CLI，先更新 skill/CLI，或使用 `--json show-app` 检查新控制面返回的字段；缺字段不能当作确认来源。

## 环境与自检

```text
DEPLOY_GO_API_BASE_URL=https://deploy.quanxinfu.com   # 可选，默认正式环境
DEPLOY_GO_API_KEY=dgx_...                             # 必填，管理端创建的外部 API Key

<cli> --version
<cli> openapi [--output <PATH>]
```

不得把 API Key 放进命令行参数、普通文件、日志或错误文本。`openapi` 只输出内置契约，不访问网络。

## 应用查询

```text
<cli> list-apps
<cli> show-app <APPLICATION_ID>
```

`list-apps` 只返回当前 Key 绑定且处于启用状态的应用。`show-app` 返回应用元数据、当前 `version`、参数 Schema、部署后验证配置和可用目标；发起写操作前先读取。

`show-app` 还返回独立的 `sources.git` 与 `sources.workspace`，未配置时为 null。Git 摘要包含配置的 `deployment_branch`、`branch_verified_at`、`status`、`source_version`、构建 Agent/节点标识与名称、节点状态、`source_materialization`；工作区摘要包含 `status`、`workspace_version` 和构建 Agent/节点信息。`build_node_status` 是节点状态，不保证 Agent 会话在线或具备能力；`verified` 是配置验证状态，不保证仓库此刻可达。此查询不触发远端解析，配置分支不表示实时远端 HEAD。

目标模式 `two_stage` 使用 Git；`two_stage_script` 使用固定工作区；`script` / `image` 不据这些配置推断来源。`list-targets` 只返回目标信息，分支在应用级查询。部署响应与 `status` 的 `source_policy`、`deployment_branch`、`resolved_commit_sha`、`release_version` 来自该次固定快照；仅 Git 分支来源返回 Git SHA，工作区和历史无来源部署为 null。发布版本只是标识。

## 创建应用

```text
<cli> create-app --name <NAME> --slug <SLUG> --environment dev|test|staging \
  [--description <TEXT>] \
  [--app-type <TYPE>] [--type-version <VERSION>] \
  [--tag <TAG>]... \
  [--parameter-schema <JSON> | --parameter-schema-file <PATH>] \
  [--verification-config <JSON> | --verification-config-file <PATH>]
```

- 只能创建非正式环境应用；`--environment prod` 会被 CLI 拒绝，服务端对 `prod` 返回 403 `external_production_environment_forbidden`。正式环境应用只能由管理员在管理面创建。
- 创建成功返回 201 与应用详情（`targets` 为空数组）；新应用自动绑定当前 Key，随后可直接执行 `register-env-file`、`create-target`、`set-workspace-source`、`deploy`。
- `--app-type` / `--type-version` 省略时使用服务端默认值 `binary` / `1`；两者必须与服务端支持的类型版本组合匹配，否则返回 422。
- `slug` 必须为 3-64 位小写字母、数字或短横线且全局唯一；重复返回 409 `application_slug_exists`。
- 不支持从模板创建，也不支持指定 `status`、`version` 或应用 ID。

## 编辑应用

```text
<cli> update-app <APPLICATION_ID> [--version <N>] \
  [--name <NAME>] [--slug <SLUG>] [--description <TEXT>] \
  [--environment dev|test|staging] \
  [--app-type <TYPE>] [--type-version <VERSION>] \
  [--tag <TAG>]... [--clear-tags] \
  [--parameter-schema <JSON> | --parameter-schema-file <PATH>] \
  [--verification-config <JSON> | --verification-config-file <PATH>]
```

- 只能编辑非正式环境应用；服务端会拒绝正式环境应用和把环境改为 `prod` 的请求。
- `--version` 省略时 CLI 先执行 `show-app` 自动获取；并发修改返回 409 时重新读取并确认。
- `--tag` 可重复传入；`--clear-tags` 清空全部标签，两者不能同时使用。
- JSON 参数必须是 object；文件内容同样按 JSON 解析。
- 至少提供一个要修改的字段，否则 CLI 直接报错。

## 登记 Env

```text
<cli> list-env-files <APPLICATION_ID>
<cli> inspect-env-file <APPLICATION_ID> <ENV_FILE_ID> --key <KEY> [--key <KEY>...]

<cli> register-env-file <APPLICATION_ID> \
  --file-name <NAME.env> --module <MODULE> --content-file <PATH>

<cli> update-env-file <APPLICATION_ID> <ENV_FILE_ID> \
  --content-file <PATH> [--version <N>]

<cli> delete-env-file <APPLICATION_ID> <ENV_FILE_ID> \
  --confirm-file-name <NAME.env> [--version <N>]
```

- 只返回元数据与同步统计；对外 API 永不返回 Env 明文，也不提供查看明文命令。
- inspect-env-file 只接受 1 到 50 个唯一 dotenv 键名，只返回指定键的存在性和 UTF-8 字节长度；缺失键返回 exists=false 且长度为 null。外围成对引号不计入值长度。
- 该检查只适用于 dev、test、staging；prod 会被服务端拒绝；键名通过 POST 请求体发送，不放入 URL。
- 普通输出和 --json 都不包含 Env 值。只能检查用户明确指定的键，不得枚举键名、读取、回显或推断密钥值。
- 内容必须来自本地文件，禁止把密钥写进命令行参数、日志或错误文本。
- 文件名必须以 `.env` 结尾，格式固定为 `dotenv-v1`，内容不支持 `$` 变量展开。
- 登记同名文件返回 409 `env_file_already_registered`，需改用 `update-env-file`。
- `--version` 省略时 CLI 先列出现有 Env 文件自动获取；并发修改返回 409 时重新读取。
- 删除必须同时给出 `--confirm-file-name` 与版本；被镜像目标引用的文件返回 409。
- 以上写操作只允许非正式环境应用。

## 部署目标（部署契约）

```text
<cli> list-targets <APPLICATION_ID>

<cli> create-target <APPLICATION_ID> \
  --node-id <NODE_ID> --script-path <PATH> --timeout-seconds <N> \
  [--target-code <CODE>] [--execution-mode script|two_stage|two_stage_script|image] \
  [--secret-file ENV_KEY=FILE_PATH]... \
  [--image-spec <JSON> | --image-spec-file <PATH>]

<cli> update-target <TARGET_ID> \
  --node-id <NODE_ID> --script-path <PATH> --timeout-seconds <N> --version <N> \
  [--target-code <CODE>] [--execution-mode <MODE>] [--secret-file ENV_KEY=FILE_PATH]...

<cli> set-target-status <TARGET_ID> --status active|disabled --version <N>
```

- `--script-path` 必须位于目标节点的工作根目录内，服务端会校验；非法路径返回 422。
- `execution_mode=image` 必须提供 `--image-spec`，且不接受 `--secret-file`。
- `update-target` 与 `set-target-status` 必须显式给出 `--version`，可从 `list-targets` 获取。
- 目标环境跟随应用环境，不可单独指定；`list-targets` 只能看到当前 Key 绑定的应用。
- 以上写操作只允许非正式环境应用。

## 固定工作区来源

```text
<cli> show-workspace-source <APPLICATION_ID>

<cli> set-workspace-source <APPLICATION_ID> \
  --build-agent-id <AGENT_ID> --workspace-path <ABSOLUTE_PATH> [--version <N>]
```

- 适用于不依赖 Git 的两阶段部署：构建节点上固定绝对路径。
- 未配置时 `show-workspace-source` 返回 404 `application_workspace_source_not_configured`。
- 首次保存返回 201；再次保存必须带 `--version`（省略时 CLI 自动读取现状）。
- 保存会作废该应用的活动部署预览，需重新生成预览后再部署。
- 只允许非正式环境应用。

## 目录（构建 Agent 与 Git 凭证）

```text
<cli> list-build-agents
<cli> list-git-credentials
```

- `list-build-agents` 只列出管理员为当前 Key 授权的节点上的 Agent，返回 `agent_id`、`node_id`、`node_name`、`node_status`、`ready` 与 `unavailable_code`；不含主机、端口、用户名和工作目录。`ready=false` 时不要使用该 Agent，原因码如 `agent_offline`。
- `node_id` 同时用于 `create-target --node-id`；列表为空表示该 Key 尚未被授权任何节点，需要管理员在管理面配置。
- `list-git-credentials` 只返回 `id`、`name`、`fingerprint`、`status`；外部不能创建、轮换、停用或读取凭证内容。

## Git 来源与部署分支

```text
<cli> show-git-source <APPLICATION_ID>

<cli> set-git-source <APPLICATION_ID> \
  --repository-url <URL> --build-agent-id <AGENT_ID> \
  [--git-credential-id <CREDENTIAL_ID>] \
  [--materialization full|sparse] [--sparse-path <PATH>]... \
  [--version <N>]

<cli> refresh-git-branches <APPLICATION_ID> [--wait] [--wait-timeout-seconds <N>]
<cli> show-git-refresh <APPLICATION_ID> <REFS_QUERY_ID>
<cli> set-git-branch <APPLICATION_ID> --branch <NAME> [--version <N>]
```

- 只允许非正式环境应用，含只读的 `show-git-source` / `show-git-refresh`（来源详情含仓库地址与凭证名，`prod` 返回 403）；构建 Agent 必须在 Key 被授权的节点内（403 `external_node_not_allowed`）。
- `--repository-url` 不得内嵌用户名密码；需要认证时用 `--git-credential-id` 引用凭证。省略凭证表示不使用凭证。
- `--materialization sparse` 必须给出至少一个 `--sparse-path`；`full` 不能带路径。
- `set-git-source` 首次创建返回 201；再次保存需要 `version`（省略时 CLI 先读取现状）。保存或变更来源会清空已验证的部署分支，并作废活动部署预览，之后必须重新 `refresh-git-branches` 再 `set-git-branch`。
- `refresh-git-branches` 异步发现远程分支；同一来源版本已有进行中的发现时返回同一条记录。`--wait` 轮询到成功才输出分支列表；失败、过期或超时都会以非零退出码报告原因码。
- `set-git-branch` 只接受最近一次未过期发现结果里的分支；发现过期返回 409 `git_ref_discovery_expired`，需要重新发现。
- 发现、读取来源不下发部署，也不返回凭证；`show-git-source` 的 `status=verified` 且 `deployment_branch` 不为空才表示可以按该分支部署。

## 应用启停、Env 同步重试与配置文件元数据

```text
<cli> set-app-status <APPLICATION_ID> --status active|archived [--version <N>]
<cli> retry-env-sync <APPLICATION_ID> <ENV_FILE_ID> [--target-id <TARGET_ID>]
<cli> list-config-files <APPLICATION_ID>
```

- 三个命令都只允许非正式环境应用，`prod` 返回 403 `external_production_application_forbidden`（含 `list-config-files`）。
- `set-app-status`：归档后应用不可部署、不出现在 `list-apps`；已运行的部署不受影响。归档状态的应用 `show-app` 会 404，因此再次启用时必须显式传入 `--version`（取归档响应里的 `version`）。**只有归档它的同一个 Key 才能重新启用**：管理员或其他 Key 归档的应用返回 403 `external_application_reactivation_forbidden`。
- `retry-env-sync` 与 `probe-runtime` 对带有正式环境目标的应用返回 403 `external_production_deployment_forbidden`，避免借非生产应用触达生产节点。
- `retry-env-sync` 只把失败的同步重新排队并返回重试条数；Env 文件必须属于该应用，否则 404；响应与审计都不含 Env 明文。
- `list-config-files` 只返回元数据：`id`、`path`、`label`、`format`、`role`、`delivery`、`sensitive`、`editable`、`current_version`、`status`、`version`，非敏感文件另带 `current_digest`；不返回内容、部署路径或说明文本。

## 部署预览、运行探测与运行状态

```text
<cli> preview-deploy <APPLICATION_ID> [--target-id <TARGET_ID>] [--release-strategy automatic|manual] [--parameter KEY=VALUE]...
<cli> probe-runtime <APPLICATION_ID>
<cli> show-runtime-status <APPLICATION_ID>
```

- `preview-deploy` 与 `probe-runtime` 只允许非正式环境应用，`prod` 返回 403 `external_production_deployment_forbidden`；它们会写入预览快照或探测任务，不是纯查询。`show-runtime-status` 虽然只读，但属于新增读取，同样只对非正式环境开放（`prod` 返回 403 `external_production_application_forbidden`）。
- `preview-deploy` 返回 `snapshot_hash`、解析出的 `deployment_branch` 与完整 `resolved_commit_sha`、`preview_expires_at` 和目标摘要；不含 Agent 标识、Env 门禁状态与镜像规格。省略 `--target-id` 预览应用全部启用目标。
- 用 `deploy --snapshot-hash <HASH>` 按预览固定的提交部署，`--parameter`、`--release-strategy`、`--release-version` 必须与预览一致；预览过期、来源或参数变化后失效，需重新预览。不传 `--snapshot-hash` 时行为与以前相同。
- `probe-runtime` 异步：返回 `queued` / `in_progress` / `failed` / `skipped`（含原因码，例如无目标、多目标应用、未配置探测端口）；稍后用 `show-runtime-status` 查看 `runtime_state`（`running` / `failed` / `checking` / `unknown` / `archived`）与 `probe_status`。多目标应用不支持平台探测。

## 部署

```text
<cli> deploy <APPLICATION_ID> \
  [--target-id <TARGET_ID>] \
  [--release-version <VERSION>] \
  [--release-strategy automatic|manual] \
  [--parameter KEY=VALUE]... \
  [--snapshot-hash <HASH>] \
  [--idempotency-key <KEY>]
```

- 省略 `--target-id` 时部署应用全部启用目标。
- `--parameter` 可重复传入；两阶段部署通常需要 `release-version` 和 `modules` 参数。
- 正式环境部署会被服务端拒绝，错误码为 `external_production_deployment_forbidden`。
- 幂等键省略时自动生成；需要可重放时显式传入。

## 状态与取消

```text
<cli> status <DEPLOYMENT_ID>
<cli> diagnose <DEPLOYMENT_ID>
<cli> logs <DEPLOYMENT_ID> [--after <SEQUENCE>] [--limit <N>]
<cli> cancel <DEPLOYMENT_ID>
```

`status` 返回部署状态、阶段、错误摘要和各目标运行状态。`diagnose` 返回任务生命周期、origin、错误码和启动前失败信息；`logs` 返回已脱敏日志分页。`cancel` 请求取消部署，不会删除部署记录。
