# 特权 Release 终态可靠收敛实施方案

规格：spec.md
状态：规划完成；实施未开始

## Technical Context

- Rust workspace 中 Agent durable task/journal 与 privileged monitor 位于 `agent/src/task_handler.rs`、`agent/src/executor.rs`、`agent/src/log_delivery.rs`；root executor durable output/job 状态位于 `agent-executor/src/release_job.rs`。
- 现有 Agent monitor 以 `job_id=release_<task_id>`、payload digest 和 `external_output_sequence` 续传输出，单批最多 128 帧。IPC output/status 请求已有 250ms 到 5s 的退避重试；本次缺口集中在本地 frame/journal/event/outbox 错误和完成任务错误的静默退出、monitor 所有权竞争，以及输出超限标记导致的终态等待。
- 协议已有 `TaskProgress`/`DeployEvent`、`TaskState`、`TaskResult`、v18 task-event receipt 与 durable log outbox；不新增消息类型或 wire 字段。release 远端脚本开始事件是 target run 离开下载态的权威信号，通用 TaskState Running 发生得更早。
- API 接收与投影位于 `api/src/agents/dispatcher.rs`；target run 使用现有 status/phase，Web 页面读取既有状态字段。相同 TaskResult 在 receipt 未提交时可能再次执行投影，当前 target run/task 更新时间或 version 会重复变化。
- 按 `data-model.md` 收敛：只在所有可读取输出 durable 保存后依据 executor terminal 结果完成任务；输出超限形成明确失败，不等待缺失帧；不可恢复完整性冲突形成 interrupted；同结果投影可安全重放。
- 默认不改 migration、协议版本、OpenAPI 或业务应用。若实现分析证明需要扩大上述边界，先更新 plan/tasks 并运行 analyze，不在实施中静默扩张。

## Constitution Check

- 只修改 Deploy Go 的 Agent、executor、API、测试和部署恢复文档，不改任何业务应用、脚本或发布物。
- 遵循 `docs/runbooks/deployment-recovery.md` 与 `docs/runbooks/agent-recovery.md` 的只读现场边界；不操作节点、不手改数据库、不重启服务、不发起/重放部署。
- 高风险部署状态机变更须做 Agent/API 聚焦真实测试与独立复核。结构门禁、analyze/converge 或主 agent 自审不替代测试和独立代码复核。
- 沿用现有 durable job、journal、outbox 和 TaskResult，不重发 ReleaseStart；不凭 executor 成功跳过输出与事件收尾。
- 本机可运行 Rust 测试；若变更触及 Linux-only cgroup/进程回收，再用本机隔离 Docker 验证。无需真实节点或生产控制面。
- 暂无新增 migration 或外部 API 契约，不触发 migration Git hook 门禁。真实部署/重启仍需具体环境授权。

## 实现方案与文件范围

### Agent monitor 收敛

在 `agent/src/task_handler.rs` 明确区分可重试 monitor 错误、终态业务结果和不可恢复完整性错误。输出持久化、journal 保存、事件重建、日志/事件 outbox 与 `complete_task` 的瞬时失败不得直接遗弃活动 monitor；按当前 durable 游标退避重试，或由同任务 monitor owner 在前一个 owner 退出后接管。每个接管者重新读取 journal 和同一个 executor job，终态任务只补发已有 TaskResult。

monitor 锁竞争不再让后续接管者无声丢弃；在锁后重新检查 journal，保证活动 owner 收敛或退出后仍有一个 waiter 可继续。不可重试的序号/完整性冲突写入安全诊断并以 `interrupted` 收敛；若 local storage 暂不可写，保留 `PrivilegedRelease` 并退避重试，不假造终态。

当 executor 明确返回 `output_limit_exceeded`/`output_truncated`，按失败结果处理并排空所有仍可读取且连续的帧；不得把“日志已被 executor 丢弃”当作等待更多帧。正常未截断的 terminal job 仍须满足 `last_sequence <= external_output_sequence` 才可完成。

复用 `agent/src/executor.rs` 与 `agent/src/log_delivery.rs` 现有原子 journal/outbox API；除非测试证明缺少注入点，不新增通用存储抽象。在 `agent/src/journal.rs::TaskJournal` 增加可选、带 serde 默认且为 `None` 时不序列化的 `deployment_id`/`target_run_id` 字段，并在发送 ReleaseStart 前写入；新版本可读取旧 journal。旧 journal 从有效 `privileged-release-task.json` 回填，若两处标识冲突则作为不可恢复完整性错误处理。修复 monitor 提前退出和重试路径时，Agent 内部结构化诊断应关联 deployment、target run、task、原 `job_id`、失败阶段/错误类别、Agent 已持久化输出序号、executor 终态与最后序号及恢复动作；executor 尚未返回状态时省略对应字段，不推断值。若核心 journal 可解析但关联字段和 descriptor 均缺失/损坏，reconcile、重连和取消恢复均不得静默跳过：以 `task_id` 推导的原 job_id 和可信 payload digest 仅执行 ReleaseOutput/ReleaseStatus 观测，排空可读取输出并等待原 job 终态；由于无法安全重建 DeployEvent，终态后以稳定错误码报告 interrupted，不能重发 ReleaseStart 或伪报成功。若核心 journal 本身缺失/不可解析，payload digest/游标/结果序号不可信，Agent 只记录 keyed-by-task 的 Unknown/恢复诊断，不向 executor 发送无可信 digest 的请求、不伪造 TaskResult、不标记成功或重新启动；此情形不纳入自动 terminal 收敛。对既有旧任务，缺失的 deployment/target 标识显式标为 unavailable。外部诊断继续遵守现有白名单，不公开内部 task/job 标识或原始 executor 文本。任何层都不得记录 Env、凭证、授权正文、业务输出正文或不必要的内部绝对路径。新 journal 格式只保证新版本读取旧 journal；遵守 `docs/runbooks/agent-recovery.md` 现有回滚边界，不让旧二进制接管仍活跃的新格式任务。

### API 阶段与终态投影

在 `api/src/agents/dispatcher.rs::handle_progress` 中识别 `stage=release` 的远端脚本开始 DeployEvent（如 `deploy.preflight.started`、`deploy.module.started` 或 `deploy.step.started`），仅当 run 仍是 `downloading` 时条件更新为 `running/release`。不以 TaskState Running 更新，因为它可能先于 artifact download。重复事件应保持投影 version/updated_at 稳定。

使 `handle_result`/`apply_result` 对相同 TaskResult 重放安全：相同结果完成尚未完成的下游投影但不重复变更已完成状态的 version/timestamp；冲突或终态后的迟到结果不得覆盖。覆盖 API 已有 receipt、事件序号与 result_json 路径，确保 receipt 写入重试不会让状态投影重复副作用。

不新增 target run 状态枚举、schema 或外部响应字段。若需新增可见错误码，只在已有受控诊断投影中增加白名单，不输出原始 executor 文本、Env、授权或业务日志正文。

### 测试与运行手册

- Agent 单测/集成测覆盖 `agent/src/task_handler.rs` privileged bridge、`agent/tests/privileged_release_recovery.rs`、`agent/tests/image_release.rs`；保留现有 300 帧 happy path、RPC 暂时失败和重连恢复，不重复实现。
- `agent-executor/tests/release_lifecycle.rs` 保留输出超限 durable job 终态测试；新增/调整 Agent bridge fixture 验证截断 job 最终失败而非无限等待。
- API 测试聚焦 `api/tests/agent_dispatcher.rs` 与 `api/tests/two_stage_deployment.rs`，覆盖下载到 release 的可信阶段迁移、相同/冲突/迟到 TaskResult、receipt 提交失败重放及部分投影恢复。
- 更新 `docs/runbooks/deployment-recovery.md` 与 `docs/runbooks/agent-recovery.md`：补充 Agent executor 成功但控制面无 TaskResult、输出超限和 monitor 本地故障的只读判别；说明如何用 deployment/target 标识关联受控 Agent journal 与原 job，同时保持外部诊断不公开内部 task/job 标识；明确不重放部署、不手工标成功。

## 阶段与依赖

1. **Agent 监控可靠性（US1，FR-001–FR-004、FR-006–FR-007）**：修复本地失败分类、monitor 接管/锁竞争和 truncated 终态；先完成 Agent 与 executor fixture。
2. **控制面状态收敛（US2，FR-005、FR-008）**：增加基于 release 开始事件的 target run 更新；使 TaskResult 重放及部分投影恢复幂等；完善 API fixture。
3. **手册、综合验证与复核（FR-006）**：同步恢复步骤，运行聚焦测试/格式/静态检查，执行独立高风险代码复核；修复 findings 后重新验证。

阶段 2 可在阶段 1 Agent 合同稳定后实施；同一文件不并行写入。阶段 3 依赖前两阶段完成。实现中发现新 schema/protocol 需求时暂停并更新唯一入口，不另开平行计划。

## 验证与验收

按 `quickstart.md` 执行：

1. Agent 从序号 699 恢复 700–721，能区分 IPC transient、local durable 写入失败、序号缺口和 output truncated；结果、游标和重复 ReleaseStart 断言符合规格。monitor 故障诊断可在 Agent 内部关联 deployment、target run、task 与原 job，并携带已知序号/终态，不包含敏感正文；外部诊断不泄露内部标识。
2. monitor 重连、重启或并发接管时只查询原 job；active monitor 意外退出后存在可验证的再接管路径，逻辑 TaskResult 唯一且 durable outbox 可重放。
3. API 收到首个 release 脚本开始事件后才离开 artifact_download；相同 TaskResult 重放、receipt 写入失败和部分投影失败均能继续收敛，不重复推进 version/timestamp；冲突/迟到结果不覆盖终态。
4. 运行 Agent/executor/API 指定测试、`cargo fmt --all -- --check`、相关 crates 的 clippy 与 `git diff --check`。高风险改动由独立复核者审查 Agent 状态机、API事务/幂等边界、敏感日志和协议兼容。
5. 若测试覆盖 Linux-only executor 行为，再按 `docs/runbooks/local-development.md` 使用隔离 Docker；未涉及该路径时不额外搭建容器。
6. 新 journal 的关联字段在 ReleaseStart 前 durable 保存；旧格式 journal 可反序列化并从 descriptor 回填；核心 journal 可信但 descriptor/关联字段缺失或损坏时 reconcile/取消恢复有明确诊断、只观测原 job，并在 executor terminal 后报告 interrupted，不静默遗漏或重启工作负载。核心 journal 本身缺失/不可解析时只产生 Unknown 诊断，不尝试无 digest 查询或合成终态。

真实节点、正式控制面、共享数据库及业务应用均不作为本地验收目标；不重发 `deployment_01M4QAXB5M38A1WZGJHQBPVVY2`。所有测试结果在实施阶段记录到 tasks/review；计划本身的结构检查不能当作实现通过。

## 运行与恢复

实现前后均遵守 `docs/runbooks/deployment-recovery.md`。若 Agent 无法安全证明 terminal，保留可追查的 interrupted/恢复诊断，不自动再次执行 release；只有正式 TaskResult 投影成功才允许控制面呈现部署成功。运行手册同步在代码实施闭环内完成。本轮仅规划，未执行节点恢复或部署。
