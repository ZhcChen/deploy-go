---
date: 2026-08-06
topic: agent-control-protocol
status: accepted
protocol_version: 2
---

# Agent 控制协议

## 边界

主控与节点 Agent 使用 WSS 双向连接传递认证续期、心跳、结构化任务、ACK、日志、状态和结果。Web 与 Flutter 不连接该通道；部署日志仍由主控持久化后通过 SSE 提供。

协议类型由 `agent-protocol/src/lib.rs` 定义，机器可读 Schema 位于 `agent-protocol/schema/agent-control.schema.json`。双方必须先校验 Schema 和协议版本，再处理业务字段。协议 v2 在 v1 基础上新增 Git refs 查询、prepare/release 两阶段任务、结构化进度和 secret lease；`deployment_execute` 保留为 v1 legacy 任务。

控制协议不是远程终端，不允许任意 shell、命令字符串、任意下载地址或在线自升级。

## Envelope

每条消息包含：

- `protocol_version`：当前固定为 `2`。v1 Agent 通过连接协商降级为 `1` 后仍可执行 legacy `deployment_execute`，但不能接收 v2 新任务类型。
- `message_id`：发送方生成的不可预测消息标识，用于关联错误和去重。
- `sent_at`：UTC RFC 3339 时间。
- `message`：带严格 `type` 的消息对象。

服务端和 Agent 默认拒绝未知字段、未知消息类型和不受支持版本。协议兼容扩展只能新增双方明确忽略的 envelope 外版本，或提升协议版本；不能依赖 Serde 默认忽略未知请求字段。

## 连接顺序

1. Agent 使用 access token 在 `Authorization` header 中完成 WSS 握手。
2. Agent 发送 `hello`，声明 Agent 版本、协议范围、OS 和架构。
3. 主控选择 `[min_protocol_version, max_protocol_version]` 与 `[MIN_SUPPORTED_PROTOCOL_VERSION, PROTOCOL_VERSION]` 的交集，取交集上限作为共同协议版本，写入 Agent 记录并返回 `hello_ack`。
4. Agent 按间隔发送 `heartbeat`；主控只接受当前连接代次。
5. 新连接接管、管理员撤销或认证最终超时后，主控关闭旧连接并将 Agent 视为离线。

Token 不得放入 WebSocket URL、query、普通 tracing 字段或协议错误详情。

## 无感续期

Agent 在 access token 到期前通过 HTTPS refresh endpoint 滚动取得新的 access/refresh token。新 refresh token 原子写入受保护凭证文件后，Agent在当前 WebSocket 发送 `auth_refresh`；主控验证后更新该连接的认证截止时间并返回 `auth_refreshed`。

同一 `rotation_id` 在提交窗口内必须返回同一轮换结果。主控收到当前连接的确认后撤销旧 refresh token；确认后的旧 token 重用会撤销整个凭证族并关闭连接。临时刷新失败只在当前 access token 有效期和有限宽限期内退避重试，不能通过无限宽限保持在线。

## 任务

`task_dispatch` 必须包含：

- `task_id`、`idempotency_key`、`deadline_at` 和 `payload_digest`。
- 严格枚举的 `task.kind` 与对应 payload。
- 部署任务所需的脚本绝对路径、工作根目录、参数 token、环境文件引用、超时和包装器版本。

任务类型：

- `system_inspect`
- `deployment_execute`（v1 legacy）
- `health_diagnose`
- `git_refs_query`
- `deployment_prepare`
- `deployment_release`

两阶段任务只接受固定 Make target：prepare 为 `deploy_go_prepare`，release 为 `deploy_go_release`。所有 v2 payload 使用 `deny_unknown_fields`，不接受任意字符串 target、shell 命令、内联私钥或带凭证 URL。Git 凭证只以 opaque lease ID 出现在 payload 中。

取消使用独立的 `task_cancel` 控制消息。在线自升级、文件管理、任意 shell 和任意 Make target 不属于协议能力。

## Git refs 查询

`git_refs_query` 由主控创建，用于让选定的在线 Build Agent 通过受限 `git ls-remote --heads` 返回分支列表。payload 必须包含：

- `refs_query_id`、`repository_url`、`timeout_seconds`。
- 可选 `git_credential_lease_id`：私有仓库通过任务绑定的一次性 secret lease 取得 SSH deploy key。

Agent 不得接受 URL 内嵌凭证；查询结果只返回分支名和完整 ref，不返回凭证、主机密钥或仓库内容。

## 两阶段部署任务

`deployment_prepare` 固定包含部署 ID、`source_policy=branch`、仓库 URL、40 位 commit SHA、任务独占 `checkout_dir`/`work_root`/`output_dir`、环境、发布版本、模块列表、Make target、可选 Git lease 和超时。Agent 必须检出不可变 commit 后再执行 `deploy-go-prepare`。

`deployment_release` 固定包含部署 ID、`target_code`、`work_root`、`checkout_dir`、已校验的 `artifact_dir`、环境、发布版本、commit SHA、模块列表、Make target、超时和 `cancel_file`。`checkout_dir` 指向 prepare 阶段检出的同一仓库工作区，Agent 用它运行发布 target，但不得从 release payload 重新拉代码或获取其他发布物。

模块列表必须使用稳定模块标识，不允许重复；路径字段必须是绝对路径并限制在任务允许根目录内。

## 进度事件

Agent 将业务脚本的 `DEPLOY_GO_EVENT` marker 解析、补全后以 `task_progress` 发送。`event` 必须包含主控补全的 `deploy_id`、`stage`、`timestamp`、`status`、`environment`、`release_version`，并按事件类型携带模块、步骤、失败阶段和恢复提示。普通 stdout/stderr 仍通过 `task_output` 发送，不得把未解析文本放进 `task_progress`。

`task_progress` 与 `task_output` 共用任务内单调 `sequence`。主控按任务和序号幂等持久化，重复消息不产生重复 SSE 日志。

## Secret lease

需要 Git 私钥时，Agent 向主控发送 `secret_lease_request`，携带任务 ID、lease ID、payload digest 和用途。主控校验任务绑定、连接 Agent、payload digest 和期限后返回 `secret_lease_response`；响应只在该连接上传递，包含短期过期时间和一次性私钥。

- Agent 只能为当前任务的 payload 换取私钥，私钥写入 `0600` 临时文件，任务结束、失败或恢复时立即清理。
- `task_dispatch` payload、journal、审计、日志和 `task_output` 不得包含私钥或 lease 内容。
- 主控端 secret lease 一次消费、短 TTL，并绑定 Agent、task 和 payload digest；过期、重复或用途不符必须返回稳定错误码。

Agent 收到任务后必须先验证期限、任务 ID、幂等键、payload digest、任务类型、路径、参数数量、输出限制和包装器版本，再返回 `task_ack`：

- `accepted`：首次接受并已持久化。
- `duplicate`：同任务和摘要已存在，返回已有状态，不再次执行。
- `rejected`：字段、权限、期限或摘要冲突，附稳定且脱敏的错误码。

同一任务 ID 或幂等键对应不同 payload digest 时必须拒绝，不能覆盖本地 journal。

## 输出与恢复

`task_output`、`task_state` 和 `task_result` 使用任务内单调递增 `sequence`。主控按任务和序号幂等持久化，重复事件不生成重复 SSE 日志。

`task_result.data` 仅用于任务类型定义的非敏感结构化结果。首版 `system_inspect` 可返回 `os_name`、`architecture`、`hostname`、`disk_available_bytes`、`work_root_accessible` 和 `secrets_root_accessible`；部署脚本输出、token、secret 内容和任意扩展字段不得放入该对象。

Agent 的 durable runner 在受保护任务目录记录 payload digest、进程 PID/start-time、stdout、stderr、读取偏移、退出码和原子完成标记。重连后主控发送 `reconcile_request`，Agent 使用 `reconcile_report` 回报可证明状态及最后序号：

- 已完成结果和未确认日志可以补传。
- 运行中进程只有在身份校验成功时继续跟踪。
- 无法验证进程归属、缺少可信完成标记或本地状态冲突时返回 `interrupted`/`unknown`，不得自动重跑。

## 错误与限制

协议错误使用稳定 `code` 和脱敏 `message`；`details` 不能包含 token、secret、环境文件内容、完整参数或内部路径。至少覆盖：

- `unsupported_protocol_version`
- `stale_connection_generation`
- `invalid_message`
- `task_expired`
- `task_payload_conflict`
- `task_type_not_allowed`
- `path_outside_allowed_root`
- `wrapper_version_unsupported`
- `credential_revoked`
- `secret_lease_expired`
- `secret_lease_reused`
- `secret_lease_purpose_mismatch`

消息大小、任务数量、参数数量、单行日志和总日志预算必须设置服务端与 Agent 双侧硬上限。达到限制时返回稳定错误或截断诊断，不能继续无限分配内存。
