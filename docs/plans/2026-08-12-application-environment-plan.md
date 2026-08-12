---
title: 应用环境标识与部署同步拓展计划
date: 2026-08-12
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: code
---

# 应用环境标识与部署同步拓展计划

## Goal Capsule

在 Deploy Go 的应用上增加环境标识（dev/test/staging/prod），并让两阶段部署按
应用环境向业务脚本注入 `DEPLOY_ENVIRONMENT`。目标是让 `qfy-voucher-hub-testing`
这类测试应用在部署时收到 `test`，不再被目标兼容字段 `prod` 误导为 production。

## 问题背景

- `deployment_targets.environment` 是只读兼容字段，创建/更新目标固定写 `prod`
  （`TARGET_ENVIRONMENT_COMPAT_VALUE`）。
- Agent 因此对全部两阶段任务固定注入 `DEPLOY_ENVIRONMENT=prod`，
  `DEPLOY_TARGET=prod`。
- 业务脚本（如 qfy-voucher-hub）按 `DEPLOY_ENVIRONMENT` 选择 testing/production
  profile，测试应用会误选 production Compose 与 Env。
- `applications` 表没有环境字段，应用身份无法作为环境来源。

## 设计决策

- **KTD1**：应用环境是唯一权威来源。`applications.environment` 使用与
  `agents.environment` 相同的四个枚举 `dev` / `test` / `staging` / `prod`。
- **KTD2**：部署目标环境跟随应用环境。目标创建/更新时不再写死 `prod`，而是绑定
  当前应用环境；应用环境变更时事务内同步该应用全部目标。
- **KTD3**：不修改 Agent 控制协议。dispatcher 已经按 `target.environment` 映射
  `Environment::Test` 等枚举，目标环境修正后现有链路自动注入正确
  `DEPLOY_ENVIRONMENT`。
- **KTD4**：现有数据通过 migration 回填：应用环境优先取目标节点 Agent 环境，
  目标环境再跟随应用环境；无法判定的保持 `prod`。
- **KTD5**：对外部署 API 只增加应用环境字段，不读取 Env、不扩展管理面能力。

## 实施单元

### U1. 应用环境字段与数据迁移

- **Files**
  - `api/migrations/0021_application_environment.sql`
  - `api/tests/migrations.rs`
- **Approach**
  - `applications` 增加 `environment TEXT NOT NULL DEFAULT 'prod'`，带
    `CHECK (environment IN ('dev','test','staging','prod'))`。
  - 回填应用环境：按目标节点关联的未吊销 Agent 环境取值，无关联保持 `prod`。
  - 回填部署目标环境：目标跟随所属应用环境，并递增 `version`、更新
    `updated_at`。
  - migration 测试覆盖：空库升级、带 Agent/目标的数据升级、回填后目标环境一致。

### U2. API 与部署目标继承应用环境

- **Files**
  - `api/src/applications/mod.rs`
  - `api/src/deployment_targets/mod.rs`
  - `api/src/external/mod.rs`
  - `api/openapi/openapi.json`、`api/openapi/external.json`
- **Approach**
  - `ApplicationResponse` 增加 `environment`；`SaveApplicationRequest` 必填
    `environment` 并在校验中限定四枚举；create/update/list/show SQL 同步。
  - 应用环境变更时，同一事务内同步该应用全部目标的环境并更新版本；审计记录
    环境变更前后值。
  - 目标 create/update 不再写死 `prod`，改为读取应用环境；对外应用列表/详情
    返回应用环境。
  - 删除 `TARGET_ENVIRONMENT_COMPAT_VALUE`，目标响应仍保留 `environment` 只读。
- **Verification**
  - applications/targets 聚焦测试通过；
  - OpenAPI 与外部 OpenAPI 契约测试通过；
  - external 测试断言应用环境返回。

### U3. Admin、模板向导与客户端同步

- **Files**
  - `admin/src/features/applications/ApplicationsPage.tsx`
  - `admin/src/features/applications/ApplicationDetailPage.tsx`
  - `admin/src/features/templates/*`
  - `admin/src/test/*`、`admin-app` 生成客户端
- **Approach**
  - 创建/编辑应用增加环境下拉，展示“环境”徽标；
  - 模板向导创建应用时默认 `prod`，允许显式选择环境；
  - 重新生成 OpenAPI 与 Admin/Flutter 客户端，保持 fixture 同步。
- **Verification**
  - `npm run api:client:check`、`npm run check --workspace deploy-go-admin`；
  - 聚焦 Vitest 与 E2E mock。

### U4. 部署链路契约测试

- **Files**
  - `api/tests/agent_dispatcher.rs`
  - `api/tests/external_api.rs`
  - `api/tests/deployment_targets_api.rs`
- **Approach**
  - 测试应用环境 `test` 时创建目标返回 `test`，dispatcher 两阶段任务使用
    `Environment::Test`；
  - 应用环境变更后目标环境同步，旧 snapshot 不变，新 preview 使用新环境；
  - 外部 API `show-app` 返回应用环境与目标环境一致。

### U5. 文档、复核与全量门禁

- **Files**
  - `docs/runbooks/application-onboarding.md`
  - `docs/standards/application-deployment-contract.md`
  - `docs/reviews/2026-08-12-application-environment-review.md`
- **Approach**
  - 明确“应用环境是 DEPLOY_ENVIRONMENT 唯一来源，目标环境只读继承”；
  - 说明测试应用创建/编辑入口，以及 qfy-voucher-hub-testing 后续改环境为 test
    的步骤；
  - 执行 focused gates 与 `make check`，记录 review。

## Verification Contract

| Gate | Command |
|---|---|
| Migration/API | `cargo test -p deploy-go-api --test migrations --test applications_api --test deployment_targets_api --test agent_dispatcher --test external_api` |
| OpenAPI/client | `make api-openapi-check`、`make api-client-check` |
| Admin | `npm run check --workspace deploy-go-admin` |
| 特权回归 | `make privileged-release-check` |
| 全量 | `make check` |
| Diff | `git diff --check` |

## Definition of Done

- 应用可创建/编辑环境，环境四枚举与服务端一致。
- 目标环境始终跟随应用环境，部署任务注入正确的 `DEPLOY_ENVIRONMENT`。
- 现有应用通过 migration 回填且目标环境同步，无重复唯一键冲突。
- OpenAPI、Admin、Flutter、对外 API 契约同步。
- 全量门禁与复核记录完成，未修改任何真实节点或发起部署。
