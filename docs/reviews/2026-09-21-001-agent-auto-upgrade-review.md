---
date: 2026-09-21
plan: docs/plans/2026-09-21-001-agent-auto-upgrade-plan.md
type: cross-review
---

# Agent 自动升级计划复核

## 复核结论

计划可以进入开发，但必须把以下约束作为实现门禁：

- 自动升级必须是 Agent、runner、executor、updater 和四个 systemd unit 的成对发布；旧 Agent 首次仍需人工 bootstrap。
- 全局租约不能代替节点维护门禁。候选检查与最终 claim 必须分开，最终 claim 在同一 SQLite 写事务中重新检查在线、空闲、能力、架构，并绑定 `node_id`、`lease_token`、`lock_epoch`。
- executor 自身会在升级中断，因此升级所有权必须由独立 root `agent-updater` oneshot 和持久化 journal 接管；不能依赖 executor 进程存活。
- WSS 重连、Env 补偿、普通任务派发和终端创建都必须经过维护门禁；旧连接不能释放新任务的门禁。
- Agent 升级消息只允许在 v17 且声明 `agent_upgrade_v1` 时下发。v11-v16 的旧消息形状和既有任务能力必须保持兼容。
- manifest 摘要必须基于共享 canonical JSON 结果；自动升级只选择 Linux amd64，历史 ARM/v3 下载兼容保留但不能成为候选。
- 管理端 WebSocket 只做失效通知，HTTP 快照是权威来源；事件需要 `boot_id`、序号、overflow/resync marker，并持续做 session/RBAC 检查。HTTP 轮询不能完全关闭。

## 已修正的计划缺口

- R8 从三个 unit 修正为 Agent、runner、executor、updater 对应的四个 unit。
- KTD11 补充 session/RBAC 存活检查、`boot_id`、序号缺口、overflow 和快照恢复规则。
- U7 明确 WebSocket 正常时只能降低轮询频率，不能关闭 HTTP 兜底。
- U8 补充 `api-migrations` runbook；验证顺序调整为先生成客户端再执行检查，并增加 staged migration 门禁。
- API 测试场景补充管理员升级 retry 的契约覆盖要求。

## 开发时重点复核

1. 先验证协议兼容和旧 fixture，再增加升级消息，避免 v16 Agent 收到未知字段。
2. 迁移必须从空库和当前 schema 同时验证，且不修改历史 migration。
3. 所有升级状态转换、lease heartbeat、report 和 lock release 必须做 CAS；重复 report 和旧 token 不能改变新任务。
4. 发布物检查必须拒绝缺 updater、缺 unit、缺 manifest 文件或架构不符的成对版本。
5. 真实节点升级不作为本轮本地开发的验证手段；正式控制面发布与 Agent bootstrap 分开执行。
