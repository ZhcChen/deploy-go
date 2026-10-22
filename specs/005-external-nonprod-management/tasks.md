# 对外 API 与 Skill 最大化管理非生产应用 任务

输入：spec.md、plan.md
状态：待执行

## 维护与执行约束

- 已有任务必须先读取后增量维护，保留稳定 ID、已完成状态和验证证据；不复制模板覆盖旧任务。不再适用的任务保留并说明原因。
- 每个任务列出需求编号、文件范围、依赖及可验证结果；阶段按用户场景或实际依赖拆分。
- 必须包含匹配风险的验证任务；可以复用已有测试或构建，不要求每个修改都新增测试，也不默认要求 TDD。
- `[P]` 仅表示无未完成依赖且文件不交叉；并行写入还须隔离 worktree，各 agent 显式指定需求目录并禁止持久化指针。
- 独立审查和真实验证不能由 analyze/converge 或主 agent 自评替代。部署需单独授权，不从开发指令推导远程运行权限。
- 本需求各阶段都会改 `api/src/external/mod.rs` 与 `api/src/lib.rs`，文件有交集，因此不使用 `[P]`，按阶段顺序串行，每阶段验证通过后单独提交推送。
- 触碰 `api/migrations/` 前先执行 `make setup-git-hooks` 与 `make verify-git-hooks`；不使用 `--no-verify`。

## 阶段 A：生产不变基线（US6，护栏）

- [x] T001 [US6] FR-018 新增 `api/tests/external_production_guard.rs`：用共享夹具 `api/tests/common/mod.rs` 创建 `prod` 应用、目标、Env 文件、部署，逐个调用现有 20 个对外接口，断言各自对 `prod` 的当前行为（写操作 403 的错误码、只读与 `cancel` 的现状）。此时不改任何生产代码，测试应直接通过。依赖：无。验收：`cargo nextest run -p deploy-go-api --test external_production_guard` 通过。
- [x] T002 [US6] FR-018 在该文件中加入表驱动的“新增写接口对 prod 必须 403”骨架（接口清单用常量列出，后续阶段每加一个接口就追加一行，未实现的接口先不列入），并把“列表中的接口集合与 `api/openapi/external.json` 的路径集合一致”作为断言，防止漏登记。依赖：T001。验收：同上。
- [x] T003 [US6] FR-018 提交阶段 A：`git diff --check`、`git diff --cached --check` 后提交推送。依赖：T002。

## 阶段 B：节点白名单（US2）

- [x] T004 [US2] FR-006 FR-007 新增 `api/migrations/0043_external_api_key_nodes.sql`：表 `external_api_key_nodes(api_key_id TEXT NOT NULL REFERENCES external_api_keys(id) ON DELETE CASCADE, node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT, created_by TEXT NOT NULL, created_at TEXT NOT NULL, PRIMARY KEY (api_key_id, node_id))` 与 `node_id` 索引；不含 `DROP TABLE` / `DROP COLUMN`，不修改任何已提交 migration；`api/tests/migrations.rs` 增加 0043 用例。依赖：T003。验收：`make setup-git-hooks verify-git-hooks`、`cargo test -p deploy-go-api --test migrations --test database_constraints` 通过。
- [x] T005 [US2] FR-006 FR-007 在 `api/src/external_keys/mod.rs` 新增 `PUT /api/v1/external-api-keys/{id}/nodes`（整体替换，管理员 + CSRF，校验节点存在且未归档，写审计）与 `require_key_node(pool, key_id, node_id, request_id)`（不在集合返回 403 `external_node_not_allowed`）；`ExternalApiKeySummary` 增加 `node_ids`；登记 `api/src/lib.rs` 的 `ApiDoc`。依赖：T004。验收：`api/tests/external_api_keys.rs` 增加设置、越权拒绝、非管理员 403、审计记录用例并通过。
- [x] T006 [US2] FR-006 在 `api/src/external/mod.rs` 的 `save_workspace_source`、部署目标创建与编辑（含更换 `node_id`）三处写入前调用 `require_key_node`；读取、已有配置的部署不变。依赖：T005。验收：新增 `api/tests/external_key_nodes.rs`，覆盖集合内成功、集合外 403、未配置集合 403、已有目标仍可部署；`prod` 仍先于白名单返回既有 403；`cargo nextest run -p deploy-go-api --test external_key_nodes --test workspace_sources_api --test deployment_targets_api --test external_api` 通过。
- [x] T007 [US2] FR-006 管理面：`admin/src/features/external-keys/ExternalApiKeysPage.tsx` 增加“可用节点”多选与保存，提示“未配置的 Key 无法通过外部 API 修改 Git 来源、工作区来源和部署目标”；更新 `admin/src/test/ExternalApiKeys.test.tsx`。依赖：T005。验收：`npm --prefix admin test -- --run src/test/ExternalApiKeys.test.tsx` 通过。
- [x] T008 [US2] FR-016 重新生成 OpenAPI 与双端 client：`make api-openapi api-client-generate`，核对 `make api-openapi-check api-client-check` 通过；同时补 `docs/runbooks/api-migrations.md` 的 0043 说明。依赖：T005 T007。
- [x] T009 [US2] 提交阶段 B（migration 与白名单后端、管理面、生成产物可分两次提交：先后端与生成产物，再管理面）；提交前 `make api-check` 的聚焦子集与 `make admin-check` 通过。依赖：T006 T008。

## 阶段 C：Git 来源、分支发现、目录（US1）

- [x] T010 [US1] FR-015 在 `api/src/application_sources/mod.rs` 抽出 `save_source_core`、`set_branch_core`、`refresh_core`（返回 discovery id），内部 handler 只保留 `require_administrator` + `verify_csrf` 后调用；不改变内部响应、状态码、审计内容。依赖：T009。验收：`cargo nextest run -p deploy-go-api --test application_sources_api --test two_stage_deployment` 全部通过且不改动这些测试。
- [x] T011 [US1] FR-002 在 `refresh_core` 内加入“同一来源版本已有进行中的发现则返回它，不重复下发 Agent 任务”，内部与外部共用；补内部回归用例。依赖：T010。验收：`application_sources_api` 增加重复发起合并用例并通过。
- [x] T012 [US1] FR-004 FR-005 在 `api/src/external/mod.rs` 新增 `GET /external/v1/build-agents`（仅 Key 可用节点内的 Agent，返回 id、name、节点状态、是否满足构建要求，不含 host/port/username/工作根目录）与 `GET /external/v1/git-credentials`（仅 id、name、fingerprint、status）；独立响应结构。依赖：T010。验收：新增 `api/tests/external_git_source.rs` 断言字段白名单、集合外节点不出现、响应不含公钥私钥。
- [x] T013 [US1] FR-001 FR-003 FR-006 FR-013 FR-014 新增 `GET/PUT /external/v1/applications/{id}/git-source`、`PUT .../git-source/branch`：Key 绑定 + 非生产守卫 + 节点白名单（构建 Agent 所属节点）+ 凭证启用校验，调用核心函数，审计写 `external_api_key_id`；保存与设置分支后作废活动预览。依赖：T012。验收：`external_git_source.rs` 覆盖成功、`prod` 403、未绑定 404、`version` 冲突 409、集合外节点 403、Agent 离线、凭证停用、分支不在发现结果、发现过期，并断言响应与审计 JSON 不含凭证关键字。
- [x] T014 [US1] FR-002 新增 `POST .../git-source/refreshes`（202）与 `GET .../git-source/refreshes/{refs_query_id}`（校验发现记录属于该应用）。依赖：T013。验收：同文件用模拟 Agent 消息走完“保存 → 发起发现 → 返回分支 → 设置分支 → 外部部署”，并覆盖来源版本变更后旧发现失效。
- [x] T015 [US1] FR-013 FR-018 把阶段 C 的写接口追加进 T002 的 prod 表驱动清单；把新接口登记进 `external` 的 `OpenApi`，核对 `enrich_openapi_security_contract` 对 `/external/v1/...` 的声明正确。依赖：T014。验收：`external_production_guard` 与 `external_openapi_contract` 通过。
- [x] T016 [US1] FR-016 重新生成 OpenAPI 与双端 client；`deploy-go-deployer/src/main.rs` 新增 `list-build-agents`、`list-git-credentials`、`show-git-source`、`set-git-source`、`refresh-git-branches`（`--wait`，轮询到完成或超时）、`set-git-branch`，更新 `deploy-go-deployer/tests/cli_help.rs` 与 fixtures。依赖：T015。验收：`cargo nextest run -p deploy-go-deployer`、`make api-openapi-check api-client-check` 通过。
- [x] T017 [US1] FR-016 FR-017 更新 `skills/deploy-go-deployer/SKILL.md`（description 与写入边界）、`references/commands.md`、`workflows.md`、`errors.md`（含 `external_node_not_allowed`、Git 相关错误码），`docs/runbooks/external-deploy-api.md`（新接口、节点白名单上线步骤：先为现有 Key 配节点）、`docs/standards/access-control.md` 与 `api-contract.md`（凭证“仅暴露元数据”、白名单与生产不变承诺）。依赖：T016。验收：`deploy-go-deployer/tests/skill_package.rs` 通过。
- [x] T018 [US1] 提交阶段 C，按“核心函数拆分”“外部接口与测试”“CLI/Skill/文档”分次提交；提交前 `make api-check` 通过。依赖：T017。

## 阶段 D：应用启停、Env 同步重试、配置文件元数据（US3 US4）

- [x] T019 [US3] FR-008 FR-015 在 `api/src/applications/mod.rs` 抽 `update_status_core`；`api/src/external/mod.rs` 新增 `PUT /external/v1/applications/{id}/status`（`active`/`archived`，`version` 乐观锁），使用新的 `require_key_application_binding`（只看 Key 绑定，不要求应用 `active`）并保留非生产守卫；归档后部署被拒绝，已运行部署不受影响。依赖：T018。验收：新增 `api/tests/external_app_management.rs`：归档、再启用、`prod` 403、未绑定 404、归档后外部部署被拒绝、`version` 冲突。
- [x] T020 [US3] FR-009 在 `api/src/application_envs/mod.rs` 抽 `retry_sync_core`；新增 `POST /external/v1/applications/{id}/env-files/{env_file_id}/sync-retry`（校验 env 文件属于该应用、非生产、可选 `target_id`），响应不含 Env 明文。依赖：T019。验收：`external_app_management.rs` 增加成功、无失败可重试、跨应用 env 文件 404、`prod` 403 用例。
- [x] T021 [US4] FR-010 在 `api/src/application_configs/mod.rs` 新增 `list_config_meta`（只查元数据列），`api/src/external/mod.rs` 新增 `GET /external/v1/applications/{id}/config-files`，使用专用响应结构：id、path、label、format、role、delivery、sensitive、editable、current_version、status、updated_at、version，敏感文件不返回 digest，响应带 `Cache-Control: no-store`。依赖：T020。验收：`external_app_management.rs` 断言响应 JSON 不含 `content`、`current_digest`（敏感文件）、`recommended_changes` 以外的正文字段，`prod` 403。
- [x] T022 [US3] FR-013 FR-018 把阶段 D 的写接口追加到 prod 清单与 OpenAPI 登记；重新生成 OpenAPI 与双端 client；CLI 新增 `set-app-status`、`retry-env-sync`、`list-config-files` 并更新 `cli_help.rs`；同步 Skill 与 runbook。依赖：T021。验收：`cargo nextest run -p deploy-go-deployer -p deploy-go-api --test external_production_guard --test external_openapi_contract`、`make api-openapi-check api-client-check` 通过。
- [x] T023 [US3] 提交阶段 D；`make api-check` 通过。依赖：T022。

## 阶段 E：预览、探测与运行状态（US5）

- [x] T024 [US5] FR-011 FR-015 在 `api/src/deployments/mod.rs` 抽 `application_preview_core` / `target_preview_core`（内部 handler 行为不变）；新增 `POST /external/v1/applications/{id}/deployment-preview` 与 `POST /external/v1/deployment-targets/{target_id}/deployment-preview`：非生产（`prod` 返回 `external_production_deployment_forbidden`）、目标须属于应用且启用、预览记录审计带 `external_api_key_id`。依赖：T023。验收：新增 `api/tests/external_preview_probe.rs`：预览后用 `snapshot_hash` 经现有外部部署确认；预览过期、来源修改后作废、`prod` 403、目标跨应用 404；`cargo nextest run -p deploy-go-api --test two_stage_deployment` 回归通过。
- [x] T025 [US5] FR-012 先读 `api/src/runtime_probe.rs` 与应用运行状态的内部读取点，记录结论到本任务备注；新增 `POST /external/v1/applications/{id}/runtime-probe`（单应用，复用 `probe_one`，非生产）和运行状态读取接口：有内部读取点则新增 `GET /external/v1/applications/{id}/runtime-status`（脱敏摘要），无则只暴露探测响应中的状态与 `runtime_status_id`，并在 plan.md 记录取舍。依赖：T024。验收：`external_preview_probe.rs` 覆盖 queued/skipped（无目标、多目标、已归档）与 `prod` 403，响应不含节点连接信息。
- [x] T026 [US5] FR-013 FR-016 FR-018 追加 prod 清单与 OpenAPI 登记；重新生成 OpenAPI 与双端 client；CLI 新增 `preview-deploy`、`probe-runtime`、`show-runtime-status`（如适用），`deploy` 增加 `--snapshot-hash`（若尚未暴露）；同步 Skill 与 runbook。依赖：T025。验收：同 T022 的命令集合通过。
- [x] T027 [US5] 提交阶段 E；`make api-check` 通过。依赖：T026。

## 阶段 F：整体回归、复核与交付

- [x] T028 整体回归：`make api-check`、`make admin-check`、`make deploy-production-check`、`make client-sensitive-check`、`git diff --check`；并跑 `bash deploy/production/test-install-contract.sh`（确认 CLI 打包契约未破坏）。依赖：T027。
- [x] T029 独立复核（高风险：权限边界、凭证泄露面、migration）：安排独立 agent 只读复核 `api/src/external/mod.rs`、`external_keys`、新增核心函数与测试，重点核对：每个新写接口都经过绑定检查 + 非生产守卫 + 节点白名单；DTO 白名单无敏感字段；内部 handler 行为未变；状态接口访问检查的放宽范围；结论写入 `docs/reviews/2026-10-external-nonprod-management-review.md`，有证据的发现返回实施并重新验证。依赖：T028。
- [x] T030 FR-017 核对 `docs/runbooks/external-deploy-api.md` 与实现一致（接口表、错误码、上线顺序、回滚）；在 `docs/solutions/` 沉淀一条可复用结论（白名单默认拒绝的上线顺序、归档应用的访问检查放宽）。依赖：T029。
- [x] T031 最终 diff 与交付核对：对照 spec 的 FR-001 到 FR-018 逐条标注证据，更新本文件“验收证据”；按小闭环提交推送 main，不自动部署，不运行 `make deployer-skill-install`，真实环境生效留待用户说“部署正式环境”。依赖：T030。

## 验收证据

阶段 A（T001–T003）：新增 `api/tests/external_production_guard.rs`，4 个用例（11 个写请求对 prod 均 403 且错误码稳定、被拒绝的写无副作用、7 个只读接口保持可读、cancel 保持可用、表覆盖与 `external.json` 操作集合一致）。`RUSTUP_TOOLCHAIN=stable cargo nextest run -p deploy-go-api --test external_production_guard` 4 通过；变异验证：临时删去 `save_workspace_source` 的生产守卫，写入用例失败并指出该接口，还原后通过；`cargo fmt`、`cargo clippy --test external_production_guard -D warnings` 通过。

阶段 B（T004–T009）：
- migration `0043_external_api_key_nodes.sql`（表名为 `external_api_key_nodes`，授权人与时间列沿用 `external_api_key_applications` 的 `granted_by`/`granted_at`；不回填）。`api/tests/migrations.rs` 新增 0042→0043 升级用例：旧 Key 保留且节点为空、重复授权被拒、被引用节点不可物理删除、删除 Key 级联清理。`cargo nextest run -p deploy-go-api --test migrations --test database_constraints` 36 通过，`make verify-git-hooks` 通过。
- `PUT /api/v1/external-api-keys/{id}/nodes`（管理员 + CSRF，整体替换，审计 `external_api_key.nodes.update`，校验节点存在且未归档）、`require_key_node` / `require_key_agent_node`，Key 摘要新增 `node_ids`；接入对外工作区来源保存、部署目标创建与编辑（状态切换不受限，保证随时可停用）。`api/tests/external_api_keys.rs` 新增 3 个用例，新增 `api/tests/external_key_nodes.rs` 5 个用例；`external_api.rs` 两个旧用例补授权。变异验证：删去工作区来源或目标编辑的白名单校验，对应用例失败；已还原。
- 管理面“管理节点”面板与 `ExternalApiKeys.test.tsx` 新增用例通过；`npm --prefix admin run typecheck`、`lint` 通过。`make api-openapi api-client-generate` 已重新生成，`make api-openapi-check api-client-check` 通过。
- `make api-check`：910/911 通过，1 个失败为 `deploy-go-agent::artifact_transfer upload_rejects_put_status_that_jumps_past_the_sent_chunk`，基线（stash 全部改动）同样偶发失败（3 次 1 次），与本需求无关；已在阶段 D 前单独修复（见 `docs/solutions/2026-10-22-artifact-upload-test-handler-must-drain-body.md`，测试夹具竞态，修复后 30 次 0 失败、撤掉 10 次失败 6 次）。
- T008 中“补 `docs/runbooks/api-migrations.md` 的 0043 说明”不适用：该 runbook 不逐个列出 migration；0043 的上线顺序在阶段 C 的 `external-deploy-api.md` 中说明。

阶段 C（T010–T018）：
- T010：`application_sources` 抽出 `save_source_core` / `set_branch_core` / `refresh_core`（内部 handler 仅保留管理员 + CSRF 后调用；审计摘要仅在外部 Key 发起时追加 `external_api_key_id`，管理面摘要不变），并把 Agent 可用性判断抽成共享的 `build_agent_unavailable`。整个 `deploy-go-api` crate 485 个测试不改动任何既有用例即全部通过。
- T011：不需要新代码——`enqueue_refs_discovery` 本就合并进行中的发现，且既有 `application_source_save_refresh_and_branch_lifecycle` 已断言重复发起返回同一 id；外部侧在 `external_git_source.rs` 里另行断言只下发 1 个 Agent 任务。
- T012–T014：新增子模块 `api/src/external/git_source.rs`（7 个接口）；`api/tests/external_git_source.rs` 4 个用例（目录只含授权节点且字段白名单；保存的各类拒绝与无副作用、审计带 Key、凭证停用；用模拟 Agent 走完 保存→发现→轮询→设置分支；来源配置后收回节点授权即不能再驱动发现/设置分支）。变异验证 4 处（保存时不校验节点、发现/分支不校验当前授权、发现/分支不校验非生产、目录不按授权过滤）均使对应用例失败，已还原。
- T015：生产护栏表追加 3 个写接口与 4 个只读基线（生产应用无来源时读取为 404）；`external_api.rs` 与 `external_openapi_contract.rs` 的路径断言更新。契约测试原把整个词 `credential` 列为禁词，与已确认的“凭证目录仅暴露元数据”冲突，收窄为 `private_key` / `public_key` / `lease_id`（`lease` 会误伤 `release`）。
- T016：`deploy-go-deployer` 新增 7 个子命令（含 `show-git-refresh`）；`deploy-go-deployer/tests/git_source_cli.rs` 6 个用例用脚本化本地服务验证实际请求（自动读取 version、请求体键集合、`--wait` 轮询、参数校验先于请求）。mock 带超时：变异让命令少发请求时 15 秒后失败而非挂起（首版曾因此卡死，已修）。`make api-openapi-check api-client-check` 通过（`openapi.json` 内部契约无变化，仅 `external.json` 变化）。
- T017：Skill（SKILL.md、commands/workflows/errors）与 `docs/runbooks/external-deploy-api.md`、`docs/standards/access-control.md` 已更新；标准文档里原本没有外部 API 章节，故新增“对外 API Key 边界”一节而不是改写。`api-contract.md` 无外部 API 内容，未改。runbook 写入了节点白名单上线步骤与回滚。
- 收尾：`make api-check` 922/923 通过，唯一失败仍是上述已知的偶发用例（此后已修复）；`make external-deploy-check` 通过。

阶段 D（T019–T023）：
- 前置：先单独修复了让 `make api-check` 连续变红的偶发测试（`5a52e35`，见 `docs/solutions/2026-10-22-artifact-upload-test-handler-must-drain-body.md`）。
- T019/T020：`applications::update_status_core`、`application_envs::retry_sync_core` 抽出，内部 handler 行为与审计摘要不变（外部调用才追加 `external_api_key_id`）；内部 489 个测试全部通过。新增 `api/src/external/app_management.rs`：`PUT /applications/{id}/status`（访问检查只看 Key 绑定、不要求应用启用，否则归档后无法重新启用；非生产守卫不要求启用，已归档的生产应用也不能经对外 API 重新启用）与 `POST .../env-files/{env_file_id}/sync-retry`（校验 Env 文件属于路径应用）。
- T021：`application_configs::list_file_metadata` 与独立的 `ApplicationConfigFileMetadata`（13 个字段；不含内容、模板摘要、部署路径、说明文本；敏感文件不返回摘要；`no-store`），`GET .../config-files`。计划写的是“`prod` 403”，实现与之一致；因此它是“新增只读但生产被拒”，在护栏里单独列为 `PROD_READ_FORBIDDEN_CASES`。
- 测试：`api/tests/external_app_management.rs` 5 个用例；变异 5 处（状态接口去生产守卫、同步重试去归属校验、配置列表去生产守卫、敏感文件也返回摘要、状态接口改用要求启用的访问检查）均使对应用例失败，已还原。`external_production_guard` 追加 2 个写接口、1 个只读被拒接口；路径断言同步更新。
- T022：CLI 新增 `set-app-status`（省略 `--version` 时读 `show-app`；归档应用 `show-app` 404，需显式 `--version`，已在错误提示与文档说明）、`retry-env-sync`（`target_id` 百分号编码）、`list-config-files`；`git_source_cli.rs` 新增 5 个用例，CLI 共 34 个测试通过；Skill 三个 references、SKILL.md 与 `external-deploy-api.md` 已更新。
- 收尾：`make api-check` 935/935 通过（首次完整通过）；`make external-deploy-check`、`make api-openapi-check api-client-check` 通过。

阶段 E（T024–T027）：
- T024：`deployments::application_preview_core` / `target_preview_core` 抽出，内部 handler 行为不变（内部 489 个测试全绿）。对外预览使用独立的 `ExternalDeploymentPreview`：不含 `agent_id`、Env 门禁状态、`image_spec`、工作区路径；单目标预览不检查 Agent 在线，不返回 `agent_online`，而不是编一个常量。新增 `api/src/external/preview.rs`。预览/探测守卫：Key 绑定 → 非生产（含“应用带生产目标”，与创建部署一致）。
- T025：读代码结论——运行状态的读取点已存在：`applications::find` 把最近一次探测快照合并进 `runtime_state` / `runtime_probe_status` / `runtime_checked_at`。因此新增 `GET .../runtime-status` 直接复用，不新增查询（plan 中“若无读取点则只暴露探测响应”的备选方案不需要）。`POST .../runtime-probe` 复用 `probe_one`（新增 `probe_application_core` 薄封装）。`runtime-status` 为只读，沿用“Key 绑定即可读”（含 prod），与既有只读接口一致，已在护栏里钉住。
- 测试：`api/tests/external_preview_probe.rs` 5 个用例（预览→用返回的 `snapshot_hash` 确认部署，部署固定在预览解析的提交上；单目标预览；生产/带生产目标/生产且无目标/未绑定/未知字段均被拒且预览表无新增；探测排队与审计；运行状态脱敏）。变异 5 处（应用预览去非生产守卫、去“无生产目标”检查、目标预览去生产目标检查、探测去非生产守卫、预览响应带出 `agent_id`）均使用例失败。**M1 起初存活**：因为 `app_prod` 带生产目标，被后面的“无生产目标”检查兜住；补了“生产应用且没有任何目标”的夹具后可被抓到——数据库约束要求目标环境与应用环境一致，所以这是唯一能区分两道检查的构造。
- 护栏：`external_production_guard` 追加 3 个写接口 + `runtime-status` 只读基线；路径断言同步。
- T026：CLI 新增 `preview-deploy`、`probe-runtime`、`show-runtime-status`，`deploy` 新增 `--snapshot-hash`（未提供时请求体中为 null，行为与以前一致）；`git_source_cli.rs` 新增 3 个用例，CLI 共 37 个测试通过。Skill 与 `external-deploy-api.md` 已更新；写入 `errors.md` 前核对了真实错误码——我起初写的 `deployment_preview_expired` 并不存在，实际为 `preview_expired` / `preview_already_confirmed`，已更正并加入包测试。
- 收尾：`make api-check` 943/943 通过；`make external-deploy-check`、`make api-openapi-check api-client-check` 通过。
- 已知取舍：对外调用共用同一服务账号，预览按“服务账号 + 应用/目标 + hash”匹配，因此 `snapshot_hash` 本身就是凭据；未改隔离粒度（plan 已声明），仅在 runbook 中提示不要泄露。

阶段 F（T028–T030）：
- T028：`make api-check` 949/949、`make admin-check`、`make deploy-production-check`、`make client-sensitive-check`、`bash deploy/production/test-install-contract.sh`、`make external-deploy-check`、`make api-openapi-check api-client-check`、`git diff --check` 全部通过。
- T029：独立 agent 只读复核，结论与处置见 `docs/reviews/2026-10-22-external-nonprod-management-review.md`。确认 4 条缺陷（新增只读接口对 prod 可读、可重新启用管理员归档的应用、归档节点卡住管理面保存、可重新启用被停用目标绕过节点白名单）并证实 1 条怀疑项（探测/Env 同步重试触达非生产应用名下的生产目标），均已修复，并以 7 个变异验证。未处理项（http(s) userinfo 令牌入库、Agent ID 枚举、SSRF 未验证）已在复核记录中说明。
- T030：`docs/runbooks/external-deploy-api.md`、`docs/standards/access-control.md`、Skill 三份 references 与 plan 已与实现对齐（含本次修订：新增只读接口对 prod 403、重新启用限制、目标重新启用需节点授权）；经验沉淀见 `docs/solutions/2026-10-22-external-api-opening-nonprod-without-widening-prod.md`。

交付核对（T031）：对外接口 20 → 34 个，CLI 新增 13 个子命令，全部 FR-001～FR-018 均有测试或检查证据（逐条映射见提交记录；FR-015 核对：相对规格提交，非 `external_*` 的测试文件只有 `migrations.rs`（纯新增 96 行、0 删除）与无关的 agent 夹具修复被改动，内部管理面既有用例未改动）。已提交并推送 main，**未部署，未运行 `make deployer-skill-install`**，真实环境生效（正式环境部署、Skill 重新安装、上线后为现有 Key 配置节点）属运维验收，需用户说“部署正式环境”后进行。

**遗留（均非阻塞）**：
1. ~~内部 Git 来源 URL 校验对 `https://TOKEN@host/...` 放行~~：对外入口已处理（见下“仓库地址策略”）；内部管理面校验未改动。
2. ~~Agent 侧对内网地址的拉取行为未验证~~：已确认 Agent 无协议/主机限制；对外入口已拦截 IP 字面量，域名解析到内网仍需 Agent 侧限制协议与出口（涉及 Agent 发版，另议）。
3. `snapshot_hash` 在共用服务账号下本身就是凭据（plan 已声明，本期未改隔离粒度）。

**仓库地址策略（遗留 1+2 的后续，用户同意）**：`validate_external_repository_url` 仅在对外 `PUT git-source` 调用；规则与边界见 `docs/runbooks/external-deploy-api.md`。已核实不影响现有应用：地址仅在保存时校验（`application_sources/mod.rs` 唯一调用点），分支发现/设置分支/预览/部署读取库内已存地址；对外 Git 来源接口尚未随任何版本部署。测试：5 个单元测试（含 `ext::`/`fd::`/本地路径、云元数据、非规范数字主机）+ 2 个集成测试（内部保存同一内网地址仍 201、已存内网来源的对外发现仍 202、对外保存被拒且错误不回显地址、域名形式的内网 Git 可保存）。变异 5 处（不调用、放行 https 用户信息、放行链路本地、放行非规范数字主机、误把策略做进内部共用校验）均被抓到。实现过程中单元测试抓到一处漏洞：`ext::sh` 被当成 scp 写法放行，已修。

待执行：无。各阶段实际执行的命令与结果、失败/跳过/未验证项及对应提交；真实环境生效（正式环境部署、Skill 重新安装）属运维验收，不在本任务内。
