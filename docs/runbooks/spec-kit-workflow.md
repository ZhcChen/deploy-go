# Spec Kit 开发工作流

## 范围与前提

本项目默认使用 Spec Kit，原则摘要见 `.specify/memory/constitution.md`，权威顺序见 `docs/standards/document-authority.md`。工作流不替代测试、代码审查、migration 门禁或真实节点运行授权。

固定 Specify CLI `1.0.13`。本轮从参考项目提交 `1aa160f9662ae2a3ce706c4996531340953e37cb` 的已验证固定版本资产引入上游文件，保留原文、hash 和 manifest；manifest 的 `installed_at` 是源资产安装时间，不是 deploy-go 安装时间。引入时间与验收见 `docs/plans/2026-10-08-002-spec-kit-workflow-migration-plan.md`。项目防护脚本也来自该次整改，适配本项目命名与规则，并修正了代码示例参与正式结构检查的问题。

Node.js（沿用现有开发工具链）和 Bash 足以运行项目入口，不需要在线下载、启动服务或 Docker。`uv` 仅用于固定版本 CLI 管理命令：

```bash
uv tool run --from specify-cli==1.0.13 specify version
uv tool run --from specify-cli==1.0.13 specify integration status --json
```

资产已经纳入仓库，禁止重复 `specify init --here --force`。不得为迁移卸载全局 CE 或修改其他项目。

## 阶段与入口

Codex 入口在 `.agents/skills/`，MiniMax Code 在 `.minimax/skills/`，共用 `.specify/` 与 `specs/`。新会话刷新后核对可见 Skill；当前会话不可见时直接读取项目 `SKILL.md`，但不得声称已通过原生入口发现验收。

| 阶段 | Codex | MiniMax Code | 职责 |
| --- | --- | --- | --- |
| specify | `$speckit-specify` | `/speckit-specify` | WHAT/WHY、范围与可验证要求 |
| clarify（按需） | `$speckit-clarify` | `/speckit-clarify` | 解决真实需求歧义 |
| plan | `$speckit-plan` | `/speckit-plan` | HOW、文件范围、验证与恢复 |
| tasks | `$speckit-tasks` | `/speckit-tasks` | 依赖、稳定任务 ID 与验收单元 |
| analyze | `$speckit-analyze` | `/speckit-analyze` | 只读核对三份产物一致性 |
| implement | `$speckit-implement` | `/speckit-implement` | 小步实施、真实验证、更新进度 |
| converge | `$speckit-converge` | `/speckit-converge` | 核对遗漏，只追加必要任务 |

constitution 已建立，不为每个任务重写。checklist 按需用于需求质量，由复核者评估，不由 implement 静默勾选；有未通过项先处理实际缺口，只在缺少关键决策时询问。已授权完整实施时连续推进，不机械逐阶段审批。

小改动直接“定位/短说明 -> 修改 -> 聚焦验证 -> 提交推送”；Bug 先复现与定位，扩大范围后再转为规格流程。复杂任务使用三份核心产物，重大风险安排独立复核。analyze/converge 不能代替代码审查与测试。

上游 `.specify/workflows/speckit/workflow.yml` 仅保留为安装资产，包含额外审批且缺少项目要求的 analyze/converge；不直接执行 `specify workflow run speckit`。`taskstoissues` 不属于默认流程，创建外部工单需用户明确指令。

## 项目防护入口

在仓库根目录显式指定需求目录，不凭 Git 分支猜测需求，不持久化活跃指针：

```bash
make spec-kit-init FEATURE=specs/001-feature
make spec-kit-check FEATURE=specs/001-feature STAGE=plan
make spec-kit-check FEATURE=specs/001-feature STAGE=tasks
make spec-kit-check FEATURE=specs/001-feature STAGE=implement
make spec-kit-verify
```

| STAGE | 必需输入 |
| --- | --- |
| clarify | spec，允许待澄清草稿 |
| plan | 就绪 spec |
| tasks | 就绪 spec、plan |
| analyze / implement / converge | 就绪 spec、plan、tasks |

init 使用 `.specify/templates/overrides/` 的模板解析栈，只排他创建不存在的 spec。已有 spec 返回续写提示和必读文件，保留所有原产物；已有 plan/tasks 却没有 spec 时拒绝重新初始化，先核对历史来源。此入口替代上游 specify 的模板复制与 `feature.json` 写入步骤，不能再复制模板覆盖。

check 拒绝缺失、空白、仅标题/注释、已知模板残留、未决 `NEEDS CLARIFICATION`、缺少 FR 编号、缺少计划验证章节、重复任务 ID 和缺少验证任务。代码示例、引用及行首“已解决：”或“resolved:”的历史记录不当作当前未决事项。只接受当前仓库的 `specs/<名称>`，拒绝目录或核心产物符号链接。

检查通过只说明结构就绪；仍须读取输出 `REQUIRED_READS` 并核对语义、覆盖率与真实验证结果。检查不证明 FR 与任务完全对应、验证命令有效或任务全部完成，也不能阻止 agent 后续手工覆盖文档；这些要求通过规则、执行和 diff 审查落实。

已有 tasks 必须先读后增量维护，保留 ID、状态、证据和人工备注；新 ID 从最大编号递增。调整未完成任务须说明实质替换原因，已完成工作不重复安排。converge 严格追加，无发现不写文件。测试、研究、数据模型和 contracts 按需生成，不机械建立新基础设施、三条用户故事或 TDD。

每次直接调用上游脚本都独立传入环境变量，不能依赖项目入口子进程的 export：

```bash
SPECIFY_INIT_DIR="$PWD" SPECIFY_FEATURE_DIRECTORY=specs/001-feature SPECIFY_FEATURE_NO_PERSIST=1 \
  bash .specify/scripts/bash/check-prerequisites.sh --json --paths-only

SPECIFY_INIT_DIR="$PWD" SPECIFY_FEATURE_DIRECTORY=specs/001-feature SPECIFY_FEATURE_NO_PERSIST=1 \
  bash .specify/scripts/bash/check-prerequisites.sh --json --require-spec --require-tasks --include-tasks
```

上述兼容性命令要求已有目录，不能替代项目 check；`--paths-only` 不代表规格就绪。所有阶段禁止手工改写 `.specify/feature.json`；已有机器指针被忽略但不擅自删除。每个 agent 都显式选择目录，避免并行分配相同编号；并行写入须隔离 worktree 且文件无交集，不改变 main 默认开发策略。

## 历史任务与交付

保留 `docs/brainstorms/`、`docs/plans/`、`docs/reviews/`、`docs/solutions/` 历史资料。旧任务可原地继续，不强行跑要求新产物的前置脚本；其中 CE 名称按当前规则解释为需求、方案、实施、审查或经验沉淀步骤，不依赖 CE 插件。

转入 specs 时先核对代码、测试和提交中的真实完成状态，新 spec 引用旧计划并保留决定和验收，tasks 只承接剩余工作，旧文标明唯一新入口。此次迁移本身使用现有迁移 plan，不生成重复规格。

按风险使用本项目既有测试、构建和静态检查；本机 Docker 可用于隔离验收，命令见 `docs/runbooks/local-development.md`。报告区分通过、失败、跳过与未验证项。实施或工作流检查不隐含真实部署、重启、清理授权；按 `AGENTS.md` 和相关 runbook 判断会话已有授权，不重复请求已经给出的许可。

默认 main 小闭环提交推送，暂存明确相关文件，检查测试、`git diff --check`、`git diff --cached --check` 和 staged diff，然后提交、fetch/rebase/push；不强推、不跳过 hooks。此流程迁移无运行产物变化，不升级 Agent 或部署控制面。

## 集成维护、升级与恢复

默认集成为 Codex，Codex/MiniMax 已有项目级资产，无需重复 install。固定版本 CLI 可检查状态：

```bash
uv tool run --from specify-cli==1.0.13 specify integration list
uv tool run --from specify-cli==1.0.13 specify integration status --json
```

手动使用另一套 Skill 不需要切换默认集成；只有明确需要 CLI 的默认集成行为时才使用 `specify integration use mcode` 或 `use codex`。切换会刷新共享模板，完成后检查 diff、默认集成、执行位并运行 `make spec-kit-verify`；不改变 Git 分支。

升级作为独立任务，核对官方 release、固定新版本、检查工作区，再使用新版本 CLI 的 `integration upgrade`；不默认 `--force` 或 `--refresh-shared-infra`，不覆盖项目定制。同步版本、manifest、验证断言与手册；不得为绕过漂移随意重算 hashes。上游更新后重新核对 AGENTS 的替代步骤是否仍适用。

资产损坏时从对应受审查提交恢复或重新按固定版本核验，不整体覆盖脏工作区。迁移回滚可撤销本轮提交，只恢复开发工具入口，不影响运行服务。没有安装 Git、部署、工单或跨模型扩展，不卸载本机全局 CE 插件。

磁盘规则变更不会自动刷新旧会话注入的 CE 规则；新会话应核对 project-doc 和可见 Skill，存在差异时先定位来源，不擅自修改全局配置。静态验证不等于原生 Skill 的真实业务完整闭环，下一项已授权功能记录实际调用、续写、测试和交付证据。
