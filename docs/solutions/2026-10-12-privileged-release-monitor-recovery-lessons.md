---
date: 2026-10-12
topic: privileged-release-monitor-recovery-lessons
related_spec: specs/004-privileged-release-monitor-recovery/
---

# 特权 release monitor 终态收敛：可复用经验

## 适用场景

修改 `agent/src/task_handler.rs` 的 `monitor_privileged_release`、reconcile / 取消恢复、日志 outbox 交付，或为这些路径写故障注入测试时参考。现场事故见 `docs/solutions/2026-10-12-privileged-release-result-not-reported.md`，规格与证据见 `specs/004-privileged-release-monitor-recovery/`。

## 经验

1. **executor 的 `truncated` 是 job 级标志，不是分页标记。** `agent-executor/src/release_job.rs` 在 job 终态时写入 `output_truncated`，之后每一页输出都带 `truncated=true`。早期测试 fake 只在最后一页返回 true，掩盖了一个真实缺陷：Agent 重启后才观察到“已因输出超限失败、且剩余可读帧超过一页”的 job，会被误判成 `interrupted`，而不是读完剩余帧再收敛为 `failed/output_limit_exceeded`。fake executor 的字段语义必须对照真实实现，并同时覆盖“运行中观察”和“重启后才观察到终态”两条时间线。
2. **退避计数要按进度，不能按轮次。** 终态 job 的多页输出若每读一页就累加退避，页数多时每页最长等 5 秒（8 页实测约 13 秒）。有新帧就立即读下一页并清零；无进展才退避，并设上限——否则 executor 宣告了更高的终态序号却始终不提供帧时会无限等待。
3. **不要在对账批处理里等长期持有的锁，也不要因此跳过派发等待者。** reconcile 抢不到 monitor owner 锁时只上报并跳过，会让“旧 owner 因旧连接关闭而退出”之后无人接管，任务停在 running 直到下一次重连。正确做法是 reconcile 完全不碰锁，把“等 owner → 持锁重读 journal → 带重试地载入 descriptor → 续接原 job”整体放进独立任务；descriptor 暂时读不到（非缺失/损坏）也要退避重试，而不是放弃监控。
4. **回执门禁只对协议 ≥ v18 的 Agent 生效。** 测试里 `UPDATE agent_task_event_receipts SET committed=0` 对低版本 fixture Agent 是对 0 行的空更新，随后的“重放不改版本/时间”断言会空转。把 fixture Agent 升到 v18，并断言该 UPDATE 恰好命中 1 行。
5. **重放测试要证明“补齐”，不只证明“稳定”。** 除了断言重复结果不改变版本和更新时间，还要把 target run / 父 deployment 篡改回运行态（模拟投影中途崩溃）再重放，确认下游被补齐到与结果一致。
6. **`reconciled()` 对任何带 `transfer_phase` 的任务（含特权 release）上报 `Accepted`**，由主控幂等重派、靠 Duplicate ACK 重新挂接 monitor。不要在对账里人为降级成 `Running`，那会阻止重派。

## 故障注入手法

- journal 暂时不可读：把 `journal.json` 换成目录（`EISDIR`，读写都失败），恢复后换回原内容。必须等 monitor 先接管并观测一次状态，否则 `recover()` 先失败会走 fail-closed 的 `Unknown`。
- descriptor 暂时不可读：把 `privileged-release-task.json` 换成目录（读返回 `EISDIR`，区别于 `NotFound` 的“缺失”）。
- outbox 写入失败：把 `log-outbox-v1` 置为 `0o500`。写失败而读正常，能精确命中 `event_outbox_delivery` / `result_delivery` 重试分支；对 root 无效，用能力探测在 root 下自动跳过，不要误报。破坏 outbox 的结构（占位文件、符号链接）会连带让 `journal.load()` 失败，打不到交付分支。
- 帧目录永久损坏：缺失、不可解析、软链接、超 1 MiB、被目录占用五种形态都应收敛为 `interrupted/release_output_persistence_failed`；任何一种退化成 transient 重试，用例都会超时失败。

## 验证断言是真守卫

临时把被测分支改坏，确认对应用例会失败，再原样恢复并校验文件一致。本轮对关键用例逐个做过变异验证；其中一次只让条件的前半失效，用例仍然通过——因为条件的另一半同样会在故障下失败并触发同一重试。要让整个条件失效，才能证明守卫真实有效。
