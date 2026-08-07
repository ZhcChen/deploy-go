---
date: 2026-08-07
topic: privileged-agent-executor
status: accepted
schema_version: 1
---

# 特权 Agent Executor 安全规范

## 目标与边界

Deploy Go 允许管理员通过节点 Agent 建立 root PTY，用于临时节点维护，并为后续文件、Env、systemd 和 Docker/Compose 结构化能力提供统一特权底座。管理端入口可以命名为“SSH”，但底层不实现 SSH 协议、不开放 SSH 端口、不使用或保存用户 SSH 私钥。

特权链路固定为：

```text
管理员浏览器 -> Deploy Go API -> Agent WSS -> 本机 Unix Socket -> root executor -> PTY
```

联网的 `deploy-go-agent` 必须继续以低权限用户运行。`deploy-go-agent-executor` 是唯一 root 进程，只接受本机 Agent 的版本化协议，不读取主控凭证、不建立网络连接，也不接受远程客户端。普通部署继续遵守 `docs/standards/application-deployment-contract.md` 和 `docs/standards/privileged-release-launcher.md`。

## 授权与默认门禁

- 终端只能由管理员创建、附着、输入、调整尺寸和关闭；HTTP 与 WebSocket 入口均须独立执行管理员 RBAC 校验。
- 每个节点的 `privileged_execution` 默认关闭，只能由管理员显式启用。关闭开关或撤销节点身份时，必须拒绝新会话并终止活动会话。
- 只有节点在线、身份有效、协议至少为 v5、Agent 声明 `pty_terminal` 且 executor 健康兼容时才能创建会话；任何事实未知时 fail closed。
- 一个节点首版最多一个活动终端会话。数据库约束和运行时 registry 必须共同阻止并发绕过。
- 普通用户不能通过前端深链、HTTP、WebSocket 或复用已有会话获得终端能力；权限降低后现有连接必须关闭。

## 进程与 Socket 隔离

- executor 以 root systemd 服务运行；Agent、部署 runner 和业务脚本仍以 `deploy-go-agent` 用户运行。
- executor 监听固定 Unix Socket，不允许配置 TCP、UDP 或其他远程监听地址。Socket 目录由 root 管理，组仅包含专用 Agent 身份，目录与 Socket 权限不得允许其他用户写入。Linux 上还必须核对 `SO_PEERCRED` PID 对应的 root 管理 Agent 可执行文件，并绑定当前 Agent PID；该校验属于纵深防御，不能替代主控 capability。
- executor 必须使用 peer credentials 校验对端 uid/gid，并拒绝仅凭消息字段声明的身份。请求不得携带 Agent token、refresh token、Git/Env secret lease 或其他主控凭证。
- executor 的运行环境使用最小 systemd 权限和明确文件系统边界；不依赖外网解析、HTTP/WSS 客户端或云凭证。
- Agent 不能直接读取 root 文件或创建 root 进程；所有特权操作必须经过 executor 的版本化、严格枚举协议。

## PTY 会话契约

首期 executor 仅开放 PTY 的 `open`、`input`、`resize`、`close` 和退出/输出事件，不开放“后台执行任意命令”的普通 RPC。open 请求不得指定 shell 路径、运行用户、任意环境变量、远程地址或允许根之外的工作目录。

shell 由 executor 本机配置固定选择，优先使用 root 登录 shell，缺失时回退 `/bin/sh`。每个会话必须具有不可预测 ID、明确所有者、创建时间、空闲超时和最长存活时间，并限制输入速率、输出缓存、单帧大小及终端行列范围。

输入、输出和 resize 使用会话级单调序号和有界缓冲。重复、乱序、未知字段、非法尺寸、超限帧和错误方向消息必须拒绝。慢消费者或输出洪泛不能阻塞 Agent 心跳、token 刷新或普通部署任务；超过预算时关闭当前会话。

浏览器入口遵守 `docs/standards/api-contract.md` 的“浏览器终端 WebSocket”契约。CSRF 通过 `Sec-WebSocket-Protocol` 中的 `csrf.<token>` 传递，不进入 URL；API 返回固定子协议 `deploy-go-terminal.v1`。registry 必须同时绑定 session ID、单一浏览器附着、Agent ID 和连接代次，旧代次响应不得注入新会话。

浏览器、API、Agent、Unix Socket 任一必要链路断开，或 shell/executor 退出时，必须进入有限清理窗口并最终终止整个 PTY 进程组。先发送正常终止信号，超时后强制结束。close 必须幂等，首版不跨 API/Agent 重启恢复、不重放输入，也不自动创建替代 shell。

## 数据最小化与审计

主控持久化以下会话元数据和最终状态：操作者、节点、Agent、会话 ID、来源请求、开始/结束时间、退出原因、退出码、输入/输出字节计数。审计必须能关联管理员请求和节点会话，但不得记录：

- 命令或按键正文。
- PTY 输出正文。
- token、私钥、Env、连接串或其他 Secret 正文。
- 可还原上述正文的调试 payload。

终端正文只允许存在于实时、有界内存缓冲，不写数据库、普通日志、tracing、任务 journal 或 durable task 输出。浏览器不得写入 `localStorage`、`sessionStorage`、`IndexedDB`、历史记录参数或遥测。错误只返回稳定、脱敏的状态码和必要元数据。

## 安装、升级与恢复

- Agent 与 executor 必须作为同版本兼容配对产物发布，manifest 包含版本、架构和 checksum；安装器校验通过后才能替换。
- 幂等安装器负责专用用户/组、二进制、executor/Agent systemd unit、Socket 权限和本机配置。executor 先启动，Agent 后启动；停止顺序相反。
- 配对 manifest、原子替换、失败恢复和卸载的数据保留边界遵守 `docs/standards/agent-installation-contract.md`。executor 当前自行创建 Unix Socket，不使用 systemd socket activation。
- 安装或升级失败必须恢复上一对可用二进制和 unit。executor 不健康时 Agent 应保持在线并继续已有部署能力，但不能声明 `pty_terminal`。
- 卸载、身份撤销和回滚必须先禁用新会话、停止并清理全部 PTY，再移除 executor 或恢复旧 Agent。
- 安装 executor 不自动开启数据库中的节点 `privileged_execution`，管理员需在确认能力健康后单独启用。

## 后续结构化能力

Env 首次导入、文件读写、systemd 和 Docker/Compose 管理应在 executor 上增加独立的结构化 operation：明确字段白名单、路径允许根、大小与超时、授权、幂等、审计和恢复语义。Web 不得通过终端发送命令并解析输出，主控也不得把任意 shell 字符串包装成普通任务。

通用 root PTY 只用于管理员维护，不替代应用 Make target、制品校验、Env 门禁、部署事件、重试和回滚。现有应用专属 launcher 暂时保留；任何迁移或删除必须单独评审并提供兼容回退。

## 实施检查

- 非管理员、开关关闭、节点离线、身份撤销、协议 v4、executor 缺失或版本不兼容均无法建立会话。
- 非授权 uid/gid 无法连接 Socket，executor 无远程监听且不读取 Agent 凭证。
- 输入输出、resize、`Ctrl+C`、主动关闭、空闲/最长超时及各链路断开均能终结 PTY，不残留 root 进程。
- 输出洪泛和慢消费者受硬上限约束，不影响心跳和部署任务。
- 数据库、审计、Agent 日志和浏览器存储中不存在终端正文或 Secret fixture。
- v4 Agent 和未启用 executor 的节点仍可执行原有部署任务；launcher 行为与 sudoers 不发生隐式变化。
