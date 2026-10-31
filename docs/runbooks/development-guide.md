# 开发工程指南

通用协作、权限、提交与验证规则以 [AGENTS.md](../../AGENTS.md) 为准；文档分类见[文档导航](../README.md)。本文维护工具链、验证映射、生成联动和架构解释；流程选择和产物定位见 [Spec Kit 工作流](spec-kit-workflow.md)。完整 API/Web 装配默认使用[独立远端开发栈](remote-development.md)，轻量本机调试命令不扩大运行授权。

## 项目概览

deploy-go 托管并执行**应用自带的部署脚本**（固定的 Make target），提供控制面、过程日志与结果追踪；它不是通用 CI，也不替代应用自己的发布脚本。核心概念（节点、Agent、应用、部署、Git 来源、两阶段部署）见 `README.md`。新部署只经由在线 Agent 的结构化任务执行，**没有 SSH fallback**（`make api-check` 会 grep 拦截 API 镜像里的 `openssh-client` / `ssh-keyscan`）。

- Cargo workspace（edition 2024）共 10 个 crate：`api`、`agent`、`agent-protocol`、`agent-executor`、`agent-updater`、`release-authorization`、`terminal-capability`、`runtime-log`、`container-template`、`deploy-go-deployer`（对外部署 API 的 CLI）。
- 客户端：`admin/`（React 19 + Vite 纯客户端 SPA，唯一的 npm 工程）、`admin-app/`（Flutter）。
- `examples/` 放业务应用接入 Demo 与内建 Compose 模板，`skills/deploy-go-deployer/` 是配套 Agent Skill。

## 工具链与环境

- Rust 固定 `1.99.0`（[rust-toolchain.toml](../../rust-toolchain.toml)，rustfmt/clippy 使用默认配置）；Node ≥ 22；Java 21（生成 API client）；Flutter 3.41.5 / Dart 3.11.3；Docker（Linux 权限类验证）；`jq`；`bats`（安装器动态测试，仅 Linux）。建议安装 `cargo-nextest`（`cargo install cargo-nextest --locked`）：`make api-check` / `make rust-test-workspace` 检测到它时使用 nextest 并补跑 doc-tests，否则退回 `cargo test`。时间敏感用例的配置在 [.config/nextest.toml](../../.config/nextest.toml)，历史性能数据见[构建与测试优化记录](../plans/2026-08-13-rust-build-test-optimization-plan.md)。
- 首次：`npm ci --prefix admin`（`admin/` 是唯一的 npm 工程，`postinstall` 固定 openapi-generator 7.24.0，生成器与 `openapitools.json` 也在 `admin/` 下），Flutter 依赖用 `make admin-app-get`。
- Makefile 只在检测到 `sccache` 时才给 make 内的 cargo 设置 `RUSTC_WRAPPER`，直接运行 `cargo` 不受影响；反之，全局 cargo 配了 `rustc-wrapper = "sccache"` 而本机没装 sccache，会让所有 cargo 命令失败。
- `target/` 可膨胀到百 GB 级，清理优先 `make rust-clean-dev`（只清 dev profile，保留 release）。**注意**：本机（macOS 27 / ld-27037.1）上，Rust 1.94.0 冷编译出的 `sqlx-macros` 动态库无法被 rustc 加载（`mis-aligned LINKEDIT string pool`），这也是项目升到 1.99.0 的直接原因（1.99.0 链接正常）；若再遇到 `dlopen … mis-aligned LINKEDIT`，先确认用的是 1.99.0 而不是旧工具链。clippy 在 macOS 上不检查 `cfg(target_os = "linux")` 的代码，需另跑 `make rust-linux-clippy`。
- `make api-run` / `make api-migrate` 默认开发库是 `.data/deploy-go.db`（`.data/` 与 `*.db` 已 gitignore，目录由 make 自动创建，可用 `DEPLOY_GO_DATABASE_URL` 覆盖；API 二进制自身的默认值仍是相对当前目录的 `deploy-go.db`）；`.env` 不会自动加载，配置靠显式 `export`。
- 仓库没有 CI（`.github/` 已删除），提交前须在本地跑对应的 make 目标。

## 常用命令

`make help` 列出常用目标；完整定义以 [Makefile](../../Makefile) 为准。先按下节选择验证，以下运行命令仅用于已有授权范围内的开发操作。

| 目的 | 命令 |
| --- | --- |
| 启动 API（`127.0.0.1:30100`） | `make api-run`（服务模式须先 export 主密钥与两把签名密钥，见 `docs/runbooks/local-development.md`） |
| 只执行 migration | `make api-migrate`（不需要主密钥） |
| Web 开发服务器（`30101`，代理 `/api`） | `make admin`（另开终端先跑 `make api-run`） |
| Flutter 管理端 | `make admin-app` |
| 查看验证目标及其覆盖范围 | [按改动范围选择验证](#按改动范围选择验证) |
| Spec Kit 防护入口 | `make spec-kit-init FEATURE=specs/<NNN-name>`、`make spec-kit-check FEATURE=… STAGE=plan/tasks/implement`、`make spec-kit-verify` |

## 按改动范围选择验证

多种范围同时命中时叠加要求。文档同步不替代代码检查，聚焦测试用于迭代，提交前仍运行匹配范围的门禁；所有改动检查最终 diff、`git diff --check` 和 `git diff --cached --check`。

| 改动范围 | 提交前检查 | 补充要求与覆盖边界 |
| --- | --- | --- |
| 人工文档、导航和任务正文 | `make docs-check` | 核对事实来源、命令与时效；标题锚点和内容语义需另核对 |
| Spec Kit wrapper、项目覆盖模板、存储解析 | `make spec-kit-verify docs-check` | 真实任务还要运行对应的 `spec-kit-check` 阶段；fixture 通过不证明外部 Vault 正文就绪 |
| Rust 源码，包括 API、共用 crate、executor、updater | `make api-check` | 名称含 API，实际为全 workspace fmt、clippy、测试和内部 OpenAPI 漂移检查；macOS 不覆盖 Linux 专属路径 |
| Agent、控制协议、安装器或 release manifest | `make agent-check`；改 Rust 源码再加 `make api-check` | agent-check 为 Agent/协议测试及安装器/manifest 契约，不含 fmt/clippy；协议变更同步当前 Schema 和兼容测试，安装器动态测试需要 Linux/Bats |
| API 路由、DTO 或 OpenAPI 契约 | `make api-check api-client-check`；对外 API 再加 `make external-deploy-check` | 先生成受影响的内部/对外 OpenAPI 及双端 client；客户端行为改变时叠加 Web/Flutter 检查 |
| 数据库 schema 或 migration | `make api-check`、迁移/约束集成测试 | 先 `make setup-git-hooks verify-git-hooks`；规则及精确命令见 [Migration 手册](api-migrations.md) |
| Web 管理端 | `make admin-check` | 覆盖 lint、类型、单元/组件测试和构建；关键交互及导航变化加 `make admin-test-e2e` |
| Flutter 管理端 | `make admin-app-check` | 覆盖格式、analyze 和测试；设备安全存储、导航与生命周期变化加 `make admin-app-test-integration DEVICE_ID=<id>`；构建见本地开发手册 |
| 独立远端开发工具或 Compose 契约 | `make dev-remote-check docs-check` | 本地渲染与 mock 隔离检查，不连接远端或启动开发栈；真实联调另按授权执行 |
| 正式环境部署脚本 | `make deploy-production-check docs-check` | 静态/fixture 契约验证，运行发布另按正式部署授权和手册执行 |
| 业务接入示例、历史 launcher、内建模板 | 按范围运行 `make deploy-contract-demo-check`、`make privileged-launcher-check`、`make app-template-check` | 内建模板嵌入 API；运行链路变化时叠加 Rust 与相关部署验证 |
| 对外 API、deployer CLI 与发布链路 | `make external-deploy-check`；仅 CLI 时用 `make deployer-check` | Rust 变更叠加 api-check；只改概览文档时核对 CLI 枚举与手册，无需重跑产品测试 |
| 多模块改动、范围难以隔离 | `make check` | 包含 docs、Spec Kit、Rust、Web、Flutter、生成产物、开发隔离和各脚本门禁；仍需下列专项检查 |

API 契约生成入口是 `make api-openapi`（内部）与 `make api-external-openapi`（对外）；双端 client 使用 `make api-client-generate`。敏感数据、会话、Env 或凭证相关改动叠加 `make client-sensitive-check`。

以下目标不在 `make check` 中，按改动的实际路径显式选择：

| 专项范围 | 额外检查 |
| --- | --- |
| 任意 Linux 专属 Rust 路径 | `make rust-linux-clippy`，并运行对应 Linux 回归；macOS clippy 不检查这些路径 |
| executor cgroup / release 生命周期 | `make agent-executor-cgroup-check` |
| runner 身份与权限隔离 | `make agent-runner-isolation-check` |
| 管理员 PTY 能力、授权、IPC 或终端交互 | `make privileged-terminal-check`（已包含 cgroup 与 runner 隔离目标） |
| 特权发布、制品交接、发布授权或恢复可靠性 | `make privileged-release-check`（已包含 cgroup 与 runner 隔离目标） |
| Web 浏览器交互 | `make admin-test-e2e`，使用 Playwright 路由 fixture，不连真实 API |
| Flutter 设备级行为 | `make admin-app-test-integration DEVICE_ID=<id>`，使用隔离 fixture |

每次交付分别报告通过、失败、跳过和未验证。可用性探测、进程 healthy 或结构检查不能代替业务验收；检查过程中提示跳过的项目不能计为已验证。运行和 Linux fixture 细节见[本地开发手册](local-development.md)。

### 运行单个测试

- Rust 集成测试在各 crate 的 `tests/` 下一文件一个目标：`cargo test -p deploy-go-api --test two_stage_deployment`，再追加名称子串即可过滤单个用例；`src/` 内的单元测试加 `--lib`。
- Web：`npm --prefix admin test -- --run src/test/DeploymentFlow.test.tsx`（vitest，可加 `-t "<名称>"`）；单个 e2e：`npm --prefix admin run test:e2e -- deployment-flow.spec.ts`。
- Flutter：`cd admin-app && flutter test <path/to/xxx_test.dart>`。
- Shell 契约测试直接 `bash` 运行，如 `bash examples/branch-deployment/test-contract.sh`、`bash agent/install/test-systemd-contract.sh`。
- `api/tests/node_telemetry_load.rs` 的容量基线标了 `#[ignore]`，仅发布准备阶段显式执行。
- Linux 单测注意：`agent-executor` 的 `release_lifecycle` 通过 `ReleaseJobManager::with_launcher` 指向 executor 二进制，只在 `DEPLOY_GO_RUN_CGROUP_V2_TEST=1` 的特权容器里执行（`make agent-executor-cgroup-check` 已包含）；直接 `cargo test` 会跳过这些用例。其他 Linux 专项按上表选择。
- API 集成测试多用内存 SQLite（`sqlite::memory:` + `db::migrate`），并直接调用 `agents::dispatcher::handle_agent_message` **模拟** Agent 消息，不依赖真实 Agent 或 WSS；共享夹具在 `api/tests/common/mod.rs`。唯一引用真实 Agent crate 的是 `api/tests/artifacts_api.rs`（制品传输客户端），改 `agent::artifact_transfer` 的公开接口会破坏它。

## 生成产物与联动

- **OpenAPI**：`api/openapi/openapi.json`（内部）与 `external.json`（对外部署 API）由代码生成并入库。新增 handler 除了写 `#[utoipa::path(operation_id = …)]`，还必须把 path 与 schema 登记进 `api/src/lib.rs` 的 `ApiDoc`（对外 API 在 `api/src/external/mod.rs` 自己的 `OpenApi`），再 `make api-openapi`。
- Cookie / CSRF / Agent bearer 这些安全声明不是写在 handler 上，而是 `enrich_openapi_security_contract`（`api/src/lib.rs`）按**硬编码路径前缀**统一注入：新增公开端点或 Agent bearer 端点（`/api/v1/agent/...`）时必须同步该函数，否则文档会把它标成 Cookie + CSRF。
- **双端 API client**：`admin/src/api/generated/` 与 `admin-app/lib/api/generated/` 由 `make api-client-generate` 从 OpenAPI 生成，**禁止手改**；OpenAPI 变了必须一并提交两端结果。校验：`make api-openapi-check api-client-check`。
- **版本号联动**：api / agent / agent-executor / agent-updater / deploy-go-deployer 的 `version` 保持一致（当前 0.3.33）。API 把自身 `CARGO_PKG_VERSION` 当作 Agent 目标版本：`agents::upgrades` worker 每 15 秒扫描版本不一致的 Agent（目前仅 x86_64、协议 ≥ v17、声明 `agent_upgrade_v1`），签发升级授权并下发 `AgentUpgradeCommand`，节点侧由 `agent-updater` 事务化替换。版本号还会写入 OpenAPI `info.version`。所以不要在普通改动里顺手改版本；升版必须同时 `make api-openapi` + `make api-client-generate`。测试里的发布 fixture 不要绑定 `CARGO_PKG_VERSION`：用 `AgentInstallation::with_api_version` / `DeployerInstallation::with_api_version` 固定到 fixture 目录里已有的版本（agent 当前为 0.3.20、deployer 为 0.3.26），否则每次升版本都会让 `make api-check` 变红。
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
- Migration 在 `api/migrations/NNNN_snake_case.sql`（当前到 0043；0043 的 `external_api_key_nodes` 表已废弃但保留，不再使用，见 `docs/runbooks/external-deploy-api.md`），不可变规则与 Git 门禁见 `AGENTS.md` 的 Migration 规则和 `docs/runbooks/api-migrations.md`。补充：新增 migration 不得含 `DROP TABLE` / `DROP COLUMN`（门禁拒绝，用"新增列 + 回填 + 旧列标 deprecated"）；改 schema 要同步 `api/tests/migrations.rs` 并跑 `cargo test -p deploy-go-api --test migrations --test database_constraints`；`db::migrate` 逐版本执行，且对重建表的 0003 / 0005 / 0008 / 0012 / 0020 在关闭外键的专用连接上跑（`FOREIGN_KEY_ISOLATED_MIGRATIONS`）。

### 前端要点

- `admin/`：单元 / 组件测试用 vitest + MSW（`src/test/`），浏览器 smoke 在 `e2e/`。保持纯客户端 SPA（`BrowserRouter`），不得启用 React Router 的 RSC / server action。
- `admin-app/`：Riverpod + Dio / CookieJar，Cookie 与 CSRF token 只进 Keychain / Keystore，不放 `--dart-define`。
- 本地联调的 Origin 白名单：`make api-run` 默认放行 `http://127.0.0.1:30101` 并关闭 cookie `Secure`；改 Web 端口必须同步 API 的 `DEPLOY_GO_ALLOWED_ORIGIN`，Flutter 的 `DEPLOY_GO_ALLOWED_ORIGIN` 必须是 API 白名单成员。
