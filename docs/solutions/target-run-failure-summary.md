---
date: 2026-09-11
topic: target-run-failure-summary
---

# 目标运行失败摘要退化为状态串

## 问题

外部调用方（CLI、其他项目）通过 `/external/v1/deployments/{id}` 查询失败部署时，只能拿到
`target_runs[].error_code = "failed"`、`result_summary = "failed"`，无法判断卡在哪一步，
`failed` 是任务状态串而不是具体错误码。

真实案例：`deployment_01M277SQ3WYH9W8Y88BXNCTY64`（测试环境，业务脚本引用了不存在的镜像 tag）
的 release 任务失败，`agent_tasks.result_json` 中已有
`{"status":"failed","exit_code":2,"error_code":"process_exited","summary":null}`，
但 run 与部署记录只落成：

| 位置 | 修复前 | 修复后 |
| --- | --- | --- |
| `deployment_target_runs.error_code` | `failed` | `process_exited` |
| `deployment_target_runs.result_summary` | `failed` | `[worker.remote.migrate] 远端部署失败` |

具体失败阶段只存在于 `agent_task_events` 事件流，外部调用方拿不到。

## 根因

- Agent 终态结果常常不带 `summary`，`apply_result` 用 `result.summary.unwrap_or(status)` 兜底，
  摘要因此退化成状态串。
- `finish_deployment_for_task` 写 run 时把 `status` 当作 `error_code`，即使上游已给出
  `process_exited` 这类具体错误码也没有透传。

## 结论

- `error_code` 优先透传上游具体错误码（Agent 结果、前置校验原因、对账中断原因），
  缺失时才退回状态串保持历史行为。
- 失败终态摘要缺具体内容时，取该任务最后一个 `deploy.step.failed` 事件，
  组成 `[failure_stage] message`；`failure_stage` 缺失时依次回退 `step_id`、`step`。
- 摘要补全是尽力而为：事件缺失、载荷不可解析或查询失败都退回原摘要，不影响终态落库。
- 不需要新增对外字段，外部接口字段语义不变，只是值变得可用。

## 排查步骤

1. 从部署详情取 `target_run_id` 与对应 release 任务的 `agent_tasks.id`。
2. 查该任务最后一个失败步骤事件：

```sql
SELECT json_extract(payload_json,'$.failure_stage'),
       json_extract(payload_json,'$.message'),
       json_extract(payload_json,'$.step')
FROM agent_task_events
WHERE task_id='<task_id>'
  AND kind='progress'
  AND json_extract(payload_json,'$.event')='deploy.step.failed'
ORDER BY sequence DESC LIMIT 1;
```

3. 对照 `agent_tasks.result_json` 的 `error_code`、`exit_code` 判断失败类型。
   本例中 `failure_stage=worker.remote.migrate` 配合 `exit_code=2`，指向远端脚本自身退出，
   而非控制面调度或制品下载链路。

## 验证

```bash
cargo test -p deploy-go-api --lib agents::dispatcher
```

`failure_summary_includes_failed_step_and_error_code` 覆盖事件存在时的补全与错误码透传，
`failure_summary_without_step_event_keeps_given_summary` 覆盖无事件时退回状态串。
