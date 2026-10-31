# Spec Kit 开发工作流

## 范围与前提

本项目默认使用 Spec Kit，按任务选择文档和工作流的入口见 `docs/README.md`，原则摘要见 `.specify/memory/constitution.md`，权威顺序见 `docs/standards/document-authority.md`。工作流不替代测试、代码审查、migration 门禁或真实节点运行授权。

固定 Specify CLI `1.0.13`。当前规格正文001–009已在Obsidian，入口与配置见本文“项目级 Obsidian 存储”，仓库 `specs/` 保留默认模式及定位说明。本轮从参考项目提交 `1aa160f9662ae2a3ce706c4996531340953e37cb` 的已验证固定版本资产引入上游文件，保留原文、hash 和 manifest；manifest 的 `installed_at` 是源资产安装时间，不是 deploy-go 安装时间。引入时间与验收见 `docs/plans/2026-10-08-002-spec-kit-workflow-migration-plan.md`。项目防护脚本也来自该次整改，适配本项目命名与规则，并修正了代码示例参与正式结构检查的问题。

Node.js（沿用现有开发工具链）和 Bash 足以运行项目入口，不需要在线下载、启动服务或 Docker。`uv` 仅用于固定版本 CLI 管理命令：

```bash
uv tool run --from specify-cli==1.0.13 specify version
```

资产已经纳入仓库，禁止重复 `specify init --here --force`。不得为迁移卸载全局 CE 或修改其他项目。

## 阶段与入口

本项目使用工具无关的Make/wrapper入口，不安装专属AI集成或依赖原生命令发现。每个阶段先指定稳定feature标识，读取返回的产物再执行阶段工作：

| 阶段 | 入口与动作 | 职责 |
| --- | --- | --- |
| specify | `make spec-kit-init FEATURE=specs/<feature>` | 排他创建或续接WHAT/WHY草稿 |
| clarify（按需） | `make spec-kit-check FEATURE=… STAGE=clarify` | 读取草稿，解决真实需求歧义 |
| plan | `make spec-kit-check FEATURE=… STAGE=plan` | 读取就绪spec后增量编写HOW |
| tasks | `make spec-kit-check FEATURE=… STAGE=tasks` | 读取spec/plan后记录依赖与验收 |
| analyze | `make spec-kit-check FEATURE=… STAGE=analyze` | 只读核对三份产物一致性 |
| implement | `make spec-kit-check FEATURE=… STAGE=implement` | 小步实施、真实验证、追加进展 |
| converge | `make spec-kit-check FEATURE=… STAGE=converge` | 核对遗漏，只追加必要任务 |

wrapper只负责定位、结构校验和spec草稿初始化，不会替人生成plan/tasks、决定需求、执行测试或签收任务。

constitution 已建立，不为每个任务重写。checklist 按需用于需求质量，由复核者评估，不由 implement 静默勾选；有未通过项先处理实际缺口，只在缺少关键决策时询问。已授权完整实施时连续推进，不机械逐阶段审批。

小改动直接“定位/短说明 -> 修改 -> 聚焦验证 -> 提交推送”；Bug 先复现与定位，扩大范围后再转为规格流程。复杂任务使用三份核心产物，重大风险安排独立复核。analyze/converge 不能代替代码审查与测试。

项目不启用自动workflow，工作流注册表为空；不使用 `specify workflow run` 或自动创建外部工单。

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

check 拒绝缺失、空白、仅标题/注释、已知模板残留、未决 `NEEDS CLARIFICATION`、缺少 FR 编号、缺少计划验证章节、重复任务 ID 和缺少验证任务。代码示例、引用及行首“已解决：”或“resolved:”的历史记录不当作当前未决事项。CLI 只接受稳定标识 `specs/<名称>`，实际目录按下述项目存储配置解析；拒绝根目录、中间目录或核心产物符号链接。

检查通过只说明结构就绪；仍须读取输出 `REQUIRED_READS` 并核对语义、覆盖率与真实验证结果。检查不证明 FR 与任务完全对应、验证命令有效或任务全部完成，也不能阻止 agent 后续手工覆盖文档；这些要求通过规则、执行和 diff 审查落实。

任务 `[x]` 只由用户签收，代理只追加实现、测试和验收证据，保留历史状态。已有 tasks 必须先读后增量维护，保留 ID、状态、证据和人工备注；新 ID 从最大编号递增。调整未完成任务须说明实质替换原因，已完成工作不重复安排。converge 严格追加，无发现不写文件。测试、研究、数据模型和 contracts 按需生成，不机械建立新基础设施、三条用户故事或 TDD。

每次直接调用上游脚本都独立传入环境变量，不能依赖项目入口子进程的 export。先从项目 check 的 JSON 取得实际 `FEATURE_DIR`，以绝对目录传 `SPECIFY_FEATURE_DIRECTORY`，不能再把稳定标识当作仓库物理路径：

```bash
# 当前环境须已提供 Vault 根或单次项目根；JSON 中只读取目录。
feature_dir="$(node scripts/ops/spec-kit.cjs check --feature specs/001-prepare-artifact-transfer --stage implement | python3 -c 'import json,sys; print(json.load(sys.stdin)["FEATURE_DIR"])')" || exit 1
SPECIFY_INIT_DIR="$PWD" SPECIFY_FEATURE_DIRECTORY="$feature_dir" SPECIFY_FEATURE_NO_PERSIST=1 \
  bash .specify/scripts/bash/check-prerequisites.sh --json --require-spec --require-tasks --include-tasks
```

上述兼容性命令要求已有目录，不能替代项目 check；`--paths-only` 不代表规格就绪。所有阶段禁止手工改写 `.specify/feature.json`；已有机器指针被忽略但不擅自删除。每个 agent 都显式选择目录，避免并行分配相同编号；并行写入须隔离 worktree 且文件无交集，不改变 main 默认开发策略。

## 历史任务与交付

保留 `docs/brainstorms/`、`docs/plans/`、`docs/reviews/`、`docs/solutions/` 历史资料。旧任务可原地继续，不强行跑要求新产物的前置脚本；其中 CE 名称按当前规则解释为需求、方案、实施、审查或经验沉淀步骤，不依赖 CE 插件。

转入 specs 时先核对代码、测试和提交中的真实完成状态，新 spec 引用旧计划并保留决定和验收，tasks 只承接剩余工作，旧文标明唯一新入口。此次迁移本身使用现有迁移 plan，不生成重复规格。

按风险使用本项目既有测试、构建和静态检查；本机 Docker 可用于隔离验收，命令见 `docs/runbooks/local-development.md`。报告区分通过、失败、跳过与未验证项。实施或工作流检查不隐含真实部署、重启、清理授权；按 `AGENTS.md` 和相关 runbook 判断会话已有授权，不重复请求已经给出的许可。

默认 main 小闭环提交推送，暂存明确相关文件，检查测试、`git diff --check`、`git diff --cached --check` 和 staged diff，然后提交、fetch/rebase/push；不强推、不跳过 hooks。此流程迁移无运行产物变化，不升级 Agent 或部署控制面。

## 集成维护、升级与恢复

项目使用Spec Kit核心脚本与覆盖模板，不绑定任何AI集成：`.specify/integration.json` 的安装列表和设置为空，init-options仅保留通用参数。核心 `.specify/integrations/speckit.manifest.json` 继续校验原始hash；专属集成已按用户要求移除，不为让测试通过重算核心hash。

升级作为独立任务，核对固定版本、共享核心资产和项目覆盖规则，完成后运行 `make spec-kit-verify` 并审查diff。不要执行可能重新生成专属资产的integration安装或强制初始化，不默认 `--force`、`--refresh-shared-infra`，不改变当前分支。历史安装记录中的工具选择仅是当时事实，当前以本节和 `AGENTS.md` 为准。

资产损坏时从对应受审查提交恢复或重新按固定版本核验，不整体覆盖脏工作区。迁移回滚可撤销本轮提交，只恢复开发工具入口，不影响运行服务。没有安装 Git、部署、工单或跨模型扩展，不卸载本机全局 CE 插件。

磁盘规则变更不会自动刷新旧会话注入；新会话核对AGENTS和runbook，存在差异先定位来源，不擅自修改全局配置。结构验证不能替代真实业务验证。


## 项目级 Obsidian 存储（2026-10-28）

`.specify/spec-kit.json` 只保存项目级 `obsidian_project_path`，当前为 `QFY/项目/deploy-go`，不存本机 Vault 绝对路径。官方 `.specify/`、`specs/` 仍保留，不重装、不修改上游资产与 manifest。使用项目wrapper解析实际目录；读取返回的 `FEATURE_DIR`、`STORAGE`、`REQUIRED_READS` 后再处理正文，不扫描物理specs目录、不复制模板覆盖正文或持久化feature.json。人工签收规则见AGENTS。

解析优先级：

1. 非空 `QFY_SPEC_KIT_OBSIDIAN_ROOT`：本次显式覆盖，必须是本次 Obsidian 项目的既有绝对根目录（与 qfy-trade-in-mall 一致），其下按相同四类候选目录查找；不会作为其他项目的默认配置。
2. 否则，项目配置 + `QFY_OBSIDIAN_VAULT_ROOT`：Vault 与项目目录必须已存在；依次扫描项目内以下候选位置。新 feature 只写第一处，按需创建其子目录，不建立四套空目录。
3. 未配置外部根时使用仓库 `specs/<feature>`，项目配置本身不触发外部写入。已明确配置的根缺失、非目录或不安全时失败，不静默回退。

候选目录：`10-项目文档/03-计划与任务/Spec Kit`、`10-项目文档/01-路线图/Spec Kit`、`10-项目文档/04-规范与运行手册/Spec Kit`、`90-历史归档/Spec Kit`。只有本项目经核对的存储树适用此约定，不套用其他项目路径。

```bash
# Vault 根目录由操作者环境提供，不能把全局变量指向某一项目。
QFY_OBSIDIAN_VAULT_ROOT="<Vault绝对路径>" make spec-kit-check FEATURE=specs/008-obsidian-spec-kit-storage STAGE=implement
# 单次覆盖指向 Obsidian 项目根，不是 Vault 根或直接包含 feature 的 Spec Kit 目录。
QFY_SPEC_KIT_OBSIDIAN_ROOT="<本次Obsidian项目绝对目录>" make spec-kit-check FEATURE=specs/008-obsidian-spec-kit-storage STAGE=implement
```

两种存储都保持 CLI `FEATURE=specs/<feature>`。`STORAGE` 为 `repository` 或 `obsidian`。外部模式先核对所有候选；多个候选重名、或存在同名仓库目录均失败，不静默续读仓库旧版本；新建写外部第一候选。init 续接不补写已有 plan/tasks、不改任务签收；草稿就绪仍由后续阶段检查。

Vault、项目、Spec Kit 中间目录、feature 和核心产物均拒绝符号链接（含悬空链接），项目相对路径拒绝绝对路径、上级跳转及空分段。check 不创建目录；init 使用排他创建，不覆盖已有产物，失败保留已创建目录便于核查，不删除他人文件。初始化并发写入受排他创建保护；不承诺防御有权限的其他进程在检查与写入之间主动替换祖先目录，实际 Vault 单写入规则仍须遵守。

迁移登记见 `docs/standards/spec-kit-storage-register.md`。001–007 的29份原文已逐项校验后迁入本项目 Obsidian，与008/009统一存放；仓库 `specs/README.md` 仅保留定位说明，原文可从Git历史恢复（006未验收边界与全部任务签收状态原样保留）；不复制为两套无条件权威规格。外部 feature 未配置 Vault 时不能读取，先配置再续接，禁止为绕过缺失重新 init 同名仓库副本。仓库代码、测试、JSON/OpenAPI、发布物、工具配置和历史审核证据留在 Git；外部正文的备份/同步由 Vault 机制负责，不伪称本仓库提交包含其正文。

回退：清除本次外部环境变量仍支持新feature的仓库模式，但已迁移的001–009须配置Vault才能读取，不自动创建同名副本。若明确需要恢复仓库存储，先按迁移登记和源Git提交核对原文，协调移走外部同名正文后再恢复，不能保留双重权威。回滚工具提交不会改远端服务或 Vault 正文。`make spec-kit-verify` 使用隔离 fixture，不依赖真实 Vault、环境变量或网络。
