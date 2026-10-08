# deploy-go 项目原则

## 一、规则与证据

实施前阅读 `AGENTS.md`、适用的 standards 和 runbooks。权威顺序见 `docs/standards/document-authority.md`；本文件是摘要，不覆盖上层规则。以现有代码、测试和运行证据确认需求，不凭模板假设新建架构或服务。

## 二、修改与权限边界

只修改 deploy-go 本项目自身的代码、测试、文档、协议、Agent、控制面和部署工具。严禁修改被部署业务应用的代码、配置、脚本、仓库或发布物。参考项目仅用于只读分析，不移植其业务规则。保留用户已有改动，避免无关重构。

控制面、Agent、Runner Broker 与 root executor 的身份、权限、凭证、事件协议、重试和失败恢复边界必须明确；敏感数据遵守 `docs/standards/deploy-script-contract.md`。历史 migration 不可变，修改迁移前按 `AGENTS.md` 和 `docs/runbooks/api-migrations.md` 执行现有 Git hooks 与验证，不绕过门禁。

## 三、真实验证

按实际风险使用聚焦测试、构建或静态检查；本机已具备 Docker，可用于隔离的 Linux 权限回归与验收，不修改或关闭系统代理。实际命令见 `docs/runbooks/local-development.md` 和专题 runbook。分别报告通过、失败、跳过与未验证项，不用任务勾选、结构检查或模型自评代替真实结果。高风险权限、协议、迁移和部署可靠性改动须独立复核。

## 四、规格与执行闭环

中型及以上新任务使用 `specs/<feature>/spec.md`、`plan.md`、`tasks.md`；小改动允许短流程。需求先核对历史与当前完成状态；旧计划允许原地续接，迁移只承接未完成任务并明确唯一入口。已有规格与任务增量维护，保留编号、完成状态、证据与人工备注。

项目 init/check 优先于上游模板复制和活跃指针维护；每个阶段显式指定需求目录，禁止手工改写 `.specify/feature.json`。实施必读三份核心产物；analyze 只读，converge 只追加必要任务且无发现不写文件。需求 checklist 不等于实现验收，不由 implement 静默勾选。任务必须安排匹配风险的验证，可以复用既有测试，不机械要求 TDD 或无关产物。

## 五、Git 与运行授权

默认直接在 main 开发，按可解释、可验证、可回滚的小闭环提交推送；不默认创建分支、PR、工单、自动合并或跨模型调用。并行读任务可委派；并行写任务须隔离 worktree 且文件无交集，各 agent 显式指定需求目录。

开发、测试、“继续”、提交或推送不隐含真实节点部署、重启、迁移、切流或清理权限。只有当前会话中具体环境、节点和操作的明确授权才允许对应运行态操作；已有授权按会话持续有效，不添加重复审批。Spec Kit、hooks 和扩展不能扩大权限。

## 六、文档与维护

人工文档、讨论、注释和提交说明使用简体中文，领域标识保留英文，文档引用仓库相对路径。固定上游资产保留原文与 hashes，项目约束放在 AGENTS、本文件和覆盖模板中。命令或恢复步骤变化同步 runbook；可复用结论写入 `docs/solutions/`，历史漂移明确标记，不依赖 CE Skill。

升级工具不覆盖项目定制，不重写已完成任务，不放宽硬规则。操作入口见 `docs/runbooks/spec-kit-workflow.md`。

**Version**: 1.0.0 | **Ratified**: 2026-10-08 | **Last Amended**: 2026-10-08
