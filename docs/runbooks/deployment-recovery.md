# 部署执行、取消与恢复

## 适用范围

本手册用于排查部署队列、脚本执行、取消、日志续传和 API 重启后的任务恢复。查看或操作真实节点前，必须在当前对话中获得针对具体节点和动作的明确授权；本地测试和代码验证不构成远程执行授权。

## 状态语义

| 状态 | 含义 | 重启后处理 |
| --- | --- | --- |
| `queued` | 已持久化，等待目标锁和全局并发名额 | 保持 `queued`，worker 自动重新领取 |
| `running` | SSH 已开始或正在执行，远端状态可能变化 | 启动恢复时标记 `interrupted`，不自动重试 |
| `canceling` | 已请求远端 TERM/KILL，但结果尚未确认 | 启动恢复时标记 `interrupted` |
| `succeeded` | 脚本退出和事件已经完成裁决 | 保持终态 |
| `failed` | 脚本、协议或执行前置条件明确失败 | 保持终态，可人工重试 |
| `canceled` | 平台确认取消流程完成 | 保持终态，可人工重试 |
| `interrupted` | 平台无法证明远端最终结果 | 保持终态，核实节点后人工重试 |

`interrupted` 不是 `failed`，也不代表远端脚本已经停止或回滚。平台不会自动重试、自动回滚或修改应用状态。

## 正常观察

1. 通过部署详情确认 `status`、`phase`、`started_at`、`finished_at`、`exit_code` 和 `protocol_complete`。
2. SSE 使用 `/api/v1/deployments/{id}/logs`，断线后把最后收到的日志 sequence 放入 `Last-Event-ID` 或 `after`。
3. 服务先从 SQLite 补发游标后的持久化日志，再等待新日志；收到 `terminal` 事件后连接正常关闭。
4. `stream-error` 表示读取暂时失败，客户端应保留最后游标并重连；`authorization-revoked` 表示会话、用户或应用授权已经失效。

日志游标必须非负，且不能超前或落在已清理区间。日志和事件达到系统设置限额时会记录 `line_truncated` 或 `log_budget_exceeded` 诊断。已终态部署超过 `log_retention_days` 后，worker 清理日志和事件，但保留部署记录。

## 取消流程

- queued 任务在数据库中直接转为 `canceled`，不会连接节点。
- running 任务先转为 `canceling`，再通过独立 SSH 请求创建取消文件并读取平台包装器写入的 PID。
- 包装器向远端进程组发送 TERM，默认等待 30 秒；仍存活时发送 KILL。
- PID 缺失、内容非法、SSH 断连或信号结果无法确认时，任务转为 `interrupted`。
- 取消只停止平台启动的脚本进程组，不宣称应用已经回滚。

不要手工修改 SQLite 中的部署状态来“解除锁”。状态错误会破坏同目标串行约束和审计事实。

## API 重启

计划重启前：

1. 查看是否存在 `running` 或 `canceling` 任务。
2. 能等待完成时优先等待；需要取消时按正常取消流程操作并确认终态。
3. 记录仍在运行的 deployment ID、目标、节点和最后日志游标。
4. 停止 API。进程退出会终止本地 SSH 客户端，但不能证明远端进程组已经停止。

启动后：

1. `/readyz` 返回 `200` 后查询重启前记录的部署。
2. queued 应继续排队；原 running/canceling 应为 `interrupted`。
3. 通过已授权的节点观察手段核实应用和脚本状态。没有真实节点授权时只记录待核实事项，不连接节点。
4. 确认远端无冲突执行后，使用 retry API 创建新 deployment；不得复用原记录或删除原日志。

## SQLite 备份与恢复

API 使用 WAL 和 busy timeout。写入期间不得只复制主 `.db` 文件，否则可能漏掉 `-wal` 中的数据。

备份前优先停止 API；不能停止时使用 SQLite backup API 或经过验证的一致性备份工具。记录应用提交、数据库路径、主文件及 WAL 文件状态和 `_sqlx_migrations` 内容。

恢复顺序：

1. 停止 API 并保留当前数据库、`-wal` 和 `-shm` 作为故障证据。
2. 恢复同一时点的一致性备份。
3. 运行 `make api-migrate`。
4. 启动 API，确认 `/readyz`、migration 和审计日志正常。
5. 按本手册核对 queued、running 和 canceling 任务，不手工猜测远端结果。

## 本地验证

以下命令只使用内存数据库、mock executor 和本地 OpenSSH fixture：

```bash
cargo test -p deploy-go-api --test deployment_runtime --test deployment_recovery
cargo test -p deploy-go-api --test deployment_executor --test end_to_end
make api-check
```

测试不得替换为真实 host、真实 SSH 密钥或共享数据库。
