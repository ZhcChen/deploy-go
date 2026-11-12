# 租约窗口从派发开始计时任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：已实施并通过本地门禁，待发布与用户签收。勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-001 至 FR-005 在 `try_dispatch` 派发成功后重新计时未取用的 Git 凭证租约，附测试模块 `lease_window_tests`。
- [ ] T002 SC-001 SC-002 变异验证：去掉调用或去掉范围条件，对应用例必须失败。
- [ ] T003 FR-008 FR-009 协议标准、排障手册、解决记录同步，登记 019。
- [ ] T004 SC-003 全量验证、自审、提交推送。
- [ ] T005 SC-004 上线与线上观察：需用户授权“部署正式环境”；记录上线后的新增失败数。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已完成 | 已验证 | 未发布 | 待用户签收 | `api/src/agents/dispatcher.rs` 新增 `git_lease_window` 与 `restart_git_lease_window`，在 `try_dispatch` 派发成功后、发消息前调用（+70 行，均为新增，无删改；新测试文件 661 行）。8 个用例（`lease_window_tests`）：复现线上场景（真实 `expire_secret_leases` 把租约清理成 `expired` 后 `resolve_secret_lease` 返回 `lease_not_available`，重新计时后取到私钥，再取被拒绝）、派发接入、门禁拒绝与 Agent 未连接时不改动、重新派发再次计时、窗口长度（构建与其他 60 秒、发布 600 秒）、只延长且不超过截止时间（截止时间已过或无法解析时不改动）、只动从未取用的租约（`granted`、取用后到期的 `expired`、`rejected`、其他任务均不变）、任务不是 `delivered`（排队、运行、失败、取消、成功）时不改动 |
| T002 | 已完成 | 已验证 | 不适用 | 待用户签收 | 7 项变异验证，每项都被对应用例抓到，随后字节级还原并重跑全绿：M1 去掉 `try_dispatch` 里的调用，2 个失败；M2 去掉 `granted_at IS NULL`，1 个；M3 去掉“只延长”条件，1 个；M4 去掉截止时间上限，1 个；M5 复活范围缩回 `issued`，5 个；M6 在派发状态更新之前重新计时，1 个；M7 去掉“任务仍是 `delivered`”条件，1 个 |
| T003 | 已完成 | 已验证 | 不适用 | 待用户签收 | 更新 `docs/standards/agent-control-protocol.md`（租约计时规则与 `secret_lease_<原因>` 错误码）、`docs/runbooks/deployment-recovery.md`（症状、原因、修复与只读统计）；新增 `docs/solutions/2026-11-12-git-lease-expired-while-queued.md`（含诊断 SQL 与验证中的坑）；登记 019；`make docs-check` 通过 |
| T004 | 已完成 | 已验证 | 不适用 | 待用户签收 | 合并工作区 `make check` 通过：rust 84s（Rust 991 个用例通过、2 个跳过，含新增 8 个）、contracts 23s、docs 5s、web 21s、client 0s，总墙钟 85s；`cargo fmt --check` 通过。自审：diff 只含 `dispatcher.rs` 四处新增（两处接入 `try_dispatch`、新函数、测试模块登记）和新测试文件；Agent、协议、schema、OpenAPI 无改动；取用校验未改；日志只含任务 ID 与行数。全量并行运行中 Agent 包的 `monitor_diagnostics_and_results_never_expose_secrets_output_or_internal_paths` 出现一次 nextest “leaky” 提示（用例通过，本次未触及 Agent 包，单独重复 4 次均无提示），按偶发记录。提交与推送见 Git 历史 |
| T005 | 未开始 | 未执行 | 未部署 | 待用户签收 | - |
