# 合并 API 集成测试二进制实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

Cargo 把 `tests/*.rs` 的每个文件和 `tests/*/main.rs` 都当作一个独立测试 crate，各自链接整个 API 库。nextest 对每个用例单独起进程，因此合并二进制不改变用例级隔离。原型已验证机制：子文件放入目录，`mod common;` 改为 `use crate::common;`，一次编译通过。API 测试无环境变量修改、无全局静态、无内部属性，42 个文件使用 `mod common;`，14 个不使用。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md)：不改测试逻辑与产品代码；测试布局属于验证策略，走完整产物；以用例清单一致作为不放松门禁的证据；保留 AGENTS 第 5 节的 migration 测试命令。

## 分组

| 二进制 | 文件 |
| --- | --- |
| agents | agent_dispatcher, agent_download, agent_end_to_end, agent_enrollment, agent_releases, agent_upgrades, agent_websocket, agents_store, env_sync_dispatcher, git_refs_dispatcher, nodes_api, node_telemetry_api, node_telemetry_load, runtime_log_ingest, runtime_probe_api |
| deployments | deployments_api, deployment_manifests, deployment_recovery, deployment_runtime, deployment_targets_api, deploy_event_protocol, two_stage_deployment, execution_spec, artifacts_api, deployer_release |
| applications | application_configs_api, application_envs_api, application_sources_api, application_templates_api, applications_api, workspace_sources_api, configuration_centers_api, git_credentials_api, ssh_credentials_api, credential_encryption |
| external | external_api, external_api_keys, external_app_management, external_git_source, external_openapi_contract, external_preview_probe, external_production_guard |
| platform | audit_api, auth_api, authorization_api, health, openapi_contract, session_lifetime, settings_api, terminal_api, terminal_authorization, terminal_store, terminal_websocket, users_api |
| 保持独立 | migrations, database_constraints |

## 实现方案与文件范围

1. 基线：在合并前记录用例清单（`cargo nextest list -p deploy-go-api` 归一化为“文件名/用例名”）、全 workspace 通过数、改 `api/src` 后的测试重建墙钟与 `target/debug/deps` 中 API 测试二进制体积。
2. 用脚本按分组 `git mv` 并改写 `mod common;`，生成每组的 `main.rs`；编译；对比清单。
3. 同步引用：Makefile、docs、AGENTS、`.config/nextest.toml`。
4. 门禁：docs-check 增加 `--test` 目标存在性检查与自检。
5. 记录布局与分组规则；补测量数据。

## 阶段与依赖

1. 基线（先做，合并前）。
2. 合并与清单对比，依赖 1。
3. 引用同步与门禁，依赖 2。
4. 全量验证、leaky 调查、自审 diff、converge、提交推送。

## 验证计划

清单逐项对比；`make api-check`；`cargo test -p deploy-go-api --test migrations --test database_constraints`；`make external-deploy-check`；`cargo test -p deploy-go-api`（抽样验证同进程并发无问题）；重建与体积前后对比；`make docs-check`（含自检）、`make spec-kit-verify`、`make whitespace-check`；`make check`。

## 风险与回退

整个改动是文件移动加一行改写，可整体回退单个提交；用例清单不一致或出现新的失败则不提交。命令引用漏改由新增的 docs-check 目标存在性检查兜底。
