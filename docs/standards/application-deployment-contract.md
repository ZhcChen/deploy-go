---
date: 2026-08-06
topic: application-deployment-contract
status: accepted
schema_version: 1
---

# 业务应用两阶段部署接入规范

## 目标

Deploy Go 将一次应用部署拆分为准备和发布两个阶段。业务应用继续拥有编译、打包、迁移、切换、重启、验证和回滚逻辑；Deploy Go 负责确定代码版本、调度 Agent、临时交接发布物、串联阶段、采集日志和裁决最终状态。

本规范不把 Deploy Go 建设为通用 CI、代码托管平台或可视化脚本编排器。

## 术语

- **构建节点**：具备源码和构建工具链、运行 Build Agent 的节点。
- **目标节点**：承载线上服务、运行 Target Agent 的节点。
- **准备阶段**：检出确定代码版本并生成不可变发布物，不改变线上运行状态。
- **发布阶段**：在目标节点消费已校验发布物，执行迁移、切换、重启和验证。
- **发布物**：准备阶段输出的文件及 `deploy-go-artifact.json` manifest。
- **Build Agent**：检出代码、执行 prepare、校验并上传发布物的 Agent。
- **Target Agent**：下载发布物、同步 Env 并执行 release 的 Agent。
- **临时制品区**：主控按 deployment 隔离保存发布物的本地目录，只承担短期中转。

Build Agent 与 Target Agent 可以位于不同节点。WSS 只传控制消息和不透明 lease ID；发布物必须通过 Agent 主动发起的 HTTPS 上传/下载，不要求目标节点开放入站文件服务。

## 应用绑定

应用接入至少需要配置：

- 应用环境标识，取值为 `dev` / `test` / `staging` / `prod`。
- Git 仓库 URL 和默认 ref。
- Git 凭证引用；Deploy Go 只保存受控凭证引用，不把凭证写入参数或日志。
- 一个构建 Agent 和一个或多个目标 Agent。
- 准备、发布工作目录的允许根目录。
- 准备和发布超时。
- 允许部署的模块及稳定模块标识。

应用环境是部署环境的唯一权威来源，部署目标上的 `environment` 只读继承
应用环境，不允许部署目标覆盖。应用环境变更时，平台在同一事务内同步该应用
全部部署目标并递增目标版本；历史 snapshot 保持冻结，只有后续部署使用新环境。

一次部署快照必须保存请求 ref、最终 commit SHA、模块、应用身份、发布版本、Build Agent、全部目标/Target Agent 和脚本契约版本。目标后续解绑不得改变历史快照。重试复用原 commit SHA，不重新解析浮动分支。

首版分支来源的配置、发现和 commit 固化规则见 `docs/standards/git-branch-deployment-contract.md`。

## Git 工作区

Git clone、fetch 和 checkout 由 Agent 的固定执行器完成，不属于业务 Make target：

- Agent 只检出主控任务指定的 commit SHA，使用 detached HEAD。
- 禁止准备或发布 target 内执行 `git pull`、隐式切换分支或改写最终 commit。
- 开始执行 target 前必须确认 `HEAD` 等于任务 commit SHA，并按应用策略拒绝脏工作区。
- Git URL、ref 和额外 fetch 参数不能通过业务部署参数覆盖。
- 并发部署使用隔离工作区，不能共享可写 checkout。

## Makefile 接口

业务仓库根目录必须提供两个固定 target：

```makefile
.PHONY: deploy-go-prepare deploy-go-release

deploy-go-prepare:
	# 生成发布物，不改变线上运行状态

deploy-go-release:
	# 消费发布物并完成服务器发布
```

规则：

- 前缀固定为 `deploy-go-`，主控不扫描、不猜测也不允许用户提交任意 target 名称。
- Agent 以参数数组调用 `make --no-print-directory deploy-go-prepare` 或 `make --no-print-directory deploy-go-release`，不拼接 shell 字符串。
- target 必须非交互、可取消、保留底层命令退出码，并遵守 `docs/standards/deploy-script-contract.md`。
- target 可以调用仓库自有 Shell、Rust、Node.js 等脚本，脚本路径和内部实现由业务应用维护。
- target 不得读取 Deploy Go 数据库、调用主控内部接口或调用 Agent CLI。
- 准备 target 不得 SSH 到目标节点，也不得自行上传发布物；发布物交接由 Agent 执行器负责。

## 运行上下文

Agent 通过环境变量传递上下文，不把敏感值或可执行 shell 片段放入命令行：

| 变量 | 阶段 | 说明 |
| --- | --- | --- |
| `DEPLOY_ID` | 两者 | 全局部署 ID |
| `DEPLOY_APP_ID` | 两者 | 应用 ID |
| `DEPLOY_ENVIRONMENT` | 两者 | 应用环境唯一来源（`dev`/`test`/`staging`/`prod`）；部署目标只读继承，不允许目标覆盖 |
| `DEPLOY_RELEASE_VERSION` | 两者 | 不可变发布版本 |
| `DEPLOY_COMMIT_SHA` | 两者 | 已检出的确定 commit |
| `DEPLOY_MODULES` | 两者 | 按任务顺序排列的逗号分隔模块 |
| `DEPLOY_CANCEL_FILE` | 两者 | 取消标记文件 |
| `DEPLOY_OUTPUT_DIR` | 准备 | 本次任务独占的发布物输出目录 |
| `DEPLOY_ARTIFACT_DIR` | 发布 | 已校验的只读发布物目录 |
| `DEPLOY_TARGET` | 发布 | 目标稳定标识，不是任意地址 |
| `DEPLOY_ENV_DIR` | 发布 | 当前任务所需 Env 的只读临时目录；任务结束后失效，不得持久化该路径 |

业务脚本必须把所有变量视为不可信输入并执行白名单或格式校验。业务参数继续按 `docs/standards/deploy-script-contract.md` 的参数 Schema 传入，不允许覆盖保留环境变量。

两阶段部署的 `release-version` 由主控在生成部署预览时自动创建，确认部署复用预览中的版本并由 snapshot 完整性校验保护，管理端不得要求用户填写。`modules` 仍以逗号分隔字符串传给业务脚本；部署目标必须在该字段的 JSON Schema 中使用 `x-options` 声明 1 到 32 个可选模块，管理端据此默认全选并允许多选。例如：

```json
{
  "type": "object",
  "properties": {
    "release-version": { "type": "string", "maxLength": 32 },
    "modules": {
      "type": "string",
      "maxLength": 512,
      "x-options": ["worker", "api", "admin"]
    }
  },
  "required": ["release-version", "modules"],
  "additionalProperties": false
}
```

## 准备阶段

`deploy-go-prepare` 必须：

1. 检查工具链、模块、环境和输出目录。
2. 只从当前 detached HEAD 构建，不能更新代码。
3. 将全部发布物写入 `DEPLOY_OUTPUT_DIR`，不得写入目标运行目录。
4. 生成 `DEPLOY_OUTPUT_DIR/deploy-go-artifact.json`。
5. 对同一 commit、参数和工具链尽可能产生可复现结果。
6. 成功时退出 `0`；任一步骤失败时输出失败事件并非零退出。

准备成功不代表部署成功，只表示发布物可以进入校验和交接阶段。

## 发布物

manifest 遵守 `docs/standards/deploy-artifact-manifest.schema.json`。最小示例：

```json
{
  "schema_version": 1,
  "release_version": "20260806183000",
  "commit_sha": "0123456789abcdef0123456789abcdef01234567",
  "artifacts": [
    {
      "module": "api",
      "path": "api/qfy-voucher-hub-api.tar.gz",
      "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      "size": 1048576
    }
  ]
}
```

Agent 必须重新计算文件大小和 SHA-256，拒绝绝对路径、`..`、符号链接逃逸、缺失文件、重复模块或 manifest 外文件。发布版本和 commit SHA 必须与任务快照一致。单文件最多 512 MiB、单次部署总计最多 2 GiB、最多 256 个文件；运行配置可以进一步收紧，不能放宽硬上限。

Build Agent 校验后使用绑定 Agent、deployment、manifest digest、purpose 和期限的 upload lease 上传。主控只写 quarantine 目录，完成校验后原子发布为不可变制品；Target Agent 使用绑定 target run 的 download lease 执行 Range 下载，并在本地 staging 再次完整校验。任何情况下发布物不得通过 WebSocket 控制消息承载。

v1 上传固定使用 initiate、顺序 `Content-Range` PUT、offset 查询和 finalize。Agent access token 负责 HTTPS 身份认证，WSS payload 只包含不可猜的 lease ID。finalize 原子消费 upload lease；download lease 在有效期内允许同一 target run 断点重试，任务终态、取消或过期后撤销。

发布物默认保留 24 小时。只有无活跃 target run、下载或重试 pin 且已过期时才能清理；上传失败超时或部署明确取消可以提前清理。局部重试只能事务性 pin 仍为 verified 且未过期的制品，否则必须重新 prepare。

手动发布部署处于 `status=running, phase=awaiting_release` 时，verified 制品即使超过普通 TTL 也必须继续保留。管理员开始 release 后主控重新授予 24 小时有效期；取消或进入终态后恢复普通清理规则。

Deploy Go 不保留历史发布物，也不提供 artifact 回退。需要回退时创建指向旧 commit 的新部署并重新 prepare/release；线上回滚动作由业务脚本显式定义。

## 发布阶段

只有准备成功、主控制品 verified、目标所需 Env 已同步且 Target Agent 下载复验通过后，主控才能执行 `deploy-go-release`。

两阶段部署支持两种冻结在 deployment snapshot 中的 `release_strategy`：

- `automatic`：默认值，prepare 成功后自动进入 release。
- `manual`：prepare、制品校验和 Env 首次登记完成后进入 `awaiting_release`，不创建 release task。管理员确认全部目标 Env 当前版本同步成功后，通过 `POST /api/v1/deployments/{id}/release` 放行；接口必须校验 CSRF、应用权限、prepare 终态、制品和 Env 门禁，并保持幂等。

人工放行只允许推进原 deployment，不得替换 commit、release version、模块、目标或制品。API/worker 重启后必须继续停留在 `awaiting_release`，不能因为 prepare 已成功而自动越过门禁。

`deploy-go-release` 必须：

1. 只读取 `DEPLOY_ARTIFACT_DIR` 中已经校验的发布物。
2. 执行目标侧预检，确认运行目录、配置、磁盘和依赖满足要求。
3. 将新版本解压到独立 release 目录，不能直接覆盖当前版本。
4. 按应用规则执行 migration、seed、切换、重启和健康检查。
5. 保留持久化数据、共享配置和外部 volume；不得隐式清库或执行 `docker compose down -v`。
6. 失败时保留准确退出码并给出恢复信息；自动回滚必须由业务脚本显式定义。

发布 target 不负责拉取代码、重新构建或下载未经 Agent 校验的发布物。跨节点发布时，Target Agent 的固定 Git 执行器可以在 target 启动前使用独立短期凭证检出任务固化的同一 commit；该动作不属于业务 target，不能解析浮动 ref。

部署目标可以由管理员开启 `privileged_release`，让协议 v8+（当前 v9）Agent 通过 root executor 执行固定 `make --no-print-directory deploy-go-release`。该开关默认关闭，可适用于两阶段或镜像直连部署目标并进入 deployment snapshot；两阶段 prepare 始终由低权限 runner 执行。镜像直连部署目标使用协议 v8 `image_spec`，没有业务 Git prepare，仍由同一 root executor 执行固定 Make target。普通部署用户可以触发管理员已授权目标，但不能修改开关或获得 root PTY。

原生特权 release 仍受 commit、artifact manifest/digest、Env gate、阶段状态、deadline 和 snapshot 约束。executor 从受控源封存 root-owned immutable bundle 后执行，不能接受任意命令、参数、Make target 或环境变量集合。协议、capability 或 executor 不兼容时必须明确失败，不得自动降级到 launcher 或低权限 release。

## 事件与日志

两个阶段都使用 `DEPLOY_GO_EVENT ` 标记，具体格式见 `docs/standards/deploy-script-contract.md`。业务脚本不需要输出阶段字段；Agent 根据当前任务补充：

- 准备任务：`stage=prepare`。
- 发布任务：`stage=release`。
- 两个任务共享 `deploy_id`，各自使用独立 `task_id` 和单调事件序号。

普通 stdout/stderr 原样保存。准备阶段和发布阶段的日志分别展示，但在同一部署时间线中按任务和序号稳定排序。

## 状态与串联

平台状态按以下顺序推进：

```text
queued -> preparing -> deploying -> verifying -> succeeded
```

任一阶段可以进入 `failed` 或 `canceled`。门禁规则：

- 准备失败、取消或协议冲突时不得创建发布任务。
- manifest 或发布物校验失败时不得创建发布任务。
- 发布失败不能反向标记准备失败。
- 取消请求作用于当前活动任务，并阻止后续阶段启动。
- 同一部署的阶段切换必须由主控持久化状态机决定，不能依赖日志文本或前端连接状态。

## 平台边界（0 入侵）

- Deploy Go 不保证同一应用多个部署的执行顺序，不提供应用级串行队列、自动锁或冲突编排；是否允许并发、如何加锁、如何避免互相覆盖由业务脚本自行实现。
- Deploy Go 只做有 TTL 的发布物中转，不提供长期 artifact 仓库或 artifact 回退；回退通过指向旧 commit 的新部署重新 prepare/release 完成。
- 一次部署覆盖应用绑定的全部有效目标，每个目标有独立运行事实；全部目标成功时整体才成功，单节点失败不得改写其他节点结果。
- 失败重试形成新 deployment，只执行失败或未执行节点，不能静默重复发布已成功节点。

## 权限与安全

- 主控 API 不直接运行应用 Make target，所有业务执行都发生在受控 Agent 上。
- 普通部署任务和业务脚本不获得通用 root、任意 shell 或 Docker 权限；需要特权发布操作时使用 Agent 原生结构化 `privileged_release`，或兼容模式的固定 launcher、systemd oneshot 与精确 sudo 白名单。
- 未启用原生特权 release 的 Docker/root 应用继续提供应用专属 launcher，遵守 `docs/standards/privileged-release-launcher.md`；launcher 由节点管理员安装为 `root:root` 的固定绝对路径，业务脚本只能以精确 sudo 白名单调用。
- launcher 输入只允许 `schema_version`、`app_id`、`operation`、`task_id`、`module`、`release_version` 和 `staging_dir`，不接受 shell、Docker 参数、URL、环境文件内容或任意命令路径。
- 管理员可在节点显式启用 `privileged_execution` 后，通过 `docs/standards/privileged-agent-executor.md` 定义的独立 PTY 通道进行 root 维护；该通道不属于部署任务上下文，不能由应用、Make target、部署参数或普通用户调用。
- root 终端不能替代 `deploy-go-prepare` / `deploy-go-release`、发布物校验、Env 门禁或部署状态机。平台自动化的文件、systemd 与 Docker/Compose 操作必须继续演进为 executor 上的结构化能力，不得通过终端录入或解析命令实现。
- 构建凭证和目标运行凭证分离，准备脚本不能读取目标节点敏感配置。
- Make target、脚本和 manifest 均属于应用发布代码，必须跟随 commit 审查。
- 日志和事件禁止输出 token、私钥、完整环境文件、连接串和临时凭证。

## 兼容模式

已有应用可以继续使用同一 Agent 和 launcher，但仍必须经过相同 HTTPS 制品协议并保留两个 Make target。`privileged_release` 按目标迁移，失败时不自动切换后端。跨节点能力在 Env release 门禁及端到端验证完成前保持默认关闭。现有脚本内部自行更新 Git、SSH 上传或同时构建并发布的行为只能作为迁移输入，不能作为正式契约继续保留。

可执行的最小接入示例位于 `examples/branch-deployment/`。
