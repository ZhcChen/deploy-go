---
date: 2026-08-07
topic: agent-reconnect-sequence-gap
plan: docs/plans/2026-08-06-004-git-branch-two-stage-deployment-plan.md
---

# Agent 重连序号缺口误判

## 问题

Agent 在执行长任务期间刷新 access token 并重建 WebSocket 时，少量输出可能已经写入 Agent journal，但尚未送达主控。重连对账如果要求双方 `last_sequence` 严格相等，会把仍在正常执行的任务错误标记为 `reconcile_mismatch`，即使业务脚本之后以 `exit_code=0` 完成。

典型证据：

- 主控 `agent_tasks.last_sequence` 落后于 Agent `journal.json.last_sequence`。
- 双方 `payload_digest` 一致，Agent 报告状态为 `Running`。
- 主控在 Agent token refresh 和新的 WebSocket Upgrade 后立即记录 `Agent 恢复对账不一致`。
- Agent systemd 日志显示同一任务仍继续执行，且本地 `completion.json` 最终成功。

## 结论

- `payload_digest` 不同、Agent 报告 `Unknown` 或 Agent 序号倒退，仍按不可信状态中断任务。
- digest 一致、状态已知且 Agent 序号领先时，说明主控缺失了重连窗口内的事件；主控将 `last_sequence` 条件推进到 Agent journal 序号，再接受后续连续事件。
- 条件更新必须同时校验旧序号和活动任务状态，避免与终态或并发消息竞争。
- 序号推进写入 warning，明确记录缺失区间；已丢失的 stdout/stderr 不伪造、不补写。

## 排查步骤

1. 从部署详情确认 `reconcile_mismatch`、阶段任务和发生时间。
2. 对照主控 `agent_tasks.last_sequence` 与节点任务目录的 `journal.json.last_sequence`、`payload_digest`。
3. 检查同一时间是否出现 `/api/v1/agent/refresh` 和新的 `/api/v1/agent/control` Upgrade。
4. 查看 Agent `completion.json` 与业务服务健康状态，不能仅根据主控误判决定再次发布。

## 验证

```bash
cargo test -p deploy-go-api --test agent_dispatcher reconnect_reconcile
```

测试同时覆盖序号领先后继续接收下一条输出，以及 payload digest 不一致仍然中断。
