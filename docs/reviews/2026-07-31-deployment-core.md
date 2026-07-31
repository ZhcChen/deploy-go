---
title: 部署内核高风险复核
date: 2026-07-31
status: accepted
plan: docs/plans/2026-07-31-api-foundation-and-deployment-core.md
unit: U8
---

# 部署内核高风险复核

## 范围

- 部署预览、确认、幂等、取消、重试和授权。
- SQLite 队列领取、目标串行、全局并发和重启恢复。
- OpenSSH 包装器、参数编码、PID 取消协议和凭证使用。
- stdout/stderr 流式持久化、事件裁决、脱敏、限额、保留和 SSE 续传。

## 已修正发现

- 取消失败后的 `interrupted` 可能被迟到执行结果覆盖：终态更新改为允许源状态 CAS，并增加交错测试。
- 配置状态变化导致取消卡在 `canceling`：取消上下文不再要求资源仍可承接新部署，无法构造或确认取消时收敛为 `interrupted`。
- 远端 PID 未生成、无效或信号失败被误报为成功：包装器增加取消文件和状态文件握手，PID 严格校验，TERM 宽限期固定为 30 秒，无法确认返回失败。
- 包装器缺少完整性校验：执行前校验发布物内置 SHA-256。
- 日志在进程退出后一次性读取：改为有界 channel 并发读取 stdout/stderr，按输出块分批提交数据库。
- 冲突的 `deploy.finished` 或退出码可能误判成功：累计协议冲突并按失败侧裁决，追加 `protocol_conflict` 诊断。
- SSE 长连接不复核授权且没有终态事件：每轮复核用户、会话和应用授权，发送明确终态后关闭。
- 幂等键未按路由隔离：确认和重试分别使用内部 route scope。
- 重试复用旧快照：重试前按当前目标重新预览并校验前置条件，生成新快照并关联原部署。
- 日志限额和保留配置未完整执行：追加截断与预算诊断，worker 定期清理已过保留期的终态部署输出但保留部署历史。

## 验证

- mock executor 覆盖成功、非零退出、协议不完整、协议冲突、非法 UTF-8、分块 UTF-8、异常事件、日志限额和敏感路径脱敏。
- paused executor 证明进程结束前日志已提交，并验证迟到结果不能覆盖 `interrupted`。
- mock OpenSSH fixture 证明严格参数和 stdout/stderr 流式输出，不访问真实节点。
- HTTP 集成测试覆盖幂等冲突、授权隐藏、排队取消、失败重试、跨路由幂等键、SSE 游标续传、终态事件和列表分页。
- 恢复测试覆盖 queued 保留、running/canceling 转为 `interrupted` 及日志保留清理。

## 结论

U8 的授权、注入、凭证泄漏、竞争、取消和恢复风险已完成聚焦复核。远程执行仍必须遵守 `AGENTS.md` 的显式授权边界；本轮验证仅使用本地 fixture 和 mock。
