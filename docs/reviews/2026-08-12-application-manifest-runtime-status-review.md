---
date: 2026-08-12
topic: application-manifest-runtime-status
status: reviewed
---

# 应用类型清单与 Redis 运行时状态复核记录

## 结论

本轮实现已通过聚焦门禁与全量 Agent/API 关键测试，可提交 main。本轮只完成
Deploy Go 平台侧的绑定能力、`deploy-go.yaml` 应用清单与只读运行时状态读取，
不修改线上 Redis 容器、数据卷、Compose 或 Env，也未连接真实节点、未创建可
执行部署目标。

## 实现内容

- migration `0022_application_manifest_and_runtime_status.sql`：
  - `applications` 增加 `app_type` / `type_version`，历史回填 `binary` / `1`；
  - `deployment_targets` 增加 `target_code`，历史回填为环境值，并新增
    `(application_id, node_id, target_code)` 唯一索引；
  - 新增 `application_runtime_statuses`，`agent_tasks` 增加唯一
    `runtime_status_id` 关联（沿用 `system_inspect` 存储类目，payload 协议类型
    为 `runtime_status_probe`）。
- API：应用与目标读写支持类型/版本/target_code；snapshot 与 dispatcher 注入
  `target_code`；新增应用运行时状态读取与管理接口；同目标 pending/running
  时拒绝重复读取；Agent/executor 能力不足时收敛为 failed。
- Agent/executor：executor 本机协议 v3 新增只读 `RuntimeStatus`，固定执行
  `docker compose --project-name <target_code> ps --format json`，Compose
  不可用时按固定 label 过滤 `docker ps`；Agent 控制协议 v9 新增
  `runtime_status_probe` 任务与 capability，经持久化 journal 调用 executor。
- 管理端：应用详情展示应用类型/版本、目标 target_code、运行时状态与失败
  原因，支持管理员重新读取；目标编辑器可配置 target_code。
- 文档：应用清单规范、Agent 控制协议 v9、executor v3 只读契约与接入 runbook
  已同步。

## 复核中发现并修复的问题

1. **Docker JSON Lines 解析**：`docker compose ps --format json` 与
   `docker ps --format json` 实际输出为逐行 JSON object，按单个 JSON value
   解析会在多容器时失败。executor 现统一归一化为 JSON array。
2. **输出白名单**：原始 Compose/ps 输出包含 `Command` 等可能携带敏感参数的
   字段。executor 现在只保留 `id`、`project`、`name`、`service`、`state`、
   `health`、`exit_code`、`publishers`，并保持 32 KiB 有界截断。
3. **失败原因透出**：API 保存运行时状态失败时原为固定文案，现透出 Agent
   `TaskResult.summary`（仍限长 1000 字符），便于管理端直接定位。
4. **旧 snapshot 兼容**：镜像部署从旧 snapshot 恢复预览时 target_code 回退
   到 environment，避免空值。
5. **静态门禁**：`cargo clippy -D warnings` 修复三处 needless_borrow /
   question_mark。

## 测试矩阵

以下门禁均通过：

```text
cargo test -p deploy-go-api --test migrations --test database_constraints \
  --test applications_api --test deployment_targets_api \
  --test agent_dispatcher --test runtime_status_api --test openapi_contract
cargo test -p deploy-go-agent-protocol -p deploy-go-agent
cargo test -p deploy-go-agent-executor
cargo test -p deploy-go-container-template
make api-openapi-check
make api-client-check
make app-template-check
make migration-git-guard
make migration-git-guard-self-test
cargo clippy --workspace --all-targets -- -D warnings
npm run check --workspace deploy-go-admin
git diff --check
```

管理端测试 125 项通过；Agent API 与协议关键测试通过；迁移与数据库约束
测试覆盖 0022 非破坏性回填与 target_code 唯一性。

## 上线与绑定步骤（待用户授权后执行）

1. 部署当前 main 到正式控制面，并确认 0022 migration 成功。
2. 应用详情将 Redis 应用编辑为 `redis` / `7`（业务仓库根目录提供
   `deploy-go.yaml`，见 `docs/standards/application-manifest.md`）。
3. 目标编辑将 `target_code` 填为 `shared-prod-redis`，对应已有 Compose 项目
   `deploy-go-shared-prod-redis`；只写控制面记录，不触发部署、不重建容器。
4. 确认节点 Agent v0.2.0 / 控制协议 v9 / executor v3，doctor 显示
   `RUNTIME_STATUS` capability 可用。
5. 在应用详情「运行时状态」选择目标并「重新读取」，验证展示 Redis 容器
   `running` / `healthy` 摘要。

## 未覆盖/后续事项

- 真实节点读取、WSL 测试节点升级与正式 Redis 绑定未执行，等待用户明确授权。
- 运行时状态任务是管理面只读能力；若 API 在写入 pending 后、入队前崩溃，
  会留下 pending 状态。后续可增加 stale pending 超时收敛，不影响本轮绑定
  与读取闭环。
