---
title: 特权 ReleaseStart 不确定结果恢复计划
date: 2026-09-30
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan
execution: code
---

# 特权 ReleaseStart 不确定结果恢复计划

## 目标

当 Agent 已发送特权 `ReleaseStart`，但响应丢失、损坏或 executor 返回可能发生于进程启动之后的错误时，不得立即将任务标记为失败并清除 durable recovery phase。Agent 应用同一 `job_id` 和 `task_payload_digest` 最多重试一次，随后通过现有 durable `ReleaseOutput`/`ReleaseStatus` monitor 收敛唯一终态。

此计划修复的是 Deploy Go Agent/executor 的通用可靠性缺口，不判定任何给定生产部署的实际失败根因。

## 已确认因果链

1. `agent/src/task_handler.rs` 在调用 executor 前将 journal 标记为 `PrivilegedRelease`。
2. executor 的 `ReleaseStart` 先检查 durable job；job 不存在时验证授权、封存输入并启动 release。成功启动后先持久化 job 状态，再向 Agent 返回 `ReleaseStarted`。
3. IPC 请求错误、响应 EOF/超时/解码错误或可能发生于启动后的 executor 错误会由 Agent 转为普通错误码。
4. `resume_cross_node_release` 当前将该错误交给 `fail_release_task`；`complete_task` 写入 `Failed` 并清空 `transfer_phase`。
5. 若 executor 已启动 job 而 ACK 未到达，Agent 不再监控 durable job；控制面得到失败结果，但节点上的 release 可能仍在执行。

既有 `agent/tests/privileged_release_recovery.rs` 覆盖 Agent 重启后的 Status/Output 接管，但没有覆盖首次 ReleaseStart 的响应丢失。已有恢复契约要求 Agent 重启后不重复启动；本计划保持该契约不变。

## 设计与边界

- 仅处理首次 `ReleaseStart` 调用的结果不确定窗口；不改变 Agent 控制协议、本机 executor wire schema、授权 claims 或 job ID 规则。
- 将结果分为“确定未启动/确定拒绝”和“可能已启动”两类。只有明确的本地未发送错误及确定的准入拒绝可以直接失败。
- IPC 写入错误、响应错误、无法验证的启动 ACK、未知 executor 错误码，以及可能发生于 `spawn` 后的 `release_spawn_failed` / `release_storage_unavailable` 均按不确定处理。
- 不确定时使用完全相同的 `ReleaseStartRequest` 最多重试一次。重试返回的任何错误都不能否定第一次可能仍在并发 admission/启动中的请求；只在第一次请求确定未发送或收到有效、确定的拒绝时直接失败。executor 以 `job_id + task_payload_digest` 查询 durable job，并以封存目录冲突阻止重复 admission；不得生成新 job ID、授权 nonce 或 release task。
- 第二次调用仍无法确认时保留 `PrivilegedRelease` phase，进入已有 monitor。monitor 只通过 Output/Status 查询和转发，不重新执行 release。保持未知任务为非终态，避免向控制面报告失败但实际 release 仍运行。
- Start 期间出现取消标记时不再重试；对同一 job 补发 cancel，并在 Start 请求结束后再检查/补发一次，覆盖 cancel 先于尚未完成 admission 的 Start 到达 executor 的竞态。失败收敛时优先尊重本地取消标记。
- executor 在持久化 `Sealing` 后发生可检测的启动错误时，尽力将 durable job 写为 terminal `Failed`，并在已经 spawn 子进程但无法写入 `Running` 状态时先终止该子进程；不能将已知失败留为永久 `Sealing`。
- monitor 对 `release_job_conflict` 终结当前 payload 冲突，不无限轮询；其他暂态 IPC/非终态状态继续对账，错误轮询采用最高 5 秒的有界退避。
- monitor 仅接受协议版本及 job ID 均匹配的 Output/Status/Exited 响应；取消标记存在时持续重发同 job cancel，直到 durable 终态，覆盖 admission 前取消未生效的情况。
- Agent 重启恢复仍只查询已有 durable job，不重放 `ReleaseStart`；既有 fail-closed 与人工诊断边界不变。
- 日志仅记录 task/job/deployment/target 标识、attempt、受控错误码和恢复动作；不得记录授权、Env、请求正文、原始 IO 文本或业务输出正文。

## 实施单元

1. **Agent/executor 状态收敛**：Agent 对不确定 Start 结果用同一请求最多重试一次；重试错误仍由 durable monitor 收敛。取消期间不发第二次 Start，并补发同 job cancel。executor 将可检测的启动失败持久化为 terminal；monitor 对 payload 冲突终结、暂态错误退避。
2. **回归测试**：在 `agent/tests/image_release.rs` 的现有端到端 fixture 覆盖首次 ACK 丢失后重试 ACK 到达、两个 ACK 均丢失后由 monitor 接管，以及重试返回冲突但首次 Start 最终成功；断言请求完全相同、模拟 durable job 仅 admission 一次、在观察窗口内只回传一个终态。另在 `agent-executor/tests/release_lifecycle.rs` 验证真实 `ReleaseJobManager` 对同一 durable job 的重复 Start 只运行一次脚本。覆盖确定拒绝/本地未发送分类、启动失败持久化、监控退避边界、取消后不再重试，并保持 Agent restart 不重发 Start 的既有测试。
3. **运行文档**：同步特权 Release 与部署恢复说明，解释 ACK 丢失时任务会保持运行并由 durable monitor 接管；持续无法查询 job 时先核对 Agent/executor 同一 job 日志与状态，不清理 job、不换 task 重放，并记录 Agent 重启后无 durable job 的恢复限制。

## 验收

- 丢弃第一次 `ReleaseStarted` 后，Agent 用相同 job ID 与 payload digest 再请求一次；executor fixture 只计一次实际启动，最终 task result 只有一条且与 durable job 终态一致。
- 两次启动响应都不确定、或重试返回错误时，Agent 不产生 `Failed` TaskResult、不清除 `PrivilegedRelease` phase，并继续可被后续 Status/Output 接管。
- 确定的连接失败、编码/帧门限失败及白名单内的准入拒绝仍及时失败；不得对明确拒绝盲目重试。
- 取消与 Start 并发时不得因先到达的 not-found cancel 而放过后续启动；同 job cancel 可重复发送，monitor 在本地 task 已终结后退出。
- executor 可检测到的 spawn/状态初始化失败不得遗留非终态 `Sealing`；持久 digest 冲突不得让当前 task 无限轮询。
- Agent 重启恢复仍不发送 `ReleaseStart`；已有取消、输出续传和唯一终态测试继续通过。
- executor IPC wire schema、部署协议、API/OpenAPI、授权格式均无变更。
- 通过 Agent/executor 聚焦测试、`cargo fmt --all --check` 和 `git diff --check`。
- 不访问真实节点，不部署正式控制面，不发起或重试任何业务部署。

## 风险与未覆盖情形

- 若 executor 在 admission 创建 durable job 前持续发生存储错误，或写入启动失败终态也失败，Agent 可能持续等待不存在的 job 或残留的 `Sealing`。本轮保留未确认状态并记录诊断，不以超时推定业务未执行；安全的无 job 恢复与重授权协议属于后续设计。
- 若 executor 在 Agent 重启前尚未收到 ReleaseStart，恢复逻辑仍只查询 durable job，不持久化包含授权/秘密 Env 的完整 Start 请求，也不自动重新请求授权；因此可能长期看到 job 未就绪。该问题需单独设计不持久化秘密的重授权/恢复协议，不通过猜测性清理或生成新 job 处理。
- 若 executor 在子进程 spawn 后且 Agent 未拿到 ACK 时崩溃，executor restart reconciliation 会按既有策略将无法确认的 job 标为失败；它不能证明业务脚本从未执行。运维仍须检查两侧 durable 状态和日志后再决定是否创建新部署。
- 当前生产部署的具体失败原因仍未确认。完成本计划只能消除“启动 ACK 不确定却报告失败”的状态机缺陷，不能证明原生产部署会成功，也不授权重试。

## 执行进度

- [x] Agent/executor 结果分类、幂等重试、取消协调与 monitor 对账。
- [x] ACK 丢失、重试冲突、executor 同 job 幂等及明确失败分类回归。
- [x] 特权 Release 与部署恢复 runbook 同步。
- [x] 最终回归、格式与 diff 复核：Agent/executor 248 项测试通过；macOS 不覆盖 Linux 专属 cgroup 分支，未验证真实节点。
