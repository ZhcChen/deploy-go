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

两阶段部署（`execution_mode=two_stage`）在外层 `status` 之上还有 `phase`：

| phase | 含义 | 恢复处理 |
| --- | --- | --- |
| `queued` | 尚未创建 prepare task | worker 按 `(deployment_id, stage='prepare')` 创建并投递 |
| `preparing` | prepare task 正在执行或已成功但 release 未创建 | 只重放 prepare，不重复执行已完成阶段 |
| `deploying` | release task 正在执行 | 等待 release reconcile；无法证明时进入 `interrupted` |
| `verifying` | release 已输出验证事件，等待终态 | 等待 release 最终结果 |

`phase` 只反映当前阶段，不替代外层 `status`。UI 不得只从日志推导阶段状态。

## 正常观察与 SSE 续传

1. 查看 deployment 的 `status`、`phase`、`exit_code` 和 `protocol_complete`。
2. SSE `/api/v1/deployments/{id}/logs` 断线后使用最后 sequence 作为 `Last-Event-ID` 或 `after`。
3. API 先补发 SQLite 中游标后的日志，再推送新事件；终态发送 `terminal`。
4. `stream-error` 保留游标后重连；`authorization-revoked` 要求重新认证或获取授权。

Agent 输出按任务内 sequence 去重。日志表使用 deployment 全局 `sequence` 排序，SSE 日志事件额外携带 `stage`、`task_id` 和 `task_sequence`，便于按阶段分组展示。migration `0011` 之前的旧日志迁移后保持 `task_id=NULL`、`task_sequence=sequence`。达到日志预算时记录诊断但不泄露 secret；日志保留期结束后只清理输出，不删除 deployment 历史。

## 两阶段 stage 任务恢复

两阶段部署在 `agent_tasks` 中对应两条任务：`stage='prepare'` 与 `stage='release'`，数据库以 `(deployment_id, stage)` 唯一约束防止重复创建。

1. 先按部署和阶段定位任务，不要只按 `deployment_id` 查单条：

```sql
SELECT id, stage, status, last_sequence, finished_at
FROM agent_tasks
WHERE deployment_id = ? ORDER BY stage;
```

2. 只有 prepare 已持久化 `succeeded` 且发布物校验通过后，worker 才会创建 release；release 失败不反向改写 prepare 终态。
3. API 重启后按数据库阶段事实继续：prepare 已成功则直接创建/投递 release，不重复执行 prepare；两个阶段任一处于不确定状态时进入 `interrupted` 并人工核实。
4. 恢复时保留两个 stage 的日志与终态；不能手工删除某一阶段 task 来解除部署锁。
5. 取消作用于当前活动 stage，并阻止后续 stage 创建；取消后遗留任务 staging 由 Agent 随任务清理。

## 取消

- queued deployment 可在数据库中直接转为 `canceled`，不投递 Agent。
- 已投递任务通过版本化 `TaskCancel` 指定 task ID，不传任意 shell 或信号命令文本。
- Agent 只终止自己 durable runner 记录且进程身份可验证的进程组。
- 无法确认进程归属或取消结果时进入 `interrupted`。

不要手工修改 SQLite 状态或删除 task/journal 来解除锁。

## API 与 Agent 重启

计划重启前记录活动 deployment、task ID 和最后日志游标。API 重启后节点先离线，Agent 重连并以新 connection generation 对账。Agent 重启后从受保护 journal 恢复 payload digest、进程 start-time、日志偏移和完成标记。

只有 task ID、digest 和进程身份一致时继续跟踪；不确定结果进入 `interrupted`。核实不存在冲突执行后使用 retry API 创建新 deployment，不复用或删除原记录。详细 Agent 故障步骤见 `docs/runbooks/agent-recovery.md`。

API 收到 SIGTERM 后先停止 HTTP 接入，再通知内部署 worker 退出并等待完成；不通过直接 abort 留下新的领取循环。启动时 worker 会恢复过期 delivery lease，并核对制品数据库与受控目录。

## 制品与 Env 恢复

1. 不手工删除 `artifacts/quarantine`、`artifacts/objects` 或修改 `deployment_artifacts` 状态。worker 启动及每小时执行 reconciliation：过期上传失败化、缺失 object 失败化、无活动 lease/run 的过期制品清理、孤儿文件清理。
2. 下载中的 object 有进程内 pin，当前下载结束前不会被清理；API 异常退出后 pin 消失，但数据库中的活动 target run/lease 仍阻止误删。无法证明引用关系时先停止 API并保留现场。
3. Agent 重连后 Env 只补偿应用当前版本，不重放已经被替代的明文版本。`pending`/`syncing` 可等待收敛；`failed` 由管理员按目标重试，成功节点不得重复下发。
4. release 报 `env_gate_failed` 时先在 Web 核对该目标的 `actual_version`、脱敏错误码和节点在线状态；不要绕过门禁或把 Env 内容写入部署参数。
5. Env 删除使用 tombstone 和同一 no-follow 路径。节点离线时删除保持待同步，重连后只执行当前删除事实。

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
cargo test -p deploy-go-api --test two_stage_deployment --test deploy_event_protocol
cargo test -p deploy-go-api --test artifacts_api --test env_sync_dispatcher
make api-check
```

这些测试不需要 OpenSSH 客户端、SSH 私钥或真实节点。
