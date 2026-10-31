# 文档入口与维护规则

`docs/` 保存 deploy-go 的当前规范、可执行手册、需求执行资料和历史工程记录。本页只负责导航与维护规则，不新增业务约束；当前事实以 `AGENTS.md`、`docs/standards/`、`docs/runbooks/` 和代码/测试为准。

## 按任务查找

| 任务 | 先读 | 接着读 |
| --- | --- | --- |
| 了解项目边界和模块 | `README.md`、`docs/runbooks/development-guide.md` | 相关模块 README |
| 修改代码或文档 | `AGENTS.md` | `docs/standards/README.md`、对应规范或手册 |
| 开发新需求 | `docs/runbooks/spec-kit-workflow.md` | wrapper 返回的 spec、plan、tasks |
| 本机运行 API/Web | `docs/runbooks/local-development.md` | `docs/runbooks/development-guide.md` |
| 远端开发联调 | `docs/runbooks/remote-development.md` | `docs/reviews/2026-10-27-isolated-remote-development-review.md` |
| 接入业务应用 | `docs/runbooks/application-onboarding.md` | `docs/standards/application-deployment-contract.md`、`docs/standards/deploy-script-contract.md` |
| 接入节点 Agent | `docs/runbooks/agent-onboarding.md` | `docs/runbooks/agent-recovery.md` |
| 排查部署失败或恢复 | `docs/runbooks/deployment-recovery.md` | `docs/runbooks/agent-recovery.md`、相关 `docs/solutions/` |
| 修改数据库结构 | `docs/runbooks/api-migrations.md` | `AGENTS.md` 的 Migration 规则、迁移测试 |
| 部署正式环境 | `docs/runbooks/systemd-deployment-production.md` | `AGENTS.md` 的正式环境授权规则 |
| 调用对外部署 API | `docs/runbooks/external-deploy-api.md` | `deploy-go-deployer/README.md` |

正式环境运行在 `qfy-test2` 的 systemd 服务中；远端开发使用独立 Compose project。开发、验证和正式部署的边界见 [`remote-development.md`](runbooks/remote-development.md) 与 [`systemd-deployment-production.md`](runbooks/systemd-deployment-production.md)，不能把开发环境操作当作正式环境授权。

## 文档职责

| 目录或文件 | 当前职责 | 使用方式 |
| --- | --- | --- |
| `AGENTS.md` | 协作、授权、提交和验证边界 | 每次工作先读 |
| `docs/standards/` | 当前产品、协议、安全和数据契约 | 从 [`standards/README.md`](standards/README.md) 按主题查找 |
| `docs/runbooks/` | 可以照着执行的运行、部署、迁移、排障和联调步骤 | 操作前和操作后核对 |
| `docs/solutions/` | 已验证的故障原因、修复结论和边界 | 遇到相同症状时参考 |
| `specs/<feature>`（由 wrapper 定位） | 当前需求的 WHAT、HOW、任务和验收证据 | 新需求和中型以上改动使用 |
| `docs/plans/` | 既有计划和迁移记录；除非明确标记为 active，不承担当前规范 | 只用于背景和未完事项核对 |
| `docs/brainstorms/` | 产品探索和需求来源 | 生成或追溯需求时参考 |
| `docs/reviews/`、`docs/handoffs/` | 复核证据、交接事实和历史上下文 | 不能单独覆盖当前规范 |

权威优先级和历史隔离规则见 [`standards/document-authority.md`](standards/document-authority.md)。本页及其他索引页只提供入口，不改变该优先级。

## 文档变更流程

1. 先根据内容分类：当前约束写 `standards/`，可执行步骤写 `runbooks/`，故障结论写 `solutions/`，需求拆解走 Spec Kit；历史背景保留在 `plans/`、`reviews/`、`handoffs/` 或 `brainstorms/`。
2. 先核对代码、测试和现有规范，再修改文档；代码与旧文档冲突时记录漂移并以当前实现为准。
3. 同一改动同时更新入口链接、命令、前置条件、验证方式和失败恢复；不要只修改计划而不更新当前 runbook 或 standard。
4. 按影响范围验证：文档入口和相对链接运行 `make docs-check`，Spec Kit 结构运行 `make spec-kit-verify`，代码或契约改动运行对应 Make 目标，所有改动运行 `git diff --check`。
5. 在提交前说明已验证、未验证、跳过和遗留项；任务签收状态只由用户更新，代理只追加证据。

## 新文档约定

- 文件名使用仓库相对路径引用；运行命令、目录和环境变量必须能在当前代码中找到。
- 新增计划、复核、交接和解决方案时，在开头记录日期、状态、来源和关联文档；历史文档不要通过改写正文伪装成当前规范。
- 如果当前规范替代了旧文档，保留旧文档并在开头注明“历史背景”和新的首选入口。
- 生成的 OpenAPI、API client、manifest 和 schema 由代码或脚本生成，不把生成文件当作文档源头。
- 涉及真实节点、远程命令、迁移、正式部署或删除数据时，必须同时遵守 `AGENTS.md` 的授权边界和对应 runbook。
