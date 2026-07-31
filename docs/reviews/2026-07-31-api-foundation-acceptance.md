---
title: API 基础与部署内核最终验收
date: 2026-07-31
status: accepted
plan: docs/plans/2026-07-31-api-foundation-and-deployment-core.md
unit: U9
---

# API 基础与部署内核最终验收

## 需求追踪

- R1-R4：唯一管理员、普通用户生命周期、session/CSRF 和 application grant 已由认证与授权集成测试覆盖。
- R5-R10：Ed25519 SSH 密钥、主密钥加密、节点绑定、host key 确认和 mock SSH 检查已覆盖。
- R11-R16：应用、部署目标、参数与路径约束、预览、snapshot hash 和幂等确认已覆盖。
- R17-R22：目标串行、全局 worker、流式日志、取消、重试和重启恢复已覆盖。
- R23-R26：POSIX token 编码、审计 API、日志脱敏/限额/保留、健康检查、分页、OpenAPI 和 migration 已覆盖。

## 端到端证据

`api/tests/end_to_end.rs` 从空内存数据库开始，通过 HTTP 完成管理员初始化、SSH 密钥生成、节点创建、host key 确认、节点检查、应用和目标创建、部署预览与确认，再由 mock executor 执行并通过详情和 SSE 验证成功终态。测试不连接真实节点，响应和日志不包含私钥。

## 契约与运维

- `api/openapi/openapi.json` 由 `make api-openapi` 生成，`make api-openapi-check` 检查漂移。
- `docs/runbooks/deployment-recovery.md` 记录 queued、running、canceling 和 interrupted 的恢复语义。
- 本地开发、migration、节点接入、主密钥轮换和部署恢复 runbook 与当前命令一致。
- `README.md` 和 `Makefile` 已列出 API、OpenAPI、UI 与全仓检查入口。

## 复核结论

最终复核重点检查了授权、参数与 shell 注入、私钥泄漏、队列竞争、取消竞态、SSE 续传、OpenAPI schema 和恢复文档。U8 复核发现及修正记录见 `docs/reviews/2026-07-31-deployment-core.md`。本计划不授权真实节点操作，也不包含 Web 或 Flutter 正式客户端工程。
