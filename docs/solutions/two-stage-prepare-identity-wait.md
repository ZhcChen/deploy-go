---
date: 2026-09-17
topic: two-stage-prepare-identity-wait
plan: docs/plans/2026-08-06-004-git-branch-two-stage-deployment-plan.md
---

# two_stage 准备阶段的进程身份等待窗口

## 问题

大仓库的 two_stage `prepare` 会被主控判为「Agent 拒绝任务」，错误码
`invalid_state`，但节点上业务脚本其实已经正常跑完。典型证据（qfy-mall
241MB 仓库、目标 timeout 3600）：

- `delivered` → +0.09s 写 `runner-spec.json` → **+5.09s 主控判失败**；
- +9.4~9.9s 才出现 `process.json`，随后 `completion.json` 为 `exit_code=0`，
  制品与 manifest 齐全；
- 三次发布失败的时间线完全一致，说明与业务脚本无关。

## 结论

`Executor::spawn_spec` 等待 `process.json` 的预算与实际耗时无关，而是硬编码的 5s；
two_stage runner 又**先**做 git 检出、**后**写 `process.json`
（`agent/src/runner.rs`，检出失败时直接写 completion 并退出）。因此 clone 时间
超过 5s 的仓库必然触发误判。

修复：身份等待预算跟随任务超时，`min(spec.timeout_seconds, 120s)`，下限 1s。

旁证：该等待不经过其他 5s 窗口——
`ExecutorClient::request` 的 5s 只用于特权 release / 终端，
`RunnerServiceClient::launch` 在 runner-service 侧是「启动即返回」，
`wait_for_completion` 的 5s 只用于任务已经产生 completion 之后的收尾。
平台侧 dispatch 截止时间是 `timeout_seconds + 60`
（`api/src/agents/dispatcher.rs` 的 `deadline_at`），120s 预算仍在窗口内。

## 残余风险

`spawn_spec` 在节点级 `admission_lock` 内执行，身份等待最长会占锁 120s。
如果未来要放宽该上限，先评估这个锁对其他任务准入的影响。

## 可复用结论

- 仓库历史里的大二进制会直接放慢每次 prepare 的全量 clone（`git clone`
  默认拉取所有分支，只瘦身单个分支无效），仓库侧控制体积比放大等待窗口更根本。
- 判断这类误判最快的三条证据：`process.json` 出现时间晚于失败时间、
  `completion.json` 为 `exit_code=0`、任务 stderr 无业务报错。
