# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

本文件是本仓库唯一的项目指引：先给出工作规则（Spec Kit 工作流、远程执行授权、修改范围边界、migration 与 Git 提交规则），再补充命令与架构。文档、注释和提交信息使用简体中文，函数名、配置键、路径、协议字段等标识保持英文。

## 项目工作规则

### 工作流

- 本项目默认使用 Spec Kit；中型及以上任务走 `specify -> plan -> tasks -> analyze -> implement -> converge`，真正存在需求歧义时按需 clarify。完成后仍须代码审查、真实验证和小闭环提交。
- 明确的小任务允许直接定位、实施与聚焦验证，不强制生成整套规格；Bug 先复现、定位根因、修复和验证，范围扩大时再转入规格流程。
- 先核对已有 spec、brainstorm、plan 和实际完成状态，优先续写，不创建平行任务。旧计划可原地继续；转入 `specs/` 时标明唯一入口，只迁移未完成工作。
- 旧计划中的 CE 技能名称视为当时的步骤描述：需求/方案/实施/审查/经验沉淀按本项目当前规则完成，不要求加载 CE；保留原有业务决定、验收条件及验证强度。
- 新建或续写规格必须明确需求目录，使用项目 init/check 入口并读取返回的必读产物；不得复制模板覆盖已有 spec/plan/tasks。命令和约束见 `docs/runbooks/spec-kit-workflow.md`。
- spec 定义 WHAT/WHY，plan 定义 HOW，tasks 记录依赖、文件范围、验证与进度；已有 tasks 增量维护，保留 ID、状态、证据和人工备注。converge 只追加必要任务，无发现不写文件。
- analyze 阶段只读；checklist 只检查需求质量，不是实现验收，不得由 implement 静默勾选。阶段检查不替代语义复核或真实测试；已授权且无阻塞的实施自主推进。
- 每个 agent 显式指定需求目录；并行写入须隔离 worktree 且文件无交集，不手工改写 `.specify/feature.json`。Spec Kit、模板、Skill 和 hooks 不得覆盖本文件、项目规范或授权，也不得默认创建/切换分支、PR、外部工单、合并、跨模型执行或部署。
- `runbook` 不作为独立阶段：涉及运行、部署、迁移、排障或联调时，实施前查阅相关 `docs/runbooks/`，审查中核对其与实现一致；变更命令、前置条件、验证或恢复步骤时，完成前同步更新。

### 产物约定

- `docs/brainstorms/`：需求澄清与方案收敛
- `specs/<feature>/`：新需求的 spec、plan、tasks，辅助产物按需生成
- `.specify/memory/constitution.md`：项目原则摘要，不覆盖权威规范
- `docs/plans/`：既有计划与执行进度，保留历史并支持续接
- `docs/reviews/`：重要改动的复核与验证记录
- `docs/runbooks/`：运行、部署、迁移、排障与联调的可执行手册
- `docs/solutions/`：问题沉淀与经验复用
- `docs/standards/`：长期有效的产品、工程与协议规范

### 语言与路径

- `CLAUDE.md`、`docs/`、`specs/` 和人工维护的工作流文件、代码注释、说明文档、提交信息默认使用简体中文；上游受管理资产保留原文和原始 hashes
- 函数名、类型名、API 名称、配置键、命令名、路径、协议字段等领域性标识保持英文，或沿用项目既有约定
- 文档内统一使用仓库相对路径

### 执行规则

- 本机已具备 Docker 环境（2026-10-08 确认），本地开发、测试、Linux 权限回归与验收允许直接使用本机隔离容器；不得沿用“本机没有 Docker”的旧假设，不修改或关闭系统代理。

- **修改范围硬性边界**：本项目排查、修复和优化部署问题时，只允许修改 `deploy-go` 本项目自身的代码、测试、文档、协议、Agent、控制面和部署工具。严禁修改被部署的任何业务应用代码、业务应用配置、业务应用脚本、业务应用仓库内容或业务应用发布物；业务应用的问题必须通过本项目的控制面、Agent、协议、诊断和部署逻辑解决。
- 纯信息型、小范围问答可直接回答，不强制创建文档
- 文档权威顺序见 `docs/standards/document-authority.md`；涉及运行、部署、迁移、排障或联调的具体操作以相关 `docs/runbooks/` 为准，历史 plan 不能替代 runbook
- 规则优先级：当前用户指令 > 当前仓库的 `CLAUDE.md`、相关 `docs/runbooks/` 与 `docs/standards/` > Spec Kit 模板、Skill 与 hooks 默认行为 > 全局默认行为
- 有现成的 brainstorm 或 plan 时优先复用和续写，不重复开平行文档
- 大任务必须先在 plan/tasks 中拆出阶段、依赖和可验收单元；默认不要把整个大任务直接作为单个执行单元
- 中型及以上任务开始改动前先确认 spec/plan/tasks 或仍在执行的旧计划；小型明确任务可直接形成短执行说明并实施
- implement 按小步、可验证方式推进，完成一个闭环后先验证，再继续实施或复核
- 实施过程中避免无关重构；如果发现范围扩张，先显式记录并确认
- 工作流默认的 worktree、功能分支、PR、自动合并或 Git 操作不得覆盖本项目直接在 `main` 开发和按小闭环提交推送的规则
- 只有在缺关键决策、缺权限或凭证、缺外部输入、存在危险不可逆操作，或工作已经完成且验证通过时，才暂停推进

### 远程执行授权

- 开发、测试或验证部署功能，不等于获得连接真实节点或执行真实部署的授权
- 只有用户在当前对话中明确要求对具体环境或节点执行部署、远程脚本、重启、迁移、切流或清理操作时，才允许执行对应运行态操作
- “继续”“验证”“提交”“推送”“开始开发”等指令不构成远程执行授权；没有明确授权时，只能进行本地实现、测试、构建、文档整理和模拟验证
- 本地 fixture、mock 服务和明确隔离的测试容器不视为真实节点，但仍应遵守最小影响和可恢复原则
- 节点凭证、脚本敏感参数和部署日志必须遵守 `docs/standards/deploy-script-contract.md` 及后续安全规范

### Migration

- 已提交并可能被共享环境应用的 migration 默认不可变，不得修改、删除、重命名或重排；修正必须新增更高版本 migration
- 只有用户明确说明相关环境可清库重建，并授权迁移链路整理时，才允许调整历史 migration
- migration 规则在 Rust API 工程初始化后应同步写入对应 runbook，并配置聚焦校验
- 修改 `api/migrations/` 时，先执行 `make setup-git-hooks`（首次或切换 clone）和 `make verify-git-hooks`，再让本地 pre-commit 对 Git index 执行 migration 门禁；不得使用 `git commit --no-verify`、`git push --no-verify`、临时改写 `core.hooksPath` 或其他跳过方式。该本地门禁不能替代 SQLx migration 测试、环境 status、备份和发布验收

### Review

- 改动完成后对照 spec/plan/tasks 或仍在执行的旧计划复核结果
- 至少执行聚焦验证，并检查明显回归、范围漂移、遗漏和与计划不一致之处
- analyze 检查规格一致性，converge 检查实现遗漏，二者不能替代代码审查或真实测试；重要或高风险改动安排独立复核，能力不可用时明确报告缺口
- 审查先报告有证据的发现；需要修正的问题返回实施并重新验证，不把主 agent 自审称为独立复核
- 需要保留复核结论时，写入 `docs/reviews/`

### 经验沉淀

- 出现关键决策、复发坑点、有效排查路径或可复用模式时，写入 `docs/solutions/`；长期操作规则同步到 standards/runbooks，不依赖 CE Skill
- 不把一次性执行日志原样沉淀进去，优先保留可复用结论

### Git 提交与推送

- 默认直接在当前主分支开发；除非用户明确要求，不额外创建功能分支
- 每完成一个可单独解释、验证和回滚的小功能块或小修复，默认及时提交并推送
- 开始改文件前先执行 `git status --short`；工作区存在无关改动时不回滚、不整理、不混入本轮提交
- 提交前至少执行与改动范围匹配的测试或静态检查，以及 `git diff --check`、`git diff --cached --check` 和 `git diff --cached`
- 只暂存本轮相关文件，默认不使用 `git add .`
- 提交信息使用简体中文，建议前缀为 `feat:`、`fix:`、`docs:`、`test:`、`chore:` 或 `refactor:`
- 提交成功后默认执行 `git fetch origin`、`git rebase origin/main`、`git push origin main`；远端存在新提交时不得强推

### 工作方式

- 优先做小而可验证的改动
- 执行过程中避免无关重构
- 不引入隐藏脚本层、后台调度器或重型流程框架，除非任务明确需要

## 项目概览

deploy-go 托管并执行**应用自带的部署脚本**（固定的 Make target），提供控制面、过程日志与结果追踪；它不是通用 CI，也不替代应用自己的发布脚本。核心概念（节点、Agent、应用、部署、Git 来源、两阶段部署）见 `README.md`。新部署只经由在线 Agent 的结构化任务执行，**没有 SSH fallback**（`make api-check` 会 grep 拦截 API 镜像里的 `openssh-client` / `ssh-keyscan`）。

- Cargo workspace（edition 2024）共 10 个 crate：`api`、`agent`、`agent-protocol`、`agent-executor`、`agent-updater`、`release-authorization`、`terminal-capability`、`runtime-log`、`container-template`、`deploy-go-deployer`（对外部署 API 的 CLI）。
- 客户端：`admin/`（React 19 + Vite 纯客户端 SPA，唯一的 npm 工程）、`admin-app/`（Flutter）。
- `examples/` 放业务应用接入 Demo 与内建 Compose 模板，`skills/deploy-go-deployer/` 是配套 Claude Code Skill。

## 工具链与环境

- Rust 固定 `1.94.0`（`rust-toolchain.toml`，无 `rustfmt.toml` / `clippy.toml`，用默认配置）；Node ≥ 22；Java 21（生成 API client）；Flutter 3.41.5 / Dart 3.11.3；Docker（Linux 权限类验证）；`jq`；`bats`（安装器动态测试，仅 Linux）。
- 首次：`npm ci --prefix admin`（`admin/` 是唯一的 npm 工程，`postinstall` 固定 openapi-generator 7.24.0，生成器与 `openapitools.json` 也在 `admin/` 下），Flutter 依赖用 `make admin-app-get`。
- Makefile 只在检测到 `sccache` 时才给 make 内的 cargo 设置 `RUSTC_WRAPPER`，直接运行 `cargo` 不受影响；反之，全局 cargo 配了 `rustc-wrapper = "sccache"` 而本机没装 sccache，会让所有 cargo 命令失败。
- `target/` 可膨胀到百 GB 级，清理优先 `make rust-clean-dev`（只清 dev profile，保留 release）。**注意**：本机（macOS 27 / ld-27037.1）冷编译出的 `sqlx-macros` 动态库无法被 rustc 加载（`mis-aligned LINKEDIT string pool`），`make rust-clean-dev` 之后 `api` crate 会无法在本机重新编译；换 `strip`、`-no_fixup_chains`、`-Wl,-x` 等链接参数均无效。`agent` / `agent-executor` 等不依赖 sqlx 的 crate 不受影响。在 Xcode / CLT 更新修复之前，不要清理 `target/`，API 的编译与测试改在 Linux 容器里做（如 `make rust-linux-clippy`、`docker run rust:1.94-bookworm cargo test -p deploy-go-api …`）。
- 默认开发库是仓库根目录的 `deploy-go.db`（`*.db` 已 gitignore）；`.env` 不会自动加载，配置靠显式 `export`。
- 仓库没有 CI（`.github/` 已删除），提交前须在本地跑对应的 make 目标。

## 常用命令

`make help` 列出全部目标；构建、检查与测试命令统一以 Makefile 为入口。

| 目的 | 命令 |
| --- | --- |
| 启动 API（`127.0.0.1:30100`） | `make api-run`（服务模式须先 export 主密钥与两把签名密钥，见 `docs/runbooks/local-development.md`） |
| 只执行 migration | `make api-migrate`（不需要主密钥） |
| Web 开发服务器（`30101`，代理 `/api`） | `make admin`（另开终端先跑 `make api-run`） |
| Flutter 管理端 | `make admin-app` |
| Rust：fmt + clippy `-D warnings` + 全 workspace 测试 + OpenAPI 漂移 | `make api-check` |
| Agent / 协议 / 安装器 / manifest | `make agent-check`；只跑 Rust 测试用 `make rust-test-fast`（有 nextest 则用之） |
| Web / Flutter 全量检查 | `make admin-check` / `make admin-app-check` |
| 全仓检查 | `make check` |
| Spec Kit 防护入口 | `make spec-kit-init FEATURE=specs/<NNN-name>`、`make spec-kit-check FEATURE=… STAGE=plan/tasks/implement`、`make spec-kit-verify` |

**不在 `make check` 里、需显式运行**：`make admin-test-e2e`（Playwright + 路由 fixture，不连真实 API）、`make admin-app-test-integration DEVICE_ID=<id>`，以及 Docker 中的 Linux 验证 `make agent-executor-cgroup-check`、`make agent-runner-isolation-check`、`make privileged-terminal-check`、`make privileged-release-check`。

### 运行单个测试

- Rust 集成测试在各 crate 的 `tests/` 下一文件一个目标：`cargo test -p deploy-go-api --test two_stage_deployment`，再追加名称子串即可过滤单个用例；`src/` 内的单元测试加 `--lib`。
- Web：`npm --prefix admin test -- --run src/test/DeploymentFlow.test.tsx`（vitest，可加 `-t "<名称>"`）；单个 e2e：`npm --prefix admin run test:e2e -- deployment-flow.spec.ts`。
- Flutter：`cd admin-app && flutter test <path/to/xxx_test.dart>`。
- Shell 契约测试直接 `bash` 运行，如 `bash examples/branch-deployment/test-contract.sh`、`bash agent/install/test-systemd-contract.sh`。
- `api/tests/node_telemetry_load.rs` 的容量基线标了 `#[ignore]`，仅发布准备阶段显式执行。
- **Linux 专属代码**：`agent` / `agent-executor` 里的 cgroup v2、PTY、`SO_PEERCRED`、runner 身份切换等代码与测试用 `cfg(target_os = "linux")` 门控，macOS 上 `cargo test` 会直接跳过它们；改动这些路径必须用上面的 Docker 目标验证，不能以 macOS 测试通过为准；这些代码在 macOS 上连 clippy 都不会检查，需额外运行 `make rust-linux-clippy`（它曾积累过 6 个只在 Linux 触发的告警）。`agent-executor` 的 `release_lifecycle` 在 Linux 下经 cgroup launcher 启动 release 进程，测试通过 `ReleaseJobManager::with_launcher` 指向 executor 二进制，且只在 `DEPLOY_GO_RUN_CGROUP_V2_TEST=1` 的特权容器里运行（`make agent-executor-cgroup-check` 已包含）；直接 `cargo test` 会跳过这几个用例。
- API 集成测试多用内存 SQLite（`sqlite::memory:` + `db::migrate`），并直接调用 `agents::dispatcher::handle_agent_message` **模拟** Agent 消息，不依赖真实 Agent 或 WSS；共享夹具在 `api/tests/common/mod.rs`。唯一引用真实 Agent crate 的是 `api/tests/artifacts_api.rs`（制品传输客户端），改 `agent::artifact_transfer` 的公开接口会破坏它。

## 生成产物与联动

- **OpenAPI**：`api/openapi/openapi.json`（内部）与 `external.json`（对外部署 API）由代码生成并入库。新增 handler 除了写 `#[utoipa::path(operation_id = …)]`，还必须把 path 与 schema 登记进 `api/src/lib.rs` 的 `ApiDoc`（对外 API 在 `api/src/external/mod.rs` 自己的 `OpenApi`），再 `make api-openapi`。
- Cookie / CSRF / Agent bearer 这些安全声明不是写在 handler 上，而是 `enrich_openapi_security_contract`（`api/src/lib.rs`）按**硬编码路径前缀**统一注入：新增公开端点或 Agent bearer 端点（`/api/v1/agent/...`）时必须同步该函数，否则文档会把它标成 Cookie + CSRF。
- **双端 API client**：`admin/src/api/generated/` 与 `admin-app/lib/api/generated/` 由 `make api-client-generate` 从 OpenAPI 生成，**禁止手改**；OpenAPI 变了必须一并提交两端结果。校验：`make api-openapi-check api-client-check`。
- **版本号联动**：api / agent / agent-executor / agent-updater / deploy-go-deployer 的 `version` 保持一致（当前 0.3.31）。API 把自身 `CARGO_PKG_VERSION` 当作 Agent 目标版本：`agents::upgrades` worker 每 15 秒扫描版本不一致的 Agent（目前仅 x86_64、协议 ≥ v17、声明 `agent_upgrade_v1`），签发升级授权并下发 `AgentUpgradeCommand`，节点侧由 `agent-updater` 事务化替换。版本号还会写入 OpenAPI `info.version`。所以不要在普通改动里顺手改版本；升版必须同时 `make api-openapi` + `make api-client-generate`。测试里的发布 fixture 不要绑定 `CARGO_PKG_VERSION`：用 `AgentInstallation::with_api_version` / `DeployerInstallation::with_api_version` 固定到 fixture 目录里已有的版本（agent 当前为 0.3.20、deployer 为 0.3.26），否则每次升版本都会让 `make api-check` 变红。
- **内建模板**：`container-template` 用 `include_str!` 直接嵌入 `examples/templates/**`，改模板文件即改 API 产物，用 `make app-template-check` 校验。
- **Agent release manifest**：现行是 v4，`agent/release/generate-manifest.sh` 直接读取 `agent-protocol` / `agent-executor` 的版本常量，改协议版本后无需手填。`agent/release/manifest.schema.json` 实际是 v3 内容的兜底文件，现行 schema 是 `manifest-v4.schema.json`，别改错。

## 架构

### 进程与信任边界

控制面与节点是**多进程、分权限**设计，改动前先确认代码跑在哪一侧：

```text
浏览器 / Flutter ──Cookie + CSRF──► api（Axum + SQLite，进程内 worker）
                                      │ WSS 控制通道（agent-protocol 信封）
                                      │ + Agent 主动发起的 HTTPS（token 刷新、制品上传/下载、运行日志）
节点：deploy-go-agent（低权限用户，唯一联网进程）
        ├─ /run/deploy-go-agent-runner/runner.sock ─► runner broker（root）─► 降权为 deploy-go-runner 执行 prepare
        └─ /run/deploy-go-agent/executor.sock ─────► deploy-go-agent-executor（root）─► PTY 终端 / 固定 release job（cgroup v2）
      deploy-go-agent-updater（root，`--resume`，事务化升级）
```

- `agent` 一个二进制承担多种角色，由首个命令行参数切换（`runner-service`、`runner`、`runner-stdin`、`executor-probe`、`privileged-release-self-test` 等，见 `agent/src/main.rs`）；四个 systemd unit 与安装器在 `agent/install/`。
- 共用边界件：`agent-protocol`（线协议类型 + JSON Schema，API / Agent / executor 共用）；`release-authorization` 与 `terminal-capability`（API 用 Ed25519 签发短期授权，executor 离线验签，分别对应特权 release / Agent 升级与 PTY）；`runtime-log`（节点诊断日志分段存储）。
- `agent-executor` 的 `runtime` feature 才引入 PTY 依赖；`agent` 以 `default-features = false` 引用其库（只要协议与授权类型），所以 PTY 代码必须留在 `cfg(feature = "runtime")` 之内。

### 两阶段部署主流程

预览（持久化 snapshot、绑定 hash，需 `confirm`）→ 入队 → `deployments::runtime::run_worker` 轮询（全局并发取自系统设置 `max_concurrent_deployments`，同一 Agent 串行）→ `agents::dispatcher` 派发：

1. `deployment_prepare`：Build Agent 检出**固定 commit SHA**（或 workspace 快照），以 `deploy-go-runner` 身份执行固定 Make target `deploy-go-prepare`（线协议 `make_target` 枚举值是下划线形式 `deploy_go_prepare`，别混淆），产出带 manifest 的 artifact；Agent 发 `artifact_prepared`，API 校验后签发 upload lease，Agent 经 HTTPS 上传（`api/src/artifacts/`）。
2. `deployment_release`：Target Agent 下载并复验 artifact、同步 Env 并过 Env gate；API 签发 release 专属授权，root executor 离线验签后把输入封存成 root 所有的 bundle，只执行固定的 `make --no-print-directory deploy-go-release`；业务脚本用 `DEPLOY_GO_EVENT` marker 汇报进度。
3. 结果经 `dispatcher::handle_result` / `apply_result` 投影到 target run 与 deployment。镜像类（`checkout_mode=artifact`）和 workspace 部署走同一条特权 release 链路，不存在无 artifact 的 release，也不回退到 launcher。

### 可靠性模型（改任务 / 日志 / 恢复代码前必读）

整条链路靠"持久化 + 幂等重放"，不假设连接不断：

- API 重启时 `deployments::runtime::recover` 把过期的 delivered 任务重新入队，把无法关联任务的 running 部署收敛为 `interrupted`。
- Agent 先把任务写入本地 journal / outbox，再按序号发 state / output / progress / result；v18 起主控回 `task_event_receipt`，Agent 只按连续回执推进确认水位，断线后重放（`agent/src/journal.rs`、`agent/src/log_delivery.rs`）。
- executor 的 release job 状态与分块日志落在 root 专用目录，Agent 重连后按 offset 续取同一个 `job_id`。**禁止**为恢复而重发 `ReleaseStart`、新建 job，或把未证明的结果伪报成功。
- `agent/src/task_handler.rs` 与 `api/src/agents/dispatcher.rs` 是体量最大的两个文件，也是这套模型的核心；其中的提前返回或吞掉的错误会表现为"节点已成功但控制面仍 running"，参见 `docs/solutions/2026-10-12-privileged-release-result-not-reported.md`。错误要有结构化诊断和可恢复状态，不能静默返回。

### 安全不变量

- 任务 payload 一律是严格枚举 + `deny_unknown_fields`：没有任意 shell / command / Make target / 环境变量 map / 带凭证 URL 的字段；Git 凭证与 Secret 只以 opaque lease ID 出现。新能力要新增结构化 operation，不能把 shell 字符串塞进普通任务。
- 能力或版本未知时 fail closed；联网 Agent 永远低权限，root 操作只能经 executor 的版本化 Unix Socket 协议。
- Secret（Env、凭证、终端正文、lease 值）不得进入日志、tracing、journal、OpenAPI 示例或浏览器持久化存储；凭证与配置类数据用 `crypto::MasterKeyRing` 加密落库；涉及 Env / 配置的响应带 `Cache-Control: no-store`。`make client-sensitive-check` 扫描客户端产物里的私钥、token 与 lease。
- 用户 Cookie 会话与 Agent 身份完全隔离；状态变更请求校验 CSRF 与 Origin；前端隐藏入口不算授权，服务端用 `AuthUser` + `require_*` 校验。
- 权威出处：`docs/standards/` 下的 `agent-control-protocol`、`privileged-agent-executor`、`deploy-script-contract`、`api-contract`、`access-control`。

### Agent 控制协议

线协议类型只定义在 `agent-protocol/src/lib.rs`：`PROTOCOL_VERSION = 18`，最低 `MIN_SUPPORTED_PROTOCOL_VERSION = 11`。初始 `hello` 固定用 v11 envelope，之后双方按最高共同版本通信；消息按版本门控（`validate_for_envelope_version`），低版本连接不得收到新消息。改线协议时同步最新的 `agent-protocol/schema/agent-control.schema.json` 与 `agent-protocol/tests/schema_compatibility.rs`；同目录下带版本号的 `agent-control-v*.schema.json` 是不可变的历史 schema，不要改。文档里“只发给协议 vN”之类的门禁都指协商版本 ≥ vN，API 侧用 `*_MIN_PROTOCOL_VERSION` 常量判断。

Agent ↔ executor 的本机协议（`agent-executor/src/protocol.rs`，当前 v4）是另一套版本号，Agent 要求与 executor **精确匹配**，所以 Agent 与 executor 必须成对发布。

### API 约定

- 每个资源一个 `api/src/<resource>/mod.rs`：`router()` + handler，挂到 `lib.rs` 的 `app()`；SQL 直接写在模块内（sqlx，无 ORM / repository 层）。
- 错误统一走 `ApiError`（稳定的 `code`、带 `request_id`，不泄露 SQL / 路径 / 密钥）；资源 ID 是带前缀的 ULID，时间为 UTC RFC 3339；请求 JSON 含未声明字段返回 `422`。
- Migration 在 `api/migrations/NNNN_snake_case.sql`（当前到 0041），不可变规则与 Git 门禁见上文「Migration」一节和 `docs/runbooks/api-migrations.md`。补充：新增 migration 不得含 `DROP TABLE` / `DROP COLUMN`（门禁拒绝，用"新增列 + 回填 + 旧列标 deprecated"）；改 schema 要同步 `api/tests/migrations.rs` 并跑 `cargo test -p deploy-go-api --test migrations --test database_constraints`；`db::migrate` 逐版本执行，且对重建表的 0003 / 0005 / 0008 / 0012 / 0020 在关闭外键的专用连接上跑（`FOREIGN_KEY_ISOLATED_MIGRATIONS`）。

### 前端要点

- `admin/`：单元 / 组件测试用 vitest + MSW（`src/test/`），浏览器 smoke 在 `e2e/`。保持纯客户端 SPA（`BrowserRouter`），不得启用 React Router 的 RSC / server action。
- `admin-app/`：Riverpod + Dio / CookieJar，Cookie 与 CSRF token 只进 Keychain / Keystore，不放 `--dart-define`。
- 本地联调的 Origin 白名单：`make api-run` 默认放行 `http://127.0.0.1:30101` 并关闭 cookie `Secure`；改 Web 端口必须同步 API 的 `DEPLOY_GO_ALLOWED_ORIGIN`，Flutter 的 `DEPLOY_GO_ALLOWED_ORIGIN` 必须是 API 白名单成员。

## 文档地图

权威顺序：用户当前指令 > 本文件 > `docs/standards/` > `docs/runbooks/` > `docs/solutions/` > `specs/<feature>/` 与 active 的 `docs/plans/` > 历史计划与总结（仅作背景），详见 `docs/standards/document-authority.md`。

- 命令、环境变量、本地联调：`docs/runbooks/local-development.md`；运行 / 部署 / 迁移 / 排障先查对应 runbook（索引 `docs/runbooks/README.md`）。
- 契约：`docs/standards/`（`api-contract`、`agent-control-protocol`、`privileged-agent-executor`、`application-deployment-contract`、`git-branch-deployment-contract`、`access-control`）。
- 复发问题与排查路径：`docs/solutions/`；进行中任务的续接：`specs/` 与 `docs/handoffs/`。
- **文档会漂移**：版本号、阈值、间隔这类数值类表述最容易过期（曾出现 standards / runbook 与代码不一致）。以代码为准——控制协议版本看 `agent-protocol/src/lib.rs`，executor 本机协议看 `agent-executor/src/protocol.rs`——并顺手修正文档。
- Spec Kit 阶段 Skill 在 `.claude/skills/speckit-*/SKILL.md`，Claude Code 原生发现，用 `/speckit-specify`、`/speckit-plan`、`/speckit-tasks`、`/speckit-analyze`、`/speckit-implement`、`/speckit-converge` 等调用；新会话才会刷新可见 Skill，当前会话看不到时直接读取对应 `SKILL.md` 并按其流程执行，但不要声称已通过原生入口发现。

## 有副作用或触达真实环境的命令

开发、测试、"继续"、提交都不构成真实环境授权（见上文「远程执行授权」）。以下目标需要当前对话对具体环境的明确授权，或会写到仓库之外：

- `make deploy-production`：经 SSH 在正式环境上构建并安装（别名 `qfy-test2`，公网为 `qfy-test`）；先读 `deploy/production/README.md` 与 `docs/runbooks/systemd-deployment-production.md`。`make deploy-production-check` 只做本地契约检查。
- **“部署正式环境”即授权**：用户在当前对话说出“部署正式环境”，就等于对 Deploy Go 唯一的正式环境（`qfy-test2` / `qfy-test` 是同一台机器）授权执行 `make deploy-production`，并默认连带让所有符合条件的节点自动串行升级，不再追问环境名或升级范围。部署前仍须按 runbook 核对服务器身份；身份不符、健康检查失败等真实异常才停下。该授权不覆盖业务应用、重发或取消业务部署、手改线上状态。对外统一称“正式环境”，不称“控制面”。
- 正式环境访问不到 Docker Hub 时远程构建会失败（不改动服务器），改用 `DEPLOY_BUILD_MODE=local make deploy-production`，详见 runbook 排障。
- `make deployer-skill-install`：写入 `~/.claude/skills/deploy-go-deployer`。
- `make credential-reencrypt`：对 `DEPLOY_GO_DATABASE_URL` 指向的库离线重加密凭证。
