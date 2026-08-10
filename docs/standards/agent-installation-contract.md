---
date: 2026-08-07
topic: agent-installation-contract
status: accepted
schema_version: 1
---

# Agent 配对安装与发布契约

## 适用范围

本规范约束 `deploy-go-agent`、runner broker 与 `deploy-go-agent-executor` 的发布清单、节点安装、升级、回滚和卸载。终端授权、结构化特权 release 和 root 边界以 `docs/standards/privileged-agent-executor.md` 为准。

## 配对发布清单

新安装只接受 `agent/release/manifest.schema.json` 定义的 `schema_version: 3`：

- `agent_version` 与 `executor_version` 必须相同，并与 API 当前发布版本一致；Agent 控制协议 v7 与 executor 本机 release 协议 v2 必须成对兼容。
- `artifacts` 必须恰好包含 `agent`、`executor` 的 Linux `x86_64`、`aarch64` 四个二进制及各自 SHA-256。
- `systemd_units` 必须同时声明 Agent、runner broker 与 executor unit；`executor_config` 必须声明本机配置模板。
- 所有节点下载 URL 必须为 HTTPS。API 对外服务 manifest 时把 URL 重写到自身版本化下载路由。
- 安装器必须先完成 manifest 结构、版本、架构和所有 checksum 校验，再修改节点文件。

API 可以读取历史 `schema_version: 1` 和 `schema_version: 2` 发布目录，保证版本列表和旧 Agent 下载不因升级中断；历史 manifest 不能被新版安装器用于开启完整的三服务能力。

GitHub Actions release workflow 当前保持整体注释禁用，但模板必须能为每种架构成对构建 Agent/executor、配对归档、checksum 与 v3 manifest。正式部署当前通过 `deploy/production/deploy.sh` 在部署机本地构建同样的配对发布目录。

## 身份、进程与 Socket

- 安装器创建 `deploy-go-agent:deploy-go-agent` 与 `deploy-go-runner:deploy-go-runner` 两个专用系统身份。联网 Agent 使用前者；业务脚本使用后者；runner broker 与 executor 以 root 运行。
- runner broker 只接受本机 Agent 精确 uid/gid 的有界启动/取消请求，从固定任务根读取 Agent 拥有且不可链接替换的 spec，并在启动 child 前清空附加组、切换到 runner uid/gid。取消 helper 同样先降权为 runner 身份，再校验进程身份并发送信号；root broker 不得把 runner 可写的 PID 文件作为 root 信号授权依据。业务 runner 不能连接 executor Socket，也不能读取 `credentials.json`。
- runner broker 必须对所有 Agent 连接实施进程级全局串行门禁；一个业务 runner 活动期间拒绝其他任务启动，完成或取消并回收 child 后才释放租约。broker 重启必须从可信进程身份恢复单个活动租约，发现多个活动 runner 时 fail-closed；串行门禁不得由 Agent 进程内锁替代。
- Agent unit 通过 `SupplementaryGroups=deploy-go-runner` 读取 runner 产生的状态和日志，但主组保持 `deploy-go-agent`，executor 的 peer gid 门禁不变。journal、spec 和 launch marker 对 runner 只读。
- `/var/lib/deploy-go-agent/tasks` 为 `3710 deploy-go-agent:deploy-go-runner`，runner 只能穿越而不能枚举；非活动任务目录为 `3700`，broker 仅在任务活动期间切换为 `3770`，回收 child 后立即恢复。`apps` 为 `2770`；长期 Env 原件位于 Agent-only 的 `secrets` `2700/0600` 边界，release 前只把当前任务声明的 Env 复制到任务内 `env/` 并通过 `DEPLOY_ENV_DIR` 暴露，任务终止后清理。`credentials.json` 始终保持 `0600 deploy-go-agent:deploy-go-agent`。
- executor 配置模板只允许安装器替换 Agent 的数字 uid/gid、release authorization 公钥，并从目标机系统账号数据库写入 uid 0 的 home 与登录 shell；允许连接的可执行文件固定为 root 管理且不可组写/全局写的 `/usr/local/bin/deploy-go-agent`。主控和安装命令不能注入可执行文件、shell、环境变量、home 或 Socket 路径；API 私钥不得进入安装命令或节点。
- executor 自行创建 `/run/deploy-go-agent/executor.sock` 并设置目录 `0750 root:deploy-go-agent`、Socket `0660 root:deploy-go-agent`。当前不使用 systemd socket activation，不安装 `deploy-go-agent.socket`。
- executor unit 不设置网络、设备、临时目录、主机管理、架构或 umask 隔离，保证 PTY 子进程具备完整 root 登录能力；仍通过 `InaccessiblePaths` 降低意外读取 Agent 凭证的概率，但不得把它视为抵抗完整 root 的安全边界。Agent unit 以 `Wants` 和 `After` 软依赖 executor 与 runner broker。
- executor 与 runner broker 先启动、Agent 后启动；停止和卸载时 Agent 先停止，再停止 runner broker 与 executor。

## 原子升级与恢复

安装器把以下对象视为单一事务：两个二进制、三个 unit、Agent 非敏感配置和 executor 本机配置。

1. 下载并校验全部输入。
2. 保留当前对象及原启用状态。
3. 停止旧 Agent 与 runner，在静止点快照并迁移受管目录 owner/mode；原子替换成对文件并执行 `daemon-reload`。
4. 先验证 executor service 与 Socket，再验证 runner broker service、Socket mode/group，最后重启并验证 Agent service。
5. 任一步失败时停止新服务、恢复整对旧对象、受管目录权限和启用/运行状态，再按 executor/runner broker -> Agent 顺序恢复旧服务。

首次安装失败时不留下半套二进制或 unit。已有 Agent 升级失败时必须恢复原有低权限 Agent，使普通部署能力不依赖 executor 成功。安装完成不自动修改数据库中的 `nodes.privileged_execution` 或 deployment target 的 `privileged_release`。

## 卸载与数据保留

`install.sh --uninstall` 先停止 Agent，再停止 runner broker 与 executor，禁用并移除三个服务、两个二进制、executor 配置和运行时 Socket。卸载保留 `credentials.json`、任务 journal、应用工作目录和 secrets，避免未经确认删除业务状态；重新分配或报废节点前应先在主控撤销 Agent 身份。

## 验证门禁

- `make agent-install-check`：安装器语法、Bats（环境存在时）、unit 静态安全契约和 `systemd-analyze verify`（Linux 环境存在时）。
- `make agent-runner-isolation-check`：在隔离 Linux 容器以不同真实 UID/GID 验证 Socket peer、任务降权、取消和凭证/executor 拒绝边界。
- `make agent-manifest-check`：v3 manifest 生成、四个架构组件、三个 unit 和配置模板 checksum。
- `make agent-release-sync-check`：历史 GitHub Release 同步脚本仍按成对发布物执行原子替换。
- `make privileged-release-check`：协议 v7、executor v2、签名授权、bundle、环境白名单、生命周期、API/Web 和旧 release 兼容聚合检查。
- `bash deploy/production/test-install-contract.sh`：生产部署本地构建并安装配对发布目录，不在服务器依赖 `jq`。

真实节点安装、升级、卸载、重启或清理仍需当前对话针对具体节点的明确授权。
