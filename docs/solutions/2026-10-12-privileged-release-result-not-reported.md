---
date: 2026-10-12
topic: privileged-release-result-not-reported
related_plan: docs/plans/2026-09-30-002-privileged-release-start-reconciliation-plan.md
---

# 节点 Release 成功但部署任务未收敛

## 适用场景

节点上的 privileged release job 已退出成功，发布物也已切换为当前版本，但 Deploy Go 的 release task、target run 或父 deployment 仍显示运行中。必须区分节点业务发布结果和 Agent 向控制面报告的任务终态；节点脚本成功不等于 Deploy Go 已确认部署成功。

## 本次只读证据

部署 `deployment_01M4QAXB5M38A1WZGJHQBPVVY2`，目标运行 `run_01M4QAXB5NGQZAVBX824GP6H7P`，应用 `app_01KZMX1RW5PYJR9GDV2JWPYF19`，目标 `target_01KZMX9VJMXXAB3ZHYEF5XD7RZ`，release version `20261012135600`，commit `389512139d175aa864a5e7cc9acf90c2082764e9`。

- 控制面父 deployment 为 `running / deploying`；prepare 成功。release task 来源为 `agent_started`，无完成时间和退出码；target run 仍为 `downloading / artifact_download`，最后更新时间约为 13:56:42 +08，最后一条控制面日志序号为 64。
- qfy-test2 上本次五个模块的 release-job 状态均为 `succeeded`、`exit_code=0`、`current_switched=true`，当前 release 均为 `20261012135600`。API 蓝绿切换、候选和前门健康检查通过，Worker 健康检查通过；这些证明节点发布已完成，不证明控制面已接收 Agent 终态。
- 对应 Agent task 为 `task_01M4QAYJYB81280B33RTM2VAZ1`，executor job 为 `release_task_01M4QAYJYB81280B33RTM2VAZ1`。Executor 记录 `succeeded / process_exited`、退出码 0、输出序号 721，`output_truncated=false`。
- 同一时点 Agent journal 仍为 `running`，`result_sequence=null`，`external_output_sequence=699`；executor 与 Agent 之间尚差 22 帧、1,032 字节。日志交付记录为 `acknowledged=109`、`pending=0`、`finalized=false`。
- Agent、runner、executor 服务当时均为 active，Agent 自 10 月 8 日启动后 `NRestarts=0`。节点与控制面快照约 49 分钟没有变化；可见日志未找到该 deployment 的明确错误或终态回报。

检查限于部署状态、任务/journal 元数据、executor 状态和脱敏诊断；未读取 Env、密钥或业务日志正文，未重发、取消、手工改状态或触发部署。

## 判断与未知项

1. 节点发布与 Deploy Go 协议终态是两个独立事实：executor 已证明 durable job 成功，但 Agent journal 未完成，控制面也没有 TaskResult。`result_sequence=null` 表明 Agent 尚未在本地持久化 TaskResult，因此当前证据不支持“TaskResult 已送达 API、仅状态投影失败”这一解释。
2. target run 的 `downloading / artifact_download` 是派发阶段留下的状态；Agent 的 TaskState 更新路径更新 task/deployment，但不会同步推进 target run。该字段在长任务期间可能是陈旧投影，不能据此判断节点仍在下载。
3. 具体触发点未能从现场证据确认。`agent/src/task_handler.rs` 的 `monitor_privileged_release` 在 journal 读取、输出序号连续性、输出持久化、journal/event 重建、日志/事件转发及 `complete_task` 等路径存在未产生明确诊断的提前返回或错误忽略；现场缺少足以区分这些分支的记录。也不能排除 monitor 仍在重试但没有形成可见终态。
4. 这与 `docs/plans/2026-09-30-002-privileged-release-start-reconciliation-plan.md` 修复的“ReleaseStart 响应不确定后避免重复启动”不是同一窗口：本次 durable executor job 已确认成功，问题位于启动之后的输出收尾、journal 完成或 TaskResult 交接。

## 代码路径

- `agent/src/task_handler.rs` 的 `monitor_privileged_release` 分页读取 executor 输出，要求先持久化连续帧、更新 journal、重建事件并完成日志/事件转发，再查询 `ReleaseExited`。只有确认输出完整且观察到终态后才完成 journal 并调用 `send_result`。
- 同文件 `send_result` 先设置并持久化 `result_sequence`，再通过 durable log delivery 交付 `TaskResult`。因此本次 `result_sequence=null` 将排查范围收窄在本地结果生成之前。
- `api/src/agents/dispatcher.rs` 的 `handle_result` / `apply_result` 才会保存 `result_json` 并完成 deployment、target run 投影。收到 TaskState 只更新 task/deployment 的运行信息；派发时建立的 target run `downloading / artifact_download` 不会由此路径推进。
- Agent 现有测试 `terminal_status_waits_until_all_paginated_executor_output_is_persisted` 覆盖 300 帧、128 帧分页限制下的成功路径；它没有覆盖本次这类持久化、journal/event 重建、outbox 转发或完成任务失败后的恢复行为。

## 修复建议

1. **让 monitor 生命周期可诊断。** 将所有提前返回和被忽略的错误改为受控错误分类及结构化日志，至少携带 task/job ID、失败阶段、Agent 已持久化输出序号、executor 最终序号/状态和恢复动作；不得记录 Env、凭证、请求正文或业务输出正文。日志本身不能替代持久化状态。
2. **监督并恢复同一 durable job。** monitor 因瞬时 IPC、日志 outbox、journal 或事件写入失败而退出时，应由受监督任务或恢复对账重新挂接相同 `job_id`，继续从已持久化序号拉取输出并观察终态；禁止重新发送 `ReleaseStart`、生成新 job 或因节点成功而跳过缺失输出。
3. **明确终态收敛契约。** 只有 executor 终态已确认、输出序号完整、所需日志/事件已 durable 持久化后，才写入 Agent terminal journal 和 TaskResult。永久数据损坏或无法安全证明结果时应留下明确、可查询的 `unknown/interrupted` 类诊断和人工恢复线索，不能静默遗留为 running，也不能把未证明的结果伪报成功。
4. **改进控制面运行态可观测性。** 为 target run 暴露 Agent 当前阶段/最后活动时间，或在正确的生命周期事件中同步阶段，避免长期显示 `artifact_download`。这属于状态投影修正，不能替代 Agent 可靠地产生和发送 TaskResult。

## 回归验证建议

- 保留现有 300 帧分页成功测试；再覆盖恰好 128 帧边界、跨多批次输出以及 executor 已退出但最后一批输出尚未持久化的恢复。
- 对第 700 帧附近的输出持久化失败、journal 写入失败、事件重建失败、输出/事件 outbox 暂时失败、`complete_task` 失败分别注入故障；验证 monitor 可从 durable 状态续跑，不留下无诊断 running，也不丢帧或产生重复 TaskResult。
- 模拟 Agent 在 executor job 成功后重启/monitor task 被取消，确认只用原 `job_id` 查询与补传，不重复执行 release；结果 ACK 丢失后重放应保持幂等。
- API 集成测试验证：没有 TaskResult 时不将父 deployment 标成功；收到同一个终态结果后，target run 与 deployment 收敛一次且重复结果安全；运行中的 target run 不长期误显示 `artifact_download`。
- 对新日志和外部诊断做敏感字段断言，确保不暴露 Env、密钥、业务日志正文或 executor 内部路径。

## 操作边界

本记录对应现场只读排查；没有手工把 deployment 标记成功，也没有重发、取消或重启节点任务。在官方终态确认前，不得将该次部署作为可晋级 production 的成功依据。后续代码修复应走本项目 Spec Kit 流程；真实节点操作仍须按 `AGENTS.md` 中的远程执行授权规则单独确认。
