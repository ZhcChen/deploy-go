---
date: 2026-08-07
topic: privileged-agent-executor
status: accepted
schema_version: 1
---

# 特权 Agent Executor 安全规范

## 目标与边界

Deploy Go executor 提供彼此隔离的 root operation：管理员临时维护使用的 PTY，以及部署状态机使用的结构化特权 release。管理端终端入口可以命名为“SSH”，但底层不实现 SSH 协议、不开放 SSH 端口、不使用或保存用户 SSH 私钥；结构化 release 不创建 PTY，也不接受任意命令。

特权链路固定为：

```text
管理员浏览器 -> Deploy Go API -> Agent WSS -> 本机 Unix Socket -> root executor -> PTY
部署状态机 -> Deploy Go API -> Agent WSS -> 本机 Unix Socket -> root executor -> 固定 release job
应用详情状态 -> Deploy Go API -> Agent WSS -> 本机 Unix Socket -> root executor -> 固定只读状态查询
```

联网的 `deploy-go-agent` 必须继续以低权限用户运行。`deploy-go-agent-executor` 是唯一常驻 root 服务，只接受本机 Agent 的版本化协议，不主动读取主控凭证，也不接受远程客户端。v11 Agent 将完整 root PTY 和固定 release job 作为标准配对能力；不存在节点 `privileged_execution` 或目标级 `privileged_release` 开关。普通部署与 launcher 历史兼容遵守 `docs/standards/application-deployment-contract.md` 和 `docs/standards/privileged-release-launcher.md`。

executor unit 继续用 `InaccessiblePaths` 隐藏 Agent 凭证路径，以降低终端中的意外读取，但这不是对完整 root 的安全边界。完整 root 可以通过主机管理能力修改 unit、进入其他 mount namespace 或检查进程，因此必须假设获准终端操作者最终能够控制整台节点并接触节点上的 Agent 身份材料。

## 授权与默认门禁

- 终端只能由管理员创建、附着、输入、调整尺寸和关闭；HTTP 与 WebSocket 入口均须独立执行管理员 RBAC 校验。
- 只有节点在线、身份有效、协议为 v11、Agent 声明 `pty_terminal` 且 executor 健康兼容时才能创建会话；任何事实未知时 fail closed。v10 及更早 Agent 不得连接控制面或执行部署任务。
- 每次 open 都必须携带 API 使用 Ed25519 私钥签发的短期单次 capability。声明绑定 `node_id`、`agent_id`、`session_id`、`connection_generation`，默认 TTL 为 15 秒且不得超过 30 秒；Agent 只能透传，不能签发或修改声明。
- executor 只持有安装时下发的 Ed25519 公钥，并在创建 PTY 前离线验签。缺失、过期、未来签发、签名错误、错绑定或重复消费均须 fail closed。
- 一个节点首版最多一个活动终端会话。数据库约束和运行时 registry 必须共同阻止并发绕过。
- 普通用户不能通过前端深链、HTTP、WebSocket 或复用已有会话获得终端能力；权限降低后现有连接必须关闭。

## 进程与 Socket 隔离

- executor 与 runner broker 以 root systemd 服务运行；联网 Agent 使用 `deploy-go-agent`，业务部署 child 使用独立的 `deploy-go-runner`。runner broker 只能按固定任务 spec 启动降权 child，不接受任意命令；业务 runner 不得读取 Agent 凭证或连接 executor Socket。
- executor 监听固定 Unix Socket，不允许配置 TCP、UDP 或其他远程监听地址。Socket 目录由 root 管理，组仅包含专用 Agent 身份，目录与 Socket 权限不得允许其他用户写入。Linux 上还必须核对 `SO_PEERCRED` PID 对应的 root 管理 Agent 可执行文件，并按连接生命周期绑定当前 Agent PID；连接关闭即释放绑定，使后续一次性 doctor/probe/self-test 进程可连接，同时仍拒绝另一个存活 PID 并发连接。该校验属于纵深防御，不能替代主控 capability。
- executor 必须使用 peer credentials 校验对端 uid/gid，并拒绝仅凭消息字段声明的身份。请求不得携带 Agent token、refresh token、Git/Env secret lease 或其他主控凭证。
- capability 验签成功后，executor 必须在 root 专用目录中以 capability 摘要为文件名，通过 `create_new` 原子写入消费标记并刷盘。目录必须非符号链接、归 executor 进程所有且权限为 `0700`；消费标记跨 executor 重启保留，存储异常时拒绝创建 PTY。
- executor 的运行环境使用最小 systemd 权限和明确文件系统边界；不依赖外网解析、HTTP/WSS 客户端或云凭证。
- Agent 不能直接读取 root 文件或创建 root 进程；所有特权操作必须经过 executor 的版本化、严格枚举协议。

## cgroup v2 与关闭边界

- Linux executor 的 systemd unit 必须设置 `Delegate=yes` 和 `KillMode=control-group`，并运行在统一 cgroup v2 层级。每个 PTY 会话在 executor 当前 cgroup 下使用独立子 cgroup；不能创建子 cgroup、缺少 `cgroup.kill` 或 child 无法在执行登录 shell 前写入 `cgroup.procs` 时，必须拒绝启动会话。
- 关闭先向 PTY 前台进程组发送 TERM；宽限期后必须写 `cgroup.kill`，等待 `cgroup.events` 返回 `populated 0`，再删除会话 cgroup。`setsid`、double-fork 和忽略 TERM 不能逃过该回收路径。
- 关闭 PTY FD 后只允许有界等待 reader thread。持续持有 PTY slave 的进程不能让 executor 的连接任务、`SessionClaim` 或后续会话永久阻塞；超时后必须分离 reader 并继续收敛会话。
- `cgroup.kill`、清空等待、目录删除或直接 child 回收失败时，executor 进程必须永久封锁新会话，直到服务重启并重新完成运行前检查；不得把清理错误降级为成功后释放会话门禁。
- cgroup 是断线和异常关闭的资源回收边界，不是对完整 root 操作者的沙箱。已获授权的 root 可以主动迁出 cgroup、修改 systemd 或停止 executor；产品和审计不得宣称能够限制恶意 root 会话。

## PTY 会话契约

首期 executor 仅开放 PTY 的 `open`、`input`、`resize`、`close` 和退出/输出事件，不开放“后台执行任意命令”的普通 RPC。open 请求不得指定 shell 路径、运行用户、任意环境变量、远程地址或允许根之外的工作目录。

安装器必须从目标机系统账号数据库读取 uid 0 的 home 和登录 shell；executor 清空服务环境后重建 `HOME`、`USER`、`LOGNAME`、`SHELL`、基准 `PATH` 和 `TERM`，从 root home 启动 login shell。root profile 可以按目标机配置继续调整环境。每个会话必须具有不可预测 ID、明确所有者、创建时间、空闲超时和最长存活时间，并限制输入速率、输出缓存、单帧大小及终端行列范围。

输入、输出和 resize 使用会话级单调序号和有界缓冲。重复、乱序、未知字段、非法尺寸、超限帧和错误方向消息必须拒绝。慢消费者或输出洪泛不能阻塞 Agent 心跳、token 刷新或普通部署任务；超过预算时关闭当前会话。

浏览器入口遵守 `docs/standards/api-contract.md` 的“浏览器终端 WebSocket”契约。CSRF 通过 `Sec-WebSocket-Protocol` 中的 `csrf.<token>` 传递，不进入 URL；API 返回固定子协议 `deploy-go-terminal.v1`。registry 必须同时绑定 session ID、单一浏览器附着、Agent ID 和连接代次，旧代次响应不得注入新会话。

浏览器、API、Agent、Unix Socket 任一必要链路断开，或 shell/executor 退出时，必须进入有限清理窗口并最终终止整个 PTY 进程组。先发送正常终止信号，超时后强制结束。close 必须幂等，首版不跨 API/Agent 重启恢复、不重放输入，也不自动创建替代 shell。

## 结构化特权 Release 契约

- executor 本机协议 v2 新增非 PTY durable release job；release 请求不复用
  terminal message、session、capability 或 replay namespace。
- API 为 release 阶段任务固定签发 release 专属 Ed25519 授权，不再从目标 snapshot 读取 `privileged_release` 开关。claims 使用独立 audience，绑定 deployment、target run、节点、Agent、snapshot、完整 commit、环境、release version、modules、输入摘要、payload digest、deadline 和 nonce。executor 离线验签，并把 nonce 消费与 job 创建原子持久化。
- Agent 完成 artifact digest、manifest、Env gate 和 commit admission 后，executor 从安全打开的源复制 checkout、artifact、manifest 与 Env，拒绝 symlink、hardlink 和非普通对象，复验签名 claims 中的摘要，再封存为 root-owned、低权限不可写 bundle。root child 不得从 Agent/runner 可写源执行。
- executor 内部固定绝对 `make` 路径和参数 `--no-print-directory deploy-go-release`，工作目录固定为 bundle checkout。请求不得携带 shell、command、executable、args、Make target 或任意环境变量 map。
- child 使用 `env_clear()`；除本机固定最小 `PATH` 外，只允许 `DEPLOY_ID`、`DEPLOY_ENVIRONMENT`、`DEPLOY_RELEASE_VERSION`、`DEPLOY_COMMIT_SHA`、`DEPLOY_MODULES`、`DEPLOY_TARGET`、`DEPLOY_ARTIFACT_DIR`、`DEPLOY_ENV_DIR`、`DEPLOY_CANCEL_FILE`。
- job 使用独立 `release-*` cgroup，取消/超时先 TERM 进程组，再以 `cgroup.kill` 收敛。cgroup 是正常任务资源回收边界，不是对获准 root 业务代码的安全沙箱。
- job 状态、payload digest 和分块日志由 root 专用目录有界持久化，支持 Agent 按 offset 重连；单 job/全局预算、低磁盘水位、保留期限和截断终态必须固定，不能因断线无限写盘。
- 在线 Agent 协议或 capability 不兼容时，主控不创建 task 并将 deployment 收敛为 failed；已选 executor 的任务不得自动转 runner 或 launcher。

release 固定特权等同于信任配置仓库和固定 ref 的写入者拥有目标节点 root 发布能力。完整 commit SHA 只证明执行对象不变，不证明代码可信；仓库、ref、节点变化后确认失效并必须重新授权。

## 数据最小化与审计

主控持久化以下会话元数据和最终状态：操作者、节点、Agent、会话 ID、来源请求、开始/结束时间、退出原因、退出码、输入/输出字节计数。审计必须能关联管理员请求和节点会话，但不得记录：

- 命令或按键正文。
- PTY 输出正文。
- token、私钥、Env、连接串或其他 Secret 正文。
- 可还原上述正文的调试 payload。

终端正文只允许存在于实时、有界内存缓冲，不写数据库、普通日志、tracing、任务 journal 或 durable task 输出。浏览器不得写入 `localStorage`、`sessionStorage`、`IndexedDB`、历史记录参数或遥测。错误只返回稳定、脱敏的状态码和必要元数据。

## 安装、升级与恢复

- Agent、runner broker 与 executor 必须作为同版本兼容配对产物发布，manifest 包含版本、架构和 checksum；安装器校验通过后才能替换。
- 幂等安装器负责两个专用用户/组、二进制、三个 systemd unit、Socket 权限和本机配置。executor 与 runner broker 先启动，Agent 后启动；停止顺序相反。
- 配对 manifest、原子替换、失败恢复和卸载的数据保留边界遵守 `docs/standards/agent-installation-contract.md`。executor 当前自行创建 Unix Socket，不使用 systemd socket activation。
- 安装或升级失败必须恢复上一对可用二进制和 unit。executor 不健康时 v11 Agent 不得声明 `pty_terminal` 或 `privileged_release`；控制面仅允许健康的 v11 Agent 执行任务。
- 卸载、身份撤销和回滚必须先禁用新会话、停止并清理全部 PTY，再移除 executor 或恢复旧 Agent。
- 安装完成后不需要额外节点开关；管理员仍只能在控制面确认 Agent 在线、身份有效且 `pty_terminal` 可用后建立终端会话。

## 后续结构化能力

Env 首次导入、文件读写、systemd 和 Docker/Compose 管理应在 executor 上增加独立的结构化 operation：明确字段白名单、路径允许根、大小与超时、授权、幂等、审计和恢复语义。Web 不得通过终端发送命令并解析输出，主控也不得把任意 shell 字符串包装成普通任务。结构化 release 只覆盖固定 Make target，不自动获得这些独立管理 API。

通用 root PTY 只用于管理员维护，不替代应用 Make target、制品校验、Env 门禁、部署事件、重试和回滚。现有应用专属 launcher 暂时保留；任何迁移或删除必须单独评审并提供兼容回退。

## 实施检查

- 非管理员、节点离线、身份撤销、协议低于 v11、executor 缺失或版本不兼容均无法建立会话。
- capability 的正常签发、篡改、过期、未来签发、错绑定、重复消费和 executor 重启后重放均有自动化验证。
- 非授权 uid/gid 无法连接 Socket，executor 无远程监听且不读取 Agent 凭证。
- 输入输出、resize、`Ctrl+C`、主动关闭、空闲/最长超时及各链路断开均能终结 PTY，不残留 root 进程。
- 隔离 Linux cgroup v2 测试覆盖 `setsid`、后台分叉、忽略 TERM、清理后复用会话，以及 root 主动迁出 cgroup 并持续持有 PTY 时 reader 仍有界退出。
- 输出洪泛和慢消费者受硬上限约束，不影响心跳和部署任务。
- 数据库、审计、Agent 日志和浏览器存储中不存在终端正文或 Secret fixture。
- v10 及更早 Agent 与 executor 缺失的节点不得连接控制面或执行部署任务；必须使用配对的 v11 Agent、runner broker 和 executor 重新安装。
- 平台不存在可修改的目标级 `privileged_release` 配置；缺失、篡改、过期、错绑定或重放的 release 授权，以及可变/越界输入、额外环境和任意命令字段，均在 spawn 前拒绝。
- release 成功、非零退出、超时、取消、Agent 断线恢复和 executor 重启均保持唯一终态，日志有界且不遗留正常任务 root 进程。
