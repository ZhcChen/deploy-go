# 特权 Release 终态可靠收敛任务

输入：`spec.md`、`plan.md`
状态：待实施

## 维护与执行约束

- 本目录是本问题唯一规格入口；实施前先核对 `spec.md`、`plan.md`、本文件及相关恢复 runbook。
- 仅修改 Deploy Go 自身的 Agent、executor、API、测试和文档；不修改业务应用，不操作真实节点，不重发部署或手工改写线上状态。
- 恢复必须续接原 `job_id` 和 payload digest，不再次发送 ReleaseStart；除非实施证据证明现有协议/状态字段不足，不新增 wire 字段、migration 或外部 API。
- 保留已有测试覆盖；只补足故障注入、竞态、输出截断、状态推进及结果重放缺口，不重复实现已验证的 happy path。
- `[P]` 仅用于无未完成依赖且文件范围不交叉的任务。本计划按 Agent 合同、控制面投影和综合复核顺序执行，无并行写入任务。
- 实施时逐项维护本清单状态与验证证据；独立审查和真实测试不能由结构检查、analyze/converge 或主 agent 自评替代。

## 阶段一：US1（P1）release 结果可靠收敛

独立验收：Agent 重启、短暂 IPC/本地持久化或 outbox 故障后，监控从原 job 与最后 durable 序号恢复；输出超限或不可恢复完整性错误不无限等待、不误报成功、不再次启动 release；关联 descriptor 丢失时也有明确诊断且不会静默遗漏。

- [ ] T001 [US1] FR-001、FR-003、FR-004、FR-006：在 `agent/src/journal.rs::TaskJournal` 增加带 serde 默认值且 None 不序列化的可选 `deployment_id`/`target_run_id`，兼容缺少新字段的旧 journal；在 `agent/src/task_handler.rs` 发送 ReleaseStart 前持久化这两个关联值，并分类 monitor 的可重试/不可恢复错误、修正本地 frame/journal/event/outbox/complete_task 错误处理及 monitor owner 接管。内部结构化诊断关联 deployment/target run/task/原 job、失败阶段/错误类别、Agent 已持久化输出序号、可确认的 executor 终态/最后序号和恢复动作；未知值标记 unavailable，不推断；外部诊断不公开内部 task/job 标识。依赖：无。验收：新增关联字段先于 ReleaseStart durable 保存，旧 journal 可读取；持久化暂时失败不会静默结束监控；owner 退出后可重新读取 journal 接管；不产生敏感正文。
- [ ] T002 [US1] FR-001、FR-004、FR-006、SC-003、SC-007：在 `agent/src/task_handler.rs` 处理核心 journal 可解析时的 descriptor 缺失、损坏或与 journal 关联值冲突；旧 journal 从有效 descriptor 回填关联值并持久化。只有核心 journal 仍提供可信 task_id、payload digest、transfer phase 与游标时，reconcile、重连和取消恢复才可在两处关联元数据均不可用后，以原 task/job 和 digest 只查询同一 executor job、接收可读输出并观察状态；因无法安全重建事件，terminal 后以稳定错误码完成 interrupted/TaskResult，并显式记录缺失关联字段。核心 journal 缺失/不可解析时沿用 fail-closed Unknown，不查询 executor、不合成结果或重启，并记录 task-id 诊断。依赖：T001。验收：两种损坏场景均无 ReleaseStart、替代 job、自动取消或成功推断；可信核心 journal 场景 terminal 后非成功收敛，核心损坏场景不伪造终态。
- [ ] T003 [US1] FR-002、FR-004：在 `agent/src/task_handler.rs` 区分输出分页未追平、明确 `output_limit_exceeded`/`output_truncated` 与永久序号/数据完整性冲突；先消费仍可读取的连续帧，再将输出超限收敛为带稳定原因的失败，永久完整性错误收敛为非成功中断，正常终态仍须满足输出游标门槛。依赖：T002。验收：截断输出不会等待不存在的帧；普通分页和终态门槛不被放宽；本地暂时不可写仍保留恢复阶段。
- [ ] T004 [US1] FR-001–FR-004、FR-006–FR-007、SC-001–SC-003、SC-005、SC-007：在 `agent/src/journal.rs`、`agent/src/task_handler.rs` 单测及 `agent/tests/privileged_release_recovery.rs` 覆盖旧 journal 反序列化/descriptor 回填、核心 journal 缺失/损坏的 fail-closed Unknown、reconcile/取消恢复中的 descriptor 缺失或损坏、699→721 接管、短暂故障、Agent 重启、monitor owner 退出/竞争、输出超限、序号缺口及 outbox 重放；仅在需要验证 executor 已有持久化事实时调整 `agent-executor/tests/release_lifecycle.rs`。断言 ReleaseStart 未重复、仅产生一个逻辑 TaskResult、核心损坏时不发送 executor 请求、不合成结果，输出和诊断边界符合规格；内部诊断关联 deployment/target run/task/原 job，外部诊断不公开 task/job 标识，样本不含 Env/密钥/授权正文/业务输出正文/不必要绝对路径。依赖：T001–T003。验收：相关测试分别证明 SC-001–SC-003、SC-005、SC-007，且不靠节点 current release 推断控制面成功。

## 阶段二：US2（P1）运行状态和未决结果可辨识

独立验收：只有可信 release 脚本开始事件才将 target run 从 `downloading/artifact_download` 推进；有效 TaskResult 后 task、run、deployment 一致收敛；重复、冲突或迟到消息不会覆盖终态或重复更新时间/版本。

- [ ] T005 [US2] FR-005、SC-004：在 `api/src/agents/dispatcher.rs::handle_progress` 识别 `stage=release` 的远端脚本开始 DeployEvent，并仅在 target run 仍为 `downloading` 时条件推进到 `running/release`；通用 TaskState `Running` 不推进阶段，重复事件不改变已稳定投影。依赖：T004（Agent 结果/进度合同稳定）。验收：首个有效开始事件推进阶段，早到/重复/终态后的旧进度均不造成错误阶段或终态回退。
- [ ] T006 [US2] FR-005、FR-008、SC-004、SC-006：核对 `api/src/agents/dispatcher.rs::handle_result`、`apply_result` 与 task-event receipt 的事务边界，使相同 TaskResult 重放可补齐未完成投影但不重复改变已完成 task/run/deployment 的状态版本或更新时间；拒绝同序号不同内容及终态后的冲突/迟到结果。依赖：T005。验收：receipt 提交失败后的重放收敛到同一终态且稳定字段不变；冲突和迟到消息不覆盖已确认投影。
- [ ] T007 [US2] FR-005–FR-008、SC-004、SC-006：在 `api/tests/agent_dispatcher.rs` 与 `api/tests/two_stage_deployment.rs` 补足 release 开始阶段推进、通用 Running 不提前推进、成功/失败/取消/中断结果投影、重复事件和 TaskResult、receipt 故障重放、序号内容冲突、终态后迟到消息及安全诊断断言。依赖：T005、T006。验收：测试证明 artifact download → release → 终态状态机正确；重复投影不改版本/更新时间；deployment/target run/task 关联沿用现有关系，外部诊断不泄露内部 task/job 标识；样本不含敏感值或业务输出正文。

## 阶段三：恢复手册、综合验证与交付

- [ ] T008 FR-004、FR-006–FR-007、SC-003、SC-005、SC-007：更新 `docs/runbooks/deployment-recovery.md` 与 `docs/runbooks/agent-recovery.md`，说明如何通过 deployment/target 标识关联受控 Agent journal 中的 task 与原 durable job；覆盖 descriptor 缺失/损坏时的原 job 观测和 interrupted 收敛，以及核心 journal 缺失/不可解析时 Unknown、无 digest 不查询 executor、不伪造终态的限制；保持外部诊断的 task/job 标识白名单，不要求读取 Env/密钥/业务输出；明确不得手工标成功、重发 release 或据节点健康/current release 晋级，并保留旧 Agent 不得接管活跃新格式任务的回滚边界。依赖：T004、T007。验收：手册不包含未经授权的节点操作命令，且与实现状态机和诊断安全边界一致。
- [ ] T009 FR-001–FR-008、SC-001–SC-007：按 `specs/004-privileged-release-monitor-recovery/quickstart.md` 运行 `cargo test -p deploy-go-agent --lib`、`agent/tests/privileged_release_recovery.rs`、`agent/tests/image_release.rs`、`agent-executor/tests/release_lifecycle.rs`、`api/tests/agent_dispatcher.rs`、`api/tests/two_stage_deployment.rs` 对应的 Agent、executor、API 聚焦测试及 `cargo fmt --all -- --check`、相关 crates clippy、`git diff --check`；按需运行 `docs/runbooks/local-development.md` 中的隔离 Docker/Linux 验证。安排独立复核者检查 Agent 状态机/竞态、journal 向后兼容、API 事务与幂等边界、敏感信息和协议兼容；修复有效发现后重跑受影响验证。依赖：T004、T007、T008。验收：逐项记录通过、失败、跳过和未验证项；高风险独立复核完成，真实节点/生产控制面未被作为本地验收目标。
- [ ] T010 FR-001–FR-008、SC-001–SC-007：对照 `specs/004-privileged-release-monitor-recovery/spec.md`、`plan.md`、`tasks.md` 及最终实现执行 Spec Kit converge；仅追加确有必要的遗漏任务，并记录已完成任务的测试/审查证据。依赖：T009。验收：无未解释的规格覆盖缺口；若发现协议、migration 或文件范围必须扩张，先更新 `specs/004-privileged-release-monitor-recovery/` 唯一规格入口并重新 analyze，不在实施中静默扩张。
- [ ] T011 FR-001–FR-008、SC-001–SC-007：复核最终 staged diff 仅包含 `specs/004-privileged-release-monitor-recovery/` 及本计划授权的 Deploy Go 实现/测试/runbook 文件，执行 `git diff --check`、`git diff --cached --check` 和 staged diff 检查；按项目约定以简体中文小闭环提交并 fetch/rebase/push `main`，不得强推或绕过 hooks。依赖：T010。验收：提交内容与规格一致、工作区既有无关改动未被纳入；不触发真实部署。

## 依赖顺序

```text
T001 -> T002 -> T003 -> T004 -> T005 -> T006 -> T007 -> T008 -> T009 -> T010 -> T011
```

US1 是 US2 的前置阶段：先稳定 Agent 的进度/结果交接合同，再调整控制面投影。每个阶段目前没有无依赖且文件不交叉的并行写入窗口；独立审查在实现 diff 可审查后执行，不与目标文件并行写入。

## 并行执行示例

- US1：T001–T003 依次修改 `agent/src/journal.rs` 和 `agent/src/task_handler.rs`，并共享恢复状态机边界；T004 在实现稳定后补测试，不与源文件写入并行。
- US2：T005 与 T006 都修改 `api/src/agents/dispatcher.rs` 且先确定阶段投影边界再处理终态幂等，串行执行；T007 在两项实现稳定后覆盖 API fixture。
- 交付：T008 runbook 需反映已实现行为；T009 独立审查与验证依赖全部实现和文档完成。当前计划无安全的并行写入任务。

## 实施策略

先交付 US1 最小闭环（T001–T004）：兼容持久化关联、同 job 恢复、明确终态分类和故障注入测试。随后交付 US2（T005–T007）：阶段推进与 TaskResult 可重放投影。最后同步 runbook，执行完整测试与独立复核，再 converge、提交。Agent 成功、节点健康或 current release 均不得绕过控制面有效 TaskResult。

## 验收证据

待实施。实施时逐项记录实际命令、结果、关键状态边界、失败/跳过/未验证项、独立复核结论及对应提交；本文件的任务勾选和 Spec Kit 结构门禁不代表实现验收通过。
