# deploy-go

轻量级自动化部署服务，通过统一管理节点、应用和部署记录，帮助团队托管并执行应用自带的部署脚本。

本项目不负责定义通用构建流水线，也不替代应用自身的发布脚本。它提供控制面、执行入口、过程日志和结果追踪，让已有脚本能够被安全、可观察地重复执行。

## 核心概念

- **节点（Node）**：承载应用和执行部署脚本的服务器。
- **应用（Application）**：需要部署的服务及其部署配置。
- **脚本（Script）**：由应用或运维人员维护的部署入口。
- **部署（Deployment）**：一次针对指定应用与节点的脚本执行记录。

## 仓库模块

| 模块 | 技术方向 | 职责 |
| --- | --- | --- |
| `api/` | Rust | 提供 API、身份与权限、节点和应用管理、部署编排、执行记录与日志管理 |
| `admin/` | Web | 面向桌面浏览器的管理端 |
| `admin-app/` | Flutter | 面向移动设备的管理端，与 Web 端共享主要业务能力 |
| `ui/` | HTML + CSS + JavaScript | Web 与 App 的可交互 UI 设计源、设计规范和交付基线 |

各正式业务模块将在对应实施阶段创建。当前 `ui/` 已提供首版可交互设计源，用于收敛信息架构、视觉规范和关键操作流程。

## 首版范围

1. 管理节点及其连接、可用性和基础信息。
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
- 访问控制规范：`docs/standards/access-control.md`

## UI 预览

在仓库根目录启动 UI 设计源静态服务器：

```bash
make ui
```

访问 `http://127.0.0.1:8050/#/entry`。

该命令底层使用 Python 静态服务器。需要排查命令或临时覆盖端口时，可使用：

```bash
make help
make ui-check
make ui UI_PORT=8051
```
