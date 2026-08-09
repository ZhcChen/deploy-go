# Agent 特权终端终审

## 结论

**No-Go，不得在正式节点启用。**

协议、API、Admin、安装器和 executor 的基础链路已经实现，默认开关保持关闭。主控 capability P1 已在 U10 关闭，每会话 cgroup v2 与有界 reader P1 已在 U11 的隔离 Linux 容器验证中关闭。真实 Linux systemd 安装、失败回滚和宿主节点完整链路尚未复核，因此整体结论仍为 No-Go。

> 2026-08-09 更新：产品语义已确认调整为完整 root 登录终端，PTY 子进程不再采用网络、设备和主机管理隔离。cgroup v2 只作为断线回收边界；完整 root 可以主动迁出 cgroup或修改 systemd，不能把它表述为 root 沙箱。

完整 root 可以绕过 mount namespace 中的 `InaccessiblePaths`、修改服务配置或检查主机进程，因此不能再把 Agent 凭证路径隐藏视为抵抗终端操作者的安全边界。启用节点必须按“终端操作者可以完全控制节点及其 Agent 身份”评估风险。

## 已验证

- Rust workspace tests 全部通过（本轮修复前基线）。
- `make privileged-terminal-check` 覆盖协议、PTY、Agent 桥接、API 授权/审计和 Admin 组件。
- Admin 15 个 Vitest 文件、92 项测试通过；Playwright 19 项通过，包含 1280px 与 390px SSH 门禁布局。
- v4/v5 Agent 仍可执行既有部署；终端仅在 v6 和 `pty_terminal` 能力存在时显示可用。
- 浏览器、API 和 Agent 输入帧已统一为 12 KiB 原始字节、16 KiB base64，并修复连接确认前输入导致的序号跳跃。
- 未附着的 `opening` 会话会在 30 秒租约过期后于下一次创建时收敛，不再永久占用节点。
- Agent close 队列投递失败时断开精确 connection generation，触发 Agent 侧 executor 清理。
- executor 增加正式 Agent 可执行文件、peer PID 绑定和 Agent Linux 不可转储加固；安装器使用真实协议 Probe，不再只检查 Socket 存在。
- 主控为每次 open 签发默认 15 秒的 Ed25519 capability，绑定节点、Agent、会话和连接代次；Agent 只透传，executor 在创建 PTY 前使用安装时下发的公钥离线验签。
- executor 使用 root 专用 `0700` 目录和原子 `create_new` 持久化 capability 消费标记；自动化测试覆盖篡改、过期、未来签发、错绑定、重复消费和 executor 重启后重放。

macOS 缺少 `systemd-analyze` 与 Bats，Linux systemd、`SO_PEERCRED`、`/proc` 和回滚验证必须在隔离 Linux 环境补做。当前 GitHub Actions workflow 保持禁用，不能把未运行的 CI 当作证据。

## 阻断项

### 已关闭 P1：executor 无法证明本次 Open 已获主控授权

`SO_PEERCRED`、Agent 可执行文件和 PID 绑定只能证明请求来自当前 Agent 进程。若正式 Agent 进程被攻破，它仍可绕过管理员 RBAC、节点 `privileged_execution` 开关和 API 审计直接请求 root PTY。

关闭证据：协议 v6 的 `TerminalOpen` 必须携带 capability；API 在 RBAC、节点开关、在线身份和连接代次校验后使用独立 Ed25519 私钥签发，executor 只持公钥并在 PTY 创建前验签。声明绑定 node/agent/session/connection generation，默认 TTL 15 秒、最大 30 秒；缺失、篡改、过期、未来签发、错绑定和跨重启重放均 fail closed。v5 Agent 保持部署兼容，但不能使用签名终端。

### 已关闭 P1：PTY 后代可脱离 session，清理没有强内核边界

当前 Linux `/proc` session 扫描与进程组 SIGKILL 能处理普通 shell 树和忽略 TERM 的后代，但进程仍可通过 `setsid`/double-fork 脱离原 session。无界 reader thread `join` 也可能被持续持有 PTY slave 的进程拖住。

关闭证据：Linux executor 在自身 systemd cgroup 下为每个终端创建独立 cgroup v2，内部 launcher 在执行登录 shell 前完成入组，失败即拒绝启动。关闭先 TERM，随后调用 `cgroup.kill` 并等待 `populated 0`；PTY FD 关闭后只有限等待 reader。清理或 child 回收失败会永久封锁当前 executor 进程的新会话，systemd 停止使用显式 `KillMode=control-group`。特权隔离容器测试覆盖 `setsid`、后台分叉、忽略 TERM、清理后立即创建下一会话，以及 root 主动迁出 cgroup 并持有 PTY 时 reader 不阻塞关闭。后一个场景同时证明 cgroup 不是完整 root 的安全沙箱。

## 其他发现

- `MIN_SUPPORTED_PROTOCOL_VERSION` 当前仍为 1，与计划中“保持 v4 下限”不一致。切断 v1-v3 会影响现存 Agent，需先盘点节点后单独决策；终端自身已严格要求 v6。
- stale opening 自动收敛目前缺少对应的 `terminal.session.finished` 审计，应在 capability/cgroup 实施前补齐。
- 输入已有累计上限和内存速率限制，但字节计数仍逐帧写 SQLite；后续可按短周期聚合，降低单字节洪泛的数据库争用。
- `cargo clippy --workspace --all-targets -- -D warnings` 被既有 `api/src/deployments/mod.rs` 8 参数告警阻断；该代码早于本功能，未混入本轮重构。
- Agent 当前把 executor 的 capability 无效、重放和存储故障统一收敛为 `protocol_error`；授权仍 fail closed，但后续应保留脱敏错误分类以改善排障。
- replay marker 当前跨重启永久保留，能够保证防重放，但长期会增长；后续应按已认证过期时间做有界清理，清理失败不得放宽授权。

## 后续执行单元

1. U12 本地实现已完成：业务 runner 与控制 Agent 身份拆分，增加 root runner broker、manifest v3、三 unit 原子安装与 fail-closed 生产入口；macOS 单元测试和静态契约已通过。
   runner 取消由 broker 启动降权 helper 执行；helper 以 runner UID 校验 `process.json` 和 PID start-time
   后发送信号，既修复 Agent 跨 UID 的 `EPERM`，也避免形成 root kill oracle。
2. 隔离 Linux 容器已证明允许身份和三组错误 UID/GID 组合的 peer 拒绝、child 实际 uid/gid、
   supplementary groups 清空、runner 无法读取 Agent 凭证或连接 executor，以及跨 UID 取消；测试同时
   发现并修复了 child 文件权限隐式依赖 systemd umask 的问题。仍需验证 Agent 恢复，并在 PID1
   systemd 环境完成首装、升级、runner 健康失败回滚和卸载顺序演练。
3. 当前共享 `deploy-go-runner` 身份不能隔离不同应用 Env 与并发任务 Git key；完成任务级 Secret 隔离前仍为正式环境阻断项。
4. 完成真实 Linux systemd 全链路、停用和旧 Agent 兼容演练。

完成真实 Linux systemd 验证并重新执行高风险复核前，管理端开关只能用于隔离开发环境，不得进入正式灰度。
