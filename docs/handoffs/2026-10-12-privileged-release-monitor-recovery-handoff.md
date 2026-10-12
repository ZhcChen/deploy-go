---
artifact_contract: "ce-handoff/v1"
created_at: "2026-10-12T17:30:00+08:00"
title: "特权 release 终态可靠收敛：本地修复完成，待 converge 与交付"
summary: "修复 Agent 特权 release 终态不回报导致父 deployment 长期 running 的缺陷。本轮已修复 durable 帧错误分类、reconcile 阻塞、终态序号空转并补齐回归测试；剩余 Spec Kit converge、一个 outbox 故障注入缺口和提交推送。"
keywords: ["deploy-go", "agent", "privileged-release", "monitor-recovery", "task-result", "handoff"]
cwd: "/Users/chen/code/deploy-go"
resume_focus: "按本文第 5 节完成 T012 故障注入、T010 converge、T011 提交推送；不连接真实节点、不部署控制面"
repository: "ZhcChen/deploy-go"
branch: "main"
head: "1b5967d"
worktree_path: "/Users/chen/code/deploy-go"
---

# 交接：特权 release 终态可靠收敛

> **后续更新（同日续作，本文已转为历史交接）**：本文描述的是 T012 / T010 / T011 之前的状态。其后已完成 T012（outbox 故障注入）和实施期复核修正 T013–T016：终态输出按进度读取并设停滞上限、修正输出超限多页误判、统一的恢复入口（修复 reconcile / 取消可能漏起 monitor）、API 单目标原子分支与回执重放测试加固、诊断敏感内容测试；并重新做了独立复核与验证。最终证据以 `specs/004-privileged-release-monitor-recovery/tasks.md` 的“验收证据”和 `docs/reviews/2026-10-12-privileged-release-monitor-recovery-review.md` 为准。下文第 2、4、5、8 节的测试数量、已知边界与“剩余任务”均已过时，仅作历史参考。
>
> 现场部署 `deployment_01M4QAXB5M38A1WZGJHQBPVVY2` **仍未恢复**；本次也没有任何真实节点或控制面验证。

## 0. 唯一规格入口与硬约束

* 唯一规格入口：`specs/004-privileged-release-monitor-recovery/`（`spec.md` / `plan.md` / `tasks.md` / `quickstart.md`）。
* 历史分析：`docs/solutions/2026-10-12-privileged-release-result-not-reported.md`。
* 现场部署 `deployment_01M4QAXB5M38A1WZGJHQBPVVY2` **尚未恢复**，本文不宣称它已恢复。
* 授权边界：只做本地代码、测试、文档和交付。**不连接真实节点、不部署控制面、不重发或取消业务部署、不手工修改线上状态、不读取 Env/密钥、不修改任何业务应用。**
* 禁止修改/暂存/提交这两个用户无关文件：
  * `docs/plans/2026-09-18-001-git-source-materialization-plan.md`
  * `docs/plans/2026-09-20-002-agent-git-materialization-ownership-plan.md`

## 1. 当前工作区状态

* 分支 `main`，HEAD `1b5967d`（`docs: 规划特权 release 终态可靠收敛`），**全部改动未提交、未推送**。
* 工作区改动（11 个文件）：

```text
 M agent/src/executor.rs                                 +130
 M agent/src/journal.rs                                   +54
 M agent/src/task_handler.rs                            +2893
 M agent/tests/privileged_release_recovery.rs            +530
 M api/src/agents/dispatcher.rs                          +187
 M api/tests/agent_dispatcher.rs                         +117
 M api/tests/two_stage_deployment.rs                     +234
 M docs/runbooks/agent-recovery.md                        +10
 M docs/runbooks/deployment-recovery.md                   +14
 M specs/004-.../quickstart.md                             +4
 M specs/004-.../tasks.md                                 +39
```

* 接手第一步必须执行 `git status --short` 并逐个 diff 复核，**不要假设工作区干净或这些改动已验证**。

## 2. 本轮已完成的验证（接手后请重跑确认）

```text
cargo fmt --all -- --check                      通过
git diff --check                                通过
cargo test -p deploy-go-agent --lib             111 passed
cargo test -p deploy-go-agent --test privileged_release_recovery   11 passed
cargo test -p deploy-go-agent --test image_release                  7 passed
cargo test -p deploy-go-agent-executor --test release_lifecycle      9 passed
cargo test -p deploy-go-api --test agent_dispatcher                37 passed
cargo test -p deploy-go-api --test two_stage_deployment           24 passed
cargo clippy -p deploy-go-agent -p deploy-go-agent-executor -p deploy-go-api --all-targets   退出码 0
```

**严格 clippy 未通过，不得写成通过**：`cargo clippy ... -- -D warnings` 退出码 101，被 10 处既有告警阻断（`api/src/agents/dispatcher.rs:200`、`:2593`、`api/src/agents/mod.rs:193`、`:209`、`api/src/agents/upgrades.rs:659`、`api/src/deployer.rs:110`、`api/src/external/mod.rs:856`、`:870`、`agent/tests/image_release.rs:200`、`:241`、`api/tests/runtime_log_ingest.rs:457`）。已逐处核对均在本次改动行范围之外（`dispatcher.rs` 首个改动行是 3359；`agent/src/` 零告警），确认是改动前基线。

## 3. 本轮修复内容（已被测试锁定，勿回退）

### 3.1 699→721 挂起测试（原始卡死点）

`monitor_resumes_executor_output_from_durable_sequence_699` 原本只改 `journal.external_output_sequence = 699` 却没铺设历史帧，与 durable 存储合同不符并永久挂起。

* `agent/src/executor.rs` 新增 `#[cfg(test)] pub(crate) fn seed_external_output_history`，按真实布局铺 1..=N 帧（帧 1 带 `DEPLOY_GO_EVENT` preflight 标记）。
* fixture 顺序改为**先播种再推进游标**；monitor/server 等待加 30s timeout；显式断言 ReleaseStart 计数为 0。
* 协议标记只能出现一次：标记在历史帧 1，恢复窗口 700–721 是纯续写数据。此前误在两处都放，导致 `deploy_event_protocol_conflict` 把终态从 Succeeded 变成 Failed。

### 3.2 永久完整性错误被误判为 transient 无限重试（本 feature 同类故障）

存储合同：帧目录在最终 `TaskResult` 回执 ACK 前不会被回收，因此游标 `N` 必然对应盘上 1..=N 的帧。原实现把这些错误包成 `ExecuteError::Io`，被 `transient_monitor_storage_error` 判为可重试，导致对同一 job 无限退避、永不收敛——与本规格要修的故障形态相同。

* `agent/src/executor.rs` 新增 `external_frame_error` / `external_frame_layout_error` / `read_external_output_frame`，把 `NotFound`/`InvalidData`/`Other`（含软链接）、`AlreadyExists`/`NotADirectory` 映射为 `InvalidState`（永久），保留 ENOSPC/EIO/EACCES 为 transient。
* 覆盖点：帧目录、目标帧、历史帧读取、重建目录、帧目录创建。
* 新增回归用例 `permanent_frame_corruption_converges_to_interrupted_instead_of_retrying_forever`（缺失/不可解析/软链接/超 1MiB 四形态）。**已验证：临时把分类改回 transient 时该用例会超时失败，不是空断言。**

### 3.3 executor 宣告更高终态序号时空转

原 `if exited.last_sequence > cursor { continue; }` 无延迟无诊断，会忙等。改为 `consecutive_terminal_sequence_gaps` 退避重试 + `terminal_output_sequence` 诊断；若同时观测到输出截断且始终追不平，按永久冲突收敛为 `interrupted`。

### 3.4 reconcile 被长期持有的 monitor owner 阻塞（本轮新引入的回归）

`privileged_monitor_lock` 是本轮新增机制，reconcile 的**顺序**循环里原本是 `lock.lock().await`，会阻塞整批对账直到 release 结束。改为 `try_lock`：已有 owner 时不等待、不重复交接，按 `Running` 上报。新增 `reconcile_does_not_block_on_an_active_privileged_monitor_owner`，**已验证改回阻塞写法会失败**。

### 3.5 补齐此前无测试驱动的分支

* Agent：`sequence_and_truncation_conflicts_converge_to_interrupted`（帧序号不连续 / 终态逆序 / 终态+意外截断）。
* Agent：`corrupt_and_conflicting_release_descriptor_only_observe_original_job`、`cancel_without_descriptor_observes_original_job_then_reports_interrupted`。
* Agent：`temporarily_unreadable_journal_retries_and_converges_from_durable_cursor`（FR-003；已连续跑 5 次无抖动）。
* API：`canceled_and_interrupted_release_results_project_target_run_and_deployment`，覆盖取消/中断终态投影、重放幂等、以及迟到结果被 `apply_result` 拒绝（断言错误码 `agent_event_after_terminal`，**已验证把该分支短路后用例失败**）。

## 4. 未验证项与已知边界（必须如实保留，不得改写成通过）

* 未运行真实节点、控制面或业务部署。
* 未做 Docker/Linux 验证：本次改动限于 Agent 状态机与 SQLite 投影，不涉及 executor Linux cgroup/进程回收分支。
* `handle_progress` 中推进 target run 与父 deployment 的两条 UPDATE **不在同一事务**；失败时可能出现 run 已 `running/release` 而 deployment 仍 `running/preparing`。两条都幂等且可自愈，属已知取舍，不在 FR-005 要求范围内。
* executor 持续宣告更高终态序号且长期不补帧时，monitor 仍会持续退避重试，没有升级为中断的次数上限。不在 FR-002/SC-003 强制收敛范围，已保留退避诊断。

## 5. 剩余任务（按顺序）

### T012 — outbox 交付故障注入（唯一测试缺口）

在 `agent/tests/privileged_release_recovery.rs` 为 monitor 层 `send_result`/`drain_outputs` 的 `result_delivery` / `event_outbox_delivery` 重试分支补故障注入。

现状：只有 `agent/src/log_delivery.rs` 自身单测覆盖 outbox 逻辑，未驱动 `monitor_privileged_release` 的交接失败分支（FR-003）。

验收：注入交付失败后 monitor 保留原恢复阶段并重投同一 job 的 durable 终态，最终只产生一个逻辑 TaskResult，不重复启动 release。可参考第 7 节的注入手法。

### T010 — Spec Kit converge

对照 `spec.md`/`plan.md`/`tasks.md` 与最终实现执行 converge，**只追加确有必要的遗漏任务，无发现不写文件**。

注意：`requirements checklist` 原本 7/7，那是需求质量检查、不是实现验收，不要改；`.specify/extensions.yml` 不存在属正常。

```bash
make spec-kit-check FEATURE=specs/004-privileged-release-monitor-recovery STAGE=implement
```

结构门禁通过不代表语义收敛。

### T011 — 提交与推送

1. `git diff --check`、`git diff --cached --check`、复核 staged diff。
2. **只暂存本轮 11 个文件**，不要 `git add .`，不得纳入两个 `docs/plans/2026-09-*` 未跟踪文件。
3. 简体中文小闭环提交（建议 `fix:` 前缀）。
4. `git fetch origin` → `git rebase origin/main` → `git push origin main`。远端有新提交时**不得强推**。
5. 不得使用 `--no-verify` 或绕过 hooks。

## 6. 交接注意事项（本轮踩过的坑）

1. **`apply_patch` 会误命中**：只给出 `        ] {` 这类短上下文时，可能匹配到文件别处并**报告成功但插错位置**。每次改动后立刻 `grep -n` 复核落点；发现插错立刻回滚。
2. **shell 里的括号**：`sed -i "" "6428s/run_sequence_conflict(conflict)/.../"` 这类含括号的正则实测**静默不匹配**，而同一行的 `s/conflict/ZZZ/` 能匹配。改整行时用 `@` 作分隔符避开括号歧义：

```bash
sed -i "" "6428s@.*@            run_sequence_conflict(scenario).await;@" agent/src/task_handler.rs
```

3. **fixture 必须符合真实存储合同**，不要为迁就错误 fixture 放松生产完整性校验。本轮的挂起测试就是 fixture 缺 1..=699 历史帧导致的。
4. **新测试要防挂起**：任何驱动 `monitor_privileged_release` 的用例都要有 timeout，否则回归成无限重试会挂死整个测试进程。
5. **新断言要验证是真守卫**：临时把被测分支短路或改坏，确认用例会失败，再改回来。本轮对 3 个关键用例都做了这个验证。
6. 并发跑 `cargo` 会争 target 锁，subagent 只读复核时不要同时编译。

## 7. 可复用的故障注入手法

注入 journal 读取失败（验证退避重试而非静默结束）：

* **不能**在 reconcile 之前破坏 journal —— `executor.recover()` 会先失败，`Unknown` 分支不会 spawn monitor。
* 正确做法：让 monitor 先接管并观测一次状态（server 第一次回 `ReleaseStatus(Running)` 并置标志位），再把 `journal.json` 换成**目录**（`fs::read` 返回 `EISDIR` → `JournalError::Io` → 走 retry 而非 fail-closed `Unknown`）。
* 断言 monitor 确实停在 journal 读取失败上：跨过 `Running` 后的 100ms 间隔后，断言 server 侧 output 请求计数**没有增加**。
* 再恢复 `journal.json`，等待终态并断言唯一 TaskResult。

参考实现：`agent/tests/privileged_release_recovery.rs::temporarily_unreadable_journal_retries_and_converges_from_durable_cursor` 及其 server helper `serve_running_then_exit_executor`。

## 8. 建议接手顺序

1. `git status --short` + 逐文件 diff 复核，确认工作区与本文一致。
2. 重跑第 2 节全部命令确认基线。
3. 做 T012。
4. 做 T010 converge（可能再次追加任务）。
5. 重跑第 2 节全部命令，并重新记录严格 clippy 的未通过事实。
6. 做 T011 提交推送。
7. 汇报时明确：本地修复已完成并提交，**现场部署 `deployment_01M4QAXB5M38A1WZGJHQBPVVY2` 未恢复**，且本次未做任何真实节点或控制面验证。
