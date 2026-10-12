---
title: 特权 release 终态可靠收敛实施复核
date: 2026-10-12
---

# 特权 release 终态可靠收敛实施复核

## 范围与结论

复核 `specs/004-privileged-release-monitor-recovery/` 的实现：Agent 对既有 durable release job 的监控、输出与结果交接，控制面对 Agent 执行态和终态的投影，以及对应的测试与恢复手册。本轮只修改 Deploy Go，未连接真实节点、未部署控制面、未重发或取消业务部署、未读取 Env 或密钥。

结论：本地代码、测试和文档已收敛，规格内的 FR/SC 均有测试或设计证据（覆盖矩阵见下）。**现场部署 `deployment_01M4QAXB5M38A1WZGJHQBPVVY2` 仍未恢复**，本记录也不证明线上根因已被消除；修复必须随 Agent/executor 成对发布到节点后才对节点侧生效，需另行授权并验证。

## 实施期复核发现与处理

前两轮只读复核已修复：descriptor 回填覆盖新终态的竞态、输出超限缺分页测试、多目标父终态回退、多目标重复结果改写时间/版本、release 开始事件未核对任务类型、单目标 run 与 deployment 终态非原子。本轮在最终实现上重新逐项对照真实协议与存储合同，又发现并修复：

1. **终态 job 的多页输出逐页指数退避，且无停滞上限（本轮实现引入的退化 + 无限等待）。** `consecutive_terminal_sequence_gaps` 对每个终态多页输出的页累加并退避（最长 5 秒/页，8 页实测约 13 秒），只在追平后清零；executor 宣告更高终态序号却持续返回空页时则无限等待。现改为按进度判断：本轮读到新帧立即读下一页并清零；无新帧才退避，连续 `TERMINAL_OUTPUT_STALL_LIMIT`（6）轮仍追不平按永久序号冲突收敛为 `interrupted/release_output_sequence_conflict`。
2. **重启后观察到的输出超限 job 被误判为 `interrupted`。** 真实 executor 的 `truncated` 是 job 级标志（终态后每一页都为 true），而旧 fake 只在最后一页返回 true。当剩余可读帧超过一页时，Agent 在第一页后就按 `release_output_truncated_unexpectedly` 收敛，丢弃剩余可读帧，也不是规格要求的 `failed/output_limit_exceeded`。现仅在“本轮无新帧且 truncated”时才立即收敛，并新增覆盖“重启后才观察到终态”时间线的用例。
3. **reconcile / 取消恢复可能漏起 monitor。** 上一轮把 reconcile 里的 `lock().await` 改成 `try_lock` 以免阻塞对账批处理，但抢不到锁时只上报 `Running` 且不派发等待者：旧 owner 因旧连接关闭而退出后，没有任何 monitor 接管，任务停在 running 直到下一次重连；descriptor 暂时读不到（`RetryLater`）时 reconcile 与取消同样不启动 monitor。现新增统一恢复入口 `recover_privileged_release_monitor`（等旧 owner → 持锁重读 journal → 带重试载入/回填 descriptor → 续接原 job），reconcile 不再触碰所有权锁。同时撤销“有 owner 时降级上报 `Running`”——`reconciled()` 对带 `transfer_phase` 的任务一律上报 `Accepted`，由主控幂等重派并以 Duplicate ACK 重新挂接，人为降级会阻止重派。
4. **帧路径被目录占用被当作 transient 重试。** `external_frame_error` 的永久白名单缺 `IsADirectory`/`NotADirectory`，该形态会无限重试。已补并加进五形态回归用例。
5. **API 单目标原子终态分支丢了旧路径的语义。** 非成功终态时旧路径会把同一 deployment 下其余未终态 run 一并收敛，原子分支漏掉；已在同一事务内补回。
6. **回执重放测试是空转。** 回执门禁只对协议 ≥ v18 的 Agent 生效，上一轮在 `agent_dispatcher.rs` 加的三处 `committed=0` 对 fixture Agent（v11/v14）实际是对 0 行的空更新；`two_stage_deployment.rs` 的重放步骤甚至没有回执。现把 fixture Agent 升到 v18，并断言该 UPDATE 恰好命中 1 行；另外补了“投影中途崩溃后重放补齐下游投影”的单目标与多目标用例。变异验证显示：撤销 v18 升级后断言报告 `left: 0`。
7. **SC-005 只有设计意图，没有测试。** 仓库里没有任何用例捕获 tracing 输出，“诊断不含 Env/密钥/授权正文/业务输出正文/内部路径”从未被验证。已新增捕获式用例，并以“让诊断多输出整个 descriptor”的变异证明它会失败。

## 规格覆盖矩阵（测试名均在仓库中可检索）

| 要求 | 主要证据 |
| --- | --- |
| FR-001 原 job 恢复、不重复启动 | `restart_resumes_persisted_privileged_release_without_second_start`、`waiting_monitor_takes_ownership_after_current_owner_exits`、`reconcile_hands_over_to_a_waiting_monitor_after_the_old_owner_exits`；各用例断言 ReleaseStart 次数为 0 |
| FR-002 输出连续持久化后才终态；输出超限稳定失败 | `terminal_status_waits_until_all_paginated_executor_output_is_persisted`、`output_limit_truncation_converges_to_failed_result_after_readable_frames`、`terminal_output_limit_job_reads_every_remaining_page_before_failing`、`sequence_and_truncation_conflicts_converge_to_interrupted`、`multi_page_terminal_output_is_drained_without_backoff_between_pages` |
| FR-003 暂时故障不静默结束 monitor | `transient_executor_output_and_status_failures_retry_until_terminal`、`temporarily_unreadable_journal_retries_and_converges_from_durable_cursor`、`temporarily_unreadable_descriptor_retries_instead_of_abandoning_the_monitor`、`output_outbox_failure_retries_without_losing_output_or_restarting_release`、`result_outbox_failure_retries_the_durable_terminal_without_a_second_result` |
| FR-004 永久不一致 → interrupted；核心 journal 损坏 → Unknown | `permanent_frame_corruption_converges_to_interrupted_instead_of_retrying_forever`（缺失/不可解析/软链接/超限/目录占用五形态）、`terminal_job_that_never_serves_declared_frames_converges_to_interrupted`、`corrupt_core_journal_reconciles_unknown_without_executor_request` |
| FR-005 release 开始事件推进 run；终态一致 | `api/tests/two_stage_deployment.rs`、`api/tests/agent_dispatcher.rs`（37 + 24 项） |
| FR-006 关联标识、回填、内部诊断 | `release_identity_is_durable_with_privileged_phase`、`legacy_release_descriptor_backfills_associations_durably`、`task_journal_*`、descriptor 缺失/损坏/冲突/取消恢复四个用例 |
| FR-007 成功只来自 Agent 终态 | 监控状态机仅在 executor `Succeeded` 且事件协议完整时成功；各终态用例 |
| FR-008 重放幂等；冲突/迟到不覆盖 | `canceled_and_interrupted_release_results_project_target_run_and_deployment`、`cross_node_prepare_fans_out_independent_releases_and_retry_skips_success`、`current_connection_events_advance_task_deployment_and_logs_once` |
| SC-005 诊断不含敏感内容 | `monitor_diagnostics_and_results_never_expose_secrets_output_or_internal_paths`：捕获完整收敛过程的 tracing 输出，断言带关联与恢复动作，且不含业务输出、带凭证仓库 URL、工作目录与任务根目录路径；`TaskResult` 不暴露内部 task/job 标识 |

## 独立复核（只读子 agent）

第一次发起的复核因会话重启被中止、没有产出结论，不计入；第二次在最终实现上完整返回，只做代码阅读、未运行任何命令。它给出 7 条发现，我逐条对照代码核实后处置如下（核实在先，未照单全收）：

| # | 发现 | 核实结论与处置 |
| --- | --- | --- |
| 1 | 事件重建的永久错误（超长行为 `ErrorKind::Other`）在终态分支被无条件重试 | **属实，已修复。** `agent/src/task_handler.rs` 的终态分支对任何错误都重试，HEAD 此处是直接停止，属本轮引入的“永远等待”。抽出 `permanent_event_data_error`，两个分支都收敛为 `interrupted/release_event_data_invalid`。新增回归用例，恢复旧判定后超时失败 |
| 2 | `consecutive_local_errors` 每轮清零，本地故障退避失效 | **属实，已修复。** 变异验证量化：700ms 窗口内热重试 59 次请求，修复后个位数。改为整轮成功后才清零 |
| 3 | `Succeeded/TimedOut + truncated=true` 被改判为 interrupted | **核实时序成立（executor 在 `try_wait` 成功后才 join 读线程）。** 属有意保守：尾部事件可能丢失、无法验证协议。写入 `data-model.md` 与恢复手册，不改代码 |
| 4 | interrupted 不取消 executor 里的 job | **属实。** 取消正在运行的生产发布比留给人工核对更危险；写入手册，明确 interrupted 只表示“无法证明结果” |
| 5 | prepare 成功分支无 phase 守卫，迟到重放可能把 phase 改回 `awaiting_release` | **HEAD 既有代码**，不在本 feature 的终态语义内，改动属范围外变更。记为已知限制，未修改 |
| 6 | 新分支对已终态 run 加守卫，与旧路径无条件覆盖不同 | **有意为之**：符合“终态后迟到结果不得覆盖已确认终态”。记入已知限制 |
| 7 | root 下 outbox 用例静默通过；断言 panic 会遗留只读目录 | **采纳。** 加 `RestoreMode` Drop 守卫；root 自动跳过的行为已在手册与本记录说明 |

复核同时确认以下区域未发现问题：reconcile / cancel / dispatch 的活性路径与 waiter 累积、终态输出的进度判断与停滞上限、锁顺序、帧存储分类、`handle_progress` / `apply_result` / 多目标保护的幂等性、单目标事务边界、新增诊断字段的敏感信息、`set_global_default` 与 lib 测试二进制内其它测试无冲突。

## 验证结果

本机 macOS，全部为本地 fixture，未连接任何真实节点或控制面：

- Agent lib 118；Agent 全部集成目标 + 协议 + executor 共 37 个测试目标、351 项通过（`privileged_release_recovery` 14、`image_release` 7、executor `release_lifecycle` 9）。API `agent_dispatcher` 37、`two_stage_deployment` 24 通过。
- `cargo test --workspace --no-fail-fast`：775 项通过；仅 3 项失败，均为 HEAD 既有、与本 feature 无关（见上“已知限制”）。
- `cargo fmt --all -- --check`、`git diff --check`、`make spec-kit-verify`（13/13）通过；`cargo clippy -p deploy-go-agent -p deploy-go-agent-executor -p deploy-go-api --all-targets` 退出码 0，仅有 11 处改动前即存在的告警，严格 `-D warnings` 因此仍不通过。
- 所有新增或改动的守卫断言都做过变异验证（改坏实现 → 用例失败 → 原样恢复并校验文件一致）。
- 测试稳定性：新增的两个用例曾出现不稳定并已修复——诊断敏感内容用例最初使用线程局部订阅者，整套并发时捕获为空，改为进程级全局订阅者；超长事件行用例最初 86 轮，整套并发时超过 15 秒超时（约一半概率失败），降到 40 轮并放宽超时。修复后整套连续运行 5 次全绿。

## 已知限制与未验证项

- 未连接任何真实节点、控制面或共享数据库；未做 Docker/Linux 验证（本次改动限于 Agent 状态机、本地存储分类与 SQLite 投影，未触及 executor 的 cgroup/进程回收分支）。
- `handle_progress` 把 target run 与父 deployment 推进到 `running` 的两条 UPDATE 不在同一事务；失败时可能出现 run 已 `running/release` 而 deployment 仍 `running/preparing`。两条都有幂等守卫，下一次进度事件即自愈，且不在 FR-005 要求内。
- 两个 outbox 故障注入用例依赖目录只读权限，root 下探测后自动跳过（打印提示），不会误报通过之外的结果；Linux/macOS 非 root 下均执行。
- 严格 `cargo clippy -- -D warnings` 被 11 处改动前即存在的告警阻断（见 tasks.md 验收证据），不是本 feature 引入。
- **`cargo test --workspace --no-fail-fast` 有 3 个失败，均与本 feature 无关，输入与 HEAD 逐字节一致（`git diff HEAD` 为空）**：
  - `api` 库测试 `agents::tests::upgrade_target_digest_matches_the_served_manifest`：已追到根因——`AgentInstallation::current()` 按包版本（0.3.30）查找发布目录，而 `agent/tests/fixtures/release` 只到 0.3.20，`current_upgrade_target()` 返回 `None` 后被 `unwrap`。
  - `api/tests/agent_releases.rs::administrator_can_list_and_clean_historical_agent_releases`（`left: Null`）与 `api/tests/deployer_release.rs::stable_and_latest_resolve_to_the_api_version_with_canonical_urls`（`left: 404`）：失败特征同样是按当前包版本解析发布物，但我**没有逐个追到根因**，只确认了它们的输入未被本次改动触及。
  - 因此 HEAD 上 `make api-check` / `make check` 本来就是红的。需要单独补 fixture 或调整测试，不属于本 feature，也未在本次顺手修改。
  - 排除这 3 个后，全 workspace 其余 772 项测试通过（102 个测试目标）。

## 后续处理（同日，独立提交）

复核与验证中记录的几项遗留问题随后在独立提交中处理，不属于 spec 004 的范围，因此未并入上面的提交：

- 3 个 HEAD 既有的测试失败：根因是测试绑定包版本而 fixture 只到 0.3.20 / 0.3.26。补齐 `DeployerInstallation::with_api_version`（与 `AgentInstallation` 对称），三个测试固定到 fixture 版本，今后升版本不再失效。
- 11 处既有 clippy 告警：`make api-check` 整条通过（严格 `clippy -D warnings`、workspace 测试、OpenAPI 漂移）。注意 clippy 自动修复曾把 `api/src/external/mod.rs` 里对外诊断的**白名单匹配**改成 `unwrap_or`（等于去掉白名单），已还原并用带理由的 `allow` 标注该误报，其余自动修复逐处核对为等价。
- prepare 成功分支缺 phase 守卫：重放同一 prepare 结果会让 version / updated_at 重复推进，已加守卫并补回归用例（无守卫时失败）。
- `handle_progress` 把 run 与父 deployment 推进到 running 的两条 UPDATE 改为同一事务。
- Docker/Linux 验证：见该提交说明。
