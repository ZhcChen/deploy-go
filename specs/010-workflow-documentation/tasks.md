# 工作流与文档首轮优化任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：首轮实施、本地验证、独立复核与 Git 交付已完成；FR-006 续作实施、本地验证、自审与 Git 交付已完成。全部任务待用户签收，勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-001 FR-005 更新 README.md、docs/README.md 与 deploy-go-deployer/README.md；首页可定位当前能力、规范、开发和历史入口，CLI 概览与命令枚举对应。无前置依赖。
- [ ] T002 FR-002 FR-003 统一 AGENTS.md、docs/runbooks/spec-kit-workflow.md、development-guide.md、runbooks/README.md 与 Makefile；形成风险分流和验证映射，check/help 包含 dev-remote-check。依赖 T001 的导航入口。
- [ ] T003 FR-004 FR-005 更新 specs/README.md、docs/plans/README.md、存储登记和任务覆盖模板；登记唯一正文、解释旧状态、分别记录实施/验证/签收，并补齐文档必需入口。依赖 T001、T002。
- [ ] T004 FR-001 FR-002 FR-003 FR-004 FR-005 验证：运行 docs-check、spec-kit-verify、dev-remote-check、whitespace-check，核对 Make 依赖并完成三个场景走查。依赖 T001–T003。
- [ ] T005 FR-004 FR-005 自审最终 diff 与范围，运行 converge 并记录通过、未验证和遗留边界；提交、fetch/rebase/push main，保持全部人工勾选原状。依赖 T004。

## 同日续作：取消强制独立复核

- [ ] T006 FR-006 更新 AGENTS.md、constitution、spec/tasks 覆盖模板、Spec Kit 工作流和历史 launcher 当前规范；取消默认或强制独立复核，明确实现者自审和风险匹配验证，保留只读参与者、权限与运行授权边界。依赖已完成的 T005；不委派复核 Agent。
- [ ] T007 FR-006 验证：运行 make docs-check、make spec-kit-verify、make whitespace-check 和本 feature 阶段检查；核对当前规则与新模板没有强制独立复核残留，历史事实和全部人工勾选保持原样。依赖 T006。
- [ ] T008 FR-006 自审最终 diff、范围与当前规则一致性，执行 converge 并记录验证证据；按小闭环提交、fetch/rebase/push main。依赖 T007。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已实现 | 通过 | 不适用 | 待用户签收 | 首页、文档导航及 CLI 概览已更新；CLI 枚举与概览全部 34 个命令一致 |
| T002 | 已实现 | 通过 | 不适用 | 待用户签收 | 流程分流、验证映射和 AGENTS 口径一致；make check 的 dry-run 调度一次 dev-remote-check，未包含开发栈 build/up |
| T003 | 已实现 | 通过 | 不适用 | 待用户签收 | 索引、旧状态解释、010 存储登记、任务覆盖模板及文档必需入口已更新；010 的仓库模式命令实测返回 STORAGE=repository |
| T004 | 已执行 | 通过 | 不适用 | 待用户签收 | 见下方本地检查和三个场景走查 |
| T005 | 已完成 | 自审、独立复核与 converge 通过 | 不适用 | 待用户签收 | 实现提交 24b9609 已推送 origin/main；存储登记修正经复核确认，最终 diff 与暂存区空白检查通过 |
| T006 | 已实现 | 通过 | 不适用 | 待用户签收 | 协作规则、原则摘要、覆盖模板、工作流与 launcher 规范已同步；独立复核只在用户明确要求时安排 |
| T007 | 已执行 | 通过 | 不适用 | 待用户签收 | 文档 189 份、Spec Kit 21 项和空白检查通过；当前规则及历史保留核对通过 |
| T008 | 已完成 | 自审与 converge 通过 | 不适用 | 待用户签收 | 范围与语义核对无遗漏，不追加任务；实现提交 773e561 已完成 fetch、rebase 核对及推送 origin/main |

所有勾选保留未签收状态。下方只增补实施与验证事实，不代替用户验收。

### 本地检查（2026-10-31）

- `make docs-check`：189 份 Markdown 的必需入口和相对文件链接通过。
- `make spec-kit-verify`：21 项回归全部通过，上游资产 hash 与覆盖模板兼容检查通过。
- `make dev-remote-check`：12 项回归全部通过，仅本地 Compose 渲染和 mock，不创建容器或连接服务器。
- `make whitespace-check`：通过。当前入口中的 42 个项目 Make 目标均存在；本轮导航中的 18 处标题锚点另行核对通过。
- `make -n check` 与 `make help`：新门禁调度一次且帮助可见；CLI 34 个子命令逐项核对概览通过。

### 三个场景走查（首轮历史）

该表记录首轮走查时的规则；默认复核要求以 FR-006 续作后的当前工作流为准。

| 场景 | 阅读与流程判断 | 验证组合及边界 |
| --- | --- | --- |
| 修复 Web 既定展示行为的局部缺陷 | README → AGENTS → 流程选择 → 客户端契约；范围明确且不触及高风险边界时使用短流程 | Web 检查；关键交互变化叠加浏览器 smoke，文档变化叠加 docs-check；不由开发指令推导运行发布授权 |
| 修改 Agent 线协议 | README → 工作流 → 协议规范 → 唯一 feature；完整六阶段，保留历史 Schema 并核对协商与兼容性 | api-check、agent-check、文档与 Schema 兼容验证；涉及 Linux 执行路径时叠加对应容器回归和 rust-linux-clippy，重要/高风险改动独立复核 |
| 续接 006 制品交接未验收项 | README → 规格索引 → 存储登记 → 配置外部根后由 wrapper 读取原 tasks；只承接剩余工作 | 登记保留组件已发布、业务现场未验收的边界；当前未配置 Vault，未复读外部正文或复验现场，不新建同名副本、不更改 T011 签收 |

上述为文档阅读路径与规则走查，不声称已实施新的 Web/协议变更或完成外部任务验收。本轮无 Rust、Web、Flutter 生产源码变化，未运行完整产品门禁、Linux 权限容器、浏览器或设备验收；外部规格正文、真实部署和全局 Skill 安装不在本轮验证范围。

### 独立只读复核（首轮历史记录）

复核者 `/root/workflow_review` 按 AGENTS.md 第 9 节检查流程分流、授权、验证映射、存储和签收边界，不修改或提交文件。确认 010 的单次清除外部根命令覆盖同名冲突；另发现存储登记中“仓库仅保留定位说明”的表述未限定 001–009，写入者已修正并明确 010 仓库正文及 Git 提交边界。复核者复查后确认无剩余发现。外部 Vault 正文与真实运行环境未纳入复核。

### Git 交付

实现提交 `24b9609`（docs: 统一项目现状、任务流程与验证入口）已完成 fetch、rebase 核对及推送 origin/main。converge 结构与语义核对无新增遗漏，不追加任务；本段只补交付事实，全部任务勾选继续保留未签收。

### FR-006 续作验证（2026-10-31）

- T006：当前入口仅在用户明确要求时安排独立复核；高风险任务继续完整流程、专项验证及实现者最终 diff 自审，模板不再要求另派复核 Agent。只读参与者、权限、协议、migration、运行授权与人工签收边界保留。
- T007：`make docs-check` 检查 189 份 Markdown 通过；`make spec-kit-verify` 21 项全部通过，无失败或跳过，覆盖模板兼容与受管资产 hash 检查通过；`make whitespace-check` 通过。
- 本 feature 的 plan、tasks、analyze、implement、converge 阶段检查均通过，返回 `STORAGE=repository`，已读取要求的三份核心产物并核对 FR-006 与 T006–T008 的范围、依赖和验证。converge 语义核对无新增遗漏，不追加任务。
- 自审与范围核对：仅修改计划内 9 份 Markdown；T001–T005 原任务和历史独立复核正文保持原样，新增任务均未勾选。当前规则中的“独立权限”属于业务脚本权限边界，予以保留。
- 本次未委派复核 Agent。没有产品代码、Make 编排或开发隔离工具变化，未重复产品运行测试与首轮 dev-remote-check；未访问外部规格或真实节点。
- T008：实现提交 `773e561`（docs: 取消强制独立复核并保留自审验证责任）已推送 origin/main；提交前的最终 diff 自审、`git diff --check` 与 `git diff --cached --check` 通过。全部任务勾选保留未签收，本段只补实际交付事实。
