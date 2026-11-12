# 租约窗口从派发开始计时实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

事故证据与机制见规格“背景与证据”。要点：租约在 `create_stage_task` 创建任务时签发、固定 60 秒；任务排队期间被 `expire_secret_leases` 清理为 `expired`；`try_dispatch` 派发时不触碰租约；`resolve_secret_lease` 取用时要求 `status='issued' AND expires_at>now`。

可选方案比较：
1. **派发时重新计时（采用）**：改动集中在 `try_dispatch` 一个咽喉点，覆盖首次派发和所有重新派发；不改租约创建与取用规则。
2. 取用时容忍过期（只要任务活跃就放行）：等于取消租约时效，削弱最多，不采用。
3. 推迟到派发时才创建租约：载荷里已含租约 ID 与摘要，需要改任务创建流程，改动面大，不采用。
4. 名额空出前不创建任务：需要重做排队与恢复逻辑，竞态多，不采用。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md)：不改 Agent、线协议、schema 与 OpenAPI；不新增任意 shell、命令或带凭证 URL 的字段；Secret 不进日志（新增日志只记录任务 ID 与受影响行数）；凭证类改动走完整流程并由实现者自审；部署需要用户当前对话的“部署正式环境”授权；不手改线上状态。

## 实现方案与文件范围

1. `api/src/agents/dispatcher.rs`：
   - 新增常量与 `git_lease_window(&TaskPayload)`（发布 600 秒，其余 60 秒）。
   - 新增 `restart_git_lease_window(state, task_id, deadline_at, window)`：计算 `min(now + window, deadline_at)`，若该时刻不晚于当前时间或 `deadline_at` 无法解析则不更新；否则 `UPDATE git_secret_leases SET status='issued', expires_at=? WHERE task_id=? AND granted_at IS NULL AND status IN ('issued','expired') AND expires_at<? AND EXISTS (SELECT 1 FROM agent_tasks WHERE id=? AND status='delivered')`。
   - `try_dispatch`：在派发状态更新成功（`rows_affected()==1`）之后、`send` 之前调用；派发被拒绝时不调用。
   - 在文件末尾登记 `lease_window_tests` 测试模块。
2. `api/src/agents/lease_window_tests.rs`（新增）：见验证计划。
3. 文档：`docs/standards/agent-control-protocol.md`（租约计时规则）、`docs/runbooks/deployment-recovery.md`（排障：`secret_lease_lease_not_available` 的含义与历史查询）、`docs/solutions/2026-11-12-lease-expired-while-queued.md`（解决记录）；登记 019。

## 验证计划

- 单元与集成（内存 SQLite，真实函数）：
  1. 复现：租约经真实 `expire_secret_leases` 变成 `expired` 后，未重新计时时 `resolve_secret_lease` 返回 `lease_not_available`；重新计时后返回私钥。
  2. `try_dispatch` 派发时重新计时（含租约已是 `expired`）；窗口约 60 秒。
  3. 门禁拒绝派发时租约不变；Agent 未连接时不变。
  4. 重新派发再次计时。
  5. 窗口选择：发布 600 秒、构建 60 秒、其他 60 秒；不超过任务截止时间；截止时间已过或无法解析时不改动；只延长不缩短。
  6. 范围：已取用（`granted`，包括已到期被清理为 `expired` 但有 `granted_at`）、`rejected`、其他任务的租约不变。
  7. 任务当前不是 `delivered`（排队、运行、失败、取消、成功）时租约不变。
- 变异验证：临时去掉 `try_dispatch` 里的调用，用例 1、2、4 必须失败；临时去掉 `granted_at IS NULL`、“只延长”条件、截止时间上限、“任务仍是 `delivered`”条件，或把复活范围缩回 `issued`、把重新计时挪到派发之前，各自对应的用例必须失败；还原后全部通过。
- `cargo test -p deploy-go-api --lib lease_window`，`make api-check`，`make docs-check`，`make whitespace-check`，`make spec-kit-verify`，`make check`。
- 线上（需授权部署后）：只读统计新增 `secret_lease_lease_not_available` 数量。

## 风险与回退

- 风险：租约窗口变长带来的凭证暴露面。控制：只延长从未被取用的租约；取用校验不变；窗口不超过任务截止时间；构建窗口保持 60 秒。
- 回退：回滚控制面二进制，无 schema 或协议变化；已延长的租约在窗口后自然过期。
