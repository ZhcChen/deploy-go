# Agent 节点接入

## 适用范围

本手册用于通过一次性安装命令接入 Linux 节点。对真实节点运行安装命令、修改权限或重启服务前，必须在当前对话中获得针对具体节点和动作的明确授权；本地 fixture 验证不构成真实节点授权。

## 前置条件

- API 已配置可信的 `DEPLOY_GO_PUBLIC_BASE_URL`，且 `/readyz` 返回 `200`。
- 部署端已同步当前 API 版本的配对 release，包含 Linux `x86_64` 与 `aarch64` 的 Agent/executor、两个 systemd unit、executor 配置模板和 SHA-256。正式部署由 `deploy/production/deploy.sh` 本机构建并上传；历史手动恢复可使用 `make agent-release-sync`。
- 节点能通过 HTTPS 访问主控的 `/api/v1/agent/install`、`/api/v1/agent/download/{version}/...`，并能通过 WSS 访问 `/api/v1/agent/control`。
- 节点管理员可使用 root 执行安装器。联网 Agent 和部署脚本最终均以低权限 `deploy-go-agent` 用户运行；只有独立、无网络的 root executor 能按管理员终端协议创建 PTY。
- 节点预装 `curl`、Python 3、systemd，以及 `sha256sum` 或 `shasum`。安装器不依赖 `jq`。

## 接入步骤

1. 唯一管理员在 Web 的 Agent 页面创建 Agent，只填写 Agent 名称和环境；主控在同一事务中创建一对一绑定的节点和离线 Agent。接管升级前已有的 legacy 节点时，从该节点详情页执行“接管此节点”，同样只填写名称和环境；API 会保留原 node、deployment target 和部署历史 ID。
2. 复制安装命令。命令已动态拼接短期 enrollment token（默认 30 分钟有效、一次性消费），不需要再单独复制或粘贴 token。命令含 token，不得写入工单、普通日志、shell history、聊天记录或仓库，Web 和客户端不持久化该命令。
3. 在已明确授权的目标 Linux 节点直接执行命令。安装器会校验 OS、架构、v2 配对 manifest、两个二进制 SHA-256、两个 systemd unit 和 executor 配置模板。
4. 安装器创建 `deploy-go-agent` 用户和同名专用组，并准备以下目录：
   - `/var/lib/deploy-go-agent`：`0700`，Agent 数据和长期 refresh token。
   - `/var/lib/deploy-go-agent/apps`：`0700`，默认 `work_root`。
   - `/var/lib/deploy-go-agent/secrets`：`0700`，默认 `secrets_root`。
   - `/etc/deploy-go-agent/config`：只包含控制通道和数据目录，不包含 token。
   - `/etc/deploy-go-agent/executor.json`：`0600 root:root`，保存允许连接 Socket 的 Agent uid/gid、固定 Agent 可执行文件，以及从系统账号数据库解析的 root home 和登录 shell。
   - `/run/deploy-go-agent/executor.sock`：executor 自建 Socket，目录为 `0750 root:deploy-go-agent`，Socket 为 `0660 root:deploy-go-agent`；不安装 systemd `.socket` unit。
5. installer 先启动 executor 并确认 Socket，再启动 Agent。安装成功只说明节点声明 `pty_terminal` 的本机条件就绪，不会自动打开数据库侧 `privileged_execution`。
6. 把应用自有脚本和所需 secret 文件放入对应根目录，并确保 `deploy-go-agent` 可读/执行。普通业务部署仍走标准脚本和受控 launcher，不能通过 root 终端替代。
7. 在 Web 等待同一 Agent/节点变为在线，核对 hostname、架构、版本和 `pty_terminal` 能力，再从节点详情执行 `SystemInspect`。
8. 只有检查确认工作目录、secret 目录和磁盘可用后，才把该节点用于部署目标；需要终端时再由管理员单独开启该节点特权开关。

## 验证

```bash
systemctl is-active deploy-go-agent
systemctl is-active deploy-go-agent-executor
systemctl show deploy-go-agent -p User -p Group -p NoNewPrivileges
systemctl show deploy-go-agent-executor -p User -p Group -p NoNewPrivileges
journalctl -u deploy-go-agent -u deploy-go-agent-executor --since '10 minutes ago' --no-pager
stat -c '%a %U:%G %n' \
  /var/lib/deploy-go-agent \
  /var/lib/deploy-go-agent/apps \
  /var/lib/deploy-go-agent/secrets \
  /run/deploy-go-agent \
  /run/deploy-go-agent/executor.sock
```

预期两个服务均为 `active`；Agent 用户和组为 `deploy-go-agent`，executor 用户为 root，PTY 子进程允许联网和管理主机。`InaccessiblePaths` 只降低误读 Agent 凭证的概率，不能防御完整 root。三个数据目录为 `700 deploy-go-agent:deploy-go-agent`，Socket 权限符合上述约束。日志不得出现 enrollment、access 或 refresh token。

## 重跑与升级

- 本地 `credentials.json` 中 Agent ID 相同且凭证有效时，重跑安装器不重新 enrollment；安装器完整校验配对发布物后原子替换并按 executor -> Agent 顺序重启。
- 本地 Agent ID 与命令不同时，安装器拒绝覆盖。
- Agent 已撤销时，管理员重新生成带 rebind 标记的一次性命令；安装器使用新 enrollment token 替换长期凭证。
- executor、Socket 或 Agent 健康检查失败时，安装器恢复上一对二进制、unit、配置和启用状态。旧环境只有 Agent 时也会恢复原 Agent，普通部署能力不因 executor 安装失败而丢失。
- 卸载前先在主控撤销 Agent，再经明确授权运行 `install.sh --uninstall`。卸载会先停止 Agent 后停止 executor，并保留凭证、任务和应用数据供人工确认。

## 本地验证

```bash
make agent-install-check
make agent-manifest-check
cargo test -p deploy-go-api --test agent_enrollment --test agent_end_to_end
curl --fail --silent \
  https://deploy.example.com/api/v1/agent/download/0_1_0/manifest.json
curl --fail --silent --output /dev/null \
  https://deploy.example.com/api/v1/agent/download/0_1_0/agent/x86_64
```

这些命令只使用隔离 fixture 或示例地址，不连接真实节点。
