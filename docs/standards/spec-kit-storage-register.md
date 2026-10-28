# Spec Kit 存储与迁移登记

更新：2026-10-28；依据：用户指定的多项目整改提示词。本文为定位/生命周期登记，不复制规格正文。默认工具配置见 `.specify/spec-kit.json`，可执行规则见 `docs/runbooks/spec-kit-workflow.md`。

## 盘点与权威

基线 main / 82dcd99，工作区干净。Specify CLI与集成资产固定1.0.13；`.specify/integrations/{speckit,claude}.manifest.json` 共32个受管理文件，原文与hash保留。项目wrapper为 `scripts/ops/spec-kit.cjs`，Make init/check/verify调用它；无仓库CI，门禁由本机Make运行。`.specify/spec-kit.json` 为本轮新增，只有Vault相对项目字段。

存量7个feature共29份Markdown；与docs下150份Markdown按完整字节SHA256比对，无完全相同正文。历史docs中存在背景引用/实施证据，属于不同生命周期，不将它们视为规格副本删除。本轮只续接既有工作流迁移计划并指向新008执行正文。运行手册、审核证据与业务计划留在Git，未来迁移另行逐项登记。

Make保持稳定FEATURE标识，无需改参数。wrapper原先硬编码仓库目录，本轮已适配。测试原有fixture中的specs是故意验证默认兼容，不替换为全局Vault。上游模板/Claude Skill中的specs和自动指针步骤保留受管原文，由项目wrapper替代；不重算manifest掩盖修改。上游Skill的任务自动勾选服从当前项目人工签收规则。

项目Obsidian目录：`QFY/项目/deploy-go`（相对Vault根）；整改前不存在，无同名feature冲突。本轮只建“10-项目文档/03-计划与任务/Spec Kit”正文目录与项目索引，不批量创建候选树，不修改其他项目或临时跨项目提示词表。

## 逐项存储登记

| 稳定标识 | 正文与默认输入 | 生命周期/边界 | 外部输入及同步关系 |
| --- | --- | --- | --- |
| specs/001-prepare-artifact-transfer | 仓库同名目录 | 历史完成；原记录含业务测试验收 | 未迁移；配置Vault仍读仓库，无副本 |
| specs/002-bounded-log-retention | 仓库同名目录 | 历史完成；以原任务证据为准 | 未迁移；配置Vault仍读仓库，无副本 |
| specs/003-deployment-application-filter | 仓库同名目录 | 历史完成 | 未迁移；配置Vault仍读仓库，无副本 |
| specs/004-privileged-release-monitor-recovery | 仓库同名目录 | 本地任务完成，现场边界保留 | 未迁移；配置Vault仍读仓库，无副本 |
| specs/005-external-nonprod-management | 仓库同名目录 | 本地实施完成，原验收边界保留 | 未迁移；配置Vault仍读仓库，无副本 |
| specs/006-privileged-release-artifact-handoff | 仓库同名目录 | 本地/组件发布完成；业务现场未验收 | 未迁移，T011未关闭；不因整改签收 |
| specs/007-isolated-remote-development | 仓库同名目录 | 开发环境验收已记录；历史状态不重写 | 未迁移；配置Vault仍读仓库，无副本 |
| specs/008-obsidian-spec-kit-storage | 项目Obsidian计划目录/同名feature | 本轮唯一执行正文，待用户签收 | 显式Vault或单次Spec Kit根；无仓库正文副本，不自动双向同步 |

001–007的29个文件逐项指纹保存在 `.specify/spec-kit-storage-baseline.json`，源HEAD为82dcd99；冻结表示本轮不重写，不阻止后续经明确任务续写剩余工作。此清单是项目迁移登记，不是上游manifest，未来迁移须审查对应登记及指纹变化。

## 默认与外部模式

未提供外部根时继续仓库模式；仓库001–007可直接检查。008正文位于Obsidian，不配置时无法读取；按runbook配置后续接，不重新init同名仓库副本。启用外部模式后仓库/外部或多候选重名一律失败，不能把副本同时称为权威。默认与外部输入的切换不会复制、删除或覆盖正文；新feature仅在当前选定存储初始化。

Obsidian规格是该feature的人类需求/执行正文，但不能覆盖仓库AGENTS、constitution摘要上层规则、standards/runbook和代码/测试事实。Git提交仅包括配置、工具、测试、定位登记与冻结证据，不包含Vault正文；外部正文备份由Vault维护，不把仓库push等同于外部文档发布。
