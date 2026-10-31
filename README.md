# deploy-go

轻量级自动化部署服务，通过统一管理节点、应用和部署记录，帮助团队托管并执行应用自带的部署脚本。

本项目不负责定义通用构建流水线，也不替代应用自身的发布脚本。它提供控制面、执行入口、过程日志和结果追踪，让已有脚本能够被安全、可观察地重复执行。

## 仓库形态与根目录入口

deploy-go 是一个多模块 monorepo。Rust 是控制面、Agent、协议和执行器等核心子系统的实现语言，但项目整体还包含 Web、Flutter、文档、示例和部署工具。根目录的 `Cargo.toml` 是 Cargo 的 **virtual workspace 清单**：它只管理列出的 Rust crates，统一共享依赖版本、`Cargo.lock`、工具链和跨 crate 测试；它不代表整个项目只能用 Rust 开发，也不包含 `admin/`、`admin-app/` 等非 Rust 模块。

根目录按职责使用以下入口：

| 目标 | 入口 | 说明 |
| --- | --- | --- |
| 了解项目和当前边界 | `README.md` | 面向所有模块的项目总览与导航 |
| 遵守协作、授权和提交规则 | `AGENTS.md` | 本仓库最高优先级的项目指引 |
| 执行项目级构建、检查和联调 | `Makefile` | 跨 Rust、Web、Flutter 和脚本的统一入口 |
| 构建或测试 Rust 子系统 | `Cargo.toml` / `Cargo.lock` | 仅作用于 Cargo workspace 的 10 个 crates |
| 开发 Web 管理端 | `admin/` | React + Vite + Vitest + Playwright |
| 开发 Flutter 管理端 | `admin-app/` | Flutter + Riverpod + Dio |
| 阅读当前规范和运行手册 | `docs/` | standards、runbooks、plans、reviews、solutions |
| 运行部署或远端开发 | `deploy/`、`docs/runbooks/` | 必须先阅读对应 runbook 并遵守授权边界 |

因此，单独使用 `cargo` 只是在操作 Rust 子系统；要验证整个项目，应优先使用 `make api-check`、`make admin-check`、`make admin-app-check` 或 `make check`。新增或修改非 Rust 模块时，不要仅凭 `cargo test` 判断项目完成度。

## 核心概念

- **节点（Node）**：承载应用和执行部署脚本的服务器。
- **Agent**：以 `deploy-go-agent` 低权限用户运行的协同程序，通过 WSS 接收结构化任务并回传日志和结果。
- **应用（Application）**：需要部署的服务及其部署配置。
- **脚本（Script）**：由应用或运维人员维护的部署入口。
- **部署（Deployment）**：一次针对指定应用与节点的脚本执行记录。
- **Git 来源（Source）**：应用绑定的 Git 仓库、SSH 凭证、构建 Agent 与固定分支；部署预览固化确定 commit。
- **两阶段部署（Two-stage）**：`prepare -> release` 两条阶段任务，构建产物只经任务 staging 临时交接，平台不长期保留。

## 仓库模块

| 模块 | 技术方向 | 职责 |
| --- | --- | --- |
| `api/` | Rust | 提供 API、身份与权限、节点和应用管理、部署编排、执行记录与日志管理 |
| `agent/` | Rust | 节点协同程序、受限脚本 runner、断线重连与任务恢复 |
| `agent-protocol/` | Rust | API 与 Agent 共用的版本化控制协议和 JSON Schema |
| `agent-executor/` | Rust | 节点上的 root 特权执行器：管理员 PTY 终端与结构化特权 release job，只经本机 Unix Socket 接受 Agent 的版本化请求 |
| `agent-updater/` | Rust | Agent 自动升级的事务化执行器，支持中断后 `--resume` |
| `release-authorization/`、`terminal-capability/` | Rust | API 签发、executor 离线验签的短期 Ed25519 授权（特权 release 与 Agent 升级；终端 PTY capability） |
| `runtime-log/` | Rust | 节点与控制面的有界诊断日志存储 |
| `container-template/` | Rust | 内建 Docker Compose 应用模板（redis / valkey / postgres / etcd），生成镜像直连部署的 template artifact |
| `deploy-go-deployer/` | Rust | 调用对外部署 API 的 CLI，配套 `skills/deploy-go-deployer/` Skill |
| `admin/` | Web | 面向桌面浏览器的管理端 |
| `admin-app/` | Flutter | 面向移动设备的管理端，与 Web 端共享主要业务能力 |
| `examples/`、`deploy/`、`scripts/` | Shell / Node / Python | 业务应用接入 Demo 与模板、正式环境部署脚本、仓库级校验与生成脚本 |

Rust API 已完成首版部署内核，`admin/` Web 正式客户端已覆盖核心管理闭环，`admin-app/` 已完成安全会话、资源与用户管理、部署预览确认、实时日志和生命周期恢复。早期的 HTML 原型设计源 `ui/` 已删除，需要时可从 git 历史查看。

## 首版范围

1. 通过一次性脚本安装 Agent，管理节点在线状态、能力和基础信息。
2. 管理应用、部署目标和脚本入口。
3. 手动发起部署，查看实时状态、输出日志和最终结果。
4. 查询应用与节点维度的部署历史。
5. 在 Web 与 App 管理端完成核心管理和部署操作。

首版不建设代码托管、通用 CI 流水线、可视化脚本编排器或容器编排平台。

## 工作流文档

项目使用 Spec Kit 工作流，具体规则见 `AGENTS.md`，操作入口见 `docs/runbooks/spec-kit-workflow.md`。`.specify/` 保留官方工具结构，`FEATURE=specs/<feature>` 保持稳定标识；中型及以上新任务读取 wrapper 返回的 spec、plan、tasks，正文存储由 `.specify/spec-kit.json` 和显式 Vault 环境变量决定，未配置时仍使用仓库 `specs/`；旧计划可原地续接。当前规范优先级见 `docs/standards/document-authority.md`，运行、部署、迁移和排障步骤统一沉淀到 `docs/runbooks/`。

```bash
make spec-kit-init FEATURE=specs/001-feature
make spec-kit-check FEATURE=specs/001-feature STAGE=plan
make spec-kit-verify
```

- 当前001–009规格正文已迁入项目Obsidian，定位与来源见 `specs/README.md`、`docs/standards/spec-kit-storage-register.md`；读取需提供Vault根或单次项目根。
- 工作流原则：`.specify/memory/constitution.md`
- 项目指引：`AGENTS.md`（工作规则）；`docs/runbooks/development-guide.md`（工具链、常用命令与架构）
- 工作流迁移记录：`docs/plans/2026-10-08-002-spec-kit-workflow-migration-plan.md`

- 产品需求：`docs/brainstorms/2026-07-30-lightweight-deployment-service.md`
- 实施计划：`docs/plans/2026-07-30-bootstrap-and-ui-design.md`
- UI 设计计划：`docs/plans/2026-07-30-ui-design.md`
- UI 完整化计划：`docs/plans/2026-07-31-ui-completion.md`
- UI 设计复核：`docs/reviews/2026-07-30-ui-design.md`
- UI 完整化复核：`docs/reviews/2026-07-31-ui-completion.md`
- 部署脚本接入契约：`docs/standards/deploy-script-contract.md`
- 业务应用两阶段部署接入规范：`docs/standards/application-deployment-contract.md`
- 应用部署契约 JSON 使用说明：`docs/standards/application-deployment-json.md`
- Git 分支部署规范：`docs/standards/git-branch-deployment-contract.md`
- 分支部署接入 Demo：`examples/branch-deployment/README.md`
- 业务应用接入手册：`docs/runbooks/application-onboarding.md`
- 访问控制规范：`docs/standards/access-control.md`
- API 与部署内核计划：`docs/plans/2026-07-31-api-foundation-and-deployment-core.md`
- API 本地开发：`docs/runbooks/local-development.md`
- 部署恢复：`docs/runbooks/deployment-recovery.md`
- Agent 接入：`docs/runbooks/agent-onboarding.md`
- Agent 恢复：`docs/runbooks/agent-recovery.md`
- 正式环境 systemd 部署：`docs/runbooks/systemd-deployment-production.md`

## API 开发

API 使用 Rust、Axum、Tokio、SQLx 和 SQLite。新部署只通过在线 Agent 执行结构化任务，不使用 SSH fallback。服务配置和安全边界见 `docs/runbooks/local-development.md`。

常用命令：

```bash
make api-migrate
make api-run
make api-openapi-check
make api-check
make deploy-contract-demo-check
make privileged-launcher-check
make admin-check
make admin-test-e2e
make client-sensitive-check
make api-image
make check
```

版本化 OpenAPI 产物位于 `api/openapi/openapi.json`。修改路由或 schema 后运行 `make api-openapi`，提交生成产物并由 `make api-check` 检查漂移。

`deploy-contract-demo-check` 验证业务准备/发布 Make target、manifest、checksum 与篡改阻断；`privileged-launcher-check` 验证受控发布 launcher 的路径、参数与 sudoers 白名单；`client-sensitive-check` 扫描客户端产物中的私钥、token 与 lease 泄漏。
