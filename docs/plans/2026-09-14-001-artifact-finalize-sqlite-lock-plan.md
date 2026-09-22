---
title: 制品上传 finalize SQLite 写锁与文件一致性修复计划
date: 2026-09-14
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: code
---

# 制品上传 finalize SQLite 写锁与文件一致性修复计划

## Goal Capsule

- **目标：** 修复 Deploy Go 控制面在制品上传 finalize 阶段的内部失败和文件/数据库收敛问题；当前最高优先级假设是 SQLite 写锁竞争，但实施前必须通过阶段化错误日志确认。
- **故障依据：** 正式请求 `req_01M2FGY1CHCZAZERCYDMBQPBGS` 在 finalize 中返回 500；对应制品文件已经进入 `artifacts/objects`，但数据库未提交 `verified/storage_key`，部署最终以 `artifact_transfer_failed` 失败。
- **范围：** API 制品 finalize 的事务边界、SQLite 写竞争处理、commit 结果确认、文件补偿、幂等重试、原始错误日志、reconciliation 规则和并发回归测试。
- **不在范围：** 被部署业务应用、业务 prepare/release 脚本、真实节点操作、生产数据库手工修复、迁移历史文件和替换 SQLite。
- **上线原则：** 先本地测试和代码复核，再按正式环境 runbook 做构建、健康检查和只读验收；本计划本身不授权生产部署或远程变更。

## Problem Frame

当前 `api/src/artifacts/http.rs` 的 `finalize_upload` 在完成归档校验后开启默认 deferred transaction。事务内先重新读取 lease，再执行文件 `rename`，然后更新 lease、`deployment_artifacts` 和 `deployment_target_runs`。多个数据库和文件错误都被转换为无上下文的 500。

正式现场显示：

1. 上传 offset 已达到 upload size，归档校验路径不是主要嫌疑；
2. 文件已从 quarantine 移到 object，但 `deployment_artifacts` 仍为 `failed` 且 `storage_key` 为空；
3. 同一正式 API 后续明确记录了 `database is locked`；
4. 当前 runbook 已要求写事务使用 `BEGIN IMMEDIATE`，但 finalize 仍使用 `.begin()`；同时已有对象摘要校验发生在事务开启之后。

这些证据支持“SQLite 写锁竞争”为最高优先级假设，但不能证明目标请求的具体失败点。修复必须先补齐失败阶段观测，再同时解决：减少 finalize 与 dispatcher/Agent 事件写事务的锁竞争；在数据库结果明确或未知时安全处理文件；并使客户端重试不会破坏已提交状态。

## Requirements Traceability

| ID | 要求 | 来源 |
|---|---|---|
| R1 | finalize 对明确的 SQLite 写竞争做有界重试；其他错误必须先被准确分类，不得假设都是锁竞争 | 正式故障、`docs/runbooks/deployment-recovery.md` |
| R2 | lease、artifact、pending target run 的状态更新必须作为一个数据库原子提交 | `api/src/artifacts/http.rs:613` |
| R3 | 不产生无法追踪、无法收敛的静默孤儿对象；数据库失败后必须立即恢复文件，或记录结构化诊断并保证 reconciliation 能依据数据库事实安全收敛 | 正式现场、制品存储约定 |
| R4 | API 日志必须记录 request ID、artifact/lease 标识、失败阶段和脱敏数据库错误类别，不记录 token、归档内容或绝对路径 | `docs/standards/api-contract.md`、生产日志约定 |
| R5 | 现有成功、校验失败、重复 finalize 和 Agent 断线恢复行为保持不变 | `api/tests/artifacts_api.rs`、`docs/runbooks/deployment-recovery.md` |
| R6 | 不修改已提交 migration；除非实施中发现确需 schema 变更，否则本修复不新增 migration | `docs/standards/sql-repository-convention.md` |
| R7 | commit 结果未知时不得盲目回滚文件或重试事务；若数据库已提交，重复 finalize 必须安全返回成功 | SQLite commit 语义、制品幂等要求 |

## Settled Decisions

### KTD1. 保持 SQLite 单主控架构，修复事务竞争而非替换数据库

正式部署拓扑是单 API 实例加 WAL SQLite。问题是 finalize 使用 deferred transaction 且错误信息被吞掉，不足以证明需要引入外部数据库。沿用已有 `BEGIN IMMEDIATE` 约定，保持部署形态和运维边界不变。

### KTD2. 归档校验留在事务外，状态提交使用短的 IMMEDIATE 事务

归档读取和 SHA-256 校验可能耗时较长，不应持有 SQLite 写锁。上传归档校验和已有 digest 对象的完整性校验都在事务外完成；校验完成后重新加载 lease，以短事务完成 lease 消费、制品验证和 pending target run 绑定。对象路径由 digest 决定且对象写入后不可变，事务内仍需再次确认数据库状态和对象存在性，避免把校验结果用于错误的 lease。事务开始阶段使用 `BEGIN IMMEDIATE`，避免 WAL 下先读后写触发快照升级失败。

### KTD3. 对锁竞争采用有限、可观测的重试

只对明确识别为 SQLite `BUSY`/`LOCKED` 类错误、且事务尚未产生不可逆结果的阶段做重试；使用 SQLx `Error::Database` 携带的 SQLite 原生错误码，不匹配错误字符串。最多 3 次尝试，第一次开始到最后一次结束的总 deadline 固定不超过 15 秒；当前每连接 5 秒 busy timeout 必须纳入该总预算，不能在预算之外叠加等待。非锁错误不重试，commit 返回错误后不得重试原事务。最终对客户端仍使用现有错误契约，不泄露 SQLite 细节。

### KTD4. 文件提升与数据库提交必须有完整补偿边界

文件 rename 不可纳入 SQLite 回滚，因此实现必须覆盖：rename 前失败、rename 后 SQL 失败、影响行数异常、明确未提交、commit 结果未知和补偿 rename 失败。明确未提交时才允许将对象移回 quarantine；commit 结果未知时先用独立连接读取 artifact/lease 事实：若已是 `verified + storage_key=digest` 且对象校验通过，则保留 object，并让原 finalize 或后续重试返回成功；若仍未提交且可确认，则执行补偿；数据库查询也失败时保留 object 并记录“提交结果未知”，禁止删除或自动重试。补偿失败必须结构化告警，后续 reconciliation 依据数据库真相安全清理；不得把手工删除作为正常恢复步骤。

### KTD5. 错误日志优先补齐，再优化实现

当前 finalize 多处 `map_err(|_| ApiError::internal(...))` 导致正式请求无法定位失败阶段。所有新改动的数据库和文件失败点必须通过统一辅助函数记录 request ID、artifact ID、lease ID、阶段和安全错误类别；响应仍只携带 request ID。

### KTD6. 定义对象与数据库状态的收敛矩阵

允许短暂存在“对象已落盘但数据库尚未 verified”的中间状态，但它必须可追踪且可收敛，不能作为静默成功。状态处理固定如下：

| 文件事实 | 数据库事实 | finalize/reconciliation 处理 |
|---|---|---|
| quarantine | uploading + active lease | 允许继续上传或 finalize |
| object | uploading + active lease | 仅在 digest/大小复核通过后继续当前 finalize；失败时恢复或记录可收敛孤儿 |
| object | verified + storage_key=digest | finalize 重试幂等成功；下载可用 |
| object | failed/无 storage_key | 不重试原 lease；由 reconciliation 清理或按新的 prepare 重新上传 |
| object | consumed lease + verified artifact | 视为已完成，重试返回成功，不重复消费 |

`deployment_artifacts` 更新必须影响 1 行；pending target run 允许影响 0 行，但必须记录预期绑定数量并验证不存在不应有的状态变化。

## Implementation Units

### U0. 增加故障阶段确认门

- **文件：** `api/src/artifacts/http.rs`、`api/tests/artifacts_api.rs`
- **内容：** 先为 finalize 的所有可能 500 分支定义阶段和安全错误分类，覆盖初始 lease 查询、路径解析、归档校验任务、校验失败后的 `fail_upload`、事务开始、对象校验/提升、三条状态更新、commit 和 commit 结果查询。以正式 request 的证据格式记录“已确认/待确认”，实施前不得把 SQLite 锁竞争当成唯一根因。
- **依赖：** 无。
- **完成标准：** 任何 finalize 500 都能通过 request ID 定位到阶段、错误类别和数据库事实是否已确认；日志不含 token、归档内容或绝对路径。

### U1. 固化 finalize 状态机和错误观测边界

- **文件：** `api/src/artifacts/http.rs`
- **内容：** 为 finalize 的前置查询、归档验证、事务开始、已有对象验证、lease 消费、artifact 更新、target run 绑定、文件提升、commit、commit 结果查询和补偿定义明确阶段；统一记录原始 `sqlx`/IO 错误的安全分类。
- **约束：** 日志不得包含 Bearer token、归档内容、manifest 明文或服务器绝对路径；不改变 401/404/409 的既有业务语义。
- **依赖：** 无。
- **完成标准：** 所有 finalize 500 分支都能从同一 request ID 判断失败阶段和错误类别；成功响应和客户端错误格式保持兼容。

### U2. 重构 finalize 的短 IMMEDIATE 事务与有界锁重试

- **文件：** `api/src/artifacts/http.rs`，必要时复用 `api/src/db/` 的现有 SQLite 错误判断模式。
- **内容：** 归档校验完成后重新读取 lease；已有 digest 对象的完整性校验在事务外完成；以 `BEGIN IMMEDIATE` 开启短事务；在明确的锁竞争阶段按 KTD3 有界重试；成功时一次性提交 lease、artifact 和 pending target run 三类状态。事务内检查关键 `UPDATE` 的影响行数；commit 结果未知时转入 KTD4 的确认流程，不得自动重试。
- **依赖：** U1 的阶段错误分类。
- **完成标准：** 并发 Agent 事件/dispatcher 写入时，瞬时锁竞争可恢复；超过预算时返回可诊断的 500，不造成半提交状态。

### U3. 完善对象文件补偿和 reconciliation 兼容性

- **文件：** `api/src/artifacts/http.rs`、必要时 `api/src/artifacts/mod.rs`。
- **内容：** 按 KTD4/KTD6 将对象提升、数据库失败、影响行数异常、commit 结果确认和补偿动作组织成显式流程；确保 transaction rollback、SQL 中途错误和明确未提交都执行适当补偿，commit 未知时保留对象并可查询。校验现有孤儿对象清理不会误删 active lease 或 pending target run 引用的对象；对 `object + failed` 明确清理和重新上传入口。
- **依赖：** U2。
- **完成标准：** 任意失败注入场景都有明确的文件/数据库结果；commit 已提交、未提交和未知三类结果分别符合 KTD4；补偿失败有告警，不能静默吞掉。

### U4. 增加 SQLite 锁竞争和文件一致性回归测试

- **文件：** `api/tests/artifacts_api.rs`；必要时新增 `api/tests/artifact_finalize_concurrency.rs`。
- **内容：** 覆盖成功 finalize、并发 finalize 单次消费、finalize 与其他写事务竞争、已有对象校验不持有写锁、lease/artifact/target-run 三类 SQL 错误、影响行数异常、commit 已提交/未提交/未知、对象补偿、幂等重试和重试上限。测试使用临时文件 SQLite/WAL 和多连接池模拟真实锁竞争，不仅依赖现有 `sqlite::memory:` 单连接 fixture；为 commit/文件失败定义窄的 `cfg(test)` 注入 seam，禁止随机 sleep、删除数据库文件或生产隐藏开关。
- **依赖：** U2、U3。
- **完成标准：** 测试能稳定复现锁竞争并验证最终状态；可证明写锁不被大文件哈希长期占用；三种 commit 结果和每类失败的文件/数据库/日志断言完整。

### U5. 更新恢复 runbook 与生产验收定义

- **文件：** `docs/runbooks/deployment-recovery.md`、必要时 `docs/runbooks/systemd-deployment-production.md`
- **内容：** 补充 finalize 失败后的只读日志/SQL 核查、commit 结果未知时禁止重试/删除/手工改库、reconciliation 收敛条件、孤儿对象观察字段、API 与现有 Agent 的兼容性，以及受控发布后的观察窗口、停止条件和回滚证据保留要求。正式验收至少记录 finalize 成功率/延迟、SQLite 锁错误次数、补偿失败数、孤儿对象数和 artifact 异常状态数。
- **依赖：** U1-U4。
- **完成标准：** 运维人员可以只读判断一次 finalize 失败属于已提交、未提交或未知；生产发布有明确 Go/No-Go、回滚和 WAL/制品目录证据保留步骤。

### U6. 聚焦复核与本地验证

- **文件：** `docs/reviews/2026-09-14-artifact-finalize-sqlite-lock-review.md`（仅在实施完成且需要保留复核结论时创建）。
- **内容：** 对照本计划检查事务原子性、文件补偿、错误脱敏、并发行为和既有 deployment recovery 语义；运行聚焦测试、`cargo fmt --check`、`cargo clippy`/项目既有 API check 及 `git diff --check`。
- **依赖：** U0-U5。
- **完成标准：** 复核无 P0/P1 问题，所有 R1-R7 和 Definition of Done 有验证证据。

## Verification Contract

### 聚焦测试

```text
cargo test -p deploy-go-api --test artifacts_api
cargo test -p deploy-go-api --test artifact_finalize_concurrency
```

若不新增独立测试文件，则将第二条替换为对应的现有测试目标，并在实施结果中说明原因。

### 静态与工程检查

```text
cargo fmt --all -- --check
make api-check
git diff --check
```

若修改 `api/migrations/`，必须额外执行项目规定的 git hooks 校验和 `cargo test -p deploy-go-api --test migrations`；本计划默认不修改 migration。

### 行为验证场景

- 正常归档：返回 200，lease 为 consumed，artifact 为 verified，target run 绑定 artifact，object 存在。
- 校验失败：返回既有 409，lease/artifact 进入失败状态，不产生 verified object 引用。
- 并发 finalize：只有一个请求消费 lease，另一个得到既有冲突语义，不出现两个 verified 结果。
- SQLite 写锁：受控持锁期间按有限预算重试；释放锁后成功，或超预算后返回带 request ID 的 500 且日志含阶段和 SQLite 错误码。
- SQL/commit 失败：分别验证数据库已提交、未提交和结果未知；已移动文件按 KTD4 处理，不能把已提交 artifact 的 object 移走。
- 影响行数异常：artifact 更新不为 1 时回滚并进入明确冲突/内部错误，不能提交 lease consumed 的半逻辑状态。
- 幂等重试：数据库已 verified 且 object 完整时，重复 finalize 返回成功，不重复消费 lease 或重新移动文件。
- 部署恢复：prepare 制品上传失败后仍符合 `docs/runbooks/deployment-recovery.md` 的重连、重试和 reconciliation 语义。

## Risks and Open Questions

- **R1：锁重试掩盖系统性写入过载。** 通过限制总重试时长、记录锁竞争指标/告警和保留 request ID 诊断，避免无限等待。
- **R2：文件补偿本身失败。** 不在请求中反复做无界补偿；记录 artifact/digest 的脱敏标识，让既有 reconciliation 按数据库状态处理，并补充 runbook 观察说明。
- **R3：测试难以稳定触发 commit 失败。** 可将文件提升/数据库提交编排抽成窄边界辅助函数，并使用测试注入点；不为测试引入生产隐藏开关。
- **R4：commit 结果未知时查询数据库也可能失败。** 此时必须保留 object、停止自动重试和删除，并让 runbook 指向人工只读核验；不能用“查询失败”推断未提交。

## Definition of Done

- [ ] finalize 的所有 500 分支都有阶段化、脱敏日志；SQLite 锁竞争仍只是经错误码确认后的分类，不被默认假设覆盖。
- [x] finalize 使用短 `BEGIN IMMEDIATE` 状态事务，不再以默认 deferred transaction 作为写入入口；大对象摘要校验不在写事务内。
- [x] `SQLITE_BUSY`/`LOCKED` 锁竞争有界重试，非锁错误不被错误重试；最终失败可按 request ID 定位阶段。
- [x] lease、artifact、pending target run 的成功更新原子提交，失败不产生半提交状态。
- [x] 文件 rename 后任一数据库失败都有恢复、可追踪告警或 reconciliation 明确收敛路径；commit 已提交时绝不移走被引用 object。
- [ ] artifact 影响行数、并发消费、幂等重试和对象校验场景已有自动化测试；commit 三态仍需注入 seam 后补充。
- [x] `docs/runbooks/deployment-recovery.md` 已说明 commit 未知、孤儿对象和正式只读验收流程。
- [x] 未修改历史 migration、未改变业务应用部署协议和既有 API 错误契约。
- [x] 聚焦测试、静态检查和 `git diff --check` 通过；已完成本地代码复核。
- [ ] 未经当前对话针对正式控制面明确授权，不执行生产部署、重启、数据库修复或孤儿文件清理。

## Execution Order

`U0 -> U1 -> U2 -> U3 -> U4 -> U5 -> U6`

实施阶段应保持每个单元可单独验证；如果 U2 发现需要改变 schema、部署协议或生产恢复语义，先暂停并更新本计划，不直接扩大范围。

## Sources

- `docs/runbooks/deployment-recovery.md`
- `docs/runbooks/systemd-deployment-production.md`
- `docs/standards/sql-repository-convention.md`
- `docs/standards/api-contract.md`
- `api/src/artifacts/http.rs`
- `api/src/artifacts/mod.rs`
- `api/src/main.rs`
- `api/tests/artifacts_api.rs`
- 正式 request `req_01M2FGY1CHCZAZERCYDMBQPBGS`
- 正式 deployment `deployment_01M2FGY0MKBNB2RVACJPATABEV`
