# Agent 故障与恢复

## 适用范围

本手册用于 Agent 离线、凭证失效、重装/升级失败及 API 或 Agent 重启后的恢复。对真实节点查看日志、重启、清理或重装前，必须获得当前对话中的具体授权。

## 离线排查顺序

1. 在 Web 核对 Agent 是否已撤销、最后在线时间、版本、hostname 和节点 ID。
2. 经明确授权后，在节点以服务用户先执行只读诊断：

   ```bash
   sudo -u deploy-go-agent /usr/local/bin/deploy-go-agent status
   sudo -u deploy-go-agent /usr/local/bin/deploy-go-agent doctor
   ```

3. `doctor` 出现 `FAIL` 时按对应检查项修复。退出码为 `0` 只表示本机配置、凭证、Agent service 与匿名 HTTPS `/readyz` 没有决定性失败；`CONTROL_CHANNEL_AUTH` 固定为 `WARN/未验证`，因此不能据此认定 WSS upgrade、Agent 鉴权或心跳成功。
4. Agent 仍离线时，执行诊断输出的固定下一步命令，检查三个 unit 状态与最近日志：

   ```bash
   systemctl status deploy-go-agent deploy-go-agent-runner deploy-go-agent-executor
   journalctl -u deploy-go-agent -u deploy-go-agent-runner -u deploy-go-agent-executor --since '30 minutes ago' --no-pager
   ```

5. 核对 API `/readyz`、公开 HTTPS 地址、WSS 反向代理、DNS、时间和 TLS 信任。必要时核对 `/etc/deploy-go-agent/config` 的控制 URL，但不读取或输出 `/var/lib/deploy-go-agent/credentials.json` 内容。不要关闭 TLS 校验；不要手动把 token 拼进命令参数、shell history 或日志中试探。

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
- 两阶段 prepare 进程已结束但制品尚未上传时，Agent 对账会以 `Accepted` 请求主控按幂等路径重新下发 prepare；恢复制品上传后再发送终态，不会把未上传制品的 prepare 误报为 `succeeded`。

## 升级失败

1. 查看安装器错误和 systemd 日志，确认失败发生在 v3 manifest 配对、checksum、unit/config 校验、executor/runner Socket、Agent 重启还是健康检查。
2. 健康检查失败时确认两个二进制、三个 unit 和 executor 配置已经成对恢复；旧环境仅有 Agent 时确认原 Agent 重新 active。
3. 不兼容 manifest 或架构必须修正发布物，不能跳过 SHA-256 或兼容矩阵。
4. rollback 后 Agent 仍离线时按“离线排查顺序”处理；不要删除 journal 来伪造恢复。

## executor、终端或特权 release 不可用

1. v11 Agent 无法建立控制连接，或节点因 executor 故障离线时，先检查 `deploy-go-agent-executor` 是否 active，以及 `/run/deploy-go-agent/executor.sock` 是否为 `0660 root:deploy-go-agent`。
2. 核对 `/etc/deploy-go-agent/executor.json` 中 uid/gid 是否与 `id deploy-go-agent` 一致；不得输出 Agent 凭证文件。
3. v11 缺少 PTY 或 release executor 能力时会在启动前退出，不能继续承担普通部署。修复或重新运行同版本安装器后，按 executor、runner、Agent 顺序恢复服务。
4. 终端清理异常时先关闭活动会话或等待其收敛，再停止 Agent、runner 和 executor。不得直接删除 Socket 来假装 PTY 已退出。
5. 需要卸载时先撤销主控身份，再运行安装器的 `--uninstall`；凭证和任务数据默认保留，是否删除必须另行确认。
6. `doctor` 显示 executor v3、`privileged_release` capability 可用后，可在获准的测试节点执行 `sudo -u deploy-go-agent /usr/local/bin/deploy-go-agent privileged-release-self-test`。该命令不替代业务部署授权，也不得在生产节点擅自执行。

## 分支刷新或任务出现 invalid_task

症状：Web 刷新 Git 分支失败；`agent_tasks` 的 `result_json` 为 `{"error_code":"invalid_task"}`；节点 `/var/lib/deploy-go-agent/tasks/<task_id>` 存在但为空（journal 未写入）。

可能根因一：Agent systemd unit 启用 `RestrictSUIDSGID=true`。任务目录已由 setgid 父目录继承为 `3700/3770` 时，旧 Agent 仍无条件执行 `chmod 3700`，被 seccomp 以 EPERM 拒绝，导致 journal 写入未发生。

处理：
1. 升级到含目录权限守卫（已允许的 setgid 权限不重复 `chmod`）的 Agent 版本，重试刷新或重新提交任务。
2. 若旧版本必须临时恢复，可在 `deploy-go-agent.service.d` 添加 `RestrictSUIDSGID=false` 并重启 Agent；该放宽应在随后的 Agent 升级中移除。
3. 空任务目录可安全保留，Agent 会将其视为无 journal 并跳过；确认后也可按最小影响清理对应空目录。

可能根因二：`tasks` 根目录残留 `.probe_*` 等非法目录名。Agent 遍历任务目录时不应把探针目录误判为任务；含目录名校验的 Agent 版本会跳过这些目录。残留目录可移到 tasks 根目录之外，无需删除。

## Git 认证失败（git_authentication_failed）

症状：GitLab 已配置部署公钥，但 Agent 分支刷新或两阶段 prepare 仍返回 `git_authentication_failed`。

根因：OpenSSH 拒绝私钥文件权限不是 `0600`。旧 Agent 以 `0640` 写入 `tasks/<task_id>/git-key` 以便 runner 组读取，OpenSSH 会按“权限过宽”拒绝。

处理：
1. 升级到写入 `0600` 的 Agent 版本。Agent 的 Git 分支刷新以 Agent 用户直接读取 `git-key`；两阶段 runner 由 root runner broker 生成属主为 runner 用户的 `runner-git-key`（`0600`）后再启动，不依赖组读取。
2. 新版本部署后重新发起任务；无需修改 GitLab 公钥。
3. 不要手工把私钥改为其他权限或复制到系统目录；任务结束由 Agent 清理 `git-key` 与 `runner-git-key`。

## 本地复演

```bash
cargo test -p deploy-go-api --test agent_websocket --test agent_dispatcher
cargo test -p deploy-go-api --test deployment_runtime --test deployment_recovery
cargo test -p deploy-go-agent --test recovery
make agent-install-check
```

恢复测试必须使用可控 clock、本地 mock WSS/HTTP 和隔离目录。
