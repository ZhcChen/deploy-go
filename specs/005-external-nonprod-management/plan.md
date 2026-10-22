# 对外 API 与 Skill 最大化管理非生产应用 实施方案

规格：spec.md
状态：就绪

## Technical Context

已核对的现状（均为当前代码，不套用示例架构）：

- 对外 API 全在 `api/src/external/mod.rs`（约 2350 行）。鉴权是 `ExternalApiKey`（只含 `id`）+ `require_key_application_access`（要求 Key 绑定且应用 `status='active'`）+ 非生产守卫（`require_external_non_production_environment` / `require_external_configurable_application` / `reject_production_environment`），内部以 `service_actor()` 调用业务核心。
- 先例：`application_workspace_sources::save_workspace_source_core(state, app_id, payload, actor_id, external_api_key_id, request_id)` 已把逻辑和鉴权拆开；`deployments::create_application_deployment` / `create_target_deployment` 已带 `external_api_key_id`，且对外部署请求体已支持 `snapshot_hash`。
- 内部 Git 来源 `api/src/application_sources/mod.rs`：`save`、`refresh`、`set_branch` 都把 `require_administrator` + `verify_csrf` 和业务逻辑写在一起，没有 `*_core`；`enqueue_refs_discovery(state, actor_id, app_id, request_id)` 和 `resolve_latest_refs` 已不依赖 `AuthUser`。
- Key 管理 `api/src/external_keys/mod.rs`：`update_applications` 是“整体替换绑定集合”模式，管理面页面是 `admin/src/features/external-keys/ExternalApiKeysPage.tsx`。
- 目标节点与构建 Agent 校验：`build_agent_policy`（Git）、`ensure_build_agent_ready`（工作区）、`deployment_targets` 的节点可部署校验，三处各有一套，没有统一的“Key 可用节点”概念，也未发现节点范围限制。
- 配置文件列表 `application_configs::list` 返回的 `ApplicationConfigFileResponse` 含 `content`、`current_digest` 等字段，只靠 `skip_serializing_if` 控制，不适合直接对外复用。
- 运行探测 `runtime_probe::probe_one` 已不依赖管理员鉴权的 `AuthUser`（只传 `actor`）；批量上限 20。
- migration 当前最大 `0042`，本需求新增 `0043`。

两个会影响方案的发现：

1. `require_key_application_access` 要求应用 `active`。应用启停外部化后，归档的应用会从该 Key 视角消失，没法再启用。方案：状态接口使用单独的访问检查（只看 Key 绑定，不要求 `active`），并仍受非生产守卫约束；其他接口行为不变。
2. 预览记录按 `actor.id` 隔离，对外统一用 `service_actor()`，所以不同 Key 的预览共用同一个 actor。预览靠 `snapshot_hash` 命中，没有 hash 取不到；本期不改隔离粒度，在 runbook 和审计里记录 Key，并让确认部署时校验预览所属应用与目标，与内部一致。

## Constitution Check

- 不修改业务应用；真实环境部署、节点操作不在实施授权内。
- 修改范围仅限本项目：`api/`、`admin/`、`admin-app/`（仅生成 client）、`deployer` CLI、`skills/`、`docs/`、`specs/`。
- migration 新增 `0043`，只追加，无 `DROP TABLE/COLUMN`；先 `make setup-git-hooks` / `make verify-git-hooks`，不使用 `--no-verify`。
- 对外 OpenAPI 变更必须同步 `ApiDoc`（`api/src/external/mod.rs` 自己的 `OpenApi`）、`enrich_openapi_security_contract`（新增 `/external/v1/...` 不需要 Agent bearer 前缀，但要确认外部 Key 声明按前缀注入）、`make api-openapi`、`make api-client-generate`。
- 版本号不顺手升；CLI/Skill 要到正式环境部署后才对用户生效，由用户另行说“部署正式环境”。
- 安全不变量：响应、日志、tracing、审计、OpenAPI 示例不含凭证、Env、配置内容、lease；新写入审计带 `external_api_key_id`；用响应 DTO 白名单而不是复用内部带可选字段的结构。

## 实现方案与文件范围

### 数据模型（migration `0043_external_api_key_nodes.sql`）

```text
external_api_key_nodes
  api_key_id  TEXT NOT NULL REFERENCES external_api_keys(id) ON DELETE CASCADE
  node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT
  created_by  TEXT NOT NULL
  created_at  TEXT NOT NULL
  PRIMARY KEY (api_key_id, node_id)
INDEX (node_id)
```

- 不迁移任何数据：未配置即空集合，默认拒绝（规格 FR-006，已确认）。
- 白名单只约束外部 API 的“新增/修改”写入：Git 来源保存、工作区来源保存、部署目标创建与编辑（含更换节点）。读、已有配置、部署不受影响。
- 校验函数 `external_keys::require_key_node(pool, key_id, node_id, request_id)`：不在集合返回 403 `external_node_not_allowed`；节点已归档按既有错误处理。

### 核心函数拆分（内部行为不变）

| 模块 | 新增核心函数 | 内部 handler 保留 |
| --- | --- | --- |
| `application_sources` | `save_source_core`、`set_branch_core`、`refresh_core`（返回 discovery id） | 管理员 + CSRF 后调用 |
| `applications` | `update_status_core` | 管理员 + CSRF 后调用 |
| `application_envs` | `retry_sync_core` | 同上 |
| `application_configs` | `list_config_meta`（只取元数据列） | 内部 `list` 不动 |
| `deployments` | 复用现有 `application_preview` 内部构建函数，抽 `application_preview_core` / `target_preview_core` | 内部 handler 不动 |
| `runtime_probe` | `probe_one` 已可用，补公开入口 | 内部批量接口不动 |

### 对外接口（`/external/v1`）

| 档 | 方法 | 路径 | operation_id | 说明 |
| --- | --- | --- | --- | --- |
| P1 | GET | `/build-agents` | `external_build_agents_list` | Key 可用节点内的构建 Agent：id、name、节点状态、是否满足构建要求；无连接信息 |
| P1 | GET | `/git-credentials` | `external_git_credentials_list` | id、name、fingerprint、status |
| P1 | GET | `/applications/{id}/git-source` | `external_application_git_source_show` | 来源详情（与 `show-app.sources.git` 同源，含 `version`），不含凭证内容 |
| P1 | PUT | `/applications/{id}/git-source` | `external_application_git_source_save` | 非生产；`version` 乐观锁；校验节点白名单与凭证状态 |
| P1 | POST | `/applications/{id}/git-source/refreshes` | `external_application_git_source_refresh` | 202；同来源已有进行中发现则返回它，不重复下发 |
| P1 | GET | `/applications/{id}/git-source/refreshes/{refs_query_id}` | `external_application_git_source_refresh_show` | 轮询 |
| P1 | PUT | `/applications/{id}/git-source/branch` | `external_application_git_source_branch_set` | 非生产；`version` |
| P2 | PUT | `/applications/{id}/status` | `external_application_status_update` | `active` / `archived`；使用不要求 `active` 的访问检查 |
| P2 | POST | `/applications/{id}/env-files/{env_file_id}/sync-retry` | `external_env_file_sync_retry` | 校验 env 文件属于该应用；可选 `target_id` |
| P2 | GET | `/applications/{id}/config-files` | `external_application_config_files_list` | 专用 DTO：id、path、label、format、role、delivery、sensitive、editable、current_version、status、updated_at、version；敏感文件不返回 digest |
| P3 | POST | `/applications/{id}/deployment-preview` | `external_application_deployment_preview` | 非生产 |
| P3 | POST | `/deployment-targets/{target_id}/deployment-preview` | `external_target_deployment_preview` | 非生产 |
| P3 | POST | `/applications/{id}/runtime-probe` | `external_application_runtime_probe` | 单应用，复用批量逻辑 |
| P3 | GET | `/applications/{id}/runtime-status` | `external_application_runtime_status` | 最新探测结果的脱敏摘要；实现前先核对内部读取来源，若没有现成读取点则只暴露探测发起响应中的 `runtime_status_id` 与状态 |

生产规则：以上所有 PUT/POST（含预览、探测）对 `prod` 应用或目标返回既有 `external_production_application_forbidden`（预览走 `external_production_deployment_forbidden`，与部署一致）；GET 沿用“Key 绑定即可读”的现状，不为生产新增敏感字段。

### 管理面（节点白名单配置）

- 内部 `PUT /api/v1/external-api-keys/{id}/nodes`（整体替换，管理员 + CSRF，审计），Key 摘要 `ExternalApiKeySummary` 增加 `node_ids`。
- `ExternalApiKeysPage.tsx` 增加“可用节点”多选，保存提示“未配置的 Key 无法通过外部 API 修改 Git 来源、工作区来源和部署目标”。
- 重新生成 `admin/src/api/generated` 与 `admin-app/lib/api/generated`。

### CLI 与 Skill

`deployer`（crate `deploy-go-deployer`）新增子命令：`list-build-agents`、`list-git-credentials`、`show-git-source`、`set-git-source`、`refresh-git-branches`（含 `--wait`）、`set-git-branch`、`set-app-status`、`retry-env-sync`、`list-config-files`、`preview-deploy`、`probe-runtime`、`show-runtime-status`；`deploy` 增加 `--snapshot-hash`（若尚未暴露）。同步 `skills/deploy-go-deployer/SKILL.md`、`references/commands.md`、`workflows.md`、`errors.md`，更新 description 与“写入边界”。

### 文档

- `docs/runbooks/external-deploy-api.md`：新接口、节点白名单上线步骤（先给现有 Key 配节点）、错误码。
- `docs/standards/access-control.md`、`api-contract.md`：把“不暴露 Git 凭证”改为“仅暴露凭证元数据”，写明白名单与生产不变承诺。
- `docs/runbooks/api-migrations.md`：补 `0043`。

## 阶段与依赖

每阶段可独立验证、提交、回滚；阶段间顺序固定（后一阶段依赖前一阶段的核心函数和回归矩阵）。

| 阶段 | 内容 | 依赖 |
| --- | --- | --- |
| A | 生产不变基线：先写覆盖现有 20 个接口对 `prod` 预期的回归测试（当前就应通过），作为后续所有阶段的护栏 | 无 |
| B | migration `0043` + 白名单管理接口 + 管理面页面 + `require_key_node`；先接入工作区来源与部署目标两处（已有接口），补测试 | A |
| C | `application_sources` 抽 `*_core`，内部回归；新增 P1 外部接口与目录接口；节点白名单接入 Git 来源 | B |
| D | P2：应用状态、Env 同步重试、配置文件元数据 | C |
| E | P3：预览、探测与运行状态 | D |
| F | OpenAPI 与双端 client 重新生成；CLI 子命令；Skill 与文档 | C/D/E 各自完成后增量做，最后统一核对 |
| G | 独立复核（权限、凭证、migration）与整体回归 | 全部 |

不并行写入：各阶段都动 `api/src/external/mod.rs` 与 `lib.rs`，文件有交集。

## 验证与验收

- 阶段 A：`cargo nextest run -p deploy-go-api --test external_production_guard`（新文件，矩阵覆盖现有 20 个接口对 `prod` 的预期）。
- 每阶段：对应 `cargo test -p deploy-go-api --test <名称>`；内部回归沿用 `application_sources`、`application_workspace_sources`、`deployment_targets`、`two_stage_deployment` 等现有测试。
- 阶段 B 额外：`make setup-git-hooks`、`make verify-git-hooks`、`cargo test -p deploy-go-api --test migrations --test database_constraints`；`admin` 组件测试 `npm --prefix admin test -- --run src/test/ExternalApiKeys.test.tsx`。
- 成功/失败用例（每个新接口）：成功路径；`prod` 403；非绑定应用 404；`version` 冲突 409；节点不在白名单 403 `external_node_not_allowed`；Agent 离线或能力不足；凭证停用；响应序列化后断言不含凭证、Env、配置内容、lease 关键字。
- 阶段 C 额外：用模拟 Agent 消息走完“保存来源 → 发起发现 → 模拟 Agent 返回分支 → 设置分支 → 预览 → 部署”，验证发现重复发起合并、过期与来源版本变更后旧结果失效。
- 阶段 F：`make api-openapi-check api-client-check`、CLI 契约测试（沿用 `deployer` 现有测试风格）、Skill 文档与 `--help` 一致性检查。
- 收尾：`make api-check`、`make admin-check`、`make deploy-production-check`、`git diff --check`；Linux 专属代码未改动，不需要 Docker 目标。
- 验证边界：不连接真实节点，不执行真实部署，不运行 `make deployer-skill-install`；这些属于上线后运维验收（SC-006）。
- 独立复核：阶段 G 由独立 agent 只读复核权限边界（生产守卫、白名单、DTO 白名单）、凭证泄露面和 migration，结论写 `docs/reviews/`。

## 运行与恢复

- 上线顺序（写入 runbook）：部署包含 `0043` 的新版本 → 管理员在管理面为现有 Key 配置可用节点 → 之后 Key 才能经外部 API 修改 Git 来源、工作区来源和部署目标；未配置前这些写操作返回 403，不影响查询、部署、取消。
- 回滚：`0043` 只新增表，回滚代码不需要回滚 schema；白名单表保留不影响旧版本。
- 无需额外后台任务；分支发现复用现有 Agent 任务和过期机制。
- 真实部署、Skill 重新安装需要用户另行明确授权（“部署正式环境”）。
