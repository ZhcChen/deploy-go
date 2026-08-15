---
title: v11 Agent 必选特权执行模式计划
date: 2026-08-15
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: 会话确认（默认 Agent 使用特权 executor，旧 Agent 不再兼容）
execution: code
---

# v11 Agent 必选特权执行模式计划

## 目标

将 Agent v11 作为唯一受支持的控制协议与发布形态。节点不再配置“特权执行”开关；在线、身份有效且 executor 可用的 v11 Agent 默认提供管理员 SSH 终端与特权 release 能力。旧 Agent 不得注册、连接或接收任务。

## 范围与约束

- 删除节点级 `privileged_execution` 的 API、管理端开关、DTO 字段和运行时查询条件。
- 保留既有数据库列作为历史 schema，不能修改或重排已发布 migration，也不进行表重建。
- 将协议最低版本提升至 v11；控制面拒绝旧 Agent 的 enrollment 与 WebSocket Hello，并让调度器对持久化的旧 Agent 行保持硬门禁。
- 保留 executor 可用性、Agent 在线和身份有效性检查；安装损坏时终端仍不可用，但不再存在管理员手动启用步骤。
- 不执行控制面部署、节点升级、业务部署、数据迁移或切流。

## 实施单元

### U1. 协议与控制面兼容基线

- `agent-protocol/src/lib.rs`、`api/src/agents/auth.rs`、`api/src/agents/websocket.rs`、`api/src/agents/dispatcher.rs`、`api/src/application_sources/mod.rs`、`api/src/deployment_targets/mod.rs`
- 将 `MIN_SUPPORTED_PROTOCOL_VERSION` 提升到 11，并复用该常量取代零散的低版本阈值；旧协议的注册、Hello 和调度均产生稳定的“不支持”结果。
- 保持 v11 executor 能力门禁：缺失 `pty_terminal` 或 `privileged_release` 时，节点不可执行相应能力，不用开关绕过。

### U2. 移除节点级开关

- `api/src/nodes/mod.rs`、`api/src/terminals/mod.rs`、`api/src/terminals/store.rs`、`api/src/lib.rs`
- 移除更新端点、审计事件和 SQL 条件中的 `privileged_execution`；终端 capability 仅报告 Agent 与 executor 的实际可用性。
- 保留会话关闭、身份撤销和在线状态的既有保护逻辑。

### U3. 管理端与 OpenAPI 契约

- `admin/src/features/nodes/*`、`admin/src/api/generated/*`、相关测试与 OpenAPI 生成物
- 移除“启用特权执行”开关与其请求；SSH 页直接以 capability 显示可用或需要重装 v11 Agent 的状态。
- Node/terminal DTO 不再暴露该历史数据库字段。

### U4. 回归与运行文档

- `api/tests/terminal_api.rs`、`api/tests/agent_websocket.rs`、调度相关测试、`admin/src/test/*`、`docs/runbooks/privileged-agent-terminal.md`、`docs/runbooks/agent-onboarding.md`
- 覆盖 v11 自动可用、旧协议拒绝、executor 缺失不可用、旧持久化 Agent 不调度、普通用户仍无终端权限以及 UI 无开关。
- 明确正式升级顺序为先部署控制面、再重新安装每个旧 Agent；不允许旧 Agent 继续承担普通部署。

## 验收

1. `protocol_version < 11` 的 enrollment 与 Hello 均被拒绝，Agent 不更新 `last_seen_at`，调度器不下发任务。
2. v11 且具备 `pty_terminal` 的在线有效 Agent 可直接创建管理员终端会话，无 `privileged_execution` 路由、字段或 UI 开关。
3. executor 缺失、Agent 离线或身份失效继续返回稳定的 capability 错误码。
4. OpenAPI/client 与 Admin 单测同步，Rust 与前端聚焦检查通过，`git diff --check` 无问题。
