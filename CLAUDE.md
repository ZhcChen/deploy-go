# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

工作流（Spec Kit）、远程执行授权、修改范围边界、migration 与 Git 提交规则以 `AGENTS.md` 为准，在此直接导入而不重复；本文件只补充命令与架构。文档、注释和提交信息使用简体中文，函数名、配置键、路径、协议字段等标识保持英文。

@AGENTS.md

## 项目概览

deploy-go 托管并执行**应用自带的部署脚本**（固定的 Make target），提供控制面、过程日志与结果追踪；它不是通用 CI，也不替代应用自己的发布脚本。核心概念（节点、Agent、应用、部署、Git 来源、两阶段部署）见 `README.md`。新部署只经由在线 Agent 的结构化任务执行，**没有 SSH fallback**（`make api-check` 会 grep 拦截 API 镜像里的 `openssh-client` / `ssh-keyscan`）。

- Cargo workspace（edition 2024）共 10 个 crate：`api`、`agent`、`agent-protocol`、`agent-executor`、`agent-updater`、`release-authorization`、`terminal-capability`、`runtime-log`、`container-template`、`deploy-go-deployer`（对外部署 API 的 CLI）。
- 客户端：`admin/`（React 19 + Vite 纯客户端 SPA，唯一的 npm workspace）、`admin-app/`（Flutter）。`ui/` 是静态 HTML 设计源，不是产品代码。
- `examples/` 放业务应用接入 Demo 与内建 Compose 模板，`skills/deploy-go-deployer/` 是配套 Codex Skill。

## 工具链与环境

- Rust 固定 `1.94.0`（`rust-toolchain.toml`，无 `rustfmt.toml` / `clippy.toml`，用默认配置）；Node ≥ 22；Java 21（生成 API client）；Flutter 3.41.5 / Dart 3.11.3；Docker（Linux 权限类验证）；`jq`；`bats`（安装器动态测试，仅 Linux）。
- 首次：根目录 `npm ci`（`postinstall` 固定 openapi-generator 7.24.0），Flutter 依赖用 `make admin-app-get`。
- Makefile 只在检测到 `sccache` 时才给 make 内的 cargo 设置 `RUSTC_WRAPPER`，直接运行 `cargo` 不受影响；反之，全局 cargo 配了 `rustc-wrapper = "sccache"` 而本机没装 sccache，会让所有 cargo 命令失败。
- `target/` 可膨胀到百 GB 级，清理优先 `make rust-clean-dev`（只清 dev profile，保留 release）。
- 默认开发库是仓库根目录的 `deploy-go.db`（`*.db` 已 gitignore）；`.env` 不会自动加载，配置靠显式 `export`。
- GitHub Actions 的 CI 已暂停自动触发（仅 `workflow_dispatch`），没有 CI 兜底，提交前须在本地跑对应的 make 目标。

## 常用命令

`make help` 列出全部目标；构建、检查与测试命令统一以 Makefile 为入口。

| 目的 | 命令 |
| --- | --- |
| 启动 API（`127.0.0.1:30100`） | `make api-run`（服务模式须先 export 主密钥与两把签名密钥，见 `docs/runbooks/local-development.md`） |
| 只执行 migration | `make api-migrate`（不需要主密钥） |
| Web 开发服务器（`30101`，代理 `/api`） | `make admin`（另开终端先跑 `make api-run`） |
| Flutter 管理端；UI 设计源预览（`30102`） | `make admin-app` / `make ui` |
| Rust：fmt + clippy `-D warnings` + 全 workspace 测试 + OpenAPI 漂移 | `make api-check` |
| Agent / 协议 / 安装器 / manifest | `make agent-check`；只跑 Rust 测试用 `make rust-test-fast`（有 nextest 则用之） |
| Web / Flutter 全量检查 | `make admin-check` / `make admin-app-check` |
| 全仓检查 | `make check` |
| Spec Kit 防护入口 | `make spec-kit-init FEATURE=specs/<NNN-name>`、`make spec-kit-check FEATURE=… STAGE=plan/tasks/implement`、`make spec-kit-verify` |

**不在 `make check` 里、需显式运行**：`make ui-test`、`make admin-test-e2e`（Playwright + 路由 fixture，不连真实 API）、`make admin-app-test-integration DEVICE_ID=<id>`，以及 Docker 中的 Linux 验证 `make agent-executor-cgroup-check`、`make agent-runner-isolation-check`、`make privileged-terminal-check`、`make privileged-release-check`。

### 运行单个测试

- Rust 集成测试在各 crate 的 `tests/` 下一文件一个目标：`cargo test -p deploy-go-api --test two_stage_deployment`，再追加名称子串即可过滤单个用例；`src/` 内的单元测试加 `--lib`。
- Web：`npm test --workspace deploy-go-admin -- --run src/test/DeploymentFlow.test.tsx`（vitest，可加 `-t "<名称>"`）；单个 e2e：`npm run test:e2e --workspace deploy-go-admin -- deployment-flow.spec.ts`。
- Flutter：`cd admin-app && flutter test <path/to/xxx_test.dart>`。
- Shell 契约测试直接 `bash` 运行，如 `bash examples/branch-deployment/test-contract.sh`、`bash agent/install/test-systemd-contract.sh`。
- `api/tests/node_telemetry_load.rs` 的容量基线标了 `#[ignore]`，仅发布准备阶段显式执行。
- **Linux 专属代码**：`agent` / `agent-executor` 里的 cgroup v2、PTY、`SO_PEERCRED`、runner 身份切换等代码与测试用 `cfg(target_os = "linux")` 门控，macOS 上 `cargo test` 会直接跳过它们；改动这些路径必须用上面的 Docker 目标验证，不能以 macOS 测试通过为准。
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
- Migration 在 `api/migrations/NNNN_snake_case.sql`（当前到 0041），不可变规则与 Git 门禁见 `AGENTS.md` 和 `docs/runbooks/api-migrations.md`。补充：新增 migration 不得含 `DROP TABLE` / `DROP COLUMN`（门禁拒绝，用"新增列 + 回填 + 旧列标 deprecated"）；改 schema 要同步 `api/tests/migrations.rs` 并跑 `cargo test -p deploy-go-api --test migrations --test database_constraints`；`db::migrate` 逐版本执行，且对重建表的 0003 / 0005 / 0008 / 0012 / 0020 在关闭外键的专用连接上跑（`FOREIGN_KEY_ISOLATED_MIGRATIONS`）。

### 前端要点

- `admin/`：单元 / 组件测试用 vitest + MSW（`src/test/`），浏览器 smoke 在 `e2e/`。保持纯客户端 SPA（`BrowserRouter`），不得启用 React Router 的 RSC / server action。
- `admin-app/`：Riverpod + Dio / CookieJar，Cookie 与 CSRF token 只进 Keychain / Keystore，不放 `--dart-define`。
- 本地联调的 Origin 白名单：`make api-run` 默认放行 `http://127.0.0.1:30101` 并关闭 cookie `Secure`；改 Web 端口必须同步 API 的 `DEPLOY_GO_ALLOWED_ORIGIN`，Flutter 的 `DEPLOY_GO_ALLOWED_ORIGIN` 必须是 API 白名单成员。

## 文档地图

权威顺序：用户当前指令 > `AGENTS.md` > `docs/standards/` > `docs/runbooks/` > `docs/solutions/` > `specs/<feature>/` 与 active 的 `docs/plans/` > 历史计划与总结（仅作背景），详见 `docs/standards/document-authority.md`。

- 命令、环境变量、本地联调：`docs/runbooks/local-development.md`；运行 / 部署 / 迁移 / 排障先查对应 runbook（索引 `docs/runbooks/README.md`）。
- 契约：`docs/standards/`（`api-contract`、`agent-control-protocol`、`privileged-agent-executor`、`application-deployment-contract`、`git-branch-deployment-contract`、`access-control`）。
- 复发问题与排查路径：`docs/solutions/`；进行中任务的续接：`specs/` 与 `docs/handoffs/`。
- **文档会漂移**：版本号、阈值、间隔这类数值类表述最容易过期（曾出现 standards / runbook 与代码不一致）。以代码为准——控制协议版本看 `agent-protocol/src/lib.rs`，executor 本机协议看 `agent-executor/src/protocol.rs`——并顺手修正文档。
- Spec Kit 阶段 Skill 在 `.agents/skills/speckit-*/SKILL.md`（面向 Codex 的入口，不在 `.claude/`）；Claude Code 下直接读取对应 `SKILL.md` 并按其流程执行，不要声称已通过原生入口发现。

## 有副作用或触达真实环境的命令

开发、测试、"继续"、提交都不构成真实环境授权（见 `AGENTS.md`「远程执行授权」）。以下目标需要当前对话对具体环境的明确授权，或会写到仓库之外：

- `make deploy-production`：经 SSH 在正式控制面上构建并安装（别名 `qfy-test2`，公网为 `qfy-test`）；先读 `deploy/production/README.md` 与 `docs/runbooks/systemd-deployment-production.md`。`make deploy-production-check` 只做本地契约检查。
- `make agent-release-sync`：历史手动同步脚本，GitHub Actions 已停用，部署不再使用。
- `make deployer-skill-install`：写入 `~/.codex/skills/deploy-go-deployer`。
- `make credential-reencrypt`：对 `DEPLOY_GO_DATABASE_URL` 指向的库离线重加密凭证。
