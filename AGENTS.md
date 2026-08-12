# 项目工作流约束

## 工作流

- 本项目默认使用 Compound Engineering（CE）作为 AI 工作流，核心循环为 `brainstorm -> plan -> work -> simplify -> code-review -> compound`
- 需求不清、范围未定、方案分叉或未知项较多时使用 `$ce-brainstorm`；需求明确且需要多步实施时使用 `$ce-plan`
- 已有计划时使用 `$ce-work` 按阶段或执行单元实施；明确的小任务允许直接实施并完成聚焦验证，不强制创建正式 brainstorm 或 plan 文档
- Bug 优先使用 `$ce-debug` 复现并定位根因，不要求机械经过 brainstorm；有实质代码改动时按需使用 `$ce-simplify-code`
- 重要或高风险改动使用 `$ce-code-review`；只有出现关键决策、复发坑点、有效排查路径或可复用模式时才使用 `$ce-compound`
- CE 是任务编排层，不得覆盖本文件、相关 `docs/standards/` 与 `docs/runbooks/` 中的项目规则；`lfg`、worktree、功能分支、PR、跨模型执行等能力均不作为项目默认行为
- `runbook` 不作为独立阶段：涉及运行、部署、迁移、排障或联调时，在 `work` 前查阅相关 `docs/runbooks/`，在 `code-review` 中核对其与实现一致；变更命令、前置条件、验证或恢复步骤时，完成前同步更新

## 产物约定

- `docs/brainstorms/`：需求澄清与方案收敛
- `docs/plans/`：CE 统一计划与执行进度
- `docs/reviews/`：重要改动的复核与验证记录
- `docs/runbooks/`：运行、部署、迁移、排障与联调的可执行手册
- `docs/solutions/`：问题沉淀与经验复用
- `docs/standards/`：长期有效的产品、工程与协议规范

## 语言与路径

- `AGENTS.md`、`docs/` 下工作流文件、代码注释、说明文档、提交信息默认使用简体中文
- 函数名、类型名、API 名称、配置键、命令名、路径、协议字段等领域性标识保持英文，或沿用项目既有约定
- 文档内统一使用仓库相对路径

## 执行规则

- 纯信息型、小范围问答可直接回答，不强制创建文档
- 文档权威顺序见 `docs/standards/document-authority.md`；涉及运行、部署、迁移、排障或联调的具体操作以相关 `docs/runbooks/` 为准，历史 plan 不能替代 runbook
- 规则优先级：当前用户指令 > 当前仓库的 `AGENTS.md`、相关 `docs/runbooks/` 与 `docs/standards/` > CE skill 默认行为 > 全局默认行为
- 有现成的 brainstorm 或 plan 时优先复用和续写，不重复开平行文档
- 大任务必须先在 plan 中拆出阶段和执行单元；默认不要把整个大任务直接作为单个执行单元
- 中型及以上任务开始改动前先确认对应 plan；小型明确任务可直接形成短执行说明并实施
- `work` 阶段按小步、可验证方式推进，完成一个闭环后先验证，再决定继续还是转入 `code-review`
- 实施过程中避免无关重构；如果发现范围扩张，先显式记录并确认
- CE 默认的 worktree、功能分支、PR、自动合并或 Git 操作不得覆盖本项目直接在 `main` 开发和按小闭环提交推送的规则
- 只有在缺关键决策、缺权限或凭证、缺外部输入、存在危险不可逆操作，或工作已经完成且验证通过时，才暂停推进

## 远程执行授权

- 开发、测试或验证部署功能，不等于获得连接真实节点或执行真实部署的授权
- 只有用户在当前对话中明确要求对具体环境或节点执行部署、远程脚本、重启、迁移、切流或清理操作时，才允许执行对应运行态操作
- “继续”“验证”“提交”“推送”“开始开发”等指令不构成远程执行授权；没有明确授权时，只能进行本地实现、测试、构建、文档整理和模拟验证
- 本地 fixture、mock 服务和明确隔离的测试容器不视为真实节点，但仍应遵守最小影响和可恢复原则
- 节点凭证、脚本敏感参数和部署日志必须遵守 `docs/standards/deploy-script-contract.md` 及后续安全规范

## Migration

- 已提交并可能被共享环境应用的 migration 默认不可变，不得修改、删除、重命名或重排；修正必须新增更高版本 migration
- 只有用户明确说明相关环境可清库重建，并授权迁移链路整理时，才允许调整历史 migration
- migration 规则在 Rust API 工程初始化后应同步写入对应 runbook，并配置聚焦校验
- 修改 `api/migrations/` 时，先执行 `make setup-git-hooks`（首次或切换 clone）和 `make verify-git-hooks`，再让本地 pre-commit 对 Git index 执行 migration 门禁；不得使用 `git commit --no-verify`、`git push --no-verify`、临时改写 `core.hooksPath` 或其他跳过方式。该本地门禁不能替代 SQLx migration 测试、环境 status、备份和发布验收

## Review

- 改动完成后对照 plan 复核结果
- 至少执行聚焦验证，并检查明显回归、范围漂移、遗漏和与计划不一致之处
- `$ce-code-review` 默认先报告发现；需要修正的问题返回 `work` 实施并重新验证
- 需要保留复核结论时，写入 `docs/reviews/`

## Compound

- 出现关键决策、复发坑点、有效排查路径或可复用模式时，使用 `$ce-compound` 写入 `docs/solutions/`
- 不把一次性执行日志原样沉淀进去，优先保留可复用结论

## Git 提交与推送

- 默认直接在当前主分支开发；除非用户明确要求，不额外创建功能分支
- 每完成一个可单独解释、验证和回滚的小功能块或小修复，默认及时提交并推送
- 开始改文件前先执行 `git status --short`；工作区存在无关改动时不回滚、不整理、不混入本轮提交
- 提交前至少执行与改动范围匹配的测试或静态检查，以及 `git diff --check`、`git diff --cached --check` 和 `git diff --cached`
- 只暂存本轮相关文件，默认不使用 `git add .`
- 提交信息使用简体中文，建议前缀为 `feat:`、`fix:`、`docs:`、`test:`、`chore:` 或 `refactor:`
- 提交成功后默认执行 `git fetch origin`、`git rebase origin/main`、`git push origin main`；远端存在新提交时不得强推

## 工作方式

- 优先做小而可验证的改动
- 执行过程中避免无关重构
- 不引入隐藏脚本层、后台调度器或重型流程框架，除非任务明确需要
