---
title: 镜像直连部署（Redis/PostgreSQL，免业务 Git 仓库）实施计划
date: 2026-08-11
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: 会话确认（用户验收后要求部署流程不走业务 Git 仓库）
execution: code
---

# 镜像直连部署实施计划

## Goal Capsule

- **目标**：新增 `image` 执行模式，让 Redis/PostgreSQL 这类有官方 Docker
  镜像的应用直接在 Deploy Go 中配置镜像、端口与 Env 后部署，不再要求业务
  Git 仓库、`deploy-go-prepare` 业务脚本或 launcher。
- **核心边界**：Executor 仍然只执行固定
  `make --no-print-directory deploy-go-release`；不接受任意 command、
  executable、args、Make target 或 env map；镜像字段使用白名单结构。
- **兼容要求**：现有 `script` 单脚本、`two_stage` Git 两阶段和 launcher
  兼容路径保持可用，不自动降级。
- **完成条件**：Redis/Postgres 模板可免 Git 完成应用创建、目标配置、Env
  首次登记、部署、逐节点状态与取消；全量门禁通过。

## 现状与问题

当前两阶段部署强绑定 Git：

1. 应用必须配置 Git 来源并固定分支；
2. prepare 任务由 Agent 检出仓库后运行业务仓库的 `deploy-go-prepare`；
3. release 任务从 checkout 运行 `deploy-go-release`；
4. 模板向导只预填字段，模板文件仍需复制到独立 Git 仓库。

这导致即使 Redis/Postgres 有官方镜像，也必须先准备业务仓库，与用户期望的
“直接用 Docker 镜像部署”不符。

另外，Env 首次登记目前只有 API 端点（
`/api/v1/agent/env-registration-leases/{lease_id}/register`），但没有调用方、
没有 Agent 上传客户端，也没有 Web 首次登记入口；模板 `prepare.sh` 也没有
生成或上传 Env manifest。镜像部署必须同时补上首次登记能力。

## 产品契约

### 镜像部署应用与目标

- R1. 应用可以选择 Git 来源或镜像模板；镜像模板不要求 Git 来源。
- R2. 本期内置模板固定为 `redis` 与 `postgres`，由 Deploy Go 平台维护模板
  文件，业务方不需要提供业务仓库。
- R3. 镜像部署目标执行模式为 `image`，必须同时开启 `privileged_release`。
- R4. 镜像部署目标只接受固定字段：
  - `template`：`redis` 或 `postgres`；
  - `image`：Docker 镜像引用（registry/path:tag 或 @digest）；
  - `host_port`：宿主端口；
  - `env_files`：本应用已登记的 Env 文件名白名单。
- R5. 容器端口、数据卷、配置挂载、健康检查和重启策略由模板固定，不接受
  任意 compose、command、args、env map 或资源参数。
- R6. 镜像部署没有业务 Git prepare；发布物由主控使用共享模板生成并托管，
  目标 Agent 通过既有 HTTPS artifact lease 下载后复验。

### 执行与安全

- R7. release 仍由 root executor 执行固定
  `make --no-print-directory deploy-go-release`；本机 executor 协议保持 v2，
  复用现有 ReleaseStart/ReleaseStatus/ReleaseOutput/ReleaseCancel、签名授权、
  durable job、cgroup、磁盘预算和日志回传。
- R8. 主控对镜像部署签发 release authorization 时，必须核对 Agent 生成的
  checkout 树摘要、artifact manifest、Env 摘要与部署快照一致；不通过则不
  授权，executor 不启动。
- R9. 镜像引用只允许安全字符，不允许空格、控制字符、shell 元字符或
  `--` 开头；端口范围 1-65535；`env_files` 必须来自应用配置。
- R10. Env 仍由应用配置唯一管理，先同步节点再发布；未同步或版本不匹配的
  目标停在 Env 门禁前。

### Env 首次登记

- R11. 镜像部署应用首版 Env 由管理员在「应用详情 → 应用配置」页面登记/导入，
  不再依赖业务仓库 prepare 上传。
- R12. 首次登记同样经过管理员重新认证、CSRF、dotenv 校验、大小/数量限制和
  加密存储；后续编辑继续走既有版本化、同步和审计。
- R13. 已有 Git 两阶段应用的 Build Agent 首次登记客户端留待后续计划，不改
  历史 `application_envs_register` 契约。

## 架构决策

- **复用 privileged release，而不是新增 executor 容器操作**：镜像部署继续
  让 executor 执行固定 Make target，最大化复用签名授权、durable job、cgroup、
  取消/超时和日志链路，避免再造一套 root 执行器。
- **平台托管 artifact**：主控用共享 `container-template` crate 生成固定
  artifact（`template/template.tar.gz` + `deploy-go-artifact.json`），存入既有
  artifact store；目标 Agent 走既有 HTTPS 下载和复验。
- **Agent 生成固定 checkout**：Agent 用同一共享 crate 在任务目录生成只读
  Makefile 与 `scripts/release.sh`，不执行 Git 检出；主控授权时用共享 crate
  复算 checkout 摘要。
- **控制协议 v8**：在 `DeploymentReleaseTask` 增加可选
  `image_spec: Option<ImageDeploySpec>`；只允许协议 v8 的 Agent 接收镜像任务，
  旧 Agent 不兼容时明确失败，不降级。
- **不新建任务 kind**：镜像部署仍使用 `deployment_release` 任务和
  `stage=release`，减少状态机分叉；API 侧跳过 prepare 阶段。

## Implementation Units

### U1. 协议与共享模板契约

- **Files**
  - 新增 `container-template/` crate；
  - `agent-protocol/src/lib.rs`、`agent-protocol/schema/agent-control.schema.json`；
  - `agent-protocol/tests/schema_compatibility.rs`；
  - `docs/standards/agent-control-protocol.md`。
- **Approach**
  - 定义 `ImageDeploySpec`、`ImageTemplate` 和校验函数；
  - `DeploymentReleaseTask` 增加 `#[serde(default)] image_spec`；
  - 控制协议版本从 v7 升到 v8；
  - `container-template` 提供 Redis/Postgres 的固定 compose、配置文件、
    Makefile、release 脚本、artifact 生成与 checkout 摘要函数。
- **Verification**
  - `cargo test -p deploy-go-agent-protocol`；
  - schema 兼容测试覆盖 v8 镜像载荷与旧载荷；
  - `make agent-check`。

### U2. 数据库与目标 API

- **Files**
  - 新增 `api/migrations/0020_image_deployment.sql`；
  - `api/src/deployment_targets/mod.rs`；
  - `api/src/lib.rs`、OpenAPI 生成物与双端 client。
- **Approach**
  - `deployment_targets.execution_mode` CHECK 增加 `image`；
  - 新增 `image_spec_json` 列；
  - 目标 create/update/list/show 支持 `image_spec`；
  - 校验镜像引用、端口、Env 文件名、`privileged_release=true`；
  - 快照 hash 包含 image spec。
- **Verification**
  - `cargo test -p deploy-go-api --test deployment_targets_api --test migrations --test database_constraints`；
  - `make api-openapi-check && make api-client-check`。

### U3. API 镜像部署状态机与平台制品

- **Files**
  - `api/src/deployments/mod.rs`、`api/src/agents/dispatcher.rs`；
  - `api/src/artifacts/mod.rs`（或新增 helper）；
  - `api/tests/two_stage_deployment.rs`、`api/tests/agent_dispatcher.rs`。
- **Approach**
  - 镜像部署 preview/confirm 不要求 Git 来源；release_version 缺省自动生成；
    resolved commit 使用镜像 spec 的固定 40 位摘要；
  - 主控按共享 crate 生成 artifact 并原子写入 artifact store；
  - dispatcher 跳过 prepare，直接为目标创建带 `artifact_download` 的 release
    任务；
  - release authorization 支持 `image_spec` 分支：核对预期 checkout 摘要、
    artifact manifest、Env 摘要；
  - 恢复、重试、取消、TTL 复用既有机制。
- **Verification**
  - 聚焦 dispatcher 与 deployment 集成测试；
  - `make api-openapi-check`。

### U4. Agent 镜像部署桥接

- **Files**
  - `agent/src/task_handler.rs`；
  - `agent/src/executor.rs`（如需要）；
  - `agent/tests/` 新增镜像 release 测试 fixture。
- **Approach**
  - `image_spec.is_some()` 时：下载 artifact → 复验 → 生成固定 checkout →
    跳过 Git 检出 → 走 privileged release admission；
  - Env 门禁与 `DEPLOY_ENV_DIR` 复用；
  - 重启后通过持久化 journal 恢复 `release_` job。
- **Verification**
  - `cargo test -p deploy-go-agent`；
  - mock executor 集成测试覆盖 env gate 失败零调用、断线恢复、唯一终态。

### U5. Web 首次 Env 登记

- **Files**
  - `api/src/application_envs/mod.rs`、`api/src/lib.rs`；
  - `admin/src/features/application-envs/`；
  - OpenAPI 与双端 client。
- **Approach**
  - 新增管理员「登记/导入」接口：重新认证后创建初始 Env 文件；
  - 应用配置空状态提供登记入口；
  - 复用 dotenv 校验、加密、版本化、同步行和审计；
  - 不在列表/日志/错误中暴露明文。
- **Verification**
  - API 权限与并发测试；
  - `npm test --workspace deploy-go-admin -- --run src/test/ApplicationEnvManagement.test.tsx`。

### U6. 管理端镜像部署 UI

- **Files**
  - `admin/src/features/targets/TargetEditor.tsx`；
  - `admin/src/features/templates/CreateFromTemplatePage.tsx`；
  - `admin/src/features/deployments/NewDeploymentPage.tsx` 与详情页；
  - 相关测试与 Playwright。
- **Approach**
  - 目标编辑器新增 `image` 模式：模板、镜像、宿主端口、Env 文件选择；
  - 模板向导镜像模式不再要求 Git 来源；
  - 部署预览/详情展示镜像与模板，不再展示固定分支/Commit；
  - 权限与 root 信任确认沿用现有交互。
- **Verification**
  - `make admin-check`；
  - 聚焦 Vitest 与 browser smoke。

### U7. 文档、模板同步与 review

- **Files**
  - `docs/runbooks/application-templates.md`、`docs/runbooks/application-onboarding.md`；
  - `examples/templates/` 与 `container-template` 同步校验；
  - `docs/reviews/2026-08-11-direct-image-deployment-review.md`。
- **Approach**
  - 说明镜像模式不需要业务 Git 仓库；
  - 新增镜像部署 runbook；
  - 执行 `make app-template-check` 与高风险 review。
- **Verification**
  - `make check` 全量通过；
  - review 记录发现与修复。

## Verification Contract

| Gate | Command | Passing signal |
|---|---|---|
| Rust | `make api-check`、`make agent-check` | 格式、clippy、workspace 测试通过 |
| OpenAPI/client | `make api-openapi-check`、`make api-client-check` | 生成物一致 |
| Admin | `make admin-check` | typecheck、lint、unit 通过 |
| 模板 | `make app-template-check` | 模板契约与镜像模板同步 |
| 特权 | `make privileged-release-check` | 既有特权链路无回归 |
| 全量 | `make check` | 全部门禁通过 |
| Diff | `git diff --check` | 无空白/冲突 |

## Definition of Done

- Redis 与 PostgreSQL 可经 `image` 模式完成：创建应用（无 Git 来源）→ 配置
  镜像目标 → Web 首次登记 Env → 发起部署 → Env 同步 → root executor 固定
  release → 健康检查通过。
- 镜像部署不支持任意命令/参数/Compose/env map；旧 launcher、低权限 release
  与 Git 两阶段保持兼容。
- 控制协议 v8、executor 本机协议 v2、Agent/executor 配对版本一致；
  `privileged_release` 能力仍然由目标开关控制。
- 本地隔离测试复现多节点镜像部署、Env 门禁失败零调用、取消/超时、断线续传。
- 未获得用户明确授权前，不连接或修改任何真实节点；WSL 与正式节点升级需单独
  申请授权。

## 范围边界

- 本期不做：任意镜像 compose 编辑器、任意 command/args、Secret 管理集成、
  Build Agent 的 Git 模式 Env 上传客户端、业务 Git 模板仓库。
- 本期不改：executor 本机协议 v2、ReleaseAdmission/ReleaseJobManager 主链路、
  历史 migration。
