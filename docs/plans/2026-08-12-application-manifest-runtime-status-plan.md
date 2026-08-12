---
title: 应用类型清单与 Redis 运行时状态展示实施计划
date: 2026-08-12
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: code
---

# 应用类型清单与 Redis 运行时状态展示实施计划

## Goal Capsule

- 增加 `deploy-go.yaml` 应用清单规范：`type` 与 `type_version` 用于识别
  Redis、PostgreSQL、普通二进制等应用类型，平台侧保存同构元数据。
- 为部署目标增加稳定的 `target_code`，让现有手工 Redis（Compose 项目
  `deploy-go-shared-prod-redis`、数据卷沿用）在不部署、不重建容器的前提下
  可被 Deploy Go 绑定与标识。
- 新增只读运行时状态链路：Agent/executor 本机协议 v3 提供
  `RuntimeStatus` 固定只读操作，控制协议 v9 新增 `runtime_status_probe`
  任务，API 保存最近状态并在应用详情展示 Redis 容器运行/健康状态。

## 关键边界

- 本计划不修改 qfy-prod-1 上的现有 Redis 容器、数据卷、Compose 或 Env。
- executor 不接受任意 command、executable、args、Make target 或 env map；
  运行时状态只执行固定 `docker compose -p <target_code> ps` 只读查询。
- 不创建可发起部署的目标；绑定/元数据与状态读取均不改变线上运行状态。
- 旧 Agent/executor、低权限 release 与 launcher 保持兼容，不自动降级。
- 未经用户明确授权不连接真实节点；本计划仅本地实现、测试与文档。

## 实施单元

### U1. 应用清单规范与模板同步

- 新增 `docs/standards/application-manifest.md`，定义 `deploy-go.yaml`：
  `schema_version`、`type`（白名单）、`type_version`、`modules`、`env_files`。
- `examples/templates/redis/deploy-go.yaml` 与
  `examples/templates/postgres/deploy-go.yaml` 同步新增。
- `container-template` 解析并校验清单，生成的固定 checkout/artifact 包含
  `deploy-go.yaml`；`make app-template-check` 覆盖同步与非法字段拒绝。

### U2. 应用类型元数据与目标稳定标识

- migration `0022_application_manifest_and_runtime_status.sql`：
  - `applications` 增加 `app_type`、`type_version`；
  - `deployment_targets` 增加 `target_code`，历史值回填为环境值；
  - 增加 `(application_id, node_id, target_code)` 唯一索引；
  - 新增 `application_runtime_statuses` 表；
  - `agent_tasks` 增加 `runtime_status_id` 唯一关联。
- applications create/update/show、targets create/update/show 返回与校验
  新字段；`execution_spec` snapshot 与 dispatcher 使用 `target_code`
  注入 `DEPLOY_TARGET`。

### U3. executor 只读运行时状态

- executor 本机协议 v2 → v3，新增 `RuntimeStatus` capability、Request/Response。
- 只读操作固定执行 Compose 项目状态查询，返回白名单字段；失败只返回错误码，
  不暴露 Env、私钥或任意命令。
- `executor-client`、doctor/probe、安装器版本检查与协议测试同步。

### U4. Agent 控制协议与运行时状态任务

- 控制协议 v8 → v9，新增 `RuntimeStatusProbeTask` 与 Agent capability。
- Agent 收到 `runtime_status_probe` 后经持久化任务 journal 调用 executor，
  断线重连后重放终态；executor 不可用或协议不兼容明确失败。
- dispatcher 创建/恢复/完成运行时状态任务，API 持久化最近状态。

### U5. 应用详情状态展示

- 新增 `GET/POST /applications/{id}/runtime-status` 管理接口与 OpenAPI/client。
- 应用详情显示 `app_type` / `type_version` 徽标、绑定目标、`target_code`、
  最近状态（pending/running/succeeded/failed）、容器运行与健康摘要、
  “重新读取”按钮。
- UI 测试覆盖空状态、成功、失败与刷新动作。

### U6. 文档、复核与门禁

- 更新 `docs/runbooks/application-onboarding.md`、
  `docs/standards/agent-control-protocol.md`、
  `docs/standards/privileged-agent-executor.md`。
- 执行聚焦测试、OpenAPI/client 同步、admin 测试与 `make check`；
  记录 `docs/reviews/2026-08-12-application-manifest-runtime-status-review.md`。

## Verification Contract

| Gate | Command |
|---|---|
| 模板 | `make app-template-check` |
| 协议/Agent | `cargo test -p deploy-go-agent-protocol -p deploy-go-agent -p deploy-go-agent-executor` |
| API | `cargo test -p deploy-go-api --test migrations --test applications_api --test deployment_targets_api --test agent_dispatcher --test runtime_status_api --test openapi_contract` |
| OpenAPI/client | `make api-openapi-check && make api-client-check` |
| Admin | `npm run check --workspace deploy-go-admin` |
| 全量 | `make check` |
| Diff | `git diff --check` |

## Definition of Done

- `deploy-go.yaml` 规范与模板同步，Redis/PostgreSQL 类型可识别。
- 应用详情可编辑并展示应用类型与版本；目标可配置稳定 `target_code`。
- 管理员可在应用详情读取 Redis 当前 Compose 容器运行/健康状态，不修改线上
  容器，不输出 Env 明文。
- 旧协议 Agent/executor 兼容下限不变，新状态任务只下发 v9 Agent。
- 未连接或修改任何真实节点，也未创建任何可执行部署。
