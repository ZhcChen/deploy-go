# Agent 特权终端终审

## 结论

**No-Go，不得在正式节点启用。**

协议、API、Admin、安装器和 executor 的基础链路已经实现，默认开关保持关闭。终审发现两个架构级 P1 尚未关闭：主控授权不能被 executor 离线证明，以及脱离原 session 的 root 后代缺少每会话 cgroup 清理。当前加固不能替代这两个边界。

## 已验证

- Rust workspace tests 全部通过（本轮修复前基线）。
- `make privileged-terminal-check` 覆盖协议、PTY、Agent 桥接、API 授权/审计和 Admin 组件。
- Admin 15 个 Vitest 文件、92 项测试通过；Playwright 19 项通过，包含 1280px 与 390px SSH 门禁布局。
- v4 Agent 仍可执行既有部署；终端仅在 v5 和 `pty_terminal` 能力存在时显示可用。
- 浏览器、API 和 Agent 输入帧已统一为 12 KiB 原始字节、16 KiB base64，并修复连接确认前输入导致的序号跳跃。
- 未附着的 `opening` 会话会在 30 秒租约过期后于下一次创建时收敛，不再永久占用节点。
- Agent close 队列投递失败时断开精确 connection generation，触发 Agent 侧 executor 清理。
- executor 增加正式 Agent 可执行文件、peer PID 绑定和 Agent Linux 不可转储加固；安装器使用真实协议 Probe，不再只检查 Socket 存在。

macOS 缺少 `systemd-analyze` 与 Bats，Linux systemd、`SO_PEERCRED`、`/proc` 和回滚验证必须在隔离 Linux 环境补做。当前 GitHub Actions workflow 保持禁用，不能把未运行的 CI 当作证据。

## 阻断项

### P1：executor 无法证明本次 Open 已获主控授权

`SO_PEERCRED`、Agent 可执行文件和 PID 绑定只能证明请求来自当前 Agent 进程。若正式 Agent 进程被攻破，它仍可绕过管理员 RBAC、节点 `privileged_execution` 开关和 API 审计直接请求 root PTY。

关闭条件：主控签发短 TTL、单次使用、绑定 node/agent/session/connection generation 的 capability；executor 使用独立信任材料离线验签，并拒绝过期、重放、错节点和开关关闭后签发的 capability。`Open` 未携带有效 capability 时必须 fail closed。

### P1：PTY 后代可脱离 session，清理没有强内核边界

当前 Linux `/proc` session 扫描与进程组 SIGKILL 能处理普通 shell 树和忽略 TERM 的后代，但进程仍可通过 `setsid`/double-fork 脱离原 session。无界 reader thread `join` 也可能被持续持有 PTY slave 的进程拖住。

关闭条件：每个终端会话进入独立 cgroup v2/systemd transient scope；关闭时先 TERM、超时后 `cgroup.kill`，并以有界方式结束 reader。测试必须覆盖 `setsid`、double-fork、忽略 TERM、持有 PTY、浏览器断线和 executor 停止，确认无 root 后代且 SessionClaim 可释放。

## 其他发现

- `MIN_SUPPORTED_PROTOCOL_VERSION` 当前仍为 1，与计划中“保持 v4 下限”不一致。切断 v1-v3 会影响现存 Agent，需先盘点节点后单独决策；终端自身已严格要求 v5。
- stale opening 自动收敛目前缺少对应的 `terminal.session.finished` 审计，应在 capability/cgroup 实施前补齐。
- 输入已有累计上限和内存速率限制，但字节计数仍逐帧写 SQLite；后续可按短周期聚合，降低单字节洪泛的数据库争用。
- `cargo clippy --workspace --all-targets -- -D warnings` 被既有 `api/src/deployments/mod.rs` 8 参数告警阻断；该代码早于本功能，未混入本轮重构。

## 后续执行单元

1. U10：主控 capability 签发、executor 离线验签和重放防护。
2. U11：Linux 每会话 cgroup v2 生命周期与有界 reader 清理。
3. U12：业务 runner 与控制 Agent 身份拆分，迁移凭证和 Socket ACL，并证明业务脚本不能读取 Agent 凭证或连接 executor。
4. U13：隔离 Linux 全链路、安装失败回滚、停用和旧 Agent 兼容演练。

完成 U10-U13 并重新执行高风险复核前，管理端开关只能用于隔离开发环境，不得进入正式灰度。
