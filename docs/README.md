# 文档入口与维护规则

本页提供当前规范、运行手册、执行任务和历史资料的阅读路径。协作与授权遵守 [AGENTS.md](../AGENTS.md)，权威顺序见[文档权威规则](standards/document-authority.md)。

## 按任务查找

| 任务 | 先读 | 接着读 |
| --- | --- | --- |
| 了解项目现状和边界 | [项目首页](../README.md) | [架构与恢复模型](runbooks/development-guide.md#架构) |
| 修复缺陷或开发新需求 | [AGENTS.md](../AGENTS.md)、[流程选择](runbooks/spec-kit-workflow.md#先选择合适的流程) | [规范目录](standards/README.md)、相关代码与测试 |
| 确定验证范围 | [验证映射](runbooks/development-guide.md#按改动范围选择验证) | 对应专题手册和 Make 目标 |
| 查找当前任务、签收和未验收边界 | [规格与任务入口](../specs/README.md) | wrapper 返回的 spec、plan、tasks；旧任务见[计划索引](plans/README.md) |
| 本机单测或轻量 API 调试 | [本地开发](runbooks/local-development.md) | [开发工程指南](runbooks/development-guide.md) |
| 完整 API/Web 联调与浏览器验收 | [独立远端开发环境](runbooks/remote-development.md) | 对应任务的验证与授权范围 |
| 接入业务应用 | [应用接入](runbooks/application-onboarding.md) | [两阶段部署契约](standards/application-deployment-contract.md)、[脚本契约](standards/deploy-script-contract.md) |
| 接入节点 Agent | [Agent 接入](runbooks/agent-onboarding.md) | [Agent 恢复](runbooks/agent-recovery.md) |
| 排查部署失败或恢复 | [部署恢复](runbooks/deployment-recovery.md) | [Agent 恢复](runbooks/agent-recovery.md)、[故障结论](solutions/) |
| 修改数据库结构 | [Migration 手册](runbooks/api-migrations.md) | AGENTS.md 的不可变规则与迁移测试 |
| 部署正式环境 | [正式部署手册](runbooks/systemd-deployment-production.md) | AGENTS.md 的正式环境授权规则 |
| 调用对外部署 API | [对外 API 手册](runbooks/external-deploy-api.md) | [CLI 能力与配置](../deploy-go-deployer/README.md) |

正式环境与独立开发 project 共用 qfy-test2。环境选择、隔离与操作授权以对应手册和当前对话为准。

## 文档职责

| 内容 | 唯一维护位置 | 使用方式 |
| --- | --- | --- |
| 协作、授权、提交边界 | [AGENTS.md](../AGENTS.md) | 每次工作先读 |
| 产品、协议、安全与数据契约 | [standards/](standards/README.md) | 按主题读取当前规范 |
| 工具链、验证映射、生成联动与架构解释 | [开发工程指南](runbooks/development-guide.md) | 按任务定位相关章节 |
| 运行、部署、迁移、排障与联调步骤 | [runbooks/](runbooks/README.md) | 操作前后核对前置条件、验证和恢复 |
| 已验证的故障原因与修复边界 | [solutions/](solutions/) | 从症状追溯结论；操作步骤仍以 runbook 为准 |
| 需求、方案、任务与验收证据 | [Spec Kit 索引](../specs/README.md)定位的实际正文 | 同一任务只保留一个执行入口 |
| 旧计划、探索、复核与交接 | [历史资料](#历史资料) | 追溯决定和证据，核对时效后使用 |

索引只提供入口。实施、验证、发布和人工签收应在对应任务中分别记录，首页和导航不复制一套进度。

## 历史资料

- [计划与执行记录](plans/README.md)：旧任务续接、历史状态解释和迁移规则。
- [产品探索](brainstorms/)：需求来源和早期取舍，不作为当前能力清单。
- [复核记录](reviews/)、[交接记录](handoffs/)：当时的代码、验证与现场事实，使用时核对日期、版本和后续变更。

保留历史正文和签收记录。高频引用的历史文档出现漂移时，补充当前首选入口；运行规则同步到 standard/runbook。

## 文档变更流程

1. 先按[流程选择](runbooks/spec-kit-workflow.md#先选择合适的流程)确认范围，再选择上表中的维护位置；优先续写现有文档。
2. 核对代码、测试和当前规范，记录实际漂移，避免把历史结论直接当作当前事实。
3. 同一改动更新入口、命令、前置条件、验证和恢复；导航引用使用标准 Markdown 相对链接，便于点击与检查。
4. 按[验证映射](runbooks/development-guide.md#按改动范围选择验证)执行检查，提交前说明通过、失败、跳过、未验证与遗留项。
5. 在任务中追加实施和验证证据；任务勾选只由用户签收。

新增计划、复核、交接和解决方案时，在开头记录日期、状态、来源和关联文档。生成的 OpenAPI、API client、manifest 和 schema 由代码或脚本维护。

`make docs-check` 检查必需入口和标准 Markdown 相对链接的文件目标；目前不校验标题锚点、行内代码路径或内容时效。语义、命令和运行行为仍需人工核对及对应验证。
