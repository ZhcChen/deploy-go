# deploy-go

轻量级自动化部署服务，通过统一管理节点、应用和部署记录，托管并执行应用自带的部署脚本，提供控制面、过程日志和结果追踪。

## 当前能力与边界

| 能力 | 当前实现 | 详细入口 |
| --- | --- | --- |
| 应用与来源 | 管理应用、目标、模块、参数和 Env；支持 Git 固定分支、固定工作区和内建镜像模板 | [应用接入](docs/runbooks/application-onboarding.md) |
| 部署闭环 | 预览固化输入；prepare/release 两阶段、自动或手动发布、Env 门禁、取消及失败重试 | [部署契约](docs/standards/application-deployment-contract.md) |
| 节点与 Agent | 安装、身份与能力检查、在线状态、遥测、成对发布及自动升级 | [Agent 接入](docs/runbooks/agent-onboarding.md) |
| 日志与恢复 | 实时日志、结构化进度、断线续传、任务持久化和重连对账 | [部署恢复](docs/runbooks/deployment-recovery.md) |
| 管理客户端 | Web 覆盖核心管理闭环；Flutter 提供安全会话、资源与用户管理、预览确认和日志；Agent 首装、重新绑定与撤销保留在 Web | [客户端契约](docs/standards/client-behavior-contract.md)、[App](admin-app/README.md) |
| 对外自动化 | OpenAPI、CLI 与 Agent Skill；支持非正式环境配置、来源、预览部署、运行探测和诊断 | [对外 API](docs/runbooks/external-deploy-api.md)、[CLI](deploy-go-deployer/README.md) |

Deploy Go 负责调度和发布物短期交接；业务应用维护自己的构建、迁移、切换、验证和回滚逻辑。项目不提供代码托管、通用 CI、可视化脚本编排或长期 artifact 仓库。新部署通过在线 Agent 执行结构化任务，release 使用 root executor；没有 SSH 或 launcher 自动回退。

上表描述仓库实现。具体功能的本地验证、组件发布、业务现场验收和人工签收分别以[任务入口与证据](specs/README.md)为准。

## 从这里开始

| 目标 | 阅读路径 |
| --- | --- |
| 理解项目和架构 | 本页 → [进程与信任边界、部署和恢复模型](docs/runbooks/development-guide.md#架构) |
| 修改代码或文档 | [AGENTS.md](AGENTS.md) → [选择短流程或完整流程](docs/runbooks/spec-kit-workflow.md#先选择合适的流程) |
| 确定需要跑什么检查 | [按改动范围选择验证](docs/runbooks/development-guide.md#按改动范围选择验证) |
| 续接任务或查剩余边界 | [Spec Kit 任务入口](specs/README.md)；旧任务见[计划与执行记录](docs/plans/README.md) |
| 接入应用或节点 | [应用接入](docs/runbooks/application-onboarding.md)、[Agent 接入](docs/runbooks/agent-onboarding.md) |
| 开发、部署或排障 | [文档导航](docs/README.md)、[运行手册目录](docs/runbooks/README.md) |

## 仓库形态与根目录入口

deploy-go 是多模块 monorepo。根目录 [Makefile](Makefile) 提供跨 Rust、Web、Flutter 和脚本的项目级入口；[Cargo.toml](Cargo.toml) 是 Rust 子系统的 virtual workspace 清单，统一 10 个 crates 的共享依赖、Cargo.lock、工具链和测试。Web 与 Flutter 使用各自工具链，按改动模块选择验证。

| 入口 | 职责 |
| --- | --- |
| [AGENTS.md](AGENTS.md) | 协作、授权、提交和验证边界 |
| [docs/README.md](docs/README.md) | 当前规范、运行手册与历史资料导航 |
| [开发工程指南](docs/runbooks/development-guide.md) | 工具链、验证映射、生成联动和架构 |
| [Spec Kit 工作流](docs/runbooks/spec-kit-workflow.md) | 流程选择、规格定位和任务续接 |

## 核心概念

- **节点（Node）**：承载应用和执行部署脚本的服务器。
- **Agent**：以低权限用户运行，通过 WSS 接收结构化任务并回传日志和结果。
- **应用（Application）**：需要部署的服务及其来源、目标、参数与 Env 配置。
- **脚本（Script）**：应用维护的固定 Make target 部署入口。
- **部署（Deployment）**：一次应用部署及其各目标的执行事实。
- **Git 来源（Source）**：仓库、凭证引用、构建 Agent 和固定分支；部署预览固化 commit。
- **两阶段部署（Two-stage）**：prepare 生成发布物，release 在目标节点消费经校验的发布物。

## 仓库模块

| 模块 | 技术方向 | 职责 |
| --- | --- | --- |
| `api/` | Rust | API、权限、资源管理、部署编排、执行记录与日志 |
| `agent/` | Rust | 节点协同、受限 runner、断线重连和任务恢复 |
| `agent-protocol/` | Rust | API 与 Agent 共用的版本化控制协议和 JSON Schema |
| `agent-executor/` | Rust | root executor，经本机 Unix Socket 执行管理员 PTY 和结构化 release job |
| `agent-updater/` | Rust | Agent 配对组件的事务化升级及中断恢复 |
| `release-authorization/`、`terminal-capability/` | Rust | API 签发、executor 离线验签的短期授权 |
| `runtime-log/` | Rust | 节点与控制面的有界诊断日志存储 |
| `container-template/` | Rust | 生成 redis / valkey / postgres / etcd 镜像模板发布物 |
| `deploy-go-deployer/` | Rust | 对外部署 API 的 CLI，配套 `skills/deploy-go-deployer/` |
| `admin/` | Web | React + Vite 管理端，Vitest 和 Playwright 验证 |
| `admin-app/` | Flutter | Riverpod + Dio 移动管理端 |
| `examples/`、`deploy/`、`scripts/` | Shell / Node / Python | 接入示例、部署工具和仓库级检查/生成脚本 |

## 开发入口

在仓库根目录运行 `make help` 查看命令。先按[工作流手册](docs/runbooks/spec-kit-workflow.md)选择流程，再按[验证映射](docs/runbooks/development-guide.md#按改动范围选择验证)执行检查。本机编辑、单测和轻量调试见[本地开发](docs/runbooks/local-development.md)；完整 API/Web 联调默认使用[独立远端开发栈](docs/runbooks/remote-development.md)。

Spec Kit 使用稳定标识 `FEATURE=specs/<feature>`，实际正文由项目 wrapper 定位；存储与访问方式见[规格索引](specs/README.md)。历史产品探索、实施计划和复核只用于追溯，统一从[文档导航](docs/README.md)查找。
