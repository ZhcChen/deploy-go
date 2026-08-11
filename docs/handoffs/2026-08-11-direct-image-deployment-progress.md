---
artifact_contract: "ce-handoff/v1"
created_at: "2026-08-11T00:00:00+08:00"
title: "镜像直连部署 U3 中途交接：当前进度与未提交改动"
summary: "U1/U2 已完成并推送；U3 主体 API 状态机与 dispatcher 已改完且 cargo check 通过，测试与提交尚未完成"
keywords: ["image-deployment", "direct-image", "U3", "dispatcher", "handoff"]
cwd: "/Users/chen/code/deploy-go"
resume_focus: "先完成 U3 聚焦测试并提交 dispatcher/API 改动，再继续 U4 Agent 镜像桥接；当前工作区有未提交改动"
repository: "ZhcChen/deploy-go"
branch: "main"
head: "566a72f"
worktree_path: "/Users/chen/code/deploy-go"
---

# 交接：镜像直连部署 U3 中途进度

## 0. 进度更新（2026-08-11 19:40）

恢复执行在验证阶段被用户暂停，转处理 `qfy-voucher-hub` 正式环境核销问题。

- U1/U2 已完成并推送：`b5eaa72`（协议 v8 与共享模板契约）、`566a72f`（目标 API
  image_spec）。
- 恢复前又新增 3 个已推送提交：
  - `ff4c851 fix: 外部 API 两阶段部署接入跨节点制品流程`
  - `8c5c31b fix: runner 降权后可读取 git 私钥`
  - `be37f37 fix: prepare 制品上传在断线重连后恢复而非误报成功`
- U3 未提交改动仍在工作区（`api/src/deployments/mod.rs`、
  `api/src/agents/dispatcher.rs`），内容与上一版交接一致。
- 已完成的验证：
  - `cargo fmt --all --check`：通过。
  - `git diff --check`：通过。
  - `cargo clippy -p deploy-go-api --all-targets -- -D warnings`：启动后被用户
    暂停，未取得结果；残留 clippy 进程已清理。
- 尚未执行：聚焦测试、image preview/confirm/dispatcher 补测、OpenAPI/双端
  client 校验、提交推送。
- 恢复后下一步：先跑 clippy/聚焦测试并补 U3 测试，提交 U3；再继续 U4 Agent
  镜像桥接。

## 1. 当前状态

- 仓库分支：`main`，最近已推送 HEAD：`566a72f`。
- 工作区有 **2 个未提交文件**，属于 U3：
  - `api/src/deployments/mod.rs`
  - `api/src/agents/dispatcher.rs`
- 未提交、未推送，当前改动不可视为完成。
- 用户要求暂停并处理紧急事项；本文件用于恢复执行进度。

## 2. 已完成并已提交（U1 + U2）

- `b5eaa72 feat: 协议 v8 与镜像直连部署共享模板契约`
  - Agent 控制协议 v8、`ImageDeploySpec`、`DeploymentReleaseTask.image_spec`。
  - `container-template` 共享模板与校验。
- `566a72f feat: 部署目标支持镜像直连执行模式与 image_spec`
  - migration `0020_image_deployment.sql`。
  - 目标 API、`execution_mode=image`、`image_spec_json`、约束与快照 hash。
  - OpenAPI/双端 client 契约已同步。

## 3. 当前 U3 未提交改动

### 已实现

`api/src/deployments/mod.rs`：
- Preview/Confirm/Response 增加 `image_spec`。
- image preview 自动生成 `release_version`、40 位固定 commit sha、模板 module、checkout digest。
- image 部署创建时主控生成平台制品并写入 artifact store，TTL 24h，幂等。
- 部署 snapshot 持久化 `_artifact_id` / `_artifact_manifest_digest` / `_artifact_archive_digest`。
- retry 复用原 verified artifact。
- Env 门禁按 `image_spec.env_files` 过滤。

`api/src/agents/dispatcher.rs`：
- image 模式跳过 prepare，直接创建 release 任务。
- release payload 携带 `image_spec`，不携带 Git 仓库/凭据。
- image 任务要求协议 v8 + privileged capability。
- artifact download lease 对单目标 image 也生效。
- Env gate 仅要求 image_spec 选择的 Env 文件。
- release authorization 核对 snapshot 中预期 checkout_tree_digest。
- 应用多目标 image 复用 `schedule_application_releases`，不创建 prepare。

### 验证状态

- `cargo check -p deploy-go-api`：**通过**。
- `cargo test`：**尚未完成**；已启动聚焦测试，但用户中止，未取得结果。
- `cargo fmt --all --check`、`cargo clippy -p deploy-go-api --all-targets -- -D warnings`：尚未执行。
- 尚未提交、未推送。

## 4. 恢复后的下一步

1. 先执行：
   - `cargo fmt --all`
   - `cargo clippy -p deploy-go-api --all-targets -- -D warnings`
   - `git diff --check`
2. 跑聚焦测试：
   - `cargo test -p deploy-go-api --test agent_dispatcher --test two_stage_deployment --test deployment_targets_api --test migrations --test database_constraints`
   - 按需补 image preview/confirm/dispatcher 测试。
3. 校验 OpenAPI/双端 client：
   - `make api-openapi && make api-client-generate`
   - `make api-openapi-check && make api-client-check`
4. 小闭环提交并推送（可拆两个提交）：
   - API 镜像 preview/confirm/平台制品；
   - dispatcher/agent task image release + 测试。
5. 继续 U4：Agent 镜像 release 桥接（`agent/src/task_handler.rs`、executor、测试 fixture）。

## 5. 注意事项

- 不得修改已提交 migration 或原计划正文。
- 未经新授权不连接 WSL/正式节点。
- 当前改动未提交，恢复后先不要丢弃工作区内容。
- 测试命令较慢，建议只跑聚焦文件；最终门禁再跑完整 `make api-check` 或等价命令。
