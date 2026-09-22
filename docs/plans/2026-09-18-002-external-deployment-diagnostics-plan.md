---
title: External 部署诊断详情与日志查询
date: 2026-09-18
status: implemented
---

# External 部署诊断详情与日志查询

## 问题与目标

当前 External API 只能返回部署汇总状态。部署失败后，调用方拿不到退出码、失败阶段/步骤和可继续排查的日志，只能转到控制台或节点日志。目标是在不暴露内部管理字段、Agent 原始敏感信息或 SSE 控制语义的前提下，提供足够完成首轮故障定位的结构化详情和只读日志分页接口。

范围包括 `api/src/external/mod.rs`、External API 测试、OpenAPI 契约和必要的 API 文档；不修改数据库结构、不改变内部 SSE 日志接口、不改变部署执行流程。

## 已确定的设计

- `ExternalDeploymentTargetRun` 增加 `exit_code`、`failure_stage`、`failure_step`、`logs_available`、`last_log_sequence`。
- 详情字段从现有 `deployment_target_runs`、对应 `agent_tasks.result_json`、失败进度事件和 `deployment_logs` 派生；无法解析时保持 `null`，不让诊断查询影响部署详情主流程。
- 新增 `GET /external/v1/deployments/{id}/logs?after=<sequence>&limit=<n>`，返回 JSON `{items,next_after,terminal}`。
- 日志接口只读已持久化日志，游标单调递增，`limit` 有明确上限，单次结果受行数限制；不支持 SSE follow、控制 Agent 或任意跨部署查询。
- 复用部署所属应用与 External API Key 的访问校验。无权限和不存在的部署保持现有外部可见行为。
- 返回日志仅允许现有安全字段：`sequence`、`stage`、`stream`、`content`、`truncated`、`created_at`。实现脱敏，至少覆盖 token、密码/密钥类键值、Bearer 凭证和私钥块；必要时将内容标记为 `truncated` 或替换敏感值。
- 保持内部日志 retention 和失效游标语义；游标早于当前保留窗口时返回验证错误，不静默返回不完整结果。

## 执行单元

### U1：补充目标运行结构化失败详情

修改 `api/src/external/mod.rs` 的 External DTO、查询行和装配逻辑。增加目标运行退出码、失败阶段/步骤、日志可用性和最后日志序号；从任务结果和已有失败事件解析可用信息，避免将 `payload_json`、内部任务 ID 或节点路径直接暴露给 External 调用方。

验收：成功、运行中、失败且有任务结果、失败但无任务结果、无日志五类状态均能稳定序列化；主详情接口不因诊断字段缺失而 500。

### U2：新增 External JSON 日志分页

在 `api/src/external/mod.rs` 注册日志路由、查询参数和响应类型。实现应用权限校验、游标有效性校验、有限 `limit`、按 `sequence` 查询和 terminal 判断。抽取/复用安全日志映射，避免复制内部 SSE 的长轮询逻辑。

验收：可连续分页、不重复不乱序；部署终态且没有更多日志时 `terminal=true`；部署进行中或仍有日志时为 `false`；无权限、过期游标、非法 limit 均返回约定错误。

### U3：同步 OpenAPI、契约测试与接口说明

更新 External OpenAPI 路由、schema、参数和安全声明；扩展 `api/tests/external_openapi_contract.rs`，确保外部文档包含新接口但不包含内部 SSE、任务结果原文、凭证等字段。必要时在 External API 说明文档中记录游标和日志保留约束。

### U4：集成测试、安全复核与本地验证

扩展 `api/tests/external_api.rs` 覆盖 API Key 绑定应用访问、跨应用拒绝、分页、terminal、limit/游标校验、失败详情和敏感内容脱敏。执行 External API、OpenAPI、部署事件相关测试及 `git diff --check`。检查既有工作区改动不被暂存或提交。

## 风险与回滚

- 日志脱敏规则过宽会降低排障信息，过窄会泄露凭证；测试固定典型 secret/token/private-key 样本，规则集中在 External 映射层。
- 日志 retention 可能导致调用方保存的游标失效；明确返回验证错误，并依赖部署详情中的 `last_log_sequence` 重新开始可用窗口。
- 本次无 schema migration，回滚只需回退 API 代码和契约变更；不会影响已存储日志或部署执行。

## 完成定义

- External 详情可表达失败阶段、步骤、退出码和日志可用性。
- External 日志查询具备权限、分页、上限、terminal 和脱敏保护。
- OpenAPI 与集成测试覆盖新增契约及拒绝路径。
- 聚焦测试通过，差异检查通过，且本轮提交不包含工作区既有无关修改。

## 执行记录

- U1-U4 已实现：External 详情增加结构化目标运行诊断；新增受 Key 权限保护的 JSON 日志分页和脱敏；同步 OpenAPI 与契约/集成测试。
- 已验证：`cargo check -p deploy-go-api`、`cargo test -p deploy-go-api --test external_api`、`cargo test -p deploy-go-api --test external_openapi_contract`、`make api-external-openapi`。
- 完整相关门禁已通过；正式控制面发布不属于本轮自动动作。
