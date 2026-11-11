# Agent 节点接入

## 适用范围

本手册用于通过一次性安装命令接入 Linux 节点。对真实节点运行安装命令、修改权限或重启服务前，必须在当前对话中获得针对具体节点和动作的明确授权；本地 fixture 验证不构成真实节点授权。

## 前置条件

- API 已配置可信的 `DEPLOY_GO_PUBLIC_BASE_URL`，且 `/readyz` 返回 `200`。
- 部署端已同步当前 API 版本的配对 release，包含 Linux `x86_64` 的 Agent/executor/updater、四个 systemd unit、executor 配置模板和 SHA-256。正式部署由 `deploy/production/deploy.sh` 在 qfy-test2 构建并上传。
- 节点能通过 HTTPS 访问主控的 `/api/v1/agent/install`、`/api/v1/agent/download/{version}/...`，并能通过 WSS 访问 `/api/v1/agent/control`。
- 节点管理员可使用 root 执行安装器。联网 Agent 使用 `deploy-go-agent`，业务脚本使用 `deploy-go-runner`；root runner broker 只按固定 spec 降权启动业务 child，独立 root executor 只提供签名 PTY、结构化特权 release 和无参数内置 self-test。
- 节点预装 `curl`、Python 3、systemd，以及 `sha256sum` 或 `shasum`。安装器不依赖 `jq`。
- 节点必须由 systemd 托管，并让 `/sys/fs/cgroup/cgroup.controllers` 暴露非空控制器；否则安装器会在 executor 激活阶段以 `cgroup_v2_missing` 失败并整对回滚。sysfs 伪文件 `stat` size 为 0，安装器按文件内容判断，不能以 `test -s` 手工复核。
- WSL 2 节点需在 `/etc/wsl.conf` 写入 `[boot] systemd=true`，Windows 侧执行 `wsl --shutdown`，重进 WSL 后确认 `/proc/1/comm` 为 `systemd` 且控制器文件非空。

## 接入步骤

1. 唯一管理员在 Web 的 Agent 页面创建 Agent，只填写 Agent 名称和环境；主控在同一事务中创建一对一绑定的节点和离线 Agent。接管升级前已有的 legacy 节点时，从该节点详情页执行“接管此节点”，同样只填写名称和环境；API 会保留原 node、deployment target 和部署历史 ID。
2. 复制安装命令。命令已动态拼接短期 enrollment token（默认 30 分钟有效、一次性消费），不需要再单独复制或粘贴 token。命令含 token，不得写入工单、普通日志、shell history、聊天记录或仓库，Web 和客户端不持久化该命令。
3. 在已明确授权的目标 Linux 节点直接执行命令。安装器会校验 OS、架构、v4 配对 manifest、Agent/executor/updater 三个 amd64 二进制、四个 systemd unit 和 executor 配置模板。
4. 安装器创建 `deploy-go-agent` 与 `deploy-go-runner` 用户和专用组，并准备以下目录：
   - `/var/lib/deploy-go-agent`：`0750 deploy-go-agent:deploy-go-runner`；其中 `credentials.json` 保持 `0600 deploy-go-agent:deploy-go-agent`，共享组不可写。
   - `/var/lib/deploy-go-agent/tasks`：`3710 deploy-go-agent:deploy-go-runner`，由 Agent 与 root runner broker 交换任务，业务 child 只获得当前任务目录。
   - `/var/lib/deploy-go-agent/apps`：`2770 deploy-go-agent:deploy-go-runner`，默认 `work_root`；安装器会按原 owner 权限同步 group 权限。
   - `/var/lib/deploy-go-agent/secrets`：目录 `2700`（临时环境可退化为 `0700`）、文件 `0600`，均为 `deploy-go-agent:deploy-go-agent`，默认 `secrets_root`。
   - `/etc/deploy-go-agent/config`：包含控制通道、数据目录、Env 同步与制品传输开关，以及任务/部署工作目录的保留期和定时回收周期，不包含 token。
   - `/etc/deploy-go-agent/executor.json`：`0600 root:root`，保存允许连接 Socket 的 Agent uid/gid、固定 Agent 可执行文件、两类授权公钥、release jobs 目录与资源策略，以及从系统账号数据库解析的 root home 和登录 shell；不保存任何签名私钥。
   - `/run/deploy-go-agent/executor.sock`：executor 自建 Socket，目录为 `0750 root:deploy-go-agent`，Socket 为 `0660 root:deploy-go-agent`；不安装 systemd `.socket` unit。
5. installer 先启动 executor、runner broker 和 updater，确认两个 Socket、executor v4 的 PTY、`DeploymentRelease`、`AgentUpgrade` capability，再启动 Agent。v11 及以上 Agent 的 PTY 与 release 是标准配对能力，不存在节点 `privileged_execution` 或目标级 `privileged_release` 开关。安装器会同时输出 `status` 与 `doctor` 命令，命令不包含 token。
6. 把应用自有脚本和所需 secret 文件放入对应根目录，并确保 `deploy-go-runner` 可读/执行。普通业务部署仍走标准脚本；需要 root 发布时固定使用 executor，不能通过 root 终端替代。
7. 在 Web 等待同一 Agent/节点变为在线，核对 hostname、架构、版本、协商协议和 `pty_terminal` 能力，再从节点详情执行 `SystemInspect`。协商到 v12 或更高版本后节点详情应进入 `supported`，首个速率样本允许显示 `warming_up`；v11 仍可部署，但显示需要升级 Agent 才能提供遥测。
8. 只有检查确认工作目录、secret 目录和磁盘可用后，才把该节点用于部署目标；管理员需要终端时，确认 Agent 在线、身份有效、协商版本不低于 v11 且 `pty_terminal` 健康后直接从“SSH”页连接。

## 验证

```bash
sudo -u deploy-go-agent /usr/local/bin/deploy-go-agent status
sudo -u deploy-go-agent /usr/local/bin/deploy-go-agent doctor
systemctl is-active deploy-go-agent
systemctl is-active deploy-go-agent-runner
systemctl is-active deploy-go-agent-executor
systemctl show deploy-go-agent -p User -p Group -p NoNewPrivileges
systemctl show deploy-go-agent-runner -p User -p Group -p NoNewPrivileges
systemctl show deploy-go-agent-executor -p User -p Group -p NoNewPrivileges
journalctl -u deploy-go-agent -u deploy-go-agent-runner -u deploy-go-agent-executor --since '10 minutes ago' --no-pager
stat -c '%a %U:%G %n' \
  /var/lib/deploy-go-agent \
  /var/lib/deploy-go-agent/tasks \
  /var/lib/deploy-go-agent/apps \
  /var/lib/deploy-go-agent/secrets \
  /run/deploy-go-agent \
  /run/deploy-go-agent/executor.sock
```

`status` 输出版本、协议、Agent ID、配置和凭证等本机静态事实；`doctor` 继续检查 systemd、匿名 HTTPS `/readyz`、runner 与 executor。`doctor` 返回 `2` 表示存在决定性 `FAIL`，返回 `0` 只表示本机和 HTTPS 前置检查没有决定性失败，不证明 WSS upgrade、Agent 鉴权或心跳成功。预期三个服务均为 `active`；Agent 为 `deploy-go-agent`，业务 child 为 `deploy-go-runner`，broker/executor 服务为 root。`InaccessiblePaths` 只降低误读 Agent 凭证的概率，不能防御完整 root。目录和 Socket 权限符合上述约束，日志不得出现 enrollment、access 或 refresh token。

节点的 online/offline 与 telemetry 的 fresh/stale/empty 是独立状态。首个差分样本预热、无 NVIDIA 硬件、GPU 驱动不可用、权限不足或超时均不得导致 Agent 离线。工作盘必须对应配置 `data_dir/apps`（默认 `/var/lib/deploy-go-agent/apps`），不能把 `/var/lib/deploy-go-agent/tasks` 当作工作根目录。`status`、`doctor` 和日志不得输出 GPU 原始命令结果、设备 UUID 或完整挂载路径。

Agent unit 保留 `RestrictSUIDSGID=true`。任务目录若已由 setgid 父目录继承为 `3700/3770`，Agent 不得再次 `chmod`，否则 systemd 会以 EPERM 拒绝并产生 `invalid_task`；含目录权限守卫的 Agent 版本会在权限已合法时跳过重复 `chmod`。

## 重跑与升级

- 本地 `credentials.json` 中 Agent ID 相同且凭证有效时，重跑安装器不重新 enrollment；安装器完整校验配对发布物后原子替换并按 executor/runner broker -> Agent 顺序重启。
- 本地 Agent ID 与命令不同时，安装器拒绝覆盖。
- Agent 已撤销时，管理员重新生成带 rebind 标记的一次性命令；安装器使用新 enrollment token 替换长期凭证。
- executor、Socket 或 Agent 健康检查失败时，安装器恢复上一对二进制、unit、配置和启用状态。旧环境只有 Agent 时也会恢复原 Agent，但协议低于 v11 的实例会被控制面拒绝，必须完成 v11 配对安装后才能恢复部署。
- 卸载前先在主控撤销 Agent，再经明确授权运行 `install.sh --uninstall`。卸载会依次停止 Agent、runner broker 和 executor，并保留凭证、任务和应用数据供人工确认。

## 同节点并发部署

默认一个节点（Agent）同一时刻只运行一个部署，其余排队。多个项目共同开发的**测试或开发节点**可以开启并发，让不同应用的部署同时运行。需要两处同时配置，缺一都不会并发：

| 位置 | 设置 | 默认 | 作用 |
| --- | --- | --- | --- |
| 节点本机 `/etc/deploy-go-agent/config` | `DEPLOY_GO_RUNNER_CONCURRENT=true` | 关闭 | 允许本机 runner broker 同时运行多个 runner（上限 16），并让 Agent 声明 `concurrent_deployments_v1` |
| Web“节点”详情页 | 同节点并发部署数（1 至 16） | 1 | 控制面允许同时派发给该节点的部署数；测试与开发环境的存量节点在升级到 0.3.35 时被设为 10 |

另外，全平台总并发（Web“系统设置 → 最大并发部署数”，默认 2）是总上限，必须不小于你想要的并发数，否则会被它压回去。

**隔离取舍，开启前必读**：所有 runner 共用同一个运行身份。开启并发后，多个应用的准备阶段同时运行，它们各自的任务目录（含该任务落盘的 Git 部署密钥与 Env 副本）会同时对所有 runner 进程可读，一个应用的准备脚本理论上可以读到另一个同时运行的应用的这些内容。发布阶段由 root executor 执行，本来就能读取节点上的全部内容，不因并发改变。因此**只在受信任的测试或开发节点开启，生产与预发布节点保持默认**。详见 [Agent 安装契约](../standards/agent-installation-contract.md)。

开启（需要对该节点的运行态操作授权）：

1. 在节点上向 `/etc/deploy-go-agent/config` 追加一行 `DEPLOY_GO_RUNNER_CONCURRENT=true`（值只能是 `true` 或 `false`，其他取值会使 broker 拒绝启动）。
2. **先**重启 `deploy-go-agent-runner`，**再**重启 `deploy-go-agent`。重启期间正在运行的任务由两者各自的恢复逻辑接管，建议在没有活动部署时操作。必须两个都重启：Agent 只在启动时查询一次 broker 的上限。
3. 在 Web 节点页确认“本机并发”显示已开启，并把并发数设为需要的值。

验证：节点页显示本机并发已开启；同时对该节点上不同应用发起多个部署，它们同时进入运行，而不是排队；`journalctl -u deploy-go-agent-runner` 里有 `runner_broker_capacity` 且 `max_concurrent_tasks=16`，`journalctl -u deploy-go-agent` 里有 `concurrent_deployments_advertised`。

回退（任选其一，立即生效到“一次一个”）：

- Web 把该节点的并发数设回 1（不需要重启，控制面不再派发多个）；或
- 在节点上把 `DEPLOY_GO_RUNNER_CONCURRENT` 改回 `false` 或删除该行，然后按同样顺序重启 runner 与 Agent。

排障：

- 并发数设了大于 1 但仍在排队：先看节点页是否显示本机并发已开启（Agent 是否声明了能力）；再看系统设置的全平台总并发是否过小；同一应用的同一部署目标本来就只能一个一个运行。
- 部署失败并出现 `runner_busy`：控制面与 broker 的上限不一致，通常是改了本机开关却只重启了其中一个服务。按上面的顺序重启两者。
- 升级后某个节点没有并发能力：旧版本 Agent 没有该能力，等待节点自动升级到 0.3.35 或更高。

## 本地存储回收

Agent 执行节点默认只保留受管任务与部署工作目录到保留期，并每小时做一次全量扫描；进程启动后
首次扫描立即执行，不等待第一个间隔。Agent 把终态结果写入 journal 后，会在后台立即回收该任务
引用的 `checkout`/`staging`/`artifact.tar`，journal 仍保留用于断线后的结果重放与诊断。
同一部署仍存在其他活跃任务时，回收会暂缓，避免复用工作目录的任务被误删。

| 配置 | 默认 | 说明 |
| --- | --- | --- |
| `DEPLOY_GO_AGENT_TASK_RETENTION_SECONDS` | `604800`（7 天） | 终态任务 journal 与任务目录保留期；达到保留期后清理整个任务目录 |
| `DEPLOY_GO_AGENT_DEPLOYMENT_RETENTION_SECONDS` | `2592000`（30 天） | `apps/deployments/<deployment_id>` 工作目录保留期；prepare 成功但尚未上传/发布完成的 staging 在保留期内不删除 |
| `DEPLOY_GO_AGENT_STORAGE_CLEANUP_INTERVAL_SECONDS` | `3600`（1 小时） | 定时清理周期；0 或非法值会回退默认值 |

三个变量由安装器写入 `/etc/deploy-go-agent/config`。旧版本升级后若配置缺失，Agent 也会使用
与上表相同的代码默认值。清理器只操作 `data_dir` 下普通相对路径，拒绝符号链接祖先和越界路径；
不清理 `credentials.json`、`secrets/`，也不替代 root executor 对
`/var/lib/deploy-go-agent-executor/release-jobs` 的 200 GiB/1 天资源策略。

需要检查占用时可先执行只读统计：

```bash
du -sh /var/lib/deploy-go-agent/tasks /var/lib/deploy-go-agent/apps/deployments
```

不要手工删除未确认的 journal 或部署目录；先对照部署记录确认对应部署已终态，再等待定时回收或
按故障恢复章节处理。

## 本地验证

```bash
make agent-install-check
make agent-manifest-check
make agent-runner-isolation-check
cargo test -p deploy-go-api --test agents -- agent_enrollment:: agent_end_to_end::
curl --fail --silent \
  https://deploy.example.com/api/v1/agent/download/<版本>/manifest.json
curl --fail --silent --output /dev/null \
  https://deploy.example.com/api/v1/agent/download/<版本>/agent/x86_64
cargo test -p deploy-go-agent --test config --test telemetry
```

上面下载地址中的 `<版本>` 用要验证的 Agent 版本替换，并把点号换成下划线（例如 0.3.33 写作 `0_3_33`）。

`make agent-runner-isolation-check` 使用本机已有的 `rust:1.99-bookworm` 镜像和预热后的
`$HOME/.cargo/registry` 离线执行，不会自动拉取镜像；Cargo target 缓存按 Docker 架构保存在
`deploy-go-runner-target-<arch>-<缓存代号>` volume（代号是脚本里的缓存命名，不等于当前 Rust 工具链版本，见 `agent/tests/run-runner-identity-container.sh`），需要释放空间时可手动删除该 volume。

这些命令只使用隔离 fixture 或示例地址，不连接真实节点。
