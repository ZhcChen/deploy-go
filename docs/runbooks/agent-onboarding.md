# Agent 节点接入

## 适用范围

本手册用于通过一次性安装命令接入 Linux 节点。对真实节点运行安装命令、修改权限或重启服务前，必须在当前对话中获得针对具体节点和动作的明确授权；本地 fixture 验证不构成真实节点授权。

## 前置条件

- API 已配置可信的 `DEPLOY_GO_PUBLIC_BASE_URL`，且 `/readyz` 返回 `200`。
- 部署端已同步当前 API 版本的配对 release，包含 Linux `x86_64` 与 `aarch64` 的 Agent/executor、三个 systemd unit、executor 配置模板和 SHA-256。正式部署由 `deploy/production/deploy.sh` 本机构建并上传；历史手动恢复可使用 `make agent-release-sync`。
- 节点能通过 HTTPS 访问主控的 `/api/v1/agent/install`、`/api/v1/agent/download/{version}/...`，并能通过 WSS 访问 `/api/v1/agent/control`。
- 节点管理员可使用 root 执行安装器。联网 Agent 使用 `deploy-go-agent`，业务脚本使用 `deploy-go-runner`；root runner broker 只按固定 spec 降权启动业务 child，独立 root executor 只提供签名 PTY、结构化特权 release 和无参数内置 self-test。
- 节点预装 `curl`、Python 3、systemd，以及 `sha256sum` 或 `shasum`。安装器不依赖 `jq`。

## 接入步骤

1. 唯一管理员在 Web 的 Agent 页面创建 Agent，只填写 Agent 名称和环境；主控在同一事务中创建一对一绑定的节点和离线 Agent。接管升级前已有的 legacy 节点时，从该节点详情页执行“接管此节点”，同样只填写名称和环境；API 会保留原 node、deployment target 和部署历史 ID。
2. 复制安装命令。命令已动态拼接短期 enrollment token（默认 30 分钟有效、一次性消费），不需要再单独复制或粘贴 token。命令含 token，不得写入工单、普通日志、shell history、聊天记录或仓库，Web 和客户端不持久化该命令。
3. 在已明确授权的目标 Linux 节点直接执行命令。安装器会校验 OS、架构、v3 配对 manifest、两个二进制 SHA-256、三个 systemd unit 和 executor 配置模板。
4. 安装器创建 `deploy-go-agent` 与 `deploy-go-runner` 用户和专用组，并准备以下目录：
   - `/var/lib/deploy-go-agent`：`0750 deploy-go-agent:deploy-go-runner`；其中 `credentials.json` 保持 `0600 deploy-go-agent:deploy-go-agent`，共享组不可写。
   - `/var/lib/deploy-go-agent/tasks`：`3710 deploy-go-agent:deploy-go-runner`，由 Agent 与 root runner broker 交换任务，业务 child 只获得当前任务目录。
   - `/var/lib/deploy-go-agent/apps`：`2770 deploy-go-agent:deploy-go-runner`，默认 `work_root`；安装器会按原 owner 权限同步 group 权限。
   - `/var/lib/deploy-go-agent/secrets`：目录 `2700`、文件 `0600`，均为 `deploy-go-agent:deploy-go-agent`，默认 `secrets_root`。
   - `/etc/deploy-go-agent/config`：只包含控制通道和数据目录，不包含 token。
   - `/etc/deploy-go-agent/executor.json`：`0600 root:root`，保存允许连接 Socket 的 Agent uid/gid、固定 Agent 可执行文件、两类授权公钥、release jobs 目录与资源策略，以及从系统账号数据库解析的 root home 和登录 shell；不保存任何签名私钥。
   - `/run/deploy-go-agent/executor.sock`：executor 自建 Socket，目录为 `0750 root:deploy-go-agent`，Socket 为 `0660 root:deploy-go-agent`；不安装 systemd `.socket` unit。
5. installer 先启动 executor 和 runner broker，确认两个 Socket、executor v2 的 PTY 与 `DeploymentRelease` capability，再启动 Agent。安装成功只说明节点具备上报 capability 的本机条件，不会自动打开数据库侧 `privileged_execution` 或目标的 `privileged_release`。安装器会同时输出 `status` 与 `doctor` 命令，命令不包含 token。
6. 把应用自有脚本和所需 secret 文件放入对应根目录，并确保 `deploy-go-runner` 可读/执行。普通业务部署仍走标准脚本和受控 launcher，不能通过 root 终端替代。
7. 在 Web 等待同一 Agent/节点变为在线，核对 hostname、架构、版本和 `pty_terminal` 能力，再从节点详情执行 `SystemInspect`。
8. 只有检查确认工作目录、secret 目录和磁盘可用后，才把该节点用于部署目标；需要终端时再由管理员单独开启该节点特权开关。

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

## 重跑与升级

- 本地 `credentials.json` 中 Agent ID 相同且凭证有效时，重跑安装器不重新 enrollment；安装器完整校验配对发布物后原子替换并按 executor/runner broker -> Agent 顺序重启。
- 本地 Agent ID 与命令不同时，安装器拒绝覆盖。
- Agent 已撤销时，管理员重新生成带 rebind 标记的一次性命令；安装器使用新 enrollment token 替换长期凭证。
- executor、Socket 或 Agent 健康检查失败时，安装器恢复上一对二进制、unit、配置和启用状态。旧环境只有 Agent 时也会恢复原 Agent，普通部署能力不因 executor 安装失败而丢失。
- 卸载前先在主控撤销 Agent，再经明确授权运行 `install.sh --uninstall`。卸载会依次停止 Agent、runner broker 和 executor，并保留凭证、任务和应用数据供人工确认。

## 本地验证

```bash
make agent-install-check
make agent-manifest-check
make agent-runner-isolation-check
cargo test -p deploy-go-api --test agent_enrollment --test agent_end_to_end
curl --fail --silent \
  https://deploy.example.com/api/v1/agent/download/0_2_0/manifest.json
curl --fail --silent --output /dev/null \
  https://deploy.example.com/api/v1/agent/download/0_2_0/agent/x86_64
```

`make agent-runner-isolation-check` 使用本机已有的 `rust:1.94-bookworm` 镜像和预热后的
`$HOME/.cargo/registry` 离线执行，不会自动拉取镜像；Cargo target 缓存按 Docker 架构保存在
`deploy-go-runner-target-<arch>-1_94_1` volume，需要释放空间时可手动删除该 volume。

这些命令只使用隔离 fixture 或示例地址，不连接真实节点。
