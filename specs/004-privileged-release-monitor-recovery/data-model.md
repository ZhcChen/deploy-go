# 特权 Release 状态模型

本方案复用现有实体，不新增数据库表或 Agent wire 字段。

## Agent Task Journal

本地 durable task 身份和恢复游标由 `agent/src/journal.rs::TaskJournal` 保存。

| 字段 | 作用 | 约束 |
| --- | --- | --- |
| `task_id`、`payload_digest` | 绑定控制面任务和不可变 payload | 恢复时必须匹配原任务 |
| `deployment_id`、`target_run_id` | 为本地恢复诊断保留部署关联 | 新特权 release 在发送 ReleaseStart 前写入；可选、反序列化有默认值且 None 不序列化，旧 journal 缺字段时兼容 |
| `state` | accepted、running 或 terminal 状态 | 未证明成功不得设为 succeeded |
| `transfer_phase` | 标识当前是否在 PrivilegedRelease 阶段 | monitor 恢复以该阶段识别，不重新启动 release |
| `external_output_sequence` | Agent 已持久化 executor 输出游标 | 只连续递增；重复读取不产生重复逻辑输出 |
| `last_sequence` | Agent 发出的有序协议消息游标 | TaskResult 使用下一个序号，重放复用原结果序号 |
| `result_sequence` | 本地已持久化 TaskResult 的序号 | 有值表示结果已进入 durable delivery 路径 |
| `exit_code`、`error_code` | durable 任务终态事实 | 输出超限或无法证明结果必须可区分 |

状态流：

```text
running + PrivilegedRelease
  -> 同 job 读取输出/状态
  -> 连续输出已 durable 保存 + executor terminal
  -> succeeded / failed / canceled + TaskResult outbox

running + PrivilegedRelease
  -> 明确不可恢复的序号/数据不一致
  -> interrupted + 稳定诊断 + TaskResult outbox
```

`job_id` 由 `release_<task_id>` 确定，恢复时必须保持不变。旧 journal 的 deployment/target run 字段缺省时，从有效 `privileged-release-task.json` 回填并持久化；descriptor 与 journal 中同时存在的标识必须一致。瞬时 IPC/journal/outbox 故障保留原状态及游标并重试；不得清除 `PrivilegedRelease` 或改变 job 身份。

若核心 journal 仍可解析并提供可信 task_id、payload digest、transfer phase 和输出游标，但 journal 与 descriptor 均无法提供完整关联元数据，Agent 以 task_id 推导原 job_id，只对该 job 执行 ReleaseOutput/ReleaseStatus 观测，不重新发送 ReleaseStart。因 descriptor 不可用而无法安全重建事件时，消费仍可读输出并等待 executor 终态，随后以稳定错误码报告 interrupted；诊断明确标出缺失字段。

若核心 journal 缺失或不可解析，task ID 仍可从受控任务目录名取得，但 payload digest、输出游标和结果序号不可信。此时只产生明确的 Unknown/恢复诊断，不向 executor 发送缺少可信 digest 的请求，不合成 TaskResult，不标记成功或重新启动；该情形须由 runbook 指引后续人工只读判断，不属于本方案自动收敛范围。

## Executor Release Job

executor durable job 由 `agent-executor` 状态文件维护。

| 事实 | 含义 |
| --- | --- |
| `job_id`、payload digest | 唯一发布执行身份 |
| job state、exit code、reason | 业务脚本实际执行终态 |
| `last_sequence` | executor 已保存的最后输出帧 |
| `output_truncated` | 输出预算耗尽，job 以 `output_limit_exceeded` 失败 |

Agent 必须区分“仍有已保存帧待读取”和“已明确丢弃超限输出”。后者不再等待不存在的帧，但只能形成显式失败，不能成功。

## TaskResult 接收与投影

`agent_task_event_receipts` 为协议消息记录序号、消息摘要和 committed 状态；`agent_task_events` 保存已处理事件。投影事实涉及 `agent_tasks`、`deployment_target_runs`、`deployments` 及其关联的 lease/check 状态。

- 同序号同内容是可重放消息；同序号不同内容是冲突。
- 投影发生后 receipt 未提交时，服务端可能再次处理相同结果；每一步必须能续做未完成部分且不重复修改已完成状态的 version/timestamp。
- 与已终结任务相冲突或试图回退终态的迟到结果拒绝应用。

## Deployment Target Run

target run 使用现有状态和阶段字段表达可见进度：

```text
pending
  -> downloading / artifact_download
  -> running / release      (首个可信 release 脚本开始 DeployEvent)
  -> succeeded | failed | canceled | expired
```

通用 TaskState Running 可能先于制品下载，不改变 target run 阶段。终态只由有效 TaskResult 及现有 deployment 聚合规则驱动；executor 状态文件不直接写入控制面终态。
