# 工作流与文档首轮优化实施方案

规格：[spec.md](spec.md)。状态：方案就绪；执行证据见 [tasks.md](tasks.md)。

## Technical Context

基线为 main / 3c4af4c，接手时工作区干净。现有文档已按 standards、runbooks 和历史资料分类；主要缺口是首页现状、流程分流、验证选择和状态解释之间的衔接。Makefile 已有 dev-remote-check，但全仓检查和 help 尚未覆盖它。

当前未配置外部规格根；本轮由项目 wrapper 初始化全新仓库 feature 010。001–009 的迁移登记和外部正文继续作为它们各自的唯一入口，不访问或复制 Vault。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md)、[文档权威规则](../../docs/standards/document-authority.md) 和 [Spec Kit 工作流](../../docs/runbooks/spec-kit-workflow.md)。单写入者在 main 小步实现，保留人工签收，按风险验证；没有真实部署或其他仓库写入。

## 实现方案与文件范围

1. README.md、docs/README.md：用当前能力、阅读路径和标准链接替代分散的早期计划入口；架构解释继续复用 development-guide，不另开重复说明。
2. AGENTS.md、docs/runbooks/spec-kit-workflow.md：统一短流程条件、完整流程触发条件和旧任务续接；补充三种场景示例与进度/签收边界。
3. docs/runbooks/development-guide.md、docs/runbooks/README.md、Makefile：在工程指南维护验证映射；补齐 dev-remote-check 的 help 和 check 依赖；其他入口引用同一映射。
4. specs/README.md、docs/plans/README.md、docs/standards/spec-kit-storage-register.md：登记 010 的独立仓库正文与来源；历史状态做兼容解释，当前任务链接实际正文，不复制旧 feature 任务。
5. .specify/templates/overrides/tasks-template.md：允许以按任务编号记录的证据区分实施、验证与签收；不改上游模板或 manifest。
6. deploy-go-deployer/README.md：按现有 CLI 枚举补齐能力分组，链接详细手册；scripts/test/docs-check.py 将规格和计划索引纳入必需入口。

## 阶段与依赖

先完成三份核心产物与 analyze，再按首页/导航、流程/验证、状态/门禁依次实施，最后运行验证和 converge。文档和 Make 依赖修改在同一闭环中保持一致，不改历史文件的状态或勾选。

## 验证与验收

- make docs-check：全部现有及新增 Markdown 的入口、相对文件链接有效；检查器不声明验证标题锚点和内容时效。
- make spec-kit-verify：覆盖模板与上游资产兼容，既有 21 项 fixture 回归通过。
- make dev-remote-check：本地 Compose 渲染及 mock 隔离回归通过，不创建容器或连接远端。
- make whitespace-check、git diff --check、git diff --cached --check，并审阅最终 staged diff。
- 检查 make check 依赖和 help，确认只增加既有隔离门禁；对三个接手场景做阅读路径与验证组合走查。
- 本轮没有 Rust、Web 或 Flutter 生产源码改动，不重复运行对应完整产品门禁；把未验证的运行行为与外部规格边界明确记入 tasks。

## 运行与恢复

无运行部署或数据变化。回退为本轮代码提交的独立 revert；不会改外部规格、历史签收或真实服务。提交后按仓库规则 fetch/rebase/push，发生他人文件冲突时停下报告。
