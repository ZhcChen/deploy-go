# deploy-go

轻量级自动化部署服务，通过统一管理节点、应用和部署记录，帮助团队托管并执行应用自带的部署脚本。

本项目不负责定义通用构建流水线，也不替代应用自身的发布脚本。它提供控制面、执行入口、过程日志和结果追踪，让已有脚本能够被安全、可观察地重复执行。

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
| `admin/` | Web | 面向桌面浏览器的管理端 |
| `admin-app/` | Flutter | 面向移动设备的管理端，与 Web 端共享主要业务能力 |
| `ui/` | HTML + CSS + JavaScript | Web 与 App 的可交互 UI 设计源、设计规范和交付基线 |

Rust API 已完成首版部署内核，`admin/` Web 正式客户端已覆盖核心管理闭环，`admin-app/` 已完成安全会话、资源与用户管理、部署预览确认、实时日志和生命周期恢复。`ui/` 继续作为可交互设计源，用于收敛信息架构、视觉规范和关键操作流程。

## 首版范围

1. 通过一次性脚本安装 Agent，管理节点在线状态、能力和基础信息。
2. 管理应用、部署目标和脚本入口。
3. 手动发起部署，查看实时状态、输出日志和最终结果。
4. 查询应用与节点维度的部署历史。
5. 在 Web 与 App 管理端完成核心管理和部署操作。

首版不建设代码托管、通用 CI 流水线、可视化脚本编排器或容器编排平台。

## 工作流文档

项目使用 Compound Engineering（CE）工作流，具体规则见 `AGENTS.md`。当前规范优先级见 `docs/standards/document-authority.md`，运行、部署、迁移和排障步骤统一沉淀到 `docs/runbooks/`。

- 产品需求：`docs/brainstorms/2026-07-30-lightweight-deployment-service.md`
- 实施计划：`docs/plans/2026-07-30-bootstrap-and-ui-design.md`
- UI 设计计划：`docs/plans/2026-07-30-ui-design.md`
- UI 完整化计划：`docs/plans/2026-07-31-ui-completion.md`
- UI 设计准备：`ui/README.md`
- UI 页面地图：`ui/docs/page-map.md`
- UI 设计复核：`docs/reviews/2026-07-30-ui-design.md`
- UI 完整化复核：`docs/reviews/2026-07-31-ui-completion.md`
- 部署脚本接入契约：`docs/standards/deploy-script-contract.md`
- 业务应用两阶段部署接入规范：`docs/standards/application-deployment-contract.md`
- Git 分支部署规范：`docs/standards/git-branch-deployment-contract.md`
- 分支部署接入 Demo：`examples/branch-deployment/README.md`
- 业务应用接入手册：`docs/runbooks/application-onboarding.md`
- 访问控制规范：`docs/standards/access-control.md`
- API 与部署内核计划：`docs/plans/2026-07-31-api-foundation-and-deployment-core.md`
- API 本地开发：`docs/runbooks/local-development.md`
- 部署恢复：`docs/runbooks/deployment-recovery.md`
- Agent 接入：`docs/runbooks/agent-onboarding.md`
- Agent 恢复：`docs/runbooks/agent-recovery.md`
- GitHub Actions 构建与发布：`docs/runbooks/github-actions-release.md`
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

## UI 预览

在仓库根目录启动 UI 设计源静态服务器：

```bash
make ui
```

访问 `http://127.0.0.1:30102/#/entry`。

该命令底层使用 Python 静态服务器。需要排查命令或临时覆盖端口时，可使用：

```bash
make help
make ui-check
make ui UI_PORT=30103
```
