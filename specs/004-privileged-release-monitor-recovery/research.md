# 特权 Release 终态可靠收敛研究

## 现场事实

- `docs/solutions/2026-10-12-privileged-release-result-not-reported.md` 记录了 executor job 成功且退出码 0、最后输出序号 721，但 Agent journal 仍运行、只持久化到 699 且 `result_sequence=null` 的只读证据。现场没有证据能判定具体失败分支。
- 因此修复针对可复现的 Agent/控制面状态机缺口，不将某条源码早退推断为该部署的确定根因，也不把节点 current release 当作控制面终态。

## Agent 与 executor

- `agent/src/task_handler.rs::monitor_privileged_release` 先以 journal 游标请求最多 128 帧，持久化连续帧并排空输出/事件后，再查询 `ReleaseStatus`。只有 executor terminal 且最后序号不超过 Agent 持久化序号，才完成 journal 并发送 TaskResult。
- 输出查询与状态查询的 IPC 错误已有 250ms 到 5s 的退避重试；`agent/tests/privileged_release_recovery.rs` 已覆盖瞬时 RPC 错误及 Agent 重启后接管同一 job。这些现有能力保留，不重复实现。
- 输出序号异常、外部帧持久化、journal 保存、事件重建、outbox drain 等本地步骤失败会直接退出 monitor；收到 ReleaseExited 后 `complete_task` 失败也会退出，完成后的 journal/outbox/result 错误则被忽略。每任务 monitor 锁竞争使用 `try_lock` 后直接返回，无法保证等待者在当前 monitor 退出后接管。
- `TaskJournal` 当前不保存 deployment/target run 标识；这些字段来自 `privileged-release-task.json`。reconcile 在 journal 已处于 `PrivilegedRelease` 而 descriptor 缺失或无法解析时，不会加入 monitor 列表，也不会生成诊断，形成静默遗留运行态。新 journal 可用可选默认字段保存关联标识以兼容旧记录；旧记录可从有效 descriptor 回填。
- 若 `JournalStore::recover` 因核心 journal 缺失/不可解析失败，reconcile 只能取得受控目录中的 task ID，不能可信取得 payload digest、输出游标或 TaskResult 序号；executor ReleaseOutput/ReleaseStatus 要求 payload digest。因此该场景只能 fail-closed 产生 Unknown 诊断，不能自动查询 executor 或合成 interrupted TaskResult；本方案仅自动收敛“核心 journal 可解析、关联字段或 descriptor 缺失/损坏”的场景。
- executor 输出超限会把 durable job 终结为 failed、原因 `output_limit_exceeded` 并置 `output_truncated=true`。Agent 把该标记当作输出尚未完整，当前终态分支持续等待，可能造成无进展轮询。
- `TaskJournal` 已持久化 `transfer_phase`、`external_output_sequence`、`result_sequence` 和终态错误码；`send_result` 已使用 durable log delivery/outbox，重连可重放已持久化结果。release task 携带 `deployment_id`，特权上下文携带 `target_run_id`，monitor 可与 task/job 标识组成内部诊断关联键。实现优先复用这些事实和恢复路径。
- 测试已有 300 帧多页成功用例、RPC 暂时失败恢复、disconnect/reconcile 接管及无重复 ReleaseStart 用例；缺少本地 journal/outbox 故障、monitor 所有权竞态、output-truncated 收敛及特定 699→721 缺口场景。

## 控制面

- `api/src/agents/dispatcher.rs` 在 artifact download lease 派发时将 target run 设为 `downloading/artifact_download`。
- Agent 的通用 TaskState `Running` 早于 artifact download，不足以作为 release 已开始的信号。远端脚本开始后已有协议 `TaskProgress`/`DeployEvent`，因此控制面可在 release 阶段收到脚本开始事件时把 target run 推进为 `running/release`，无需新增 wire 消息。
- 当前 `handle_progress` 持久化事件但不会推进 target run；`handle_result` 才终结 run 与 deployment。
- protocol v18+ 的 receipt 可拒绝同序号不同内容，并在 receipt 已 committed 时跳过消息处理。但若业务投影已完成而 receipt 提交失败，重放会再次执行 `apply_result`；task/run 的更新时间或版本可能重复变化。需要让结果投影在部分失败后也可安全重放。
- target run 的可用状态字段和 UI label 已包含 `downloading`、`running` 及终态，无需新状态枚举。使用条件更新避免重复事件提升版本。

## 设计决策

1. **不重启 release 工作负载。** 恢复只查询相同 `job_id` 与 payload digest，从最后一个 durable 输出序号续传；不重新发送 ReleaseStart，不生成新 job。
2. **区分等待更多输出与输出已不可恢复。** transient IPC/本地写入失败按退避继续；正常分页追平后处理 executor terminal。明确的 `output_limit_exceeded` 作为失败终态并保留可读取输出；无法恢复的序号/数据完整性冲突作为中断终态，不能一直把任务留在 running，也不能假报成功。
3. **保证 monitor 所有权可接续。** 同一任务最多一个 monitor 执行；并发接管者应等待当前 owner 收敛或退出后重新读取 journal，而不是因锁竞争无声返回。
4. **阶段切换依赖权威进度事件。** 只用 `stage=release` 的远端脚本开始 DeployEvent 推进 target run；不使用早于 artifact download 的通用 TaskState Running。
5. **结果投影采用可重放语义。** 相同终态消息重复执行必须使 task/run/deployment 和关联状态稳定；冲突或迟到的终态拒绝覆盖。receipt 提交失败后再投递相同消息，必须能补齐未完成投影且不重复修改已完成事实。
6. **默认不加 migration 或协议版本。** 先复用现有 TaskProgress、TaskResult、receipt、journal 和 target run 字段；只有 plan/实现验证证明能力不足时才重新评估。
7. **诊断在受控边界内可关联。** 新 journal 持久化 deployment/target run 标识，旧 journal 从有效 descriptor 回填；核心 journal 可信但两处关联值都不可用时，以 task/job 关联并显式报告缺失字段，只观测原 job，终态后报告 interrupted。核心 journal 本身无效时仅产生 Unknown 诊断，不进行 executor 查询或自动终态投影。Agent 内部诊断包含可确认的失败阶段、错误类别、Agent/executor 序号和终态及恢复动作。外部诊断继续遵守既有白名单，不公开内部 task/job 标识；所有层均不记录 Env、凭证、授权正文、业务输出正文或不必要绝对路径。

## 排除方案

- 不在 `TaskState::Running` 时直接把 target run 标为 release：该事件发生在 artifact download 之前。
- 不以 executor `succeeded` 单独跳过输出接收、事件解析和 journal/outbox 收尾。
- 不因 monitor 出错重新执行 ReleaseStart；不手工更新线上任务状态。
- 不为修复本问题修改业务应用或其脚本，也不扩大为通用 deployment 重试/自动回滚框架。
