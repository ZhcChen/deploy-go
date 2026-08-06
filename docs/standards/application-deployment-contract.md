---
date: 2026-08-06
topic: application-deployment-contract
status: accepted
schema_version: 1
---

# 业务应用两阶段部署接入规范

## 目标

Deploy Go 将一次应用部署拆分为准备和发布两个阶段。业务应用继续拥有编译、打包、迁移、切换、重启和验证逻辑；Deploy Go 负责确定代码版本、调度 Agent、传输发布物、串联阶段、采集日志和裁决最终状态。

本规范不把 Deploy Go 建设为通用 CI、代码托管平台或可视化脚本编排器。

## 术语

- **构建节点**：具备源码和构建工具链、运行 Build Agent 的节点。
- **目标节点**：承载线上服务、运行 Target Agent 的节点。
- **准备阶段**：检出确定代码版本并生成不可变发布物，不改变线上运行状态。
- **发布阶段**：在目标节点消费已校验发布物，执行迁移、切换、重启和验证。
- **发布物**：准备阶段输出的文件及 `deploy-go-artifact.json` manifest。

构建节点和目标节点可以是同一节点。此时仍保留两个阶段和两个任务，只是发布物通过同节点受控 staging 目录交接。

## 应用绑定

应用接入至少需要配置：

- Git 仓库 URL 和默认 ref。
- Git 凭证引用；Deploy Go 只保存受控凭证引用，不把凭证写入参数或日志。
- 构建 Agent 和目标 Agent。
- 准备、发布工作目录的允许根目录。
- 准备和发布超时。
- 允许部署的模块及稳定模块标识。

一次部署快照必须保存请求 ref、最终 commit SHA、模块、环境、发布版本、两个 Agent 和脚本契约版本。重试复用原 commit SHA，不重新解析浮动分支。

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
| `DEPLOY_ENVIRONMENT` | 两者 | 部署环境 |
| `DEPLOY_RELEASE_VERSION` | 两者 | 不可变发布版本 |
| `DEPLOY_COMMIT_SHA` | 两者 | 已检出的确定 commit |
| `DEPLOY_MODULES` | 两者 | 按任务顺序排列的逗号分隔模块 |
| `DEPLOY_CANCEL_FILE` | 两者 | 取消标记文件 |
| `DEPLOY_OUTPUT_DIR` | 准备 | 本次任务独占的发布物输出目录 |
| `DEPLOY_ARTIFACT_DIR` | 发布 | 已传输并校验的只读发布物目录 |
| `DEPLOY_TARGET` | 发布 | 目标稳定标识，不是任意地址 |

业务脚本必须把所有变量视为不可信输入并执行白名单或格式校验。业务参数继续按 `docs/standards/deploy-script-contract.md` 的参数 Schema 传入，不允许覆盖保留环境变量。

## 准备阶段

`deploy-go-prepare` 必须：

1. 检查工具链、模块、环境和输出目录。
2. 只从当前 detached HEAD 构建，不能更新代码。
3. 将全部发布物写入 `DEPLOY_OUTPUT_DIR`，不得写入目标运行目录。
4. 生成 `DEPLOY_OUTPUT_DIR/deploy-go-artifact.json`。
5. 对同一 commit、参数和工具链尽可能产生可复现结果。
6. 成功时退出 `0`；任一步骤失败时输出失败事件并非零退出。

准备成功不代表部署成功，只表示发布物可以进入校验和传输阶段。

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

Agent 必须重新计算文件大小和 SHA-256，拒绝绝对路径、`..`、符号链接逃逸、缺失文件、重复模块或 manifest 外文件。发布版本和 commit SHA 必须与任务快照一致。

跨节点发布物不得通过 WebSocket 控制消息承载。实现应使用受控 artifact HTTP 上传/下载和单次短期凭证；同节点使用任务独占 staging。任一方式都必须先完整落盘并校验，再创建发布任务。

## 发布阶段

只有准备、manifest 校验和传输全部成功后，主控才能创建 `deploy-go-release` 任务。

`deploy-go-release` 必须：

1. 只读取 `DEPLOY_ARTIFACT_DIR` 中已经校验的发布物。
2. 执行目标侧预检，确认运行目录、配置、磁盘和依赖满足要求。
3. 将新版本解压到独立 release 目录，不能直接覆盖当前版本。
4. 按应用规则执行 migration、seed、切换、重启和健康检查。
5. 保留持久化数据、共享配置和外部 volume；不得隐式清库或执行 `docker compose down -v`。
6. 失败时保留准确退出码并给出恢复信息；自动回滚必须由业务脚本显式定义。

发布 target 不负责拉取代码、重新构建或下载未经 Agent 校验的发布物。

## 事件与日志

两个阶段都使用 `DEPLOY_GO_EVENT ` 标记，具体格式见 `docs/standards/deploy-script-contract.md`。业务脚本不需要输出阶段字段；Agent 根据当前任务补充：

- 准备任务：`stage=prepare`。
- 发布任务：`stage=release`。
- 两个任务共享 `deploy_id`，各自使用独立 `task_id` 和单调事件序号。

普通 stdout/stderr 原样保存。准备阶段和发布阶段的日志分别展示，但在同一部署时间线中按任务和序号稳定排序。

## 状态与串联

平台状态按以下顺序推进：

```text
queued -> preparing -> transferring -> deploying -> verifying -> succeeded
```

任一阶段可以进入 `failed` 或 `canceled`。门禁规则：

- 准备失败、取消或协议冲突时不得创建发布任务。
- manifest 或传输校验失败时不得创建发布任务。
- 发布失败不能反向标记准备失败。
- 取消请求作用于当前活动任务，并阻止后续阶段启动。
- 同一部署的阶段切换必须由主控持久化状态机决定，不能依赖日志文本或前端连接状态。

## 权限与安全

- 主控 API 不直接运行应用 Make target，所有业务执行都发生在受控 Agent 上。
- Agent 不获得通用 root 或 Docker 权限；需要特权操作时使用固定 launcher、systemd oneshot 或精确 sudo 白名单。
- 构建凭证和目标运行凭证分离，准备脚本不能读取目标节点敏感配置。
- Make target、脚本和 manifest 均属于应用发布代码，必须跟随 commit 审查。
- 日志和事件禁止输出 token、私钥、完整环境文件、连接串和临时 artifact 凭证。

## 兼容模式

已有应用可以先用同一 Agent 和同节点 staging 接入，但仍必须保留两个 Make target。现有脚本内部自行更新 Git、SSH 上传或同时构建并发布的行为只能作为迁移输入，不能作为正式契约继续保留。

可执行的最小接入示例位于 `examples/branch-deployment/`。
