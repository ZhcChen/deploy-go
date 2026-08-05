# Agent 节点接入

## 适用范围

本手册用于通过一次性安装命令接入 Linux 节点。对真实节点运行安装命令、修改权限或重启服务前，必须在当前对话中获得针对具体节点和动作的明确授权；本地 fixture 验证不构成真实节点授权。

## 前置条件

- API 已配置可信的 `DEPLOY_GO_PUBLIC_BASE_URL`、`DEPLOY_GO_AGENT_MANIFEST_URL` 和 `DEPLOY_GO_AGENT_MANIFEST_PATH`，且 `/readyz` 返回 `200`。
- release manifest 包含当前主控兼容的 Linux `x86_64` 或 `aarch64` Agent、SHA-256 和 systemd unit。
- 节点能通过 HTTPS 访问主控和发布物，并能通过 WSS 访问 `/api/v1/agent/control`。
- 节点管理员可使用 root 执行安装器。Agent 和部署脚本最终均以低权限 `deploy-go-agent` 用户运行，平台不会下发 root、任意 shell 或隐式 sudo。

## 接入步骤

1. 唯一管理员在 Web 的 Agent 页面创建 Agent，只填写 Agent 名称和环境；主控在同一事务中创建一对一绑定的节点和离线 Agent。接管升级前已有的 legacy 节点时，从该节点详情页执行“接管此节点”，同样只填写名称和环境；API 会保留原 node、deployment target 和部署历史 ID。
2. 复制安装命令。命令已动态拼接短期 enrollment token（默认 30 分钟有效、一次性消费），不需要再单独复制或粘贴 token。命令含 token，不得写入工单、普通日志、shell history、聊天记录或仓库，Web 和客户端不持久化该命令。
3. 在已明确授权的目标 Linux 节点直接执行命令。安装器会校验 OS、架构、manifest、二进制 SHA-256 和 systemd unit 安全项。
4. 安装器创建 `deploy-go-agent` 用户，并准备以下目录：
   - `/var/lib/deploy-go-agent`：`0700`，Agent 数据和长期 refresh token。
   - `/var/lib/deploy-go-agent/apps`：`0700`，默认 `work_root`。
   - `/var/lib/deploy-go-agent/secrets`：`0700`，默认 `secrets_root`。
   - `/etc/deploy-go-agent/config`：只包含控制通道和数据目录，不包含 token。
5. 把应用自有脚本和所需 secret 文件放入对应根目录，并确保 `deploy-go-agent` 可读/执行。需要额外系统权限时由节点管理员显式配置最小权限；平台不自动提权。
6. 在 Web 等待同一 Agent/节点变为在线，核对 hostname、架构和 Agent 版本，再从节点详情执行 `SystemInspect`。
7. 只有检查确认工作目录、secret 目录和磁盘可用后，才把该节点用于部署目标。

## 验证

```bash
systemctl is-active deploy-go-agent
systemctl show deploy-go-agent -p User -p Group -p NoNewPrivileges
journalctl -u deploy-go-agent --since '10 minutes ago' --no-pager
stat -c '%a %U:%G %n' \
  /var/lib/deploy-go-agent \
  /var/lib/deploy-go-agent/apps \
  /var/lib/deploy-go-agent/secrets
```

预期服务为 `active`，用户和组均为 `deploy-go-agent`，`NoNewPrivileges=yes`，三个数据目录为 `700 deploy-go-agent:deploy-go-agent`。日志不得出现 enrollment、access 或 refresh token。

## 重跑与升级

- 本地 `credentials.json` 中 Agent ID 相同且凭证有效时，重跑安装器不重新 enrollment；安装器校验新二进制后原子替换并重启服务。
- 本地 Agent ID 与命令不同时，安装器拒绝覆盖。
- Agent 已撤销时，管理员重新生成带 rebind 标记的一次性命令；安装器使用新 enrollment token 替换长期凭证。
- 新服务未通过 `systemctl is-active` 时，安装器恢复 `.previous` 二进制并重启旧版本。

## 本地验证

```bash
make agent-install-check
make agent-manifest-check
cargo test -p deploy-go-api --test agent_enrollment --test agent_end_to_end
```

这些命令只使用隔离 fixture，不连接真实节点。
