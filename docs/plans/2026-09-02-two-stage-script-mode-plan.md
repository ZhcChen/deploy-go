---
title: 新增通用两阶段脚本模式（two_stage_script）实施计划
date: 2026-09-02
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: 会话确认（新增模式，不修改既有 Git 两阶段与镜像模式）
execution: code
---

# 新增通用两阶段脚本模式实施计划

## Goal Capsule

- **目标**：新增 `two_stage_script` 执行模式，使 ClickHouse 等不依赖 Git
  来源的应用，可以在构建机器本地工作区执行 prepare，在目标机器执行 release
  （解压、拉镜像、启动、健康检查等），全程不要求 Git 来源或固定分支。
- **核心边界**：不改动现有 `script`、`two_stage`、`image` 三个模式；新增
  模式继续固定执行 `make deploy-go-prepare` / `make deploy-go-release`，
  不接受任意 root 命令或任意 Make target。
- **完成条件**：应用可无 Git 来源完成“创建工作区来源 -> 创建目标 -> 部署
  预览 -> prepare 打包 -> release 到目标机器”的端到端流程，并通过全量门禁。

## 产品契约

- R1. 新增执行模式 `two_stage_script`，部署目标仍支持模块、参数、Env 门禁、
  验证配置、多目标发布和取消/重试。
- R2. 新增独立 `application_workspace_sources` 来源，不修改既有
  `application_sources` Git 来源表。
- R3. 工作区来源配置包含：
  - `build_agent_id`：构建 Agent；
  - `workspace_path`：构建机器上的固定本地工作区；
  - `workspace_version`：管理员保存/修改时递增；
  - `status`：已验证状态。
- R4. prepare 在构建 Agent 上从固定工作区快照后执行
  `make deploy-go-prepare`，不执行 Git checkout。
- R5. release 在目标 Agent 上消费已校验发布物，使用原生特权 executor 固定
  执行 `make deploy-go-release`；镜像拉取、容器启动、健康检查等由业务脚本
  在目标机器完成。
- R6. 部署快照保存 `source_policy=workspace`、`workspace_path`、
  `workspace_version`，不保存 Git 分支/commit；内部使用固定的 workspace
  摘要作为 manifest `commit_sha` 兼容值。
- R7. 旧模式行为不变：Git `two_stage`、`image`、`script` 的校验、快照、
  调度和 Agent 任务结构均不回归。

## 架构决策

- **独立来源表而非放宽 Git 来源表**：`application_sources` 继续只表示 Git
  来源，新增 `application_workspace_sources`，从数据和 API 层面隔离两种来源。
- **复用两阶段状态机和 artifact 链路**：`two_stage_script` 在部署状态机、
  target run、artifact 上传/下载、Env 门禁、取消与恢复上完全复用现有
  `two_stage` 机制。
- **Agent 协议升级到 v14**：`SourcePolicy` 增加 `Workspace`，
  `DeploymentPrepareTask` 增加可选 `workspace_path`；旧 Agent 不接收新模式
  任务，失败时明确报错，不降级。
- **prepare 先快照工作区**：Agent 将固定工作区复制到任务 staging 的
  `checkout` 目录再执行，避免构建期间工作区被修改，并继续满足
  `checkout_dir` 必须在 `work_root` 内的安全约束。
- **release 统一消费已验证 artifact**：为避免同节点“无 artifact release”与
  特权 release 的 artifact/manifest 门禁互相矛盾，`two_stage_script` 的
  release 与控制面两阶段任务保持一致，总是先等待 `deployment_artifacts`
  verified，再走跨节点 artifact checkout。平台不提供无发布物的特权 release；
  同节点部署同样下发并校验包含 `deploy-go-workspace.tar.gz` 的统一发布物，
  再在目标 staging 还原 checkout 后执行固定 Make target。
- **存储层不修改 execution_mode CHECK**：对外 `two_stage_script` 在存储层
  使用 `execution_mode='two_stage' + workspace_script=1`，API/快照/调度再
  展开为 `two_stage_script` 语义，避免重建表或改写旧 CHECK。

## Implementation Units

### U1. Migration 与工作区来源 API

- **Files**
  - 新增 `api/migrations/0035_two_stage_script_workspace.sql`；
  - 新增 `api/src/application_workspace_sources/mod.rs`；
  - `api/src/lib.rs`；
  - `api/src/deployment_targets/mod.rs`。
- **Approach**
  - `deployment_targets` 增加 `workspace_script` 布尔列；对外执行模式仍为
    `two_stage_script`，存储层使用 `two_stage` + `workspace_script=1`；
  - 新建 `application_workspace_sources` 表；
  - 新增 `GET/PUT /applications/{id}/workspace-source` 管理接口；
  - `two_stage_script` 目标创建/更新校验工作区来源已保存、构建 Agent 在线、
    目标 Agent 具备特权 release 能力；
  - 保存/修改工作区来源时递增 `workspace_version` 并使 active preview 失效。
- **Verification**
  - `cargo test -p deploy-go-api --test migrations --test database_constraints
    --test deployment_targets_api`
  - `make api-openapi-check`

### U2. 部署预览、快照与调度

- **Files**
  - `api/src/deployments/mod.rs`；
  - `api/src/agents/dispatcher.rs`；
  - `api/tests/deployments_api.rs`、`api/tests/agent_dispatcher.rs`。
- **Approach**
  - `build_target_preview` 增加 `two_stage_script` 分支；
  - 快照写入 `source_policy=workspace` 与 workspace 字段；
  - dispatcher 候选查询、`ensure_deployment_task`、`create_stage_task` 支持
    `two_stage_script`；
  - prepare 任务使用 `SourcePolicy::Workspace`，release 任务使用 artifact
    模式，不注入 Git 凭证；
  - 多目标复用同一发布物。
- **Verification**
  - `cargo test -p deploy-go-api --test two_stage_deployment --test
    agent_dispatcher`

### U3. Agent 协议与执行器

- **Files**
  - `agent-protocol/src/lib.rs`；
  - `agent-protocol/schema/agent-control.schema.json`；
  - `agent-protocol/tests/schema_compatibility.rs`；
  - `agent/src/executor.rs`、`agent/src/runner.rs`；
  - `agent/tests/`。
- **Approach**
  - 控制协议版本升级到 v14；
  - `SourcePolicy` 增加 `Workspace`，`DeploymentPrepareTask` 增加可选
    `workspace_path`；
  - `execute_prepare` 按来源策略分支：workspace 时校验并快照工作区到
    `checkout_dir`，不执行 Git；
  - workspace 快照拒绝符号链接、路径逃逸，并受 staging 大小/文件数限制；
  - release 对 workspace 任务统一消费已验证 artifact（包含 workspace 快照），
    再按 `ReleaseCheckoutMode::WorkspaceArtifact` 还原 checkout 后执行。
- **Verification**
  - `cargo test -p deploy-go-agent-protocol -p deploy-go-agent`
  - `make agent-check`

### U4. 管理端

- **Files**
  - `admin/src/features/applications/ApplicationSourceSection.tsx` 或新增
    `WorkspaceSourceSection.tsx`；
  - `admin/src/features/targets/TargetEditor.tsx`、`labels.ts`；
  - `admin/src/features/deployments/`；
  - `admin/src/api/generated/` 相关 client。
- **Approach**
  - 应用详情可配置“脚本两阶段工作区来源”；
  - 目标编辑器支持 `two_stage_script` 模式，不要求 Git 来源；
  - 部署预览展示工作区路径/版本，不展示 Git 分支；
  - 部署详情展示 prepare/release 与 workspace 摘要。
- **Verification**
  - `make api-client-check`
  - `make admin-check`

### U5. 文档、示例与回归

- **Files**
  - `docs/runbooks/application-onboarding.md`；
  - `docs/runbooks/application-templates.md`；
  - `docs/standards/application-deployment-contract.md`；
  - `docs/standards/agent-control-protocol.md`；
  - 新增 `examples/workspace-two-stage/` 最小示例。
- **Approach**
  - 记录 `two_stage_script` 的接入、脚本接口、工作区要求和安全边界；
  - 示例提供 prepare/release、manifest 与契约测试；
  - 全量检查确保旧模式无回归。
- **Verification**
  - `make check`

## Verification Contract

| Gate | Command | Passing signal |
|---|---|---|
| Rust | `make api-check`、`make agent-check` | 格式、clippy、workspace 测试通过 |
| OpenAPI/client | `make api-openapi-check`、`make api-client-check` | 生成物一致 |
| Admin | `make admin-check` | typecheck、lint、unit 通过 |
| 全量 | `make check` | 全部门禁通过 |
| Diff | `git diff --check` | 无空白/冲突 |

## Definition of Done

- ClickHouse 类应用可无 Git 来源完成：创建应用 -> 保存工作区来源 -> 创建
  `two_stage_script` 目标 -> 生成部署预览 -> prepare 打包 -> 目标机器
  release。
- `two_stage_script` 不使用 Git 凭证、分支发现或 commit 解析。
- 现有 `script`、`two_stage`、`image` 模式测试全量通过。
- 未获得用户明确授权前，不连接或修改任何真实节点。

## 范围边界

- 本期先支持固定本地工作区来源；外部 CI 直接上传发布物（`artifact_upload`
  来源）作为后续扩展，不在本期实现。
- 本期不放开任意 root 命令、任意 Make target 或任意 script path。
- 本期不修改历史 migration；Agent 需要升级到支持 v14 的版本后使用新模式，
  旧 Agent 不会收到新模式任务。
