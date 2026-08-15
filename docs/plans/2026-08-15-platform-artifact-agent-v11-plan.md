---
title: 平台发布物下发与 Agent v11 通用镜像发布计划
date: 2026-08-15
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: 会话确认（后续模板不应要求重复升级 Agent）
execution: code
---

# 平台发布物下发与 Agent v11 通用镜像发布计划

## 目标

将镜像直连的模板渲染边界从 Agent 二进制移动到 Deploy Go 控制面生成的受控
artifact。Agent v11 只验证并从 artifact 中提取固定 checkout 文件，然后复用既有
artifact、Env 门禁、签名授权和 root executor 发布链路。新增模板只改控制面、模板
目录与管理端，不再增加 Agent 协议枚举或发布逻辑。

## 范围与边界

- 本期将 Redis、PostgreSQL 与 etcd 统一到 `artifact` checkout 模式；不接受任意
  Compose、命令、Make target 或在线脚本 URL。
- `image_spec` 保留在 API 的目标配置、快照和控制面模板构建层，不再随
  `DeploymentReleaseTask` 下发给 Agent。
- Agent 只允许从已下载、摘要和 manifest 均已校验的单模块 platform artifact 中
  提取 `Makefile`、`scripts/release.sh` 与 `deploy-go.yaml`，并使用固定权限写入
  task checkout。
- 新增通用 `checkout_mode=artifact` 协议字段；缺省仍是 Git checkout，以保留旧的
  Git 两阶段发布语义。平台 artifact 任务要求协议 v11；旧 Agent 不接收该任务。
- 不执行任何真实节点部署、Agent 升级、控制面重启、数据迁移或业务切流。

## 实施单元

### U1. v11 协议与控制面调度

- `agent-protocol/src/lib.rs`、`agent-protocol/schema/agent-control.schema.json`、
  `agent-protocol/tests/schema_compatibility.rs`
- `api/src/agents/dispatcher.rs`、`api/src/deployments/mod.rs`、
  `api/src/deployment_targets/mod.rs`
- 将协议版本提升为 v11，并为 release task 增加不含模板知识的 checkout 模式。
- 移除 `ImageTemplate` / `ImageDeploySpec` 对 Agent 协议 payload 的占用；控制面
  继续使用容器模板 crate 构建和校验 image spec。
- dispatcher 在 image 模式中要求 v11 与 `privileged_release` 能力；旧版本明确失败，
  Redis/PostgreSQL 不再维持 v8 例外。
- 验证 image target 创建、预览、release 调度和授权均不再依赖 Agent 模板枚举。

### U2. 平台 artifact 固定 checkout 布局

- `container-template/src/lib.rs`、`examples/templates/*/scripts/release.sh`
- 平台生成的 `template.tar.gz` 除现有 Compose 和 manifest 外，包含固定 checkout
  文件；Git 两阶段产物保留原有布局，release 脚本以受限的两种固定文件清单兼容。
- 控制面以平台 artifact 中 checkout 文件计算预期目录摘要；授权时继续绑定
  artifact manifest、Env 摘要、部署 snapshot 和 checkout 摘要。
- 验证 archive 的确定性、布局、摘要与非默认 etcd 客户端端口。

### U3. Agent 通用 artifact checkout

- `agent/src/task_handler.rs`、`agent/src/executor.rs`、`agent/Cargo.toml`
- 下载并校验 artifact 后，仅在 `checkout_mode=artifact` 下执行白名单 tar 解包；拒绝
  缺失、重复、符号链接、路径逃逸或额外 checkout 项，且不读取模板名和 Compose 内容。
- 以原子目录替换生成 checkout，固定文件权限，之后复用现有 privileged release
  admission、恢复与取消链路。
- 验证通用 artifact 成功发布、篡改/布局异常失败且 executor 零调用、Git release
  仍走原检出路径。

### U4. 文档、生成物与复核

- `docs/standards/agent-control-protocol.md`
- `docs/standards/agent-installation-contract.md`
- `docs/runbooks/application-templates.md`
- `docs/runbooks/privileged-agent-release.md`
- 清除 v10 的 etcd 专用表述，说明 v11 是一次性的通用 artifact checkout 能力升级。
- 更新 OpenAPI/client（仅 API image template 枚举变动时）并执行模板、Rust、前端
  静态检查和 diff 检查。

## 验收场景

1. etcd image target 仅由 v11、具备特权 release capability 的在线 Agent 创建和调度。
2. release task 不包含 `image_spec` 或模板名称，携带 `checkout_mode=artifact` 与现有
   artifact 下载引用。
3. Agent 从控制面 artifact 生成 checkout，目录摘要与控制面预期一致，executor 只
   收到固定 `make --no-print-directory -C <checkout> deploy-go-release`。
4. archive 包含路径逃逸、重复 checkout 文件、符号链接、非普通文件或缺失固定文件时，
   任务失败且不请求 release authorization、不调用 executor；非 checkout 的模板内容
   不会被复制到 checkout。
5. 既有 Git 两阶段 release 未携带 `checkout_mode` 时保持 Git 检出与校验行为。
6. 新模板仅需在 `container-template` 注册，不改变 Agent 的模板识别逻辑。

## 验证命令

- `cargo test -p deploy-go-agent-protocol`
- `cargo test -p deploy-go-container-template`
- `cargo test -p deploy-go-agent --test image_release --test executor`
- `cargo test -p deploy-go-api --lib image_release_requires_artifact_checkout_v11`
- `cargo test -p deploy-go-api --test deployment_targets_api --test agent_dispatcher`
- `make app-template-check`
- `cargo fmt --all --check`
- `cargo clippy -p deploy-go-api -p deploy-go-agent -p deploy-go-container-template -p deploy-go-agent-protocol --all-targets -- -D warnings`
- `make api-openapi-check && npm run api:client:check && npm run check --workspace deploy-go-admin`
- `git diff --check`
