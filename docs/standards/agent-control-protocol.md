---
date: 2026-08-06
topic: agent-control-protocol
status: accepted
protocol_version: 5
---

# Agent 控制协议

## 边界

主控与节点 Agent 使用 WSS 双向连接传递认证续期、心跳、结构化任务、ACK、日志、状态和结果。Web 与 Flutter 不连接该通道；部署日志仍由主控持久化后通过 SSE 提供。

协议类型由 `agent-protocol/src/lib.rs` 定义，机器可读 Schema 位于 `agent-protocol/schema/agent-control.schema.json`。双方必须先校验 Schema 和协议版本，再处理业务字段。当前协议 v5 在 v4 基础上新增独立 PTY 会话流；v4 新增 `env_sync` 任务和 release Env 版本门禁；v3 负责跨节点 artifact 授权握手和 HTTPS 传输引用。`deployment_execute` 保留为 v1 legacy 任务，未携带 artifact 引用的两阶段任务保留为 v2 同节点兼容路径。

协议 v4 及普通结构化任务不是远程终端，不允许携带任意 shell、命令字符串、任意下载地址或在线自升级。协议 v5 可以新增独立、不可重放的 PTY 会话流，但只能在 `docs/standards/privileged-agent-executor.md` 定义的管理员授权、节点显式开关和本机 root executor 边界内使用；不得把该例外扩展为普通任务的任意命令字段。

## Envelope

每条消息包含：

- `protocol_version`：当前 Schema 固定为 `5`。Rust 协议类型仍接受协商后的 v1-v5 envelope；旧 Agent 只能接收对应版本支持的任务，v1 只执行 legacy `deployment_execute`，v2 不能接收 artifact 字段，v3 不能接收 Env 同步任务和 release Env 门禁字段，v4 不能接收 PTY 消息。
- `message_id`：发送方生成的不可预测消息标识，用于关联错误和去重。
- `sent_at`：UTC RFC 3339 时间。
- `message`：带严格 `type` 的消息对象。

服务端和 Agent 默认拒绝未知字段、未知消息类型和不受支持版本。协议兼容扩展只能新增双方明确忽略的 envelope 外版本，或提升协议版本；不能依赖 Serde 默认忽略未知请求字段。

## 连接顺序

1. Agent 使用 access token 在 `Authorization` header 中完成 WSS 握手。
2. Agent 发送 `hello`，声明 Agent 版本、协议范围、OS、架构和可选能力集合。`pty_terminal` 只能由协议上限至少为 v5 且本机 executor 健康兼容的 Agent 声明；旧 Agent 不携带 `capabilities` 时按空集合处理。
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
- `env_sync`（v4）

两阶段任务只接受固定 Make target：prepare 为 `deploy_go_prepare`，release 为 `deploy_go_release`。所有 v2 payload 使用 `deny_unknown_fields`，不接受任意字符串 target、shell 命令、内联私钥或带凭证 URL。Git 凭证只以 opaque lease ID 出现在 payload 中。

取消使用独立的 `task_cancel` 控制消息。在线自升级、文件管理、任意 shell 和任意 Make target 不属于普通任务能力。协议 v5 的 PTY 输入是已授权终端会话的短生命周期字节流，不是 `task_dispatch` payload，也不得写入 durable task journal、任务日志或结果。

## 特权 PTY 会话边界

PTY 是协议 v5 起独立于 durable task 的可选能力，内部统一使用 `terminal` / `pty_session` 术语；管理端可以把入口显示为“SSH”，但系统不实现 SSH server、不开放 SSH 端口，也不保存 SSH 私钥。v4 Agent 必须继续支持原有结构化任务，但不得声明或接收 PTY 消息。

创建会话必须同时满足：

- 调用者通过主控 API 的管理员 RBAC 校验；前端隐藏入口不能替代服务端授权。
- 节点身份有效且在线，共同协议版本至少为 v5，Agent 明确上报兼容的 `pty_terminal` 和 executor 健康能力。
- 节点的 `privileged_execution` 开关由管理员显式启用；该开关默认关闭，关闭或撤销身份时必须拒绝新会话并终止活动会话。
- 联网的 `deploy-go-agent` 继续以低权限用户运行；root shell 只能由不联网的 `deploy-go-agent-executor` 通过受限 Unix Socket 创建。

PTY open 消息只能携带会话 ID、会话序号和终端行列。主控不得下发 shell 二进制路径、启动用户、任意环境变量集合、工作目录、远程地址或后台命令；shell 和各类运行限额由 executor 本机受信配置固定选择。一个节点首版最多一个活动会话，并同时限制空闲时间、最长存活时间、输入速率、输出缓存和单帧大小。

v5 消息和方向固定为：主控到 Agent 的 `terminal_open`、`terminal_input`、`terminal_resize`、`terminal_close`，以及 Agent 到主控的 `terminal_opened`、`terminal_output`、`terminal_exited`。每个发送方向分别维护严格递增且不可重复的 `sequence`，避免并发输入与输出争用同一序号；主控方向以 `terminal_open.sequence=0` 开始，Agent 方向以 `terminal_opened.sequence=1` 开始。收到错误会话、该方向重复或非预期序号必须拒绝并关闭会话，输入不进入 durable journal，也不得重放。

`terminal_input` 和 `terminal_output` 的 `encoding` 当前只允许 `base64`。`terminal_input.data` 编码后单帧最多 16,384 字节，对应最多 12,288 字节原始输入；该上限为 Agent 到 executor 的 JSON 帧保留最坏情况余量。终端列数范围为 1-500，行数范围为 1-1,000。消息使用 `additionalProperties=false` / `deny_unknown_fields`，未知字段、未知编码、非法尺寸和超限帧必须在进入执行器前拒绝。

`terminal_close.reason` 使用严格枚举，覆盖管理员关闭、浏览器断开、授权撤销、空闲超时、最长存活超时和协议错误；`terminal_exited` 返回严格退出原因和可空退出码，覆盖进程退出、对端断开、输出超限及 executor 不可用。close 必须幂等。任一必要链路断开、身份或开关失效、超时、shell 退出时，系统必须最终清理进程组。首版不跨 API 或 Agent 重启恢复会话，也不静默创建替代 root shell。

主控只持久化操作者、节点、Agent、会话 ID、来源请求、开始/结束时间、退出原因、退出码和字节计数等元数据。命令正文、终端输出、token 和 Secret 正文不得进入数据库、审计、任务日志、tracing 或浏览器持久化存储。后续 Env、文件、systemd 和 Docker/Compose 能力必须新增严格枚举的结构化操作，不能通过解析终端输出实现平台功能。

## Git refs 查询

`git_refs_query` 由主控创建，用于让选定的在线 Build Agent 通过受限 `git ls-remote --heads` 返回分支列表。payload 必须包含：

- `refs_query_id`、`repository_url`、`timeout_seconds`。
- 可选 `git_credential_lease_id`：私有仓库通过任务绑定的一次性 secret lease 取得 SSH deploy key。

Agent 不得接受 URL 内嵌凭证；查询结果只返回分支名和完整 ref，不返回凭证、主机密钥或仓库内容。

## 两阶段部署任务

`deployment_prepare` 固定包含部署 ID、`source_policy=branch`、仓库 URL、40 位 commit SHA、任务独占 `checkout_dir`/`work_root`/`output_dir`、环境、发布版本、模块列表、Make target、可选 Git lease 和超时。v3 跨节点任务额外包含一次性 `artifact_upload.authorization_id`。Agent 必须检出不可变 commit 后再执行 `deploy-go-prepare`。

v2 同节点 `deployment_release` 继续使用 prepare 阶段的 `checkout_dir` 和 `artifact_dir`。v3 跨节点任务额外包含 `target_run_id`、artifact download lease、archive/manifest digest、仓库 URL和目标任务独立 Git credential lease。Target Agent 的固定执行器检出同一固化 commit 到任务隔离 checkout；业务 `deploy-go-release` target 仍不得自行拉代码、切换 ref 或获取其他发布物。

prepare 成功后，Build Agent 先创建确定性 archive，并发送 `artifact_prepared`，其中只包含任务绑定、manifest 元数据和摘要，不包含 token 或制品字节。主控验证当前连接、prepare task、authorization ID、deployment 和 manifest 后，事务创建 artifact 与 upload lease，并以 `artifact_upload_authorized` 返回 opaque lease ID；拒绝响应只返回稳定错误码。Build Agent 随后使用现有 access token 通过 HTTPS 上传。该握手解决 manifest 只能在 prepare 后确定的问题，同时保证 lease 在使用前已绑定真实 manifest digest。

模块列表必须使用稳定模块标识，不允许重复；路径字段必须是绝对路径并限制在任务允许根目录内。

## 进度事件

Agent 将业务脚本的 `DEPLOY_GO_EVENT` marker 解析、补全后以 `task_progress` 发送。`event` 必须包含主控补全的 `deploy_id`、`stage`、`timestamp`、`status`、`environment`、`release_version`，并按事件类型携带模块、步骤、失败阶段和恢复提示。普通 stdout/stderr 仍通过 `task_output` 发送，不得把未解析文本放进 `task_progress`。

`task_progress` 与 `task_output` 共用任务内单调 `sequence`。主控按任务和序号幂等持久化，重复消息不产生重复 SSE 日志。

## Secret lease

需要 Git 私钥时，Agent 向主控发送 `secret_lease_request`，携带任务 ID、lease ID、payload digest 和用途。主控校验任务绑定、连接 Agent、payload digest 和期限后返回 `secret_lease_response`；响应只在该连接上传递，包含短期过期时间和一次性私钥。

- Agent 只能为当前任务的 payload 换取私钥，私钥写入 `0600` 临时文件，任务结束、失败或恢复时立即清理。
- `task_dispatch` payload、journal、审计、日志和 `task_output` 不得包含私钥或 lease 内容。
- 主控端 secret lease 一次消费、短 TTL，并绑定 Agent、task 和 payload digest；过期、重复或用途不符必须返回稳定错误码。

## 应用 Env 同步

`env_sync` 只在协议 v4 中下发，payload 包含 `env_sync_id`、应用 Slug、`.env` 文件名、Env version、SHA-256 digest、`write`/`delete` 动作和 opaque `secret_lease_id`，不包含 Env 明文或主控指定的绝对路径。

Agent 使用当前 Bearer access token 通过 HTTPS 单次读取 `application_env` lease。主控校验 lease 与 Agent、同步事实、版本和期限绑定后即消费 lease，并以 `application/octet-stream` 和 `Cache-Control: no-store` 返回明文。WSS payload、Agent journal、任务结果、日志和审计只记录标识与摘要。网络中断或失败后的恢复必须创建新任务和新 lease，不得重放已消费 lease。

Agent 仅在 `DEPLOY_GO_AGENT_ENV_SYNC_ENABLED=true` 时执行同步，并在受控 `secrets_root/<application_slug>/<file_name>` 下通过 dirfd/no-follow、同目录临时文件、`0600` 权限、`fsync` 和原子 rename 写入；删除使用相同路径边界。父目录 symlink、hardlink、目录和非普通目标必须拒绝。`deployment_release.required_env` 为每个当前 Env 版本携带 `write`/`delete` 动作；Agent 在执行前分别复验 digest 或文件不存在，不匹配时返回 `env_gate_failed`，且不得启动业务 runner。

主控仅在目标节点当前 Env sync 全部为 `succeeded` 且实际版本匹配时创建 release；离线节点保持 pending，重连后只补发当前版本，旧 pending 版本收敛为 `superseded`。失败重试只重置未收敛节点，不重复同步已成功节点。

## v3/v4/v5 演进门禁

跨节点 artifact 必须提升到协议 v3，应用 Env 同步和 release Env 门禁必须提升到协议 v4，PTY 必须提升到协议 v5 并同时声明 `pty_terminal`；旧 Agent 不得接收或猜测这些字段。协议提升必须同时完成 Rust 类型、机器 Schema、双方 handler 和兼容测试，并满足：

- `deployment_prepare` 只新增 opaque authorization ID；prepare 后通过 `artifact_prepared` / `artifact_upload_authorized` 换取 upload lease。制品内容和 access token 不进入 payload、journal或日志。
- `deployment_release` 使用 target run ID、artifact download lease ID 和 digest，不接受任意下载 URL；Target Agent 下载复验后才能执行 release。
- `env_sync` 只包含应用 Slug、文件名、Env version、digest 和 application Env secret lease ID，不包含明文或主控指定的绝对路径。
- Artifact HTTPS 请求使用现有 Agent access token 认证；lease 绑定 Agent、deployment、target run、purpose、digest 和期限。upload lease 在 finalize 时原子消费，download lease 仅允许绑定目标在有效期内 Range 重试。
- 应用 Env 使用独立 `application_env` purpose；Agent 在受控 `secrets_root` 下逐段 no-follow 写入，拒绝 symlink、hardlink 和非普通文件。

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
