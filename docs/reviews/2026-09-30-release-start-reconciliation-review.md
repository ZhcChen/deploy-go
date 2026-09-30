---
title: 特权 ReleaseStart 结果不确定修复复核
date: 2026-09-30
---

# 特权 ReleaseStart 结果不确定修复复核

## 范围与结论

复核 `docs/plans/2026-09-30-001-release-executor-ipc-diagnostics-plan.md` 的诊断改动，以及 `docs/plans/2026-09-30-002-privileged-release-start-reconciliation-plan.md` 的状态机修复。本轮只修改 Deploy Go，未操作真实节点、未发布控制面、未重发业务部署。

可确认的修复是：第一次 ReleaseStart 结果不确定后，Agent 最多重试同一请求一次，随后监控原 durable job，不因第二次错误推定第一次未启动。该结论不能证明原生产部署的实际失败根因。

## 复核发现与处理

- 首次启动不确定后，重试返回确定拒绝仍需查询原 job。已覆盖 ACK 丢失、两次 ACK 丢失、重试冲突和明确拒绝。
- 取消可能早于 admission。monitor 在取消标记存在时对同一 job 补发 cancel，直到终态；覆盖不再重试 Start、取消补发及唯一终态。终态后重复取消不增加 executor 请求。
- monitor 原先未校验响应绑定。现在 Output、Status、Exited 均校验协议版本和 job ID；错误版本/job 的输出不落盘，错误终态不被接受。
- executor 在持久化 Sealing 后可检测的启动失败尽力写入 Failed；若 spawn 后不能写 Running，先终止子进程与进程组。终态落盘失败记录稳定 job/error 日志。
- 重试重构曾丢失诊断元数据，已恢复 IPC 帧长度、上限、IO 类别、期望与实际协议版本、job 匹配布尔值；不记录正文、Env 或授权。
- 幂等测试移除固定睡眠和完整快照相等假设，改为身份字段一致、实际脚本副作用恰好一次。

## 验证与覆盖

- `cargo test -p deploy-go-agent -p deploy-go-agent-executor`：248 项通过。
- 诊断阶段已通过 External API 21 项与 External OpenAPI 契约 9 项测试。
- `cargo fmt --all --check`、`git diff --check` 通过。
- 本机为 macOS，Linux 专属 cgroup 测试未执行；本结果不能代替 Linux 配对 Agent/executor 验证。
- 独立复核覆盖正确性、可靠性、测试、项目规范及对抗性检查。Claude 跨模型审查返回 402、未执行有效审查，使用本地独立复核补位；未将其计为通过的跨模型审查。

## 已知限制与发布边界

- Agent 在发送 Start 前崩溃，或 executor 在建立 durable job 前持续存储失败，可能留下没有 job 的恢复任务。当前恢复不持久化带秘密的 Start，不重新授权重放；保持未确认状态，需只读核查。
- 如果写 Failed 本身也失败，磁盘仍可能保留 Sealing；本轮新增诊断，未引入新的持久化机制。
- 以上情形不能通过超时、清理目录或创建新部署安全推定业务尚未执行。
- 当前没有生产故障根因已证实或修复已在线生效的结论。发布必须成对交付 Agent/executor，并在明确授权的环境验证；仅更新 API 不会使节点侧修复生效。
