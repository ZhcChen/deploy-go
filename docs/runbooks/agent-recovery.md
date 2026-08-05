# Agent 故障与恢复

## 适用范围

本手册用于 Agent 离线、凭证失效、重装/升级失败及 API 或 Agent 重启后的恢复。对真实节点查看日志、重启、清理或重装前，必须获得当前对话中的具体授权。

## 离线排查顺序

1. 在 Web 核对 Agent 是否已撤销、最后在线时间、版本、hostname 和节点 ID。
2. 核对 API `/readyz`、公开 HTTPS 地址和 WSS 反向代理是否可用。
3. 经明确授权后在节点检查 `systemctl status deploy-go-agent` 与 `journalctl -u deploy-go-agent`。
4. 核对 `/etc/deploy-go-agent/config` 的控制 URL，不读取或输出 `/var/lib/deploy-go-agent/credentials.json` 内容。
5. 核对 DNS、时间和 TLS 信任。不要关闭 TLS 校验；排查时不要手动把 token 拼进命令参数、shell history 或日志中试探。

Agent 会退避重连。access token 有效期为 30 分钟，并在到期前通过 refresh token 换取新 access/refresh token；同一 WebSocket 上的 `auth.refresh` 成功不会把节点短暂标为离线。确认后的旧 refresh token 被重用时，整个凭证族会被撤销并留下审计记录。

## 撤销与重新绑定

管理员撤销 Agent 后，主控关闭活动 WebSocket、撤销 enrollment/access/refresh 凭证并把节点置为离线。恢复时：

1. 在 Web 为同一 Agent 重新生成安装命令。
2. 经授权在原节点运行带 rebind 标记的新命令。
3. 核对安装器没有覆盖不同 Agent ID，服务恢复在线后重新执行 `SystemInspect`。
4. 不删除节点、部署目标或历史部署记录。

若本地凭证文件损坏或丢失，不手工构造 refresh token；使用上述 rebind 流程。若怀疑 token 泄露，先撤销 Agent，再排查日志和命令历史。

## 重启与任务恢复

- API 重启时 Agent 节点先标为离线；Agent 重连后以新的 connection generation 取代旧连接。
- Agent 报告本地 durable runner journal 中的任务 ID、payload digest、状态和日志偏移，API 进行 reconcile。
- task ID/digest 一致且进程身份可验证时继续跟踪并补传日志；不一致、PID 身份不可证明或缺少原子完成标记时进入 `interrupted`。
- `interrupted` 不会自动重跑。核对应用状态后通过 retry 创建新 deployment，保留原记录和日志。
- 投递租约未 ACK 时可以重投同一 task；Agent 必须按 task ID 和 digest 返回已有状态，不能重复启动脚本。

## 升级失败

1. 查看安装器错误和 systemd 日志，确认失败发生在下载、checksum、unit 校验、重启还是健康检查。
2. 健康检查失败时确认 `.previous` 已恢复且旧服务重新 active。
3. 不兼容 manifest 或架构必须修正发布物，不能跳过 SHA-256 或兼容矩阵。
4. rollback 后 Agent 仍离线时按“离线排查顺序”处理；不要删除 journal 来伪造恢复。

## 本地复演

```bash
cargo test -p deploy-go-api --test agent_websocket --test agent_dispatcher
cargo test -p deploy-go-api --test deployment_runtime --test deployment_recovery
cargo test -p deploy-go-agent --test recovery
make agent-install-check
```

恢复测试必须使用可控 clock、本地 mock WSS/HTTP 和隔离目录。
