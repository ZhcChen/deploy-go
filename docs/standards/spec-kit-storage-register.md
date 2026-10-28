# Spec Kit 存储与迁移登记

更新：2026-10-28（存量迁移完成）；依据：用户指定的多项目整改提示词。本文为定位/生命周期登记，不复制规格正文。默认工具配置见 `.specify/spec-kit.json`，可执行规则见 `docs/runbooks/spec-kit-workflow.md`。

## 盘点与权威

基线 main / 82dcd99，工作区干净。Specify CLI与集成资产固定1.0.13；核心 `.specify/integrations/speckit.manifest.json` 保留原文与hash；早期的专属集成已按后续用户请求移除。项目wrapper为 `scripts/ops/spec-kit.cjs`，Make init/check/verify调用它；无仓库CI，门禁由本机Make运行。`.specify/spec-kit.json` 为本轮新增，只有Vault相对项目字段。

存量7个feature共29份Markdown；与docs下150份Markdown按完整字节SHA256比对，无完全相同正文。历史docs中存在背景引用/实施证据，属于不同生命周期，不将它们视为规格副本删除。首轮续接既有工作流迁移计划并指向008执行正文；用户随后明确要求把specs也整理进Obsidian，本次在同一008中增量执行。运行手册、审核证据与业务计划留在Git，未来迁移另行逐项登记。

Make保持稳定FEATURE标识，无需改参数。wrapper原先硬编码仓库目录，本轮已适配。测试原有fixture中的specs是故意验证默认兼容，不替换为全局Vault。核心上游模板中的默认specs路径保留原文，实际定位由项目wrapper负责；不重算manifest掩盖修改。项目不安装专属阶段Skill，任务签收按AGENTS执行。

项目Obsidian目录：`QFY/项目/deploy-go`（相对Vault根）；整改前不存在，无同名feature冲突。本次将存量全部迁入已建立的“10-项目文档/03-计划与任务/Spec Kit”正文目录并更新索引，不批量创建候选树，不修改其他项目或临时跨项目提示词表。

## 逐项存储登记

| 稳定标识 | 当前唯一正文 | 生命周期/边界 | 外部输入及同步关系 |
| --- | --- | --- | --- |
| specs/001-prepare-artifact-transfer | 项目Obsidian计划目录/同名feature | 历史完成；原记录含业务测试验收 | 已迁移；显式Vault或单次项目根读取，仓库无正文副本 |
| specs/002-bounded-log-retention | 项目Obsidian计划目录/同名feature | 历史完成；以原任务证据为准 | 已迁移；显式Vault或单次项目根读取，仓库无正文副本 |
| specs/003-deployment-application-filter | 项目Obsidian计划目录/同名feature | 历史完成 | 已迁移；显式Vault或单次项目根读取，仓库无正文副本 |
| specs/004-privileged-release-monitor-recovery | 项目Obsidian计划目录/同名feature | 本地任务完成，现场边界保留 | 已迁移；显式Vault或单次项目根读取，仓库无正文副本 |
| specs/005-external-nonprod-management | 项目Obsidian计划目录/同名feature | 本地实施完成，原验收边界保留 | 已迁移；显式Vault或单次项目根读取，仓库无正文副本 |
| specs/006-privileged-release-artifact-handoff | 项目Obsidian计划目录/同名feature | 本地/组件发布完成；业务现场未验收 | 已迁移；T011未关闭，不因整理签收 |
| specs/007-isolated-remote-development | 项目Obsidian计划目录/同名feature | 开发环境验收已记录；历史状态不重写 | 已迁移；显式Vault或单次项目根读取，仓库无正文副本 |
| specs/008-obsidian-spec-kit-storage | 项目Obsidian计划目录/同名feature | 本轮执行正文，待用户签收 | 显式Vault或单次项目根；无仓库正文副本，不自动双向同步 |
| specs/009-agent-guidance-cleanup | 项目Obsidian计划目录/同名feature | 通用指引与专属资产清理，待用户签收 | 项目Vault正文，无仓库副本 |

001–007的29个文件逐项指纹保存在 `.specify/spec-kit-storage-baseline.json`，最初捕获源HEAD为82dcd99，迁移源HEAD为8307bbd（两者正文一致）；每项记录Vault相对目标目录，旧sha256键保留原仓库来源路径。迁移先排他复制、逐字节与SHA256全校验后才移除仓库正文，没有修改原文或勾选状态。本清单是项目迁移登记，不是上游manifest；后续经明确任务续写会自然改变正文，不能为匹配快照回滚用户改动。

## 当前默认与外部输入

项目级 `.specify/spec-kit.json` 保存 `obsidian_project_path=QFY/项目/deploy-go`；`QFY_OBSIDIAN_VAULT_ROOT`只提供Vault根。参考qfy-trade-in-mall，单次 `QFY_SPEC_KIT_OBSIDIAN_ROOT` 现在指向Obsidian项目根，同样扫描四类候选；不再使用首轮“直接包含feature的Spec Kit根”的语义。

001–009均在 `10-项目文档/03-计划与任务/Spec Kit/<feature>`，当前默认输入为经项目配置解析的Obsidian正文；仓库仅保留 `specs/README.md` 定位说明和Git历史。没有外部配置时仍支持全新feature的仓库模式，已迁移feature的check会明确缺少产物，不得重新init同名副本。外部模式发现仓库同名目录或多个候选均失败；不自动复制、删除或双向同步正文。

Obsidian规格是该feature的人类需求/执行正文，但不能覆盖仓库AGENTS、standards/runbook和代码/测试事实。Git提交仅包括工具、测试、定位登记、迁移指纹与旧正文的删除，不包含Vault正文；备份由Vault维护，原001–007也可从迁移源Git提交恢复，不把仓库push等同于外部文档发布。恢复仓库存储前必须先协调外部同名版本，避免双重权威。

后续指引清理只更新失效引用与工具称谓，没有更改任务签收/验收结论；原迁移指纹仍代表源Git快照，不要求后续编辑的正文保持旧摘要。
