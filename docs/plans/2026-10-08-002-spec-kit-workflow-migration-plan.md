---
title: Spec Kit 工作流迁移
status: completed
date: 2026-10-08
---

# Spec Kit 工作流迁移

## 目标与边界

将 deploy-go 的默认 AI 开发流程从 CE 切换为 Spec Kit，参考 qfy-voucher-hub 已完成的项目入口与防护机制。仅修改本项目开发工具与文档；不修改业务应用、运行配置、历史 migration 或本机全局插件，不连接节点或部署。

保留 main 直接开发、小闭环提交推送、本机 Docker 验证、系统代理保护、远程执行授权、migration 门禁和禁止修改业务应用的硬规则。参考项目的 dev/test/production 晋级与正式发布人工专属规则不移植。

## 执行单元

- [x] U1：核对参考项目的固定版本资产、阶段入口与覆盖模板，盘点当前有效 CE 引用。
- [x] U2：引入固定 Specify CLI 1.0.13 的 Codex/MiniMax 项目级资产、显式需求目录入口与正反例测试；保持上游 hashes，不安装扩展。
- [x] U3：更新 AGENTS、constitution、README、文档权威与工作流 runbook；适配本项目范围和发布权限，历史计划保留续接。
- [x] U4：验证阶段输入、防覆盖、路径隔离、无指针或分支副作用、资产完整性与 CLI 集成状态；独立复核后修正发现。
- [x] U5：记录真实验证和未验证边界，审查最终 diff，只提交推送本轮文件。

## 验证与验收

- `make spec-kit-verify`：初始化及阶段检查正反例、上游文件 hashes、Bash 语法与执行位。
- `node --check scripts/ops/spec-kit.cjs`、`node --check scripts/test/spec-kit.test.cjs`。
- 固定 CLI 的 `version`、`integration status --json`；默认 Codex，两套项目级入口不依赖全局 CE。
- `make -n spec-kit-init FEATURE=specs/001-example` 与阶段命令解析；默认 Make 目标仍为 help。
- 当前 Git 分支、既有未跟踪计划及参考仓库内容保持原状；`git diff --check`、暂存检查通过。
- 新会话 Skill 发现与真实业务完整闭环不伪造验收，留待刷新会话后的实际需求验证。

## 旧计划与恢复

历史 CE 文档作为背景保留，原有未完成计划允许原地继续。转入 specs 时先核对代码与完成证据，只迁移剩余任务，并在旧文注明唯一新入口。此次迁移沿用本文，不重复创建规格。

无服务运行变化。回滚本轮提交即可恢复旧开发入口；不卸载全局 CE，不改变节点 Agent 或控制面版本。

## 验证证据

- 来源仓库 HEAD 为 `1aa160f9662ae2a3ce706c4996531340953e37cb`，参考仓库只读且工作区干净。
- `make spec-kit-verify`：13 组通过，包含拒绝模板/未决澄清/重复任务、已有产物保护、并发初始化、路径与符号链接隔离、阶段输入、无指针写入、代码示例不能冒充正式结构，以及真实 Git fixture 不创建或切换分支。
- 两个 CJS 文件语法检查通过；32 个受管理资产 hashes 匹配，6 个 Bash 脚本语法及执行位通过。
- 固定 CLI `version` 为 `1.0.13`；`integration status --json` 为 `ok`，默认 Codex，安装 Codex/MiniMax，missing/modified/invalid 均为 0。
- Make dry-run 的 init/check 参数正确，默认目标仍为 help；活跃指针忽略规则生效，当前分支保持 main，`git diff --check` 通过。
- 本轮未修改运行代码，未运行无关业务服务/全仓业务测试，未连接节点或部署。原生 Skill 在新会话中的发现和真实业务完整闭环未验证，不作为本轮静态验收结论。
- 独立复核发现直接脚本示例缺少项目根目录固定、结构校验读取未过滤原文两项 P2；已固定 `SPECIFY_INIT_DIR` 并统一使用过滤后的正文校验，补充反例后 13 组通过。
- 修正的独立聚焦复核通过，无新的相关发现；结论见 `docs/reviews/2026-10-08-spec-kit-workflow-migration-review.md`。

## 2026-10-28 项目级外部规格适配（续接）

本次按用户指定的多项目整改提示词续接工作流迁移；不改历史签收状态，不重做已完成任务。基线 main / 82dcd99，工作区干净，原有 13 组测试通过。受管理 Spec Kit 1.0.13 与 Claude 资产保持原文/hash。

实施范围：项目 wrapper、对应测试、`.specify/spec-kit.json`、项目覆盖模板、当前规则与 runbook，以及新的迁移登记。Obsidian 只写本项目 `QFY/项目/deploy-go`。现有 001–007 原文冻结保留；外部模式仍能读取仓库中的历史 feature，若与外部目录同名则拒绝，不静默选版本。006 业务现场未验收边界不变。现有 docs 中的计划、复核、手册为不同生命周期产物，本次不删除或复制为另一套正文。

项目配置仅记录 Vault 相对项目路径；未设置 Vault 环境变量仍回退仓库。显式覆盖为直接包含 feature 的 Spec Kit 根目录；项目模式扫描已核对的四个候选目录，新 feature 仅写“03-计划与任务”位置。根/中间目录/产物均拒绝符号链接；外部根必须已存在，不可用时明确失败。所有调用继续传稳定 `FEATURE=specs/<feature>`，向上游传实际绝对目录并禁止指针持久化。

本次执行正文已通过真实项目init建立为稳定标识 `specs/008-obsidian-spec-kit-storage`，位于本项目Obsidian计划目录；spec/plan/tasks由wrapper解析。本节只保留续接摘要，不再作为平行任务清单。逐项存量与权威登记见 `docs/standards/spec-kit-storage-register.md`。原有历史已完成记录不改写；新任务仅由用户签收。
