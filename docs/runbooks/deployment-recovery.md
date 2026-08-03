# 部署执行、取消与恢复

## 适用范围

本手册用于排查 Agent 部署队列、取消、日志续传和 API/Agent 重启恢复。操作真实节点前必须获得当前对话中针对具体节点和动作的明确授权。

## 状态语义

| 状态 | 含义 | 恢复处理 |
| --- | --- | --- |
| `queued` | 已持久化，等待在线 Agent 和目标锁 | 保持排队，worker 重新领取 |
| `running` | Agent 已 ACK 且脚本正在运行 | 等待 Agent reconcile；无法证明时进入 `interrupted` |
| `canceling` | 已下发结构化取消任务，终态未确认 | 等待 Agent 结果；无法证明时进入 `interrupted` |
| `succeeded` / `failed` / `canceled` | 已确认终态 | 保持终态 |
| `interrupted` | 进程身份或最终结果无法证明 | 核实后人工 retry，不自动重跑 |

`interrupted` 不代表脚本已停止、失败或回滚。平台不接管应用回滚。

## 正常观察与 SSE 续传

1. 查看 deployment 的 `status`、`phase`、`exit_code` 和 `protocol_complete`。
2. SSE `/api/v1/deployments/{id}/logs` 断线后使用最后 sequence 作为 `Last-Event-ID` 或 `after`。
3. API 先补发 SQLite 中游标后的日志，再推送新事件；终态发送 `terminal`。
4. `stream-error` 保留游标后重连；`authorization-revoked` 要求重新认证或获取授权。

Agent 输出按任务内 sequence 去重。达到日志预算时记录诊断但不泄露 secret；日志保留期结束后只清理输出，不删除 deployment 历史。

## 取消

- queued deployment 可在数据库中直接转为 `canceled`，不投递 Agent。
- 已投递任务通过版本化 `TaskCancel` 指定 task ID，不传任意 shell 或信号命令文本。
- Agent 只终止自己 durable runner 记录且进程身份可验证的进程组。
- 无法确认进程归属或取消结果时进入 `interrupted`。

不要手工修改 SQLite 状态或删除 task/journal 来解除锁。

## API 与 Agent 重启

计划重启前记录活动 deployment、task ID 和最后日志游标。API 重启后节点先离线，Agent 重连并以新 connection generation 对账。Agent 重启后从受保护 journal 恢复 payload digest、进程 start-time、日志偏移和完成标记。

只有 task ID、digest 和进程身份一致时继续跟踪；不确定结果进入 `interrupted`。核实不存在冲突执行后使用 retry API 创建新 deployment，不复用或删除原记录。详细 Agent 故障步骤见 `docs/runbooks/agent-recovery.md`。

## SQLite 备份与恢复

SQLite 使用 WAL。优先停止 API 后备份；在线备份必须使用 SQLite backup API 或经过验证的一致性工具，不能只复制主 `.db`。

恢复顺序：

1. 停止 API，保留当前数据库、`-wal` 和 `-shm` 作为证据。
2. 恢复同一时点的一致性备份并运行 `make api-migrate`。
3. 启动 API并确认 `/readyz`、migration 和审计正常。
4. 等待 Agent 重连和 reconcile，再核对 queued/running/canceling；不手工猜测终态。

## 本地验证

```bash
cargo test -p deploy-go-api --test deployment_runtime --test deployment_recovery
cargo test -p deploy-go-api --test agent_dispatcher --test agent_end_to_end
make api-check
```

这些测试不需要 OpenSSH 客户端、SSH 私钥或真实节点。
