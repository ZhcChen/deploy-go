# 特权 Release 终态可靠收敛任务

输入：`spec.md`、`plan.md`
状态：全部任务完成（T001–T017），converge 结论为 converged；现场部署未恢复，节点侧生效需 Agent/executor 成对发布并另行授权验证
交接：`docs/handoffs/2026-10-12-privileged-release-monitor-recovery-handoff.md`

## 维护与执行约束

- 本目录是本问题唯一规格入口；实施前先核对 `spec.md`、`plan.md`、本文件及相关恢复 runbook。
- 仅修改 Deploy Go 自身的 Agent、executor、API、测试和文档；不修改业务应用，不操作真实节点，不重发部署或手工改写线上状态。
- 恢复必须续接原 `job_id` 和 payload digest，不再次发送 ReleaseStart；除非实施证据证明现有协议/状态字段不足，不新增 wire 字段、migration 或外部 API。
- 保留已有测试覆盖；只补足故障注入、竞态、输出截断、状态推进及结果重放缺口，不重复实现已验证的 happy path。
- `[P]` 仅用于无未完成依赖且文件范围不交叉的任务。本计划按 Agent 合同、控制面投影和综合复核顺序执行，无并行写入任务。
- 实施时逐项维护本清单状态与验证证据；独立审查和真实测试不能由结构检查、analyze/converge 或主 agent 自评替代。

## 阶段一：US1（P1）release 结果可靠收敛

独立验收：Agent 重启、短暂 IPC/本地持久化或 outbox 故障后，监控从原 job 与最后 durable 序号恢复；输出超限或不可恢复完整性错误不无限等待、不误报成功、不再次启动 release；关联 descriptor 丢失时也有明确诊断且不会静默遗漏。

- [x] T001 [US1] FR-001、FR-003、FR-004、FR-006：在 `agent/src/journal.rs::TaskJournal` 增加带 serde 默认值且 None 不序列化的可选 `deployment_id`/`target_run_id`，兼容缺少新字段的旧 journal；在 `agent/src/task_handler.rs` 发送 ReleaseStart 前持久化这两个关联值，并分类 monitor 的可重试/不可恢复错误、修正本地 frame/journal/event/outbox/complete_task 错误处理及 monitor owner 接管。内部结构化诊断关联 deployment/target run/task/原 job、失败阶段/错误类别、Agent 已持久化输出序号、可确认的 executor 终态/最后序号和恢复动作；未知值标记 unavailable，不推断；外部诊断不公开内部 task/job 标识。依赖：无。验收：新增关联字段先于 ReleaseStart durable 保存，旧 journal 可读取；持久化暂时失败不会静默结束监控；owner 退出后可重新读取 journal 接管；不产生敏感正文。
- [x] T002 [US1] FR-001、FR-004、FR-006、SC-003、SC-007：在 `agent/src/task_handler.rs` 处理核心 journal 可解析时的 descriptor 缺失、损坏或与 journal 关联值冲突；旧 journal 从有效 descriptor 回填关联值并持久化。只有核心 journal 仍提供可信 task_id、payload digest、transfer phase 与游标时，reconcile、重连和取消恢复才可在两处关联元数据均不可用后，以原 task/job 和 digest 只查询同一 executor job、接收可读输出并观察状态；因无法安全重建事件，terminal 后以稳定错误码完成 interrupted/TaskResult，并显式记录缺失关联字段。核心 journal 缺失/不可解析时沿用 fail-closed Unknown，不查询 executor、不合成结果或重启，并记录 task-id 诊断。依赖：T001。验收：两种损坏场景均无 ReleaseStart、替代 job、自动取消或成功推断；可信核心 journal 场景 terminal 后非成功收敛，核心损坏场景不伪造终态。
- [x] T003 [US1] FR-002、FR-004：在 `agent/src/task_handler.rs` 区分输出分页未追平、明确 `output_limit_exceeded`/`output_truncated` 与永久序号/数据完整性冲突；先消费仍可读取的连续帧，再将输出超限收敛为带稳定原因的失败，永久完整性错误收敛为非成功中断，正常终态仍须满足输出游标门槛。依赖：T002。验收：截断输出不会等待不存在的帧；普通分页和终态门槛不被放宽；本地暂时不可写仍保留恢复阶段。
- [x] T004 [US1] FR-001–FR-004、FR-006–FR-007、SC-001–SC-003、SC-005、SC-007：在 `agent/src/journal.rs`、`agent/src/task_handler.rs` 单测及 `agent/tests/privileged_release_recovery.rs` 覆盖旧 journal 反序列化/descriptor 回填、核心 journal 缺失/损坏的 fail-closed Unknown、reconcile/取消恢复中的 descriptor 缺失/损坏/关联冲突、699→721 接管、短暂故障、Agent 重启、monitor owner 退出/竞争与 reconcile 不阻塞、输出超限、序号缺口、终态逆序、意外截断、durable 帧缺失/损坏与 journal 暂时不可读；仅在需要验证 executor 已有持久化事实时调整 `agent-executor/tests/release_lifecycle.rs`。断言 ReleaseStart 未重复、仅产生一个逻辑 TaskResult、核心损坏时不发送 executor 请求、不合成结果，输出和诊断边界符合规格；内部诊断关联 deployment/target run/task/原 job，外部诊断不公开 task/job 标识，样本不含 Env/密钥/授权正文/业务输出正文/不必要绝对路径。依赖：T001–T003。验收：相关测试分别证明 SC-001–SC-003、SC-005、SC-007，且不靠节点 current release 推断控制面成功。monitor 层 `send_result`/`drain_outputs` 的 outbox 交付失败注入由 T012 补齐。

## 阶段二：US2（P1）运行状态和未决结果可辨识

独立验收：只有可信 release 脚本开始事件才将 target run 从 `downloading/artifact_download` 推进；有效 TaskResult 后 task、run、deployment 一致收敛；重复、冲突或迟到消息不会覆盖终态或重复更新时间/版本。

- [x] T005 [US2] FR-005、SC-004：在 `api/src/agents/dispatcher.rs::handle_progress` 识别 `stage=release` 的远端脚本开始 DeployEvent，并仅在 target run 仍为 `downloading` 时条件推进到 `running/release`；通用 TaskState `Running` 不推进阶段，重复事件不改变已稳定投影。依赖：T004（Agent 结果/进度合同稳定）。验收：首个有效开始事件推进阶段，早到/重复/终态后的旧进度均不造成错误阶段或终态回退。
- [x] T006 [US2] FR-005、FR-008、SC-004、SC-006：核对 `api/src/agents/dispatcher.rs::handle_result`、`apply_result` 与 task-event receipt 的事务边界，使相同 TaskResult 重放可补齐未完成投影但不重复改变已完成 task/run/deployment 的状态版本或更新时间；拒绝同序号不同内容及终态后的冲突/迟到结果。依赖：T005。验收：receipt 提交失败后的重放收敛到同一终态且稳定字段不变；冲突和迟到消息不覆盖已确认投影。
- [x] T007 [US2] FR-005–FR-008、SC-004、SC-006：在 `api/tests/agent_dispatcher.rs` 与 `api/tests/two_stage_deployment.rs` 补足 release 开始阶段推进、通用 Running 不提前推进、成功/失败/取消/中断结果投影、重复事件和 TaskResult、receipt 故障重放、序号内容冲突、终态后迟到消息及安全诊断断言。依赖：T005、T006。验收：测试证明 artifact download → release → 终态状态机正确；重复投影不改版本/更新时间；deployment/target run/task 关联沿用现有关系，外部诊断不泄露内部 task/job 标识；样本不含敏感值或业务输出正文。补充记录：取消/中断终态投影与 `apply_result` 终态冲突分支此前无测试驱动，本轮新增 `canceled_and_interrupted_release_results_project_target_run_and_deployment` 覆盖两类终态投影、重放幂等和迟到结果被拒（断言稳定错误码 `agent_event_after_terminal`，确认命中终态冲突分支而非回执层）。

## 阶段三：恢复手册、综合验证与交付

- [x] T008 FR-004、FR-006–FR-007、SC-003、SC-005、SC-007：更新 `docs/runbooks/deployment-recovery.md` 与 `docs/runbooks/agent-recovery.md`，说明如何通过 deployment/target 标识关联受控 Agent journal 中的 task 与原 durable job；覆盖 descriptor 缺失/损坏时的原 job 观测和 interrupted 收敛，以及核心 journal 缺失/不可解析时 Unknown、无 digest 不查询 executor、不伪造终态的限制；保持外部诊断的 task/job 标识白名单，不要求读取 Env/密钥/业务输出；明确不得手工标成功、重发 release 或据节点健康/current release 晋级，并保留旧 Agent 不得接管活跃新格式任务的回滚边界。依赖：T004、T007。验收：手册不包含未经授权的节点操作命令，且与实现状态机和诊断安全边界一致。
- [x] T009 FR-001–FR-008、SC-001–SC-007：按 `specs/004-privileged-release-monitor-recovery/quickstart.md` 运行 `cargo test -p deploy-go-agent --lib`、`agent/tests/privileged_release_recovery.rs`、`agent/tests/image_release.rs`、`agent-executor/tests/release_lifecycle.rs`、`api/tests/agent_dispatcher.rs`、`api/tests/two_stage_deployment.rs` 对应的 Agent、executor、API 聚焦测试及 `cargo fmt --all -- --check`、相关 crates clippy、`git diff --check`；按需运行 `docs/runbooks/local-development.md` 中的隔离 Docker/Linux 验证。安排独立复核者检查 Agent 状态机/竞态、journal 向后兼容、API 事务与幂等边界、敏感信息和协议兼容；修复有效发现后重跑受影响验证。依赖：T004、T007、T008。验收：逐项记录通过、失败、跳过和未验证项；高风险独立复核完成，真实节点/生产控制面未被作为本地验收目标。完成证据：最终验证见下“验收证据”；独立复核结论与处置见 `docs/reviews/2026-10-12-privileged-release-monitor-recovery-review.md`（第二次发起的只读子 agent，第一次因会话重启中止、不计入）。
- [x] T010 FR-001–FR-008、SC-001–SC-007：对照 `specs/004-privileged-release-monitor-recovery/spec.md`、`plan.md`、`tasks.md` 及最终实现执行 Spec Kit converge；仅追加确有必要的遗漏任务，并记录已完成任务的测试/审查证据。依赖：T009。验收：无未解释的规格覆盖缺口；若发现协议、migration 或文件范围必须扩张，先更新 `specs/004-privileged-release-monitor-recovery/` 唯一规格入口并重新 analyze，不在实施中静默扩张。 完成证据：在 T013–T017 全部完成、最终验证与独立复核之后执行 converge，逐项对照 FR-001–008、SC-001–007、US1/US2、plan 三组决策与 constitution，无可执行缺口，结论为 converged，converge 本身未追加任何任务。
- [x] T011 FR-001–FR-008、SC-001–SC-007：复核最终 staged diff 仅包含 `specs/004-privileged-release-monitor-recovery/` 及本计划授权的 Deploy Go 实现/测试/runbook 文件，执行 `git diff --check`、`git diff --cached --check` 和 staged diff 检查；按项目约定以简体中文小闭环提交并 fetch/rebase/push `main`，不得强推或绕过 hooks。依赖：T010。验收：提交内容与规格一致、工作区既有无关改动未被纳入；不触发真实部署。 完成证据：仅暂存本任务的 16 个文件，未纳入 `docs/plans/2026-09-18-001-git-source-materialization-plan.md` 与 `docs/plans/2026-09-20-002-agent-git-materialization-ownership-plan.md`；提交哈希见 `git log`；未使用 `--no-verify`，未强推。
- [x] T012 FR-003、SC-001：在 `agent/tests/privileged_release_recovery.rs` 为 monitor 层 `send_result`/`drain_outputs` 的 outbox 与日志交付失败补故障注入用例（现有仅 `agent/src/log_delivery.rs` 自身单测覆盖 outbox 自身逻辑，未驱动 `monitor_privileged_release` 的 `result_delivery`/`event_outbox_delivery` 重试分支）。依赖：T004、T009。验收：注入交付失败后 monitor 保留原恢复阶段并重投同一 job 的 durable 终态，最终只产生一个逻辑 TaskResult；不重复启动 release。完成证据：`output_outbox_failure_retries_without_losing_output_or_restarting_release`（`event_outbox_delivery` 分支）与 `result_outbox_failure_retries_the_durable_terminal_without_a_second_result`（`result_delivery` 分支）。以 outbox 目录 `0o500` 注入写失败（读仍正常，能精确命中交付分支；root 下探测后自动跳过）；断言故障期间不交付输出/终态、journal 保留 `Running + PrivilegedRelease`（或已 durable 的终态且 `result_sequence` 为空）、清除故障后只产生一个 TaskResult、消息序号连续无重复、ReleaseStart 次数为 0。已做变异验证：整体失效 `event_outbox_delivery` 条件，或让终态交付失败后直接丢弃结果，对应用例均失败。

## 阶段四：实施期复核修正

独立复核前的自查与对真实实现的逐项对照中发现的缺陷，均在 `spec.md` 既有 FR/SC 范围内，不新增 wire 字段、migration 或外部 API。

- [x] T013 [US1] FR-002、FR-004、SC-002、SC-003：`truncated` 是 executor 的 job 级标志（终态后每一页都为 true），上一轮的 fake 只在最后一页返回 true，掩盖了缺陷。修复 `agent/src/task_handler.rs::monitor_privileged_release` 的终态输出处理：本轮读到新帧就立即读下一页且不退避；无新帧才退避，连续 `TERMINAL_OUTPUT_STALL_LIMIT`（6）轮仍落后于宣告序号按永久序号冲突收敛为 `interrupted/release_output_sequence_conflict`，无新帧且 `truncated=true` 立即收敛为 `release_output_truncated_unexpectedly`；`agent/src/executor.rs::external_frame_error` 把 `IsADirectory`/`NotADirectory` 也归为永久完整性错误；单元测试构建下压缩退避 25 倍（`monitor_retry_sleep`）。依赖：T003。验收：Agent 重启后观察到“已因输出超限失败且剩余可读帧超过一页”的 job，读完全部页后收敛为 `failed/output_limit_exceeded`；多页终态输出不逐页退避；终态序号长期追不平有上限而非无限等待。证据：`terminal_output_limit_job_reads_every_remaining_page_before_failing`、`terminal_job_that_never_serves_declared_frames_converges_to_interrupted`、`permanent_frame_corruption_…`（新增“目录占用帧路径”形态）、集成测试 `multi_page_terminal_output_is_drained_without_backoff_between_pages`（旧逻辑 8 页耗时约 13 秒，现 3 秒内）；均做过变异验证。
- [x] T014 [US1] FR-003、FR-004、FR-006、SC-001、SC-007：reconcile 在已有 monitor owner 时仅上报并跳过，owner 因旧连接关闭而退出后无人接管；descriptor 暂时读不到（`RetryLater`）时 reconcile 与取消恢复都不启动 monitor，只能等下一次重连。新增 `TaskHandler::recover_privileged_release_monitor` 作为 reconcile 与取消恢复的统一入口：等待旧 owner 退出 → 持锁重读 journal → 带重试地载入并回填 descriptor → 续接原 job；reconcile 不再碰所有权锁，特权任务按 `reconciled()` 的既有语义上报 `Accepted`，由主控幂等重派并以 Duplicate ACK 重新挂接。依赖：T002、T004。验收：旧 owner 退出后由 reconcile 派发的等待者接管并给出唯一终态；descriptor 暂时不可读时 monitor 退避重试、恢复后接管，期间不查询 executor、不重新启动 release。证据：`reconcile_hands_over_to_a_waiting_monitor_after_the_old_owner_exits`、`temporarily_unreadable_descriptor_retries_instead_of_abandoning_the_monitor`、重写 fixture 的 `reconcile_does_not_block_on_an_active_privileged_monitor_owner`（改用生产形态的 `begin_privileged_release`）；均做过变异验证。
- [x] T015 [US2] FR-005、FR-008、SC-004、SC-006：`finish_deployment_for_task` 的单目标原子分支漏掉了旧路径“非成功终态时其余未终态 run 一并收敛”的语义，已在同一事务内补回。测试层面：回执门禁只对协议 ≥ v18 的 Agent 生效，上一轮三处 `committed=0` 实际是对 0 行的空更新，重放用例并未覆盖“回执未提交”。现把 fixture Agent 升到 v18，并断言该 UPDATE 恰好命中 1 行；新增“投影中途崩溃（agent task 已终态、run/父 deployment 仍在运行态）后重放同一 TaskResult 补齐下游投影”的单目标与多目标用例。依赖：T006、T007。验收：重放同一结果补齐未完成投影且不重复改变已完成状态的版本/时间；回执重放测试能因 fixture 失效而失败。证据：`canceled_and_interrupted_release_results_project_target_run_and_deployment`（补强）、`cross_node_prepare_fans_out_independent_releases_and_retry_skips_success`（补强）、`current_connection_events_advance_task_deployment_and_logs_once`（补强）；变异验证显示：撤销 v18 升级时断言报告 `left: 0`，让 `apply_result` 对相同终态提前返回时两条补齐断言均失败。
- [x] T016 [US1] FR-006、SC-005：T004 声称“样本不含 Env/密钥/授权正文/业务输出正文/不必要绝对路径”，但仓库里没有任何测试捕获并检查 tracing 输出，该要求只停留在设计意图。新增 `monitor_diagnostics_and_results_never_expose_secrets_output_or_internal_paths`：捕获一次完整“停滞 → 中断”收敛过程的全部 tracing 输出，断言结构化诊断确实带上 deployment/task/job 关联与恢复动作，同时不含业务输出正文、带凭证的仓库 URL、descriptor 中的工作目录路径与任务根目录路径；并断言对控制面的 `TaskResult` 只带稳定错误码，`summary/data` 不暴露内部 task/job 标识。依赖：T013。验收：用例通过；变异验证（让诊断多输出整个 descriptor）后用例失败并报告 `SENTINEL_REPO_PASSWORD`。注意：该用例最初用线程局部 tracing 订阅者，单独运行通过、整套并发运行时捕获为空（callsite 兴趣缓存），已改为进程级全局订阅者 + 本用例独有哨兵值，并在整套测试下重新做过变异验证。
- [x] T017 [US1] FR-002、FR-003、FR-004、SC-003：独立只读复核（子 agent，第二次发起；第一次因会话重启被中止、无结论）返回 7 条发现，逐条对照代码核实后的处置：(1) 采纳并修复——事件重建的永久数据错误（超长事件行为 `ErrorKind::Other`）在终态分支被无条件重试，HEAD 此处是直接停止，属本轮引入的“永远等待”；现抽出 `permanent_event_data_error`（`InvalidData` 与 `Other`），非终态与终态两个分支都收敛为 `interrupted/release_event_data_invalid`。(2) 采纳并修复——`consecutive_local_errors` 在每轮 ReleaseOutput 成功后被清零，本地存储持续故障时退避永远停在第一档并反复刷日志（变异验证量化：700ms 窗口内 59 次请求 vs 个位数）；改为整轮成功后才清零。(3) 属有意保守，写入 `data-model.md` 与恢复手册：`Succeeded/TimedOut + truncated=true` 一律 `interrupted`。(4) 属实，写入手册：`interrupted` 不会取消 executor 里仍在运行的 job，也不重跑。(5) 为 HEAD 既有代码（prepare 成功分支无 phase 守卫），不在本 feature 终态语义内，记为已知限制，未修改。(6) 有意为之：run 已终态时不被迟到结果覆盖，符合“终态后迟到结果不得覆盖已确认终态”，记入复核记录。(7) 采纳：outbox 故障注入用例加 `RestoreMode` Drop 守卫，断言 panic 时也恢复目录权限；root 下自动跳过的行为已在手册与复核记录中说明。依赖：T013、T014。验收：新增 `oversized_event_line_converges_to_interrupted_instead_of_retrying_forever`、`persistent_local_failure_backs_off_instead_of_retrying_at_the_first_delay`，均做过变异验证（恢复旧判定后超时失败；恢复每轮清零后报告 59 次请求）。

## 依赖顺序

```text
T001 -> T002 -> T003 -> T004 -> T005 -> T006 -> T007 -> T008 -> T009 -> T010 -> T011
T004 + T009 -> T012（测试缺口补齐；必须在 T011 提交前完成，可与 T010 并行）
T003 -> T013；T002 + T004 -> T014；T006 + T007 -> T015；T013 -> T016；T013 + T014 -> T017（实施期复核修正，均须在 T010 converge 之前完成）
```

US1 是 US2 的前置阶段：先稳定 Agent 的进度/结果交接合同，再调整控制面投影。每个阶段目前没有无依赖且文件不交叉的并行写入窗口；独立审查在实现 diff 可审查后执行，不与目标文件并行写入。

## 并行执行示例

- US1：T001–T003 依次修改 `agent/src/journal.rs` 和 `agent/src/task_handler.rs`，并共享恢复状态机边界；T004 在实现稳定后补测试，不与源文件写入并行。
- US2：T005 与 T006 都修改 `api/src/agents/dispatcher.rs` 且先确定阶段投影边界再处理终态幂等，串行执行；T007 在两项实现稳定后覆盖 API fixture。
- 交付：T008 runbook 需反映已实现行为；T009 独立审查与验证依赖全部实现和文档完成。当前计划无安全的并行写入任务。

## 实施策略

先交付 US1 最小闭环（T001–T004）：兼容持久化关联、同 job 恢复、明确终态分类和故障注入测试。随后交付 US2（T005–T007）：阶段推进与 TaskResult 可重放投影。最后同步 runbook，执行完整测试与独立复核，再 converge、提交。Agent 成功、节点健康或 current release 均不得绕过控制面有效 TaskResult。

## 验收证据

最终验证（2026-10-12，macOS 本机，全部为本地 fixture，未连接任何真实节点或控制面）：

- Agent lib 118；Agent 全部集成目标 + 协议 + executor 共 37 个测试目标通过（含 `privileged_release_recovery` 14、`image_release` 7、executor `release_lifecycle` 9）。API `agent_dispatcher` 37、`two_stage_deployment` 24 通过。
- `cargo test --workspace --no-fail-fast`：775 项通过（102 个通过的测试目标）；唯一失败的 3 项为 HEAD 上既有、与本 feature 无关（见下）。
- `cargo fmt --all -- --check`、`git diff --check`、`make spec-kit-verify`（13/13）通过。`cargo clippy -p deploy-go-agent -p deploy-go-agent-executor -p deploy-go-api --all-targets` 退出码 0。
- 所有新增或改动的守卫断言都做过变异验证（临时改坏实现，确认用例失败，再原样恢复并校验文件一致），包括：终态多页不退避、停滞上限、输出超限多页、reconcile 等待者接管、descriptor 暂时不可读重试、帧目录损坏五形态、outbox 两个交付分支、诊断敏感内容、回执 v18 命中行数、投影补齐（单目标/多目标）、超长事件行、本地故障退避。

本轮相对前序记录的修复与新增见 T012–T017；此前已有且仍然成立的修复：699→721 fixture 按真实布局补齐 1..=699 帧并加 30s 超时；帧缺失/软链接/不可解析按存储合同归为永久完整性错误；executor 宣告更高终态序号不再空转。

lint 与未验证项（不得写成已通过）：

- 严格 `cargo clippy -- -D warnings` **未通过**，被 11 处改动前即存在的告警阻断：`api/src/agents/dispatcher.rs:200`、`:2593`、`api/src/agents/mod.rs:193`、`:209`、`api/src/agents/upgrades.rs:659`、`api/src/deployer.rs:110`、`api/src/external/mod.rs:856`、`:870`、`agent/tests/image_release.rs:200`、`:241`、`api/tests/runtime_log_ingest.rs:457`（此前记录写“10 处”，实际列出并复现的是 11 处，已更正）。这些位置都不在本 feature 的改动行范围内，`agent/src/` 零告警，未为本 feature 扩大清理范围。
- **HEAD 上 `cargo test --workspace` 本来就有 3 个失败**，与本 feature 无关，输入与 HEAD 逐字节一致：`agents::tests::upgrade_target_digest_matches_the_served_manifest`（已追到根因：fixture 只到 0.3.20，包版本 0.3.30，`current()` 找不到同名发布目录）、`agent_releases::administrator_can_list_and_clean_historical_agent_releases`、`deployer_release::stable_and_latest_resolve_to_the_api_version_with_canonical_urls`（后两项失败特征同为按当前包版本解析发布物，未逐个追到根因）。因此 HEAD 上 `make api-check` 本来就是红的，需单独处理。
- 未运行真实节点、控制面或业务部署；未做 Docker/Linux 验证（改动限于 Agent 状态机、本地存储分类与 SQLite 投影，未触及 executor 的 cgroup/进程回收分支）。
- 已知边界：`handle_progress` 把 run 与父 deployment 推进到 `running` 的两条 UPDATE 不在同一事务，失败时可自愈；prepare 成功分支无 phase 守卫（HEAD 既有）；两个 outbox 用例依赖目录只读权限，root 下自动跳过。
- 现场部署 `deployment_01M4QAXB5M38A1WZGJHQBPVVY2` **仍未恢复**；修复须随 Agent/executor 成对发布到节点后才对节点侧生效，需另行授权。

任务勾选和 Spec Kit 结构门禁不替代语义复核或真实测试。
