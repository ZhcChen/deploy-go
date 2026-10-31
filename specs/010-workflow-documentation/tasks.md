# 工作流与文档首轮优化任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：实施、本地验证与独立复核已完成，Git 交付进行中；勾选仅由用户签收。

## 实施与验证

- [ ] T001 FR-001 FR-005 更新 README.md、docs/README.md 与 deploy-go-deployer/README.md；首页可定位当前能力、规范、开发和历史入口，CLI 概览与命令枚举对应。无前置依赖。
- [ ] T002 FR-002 FR-003 统一 AGENTS.md、docs/runbooks/spec-kit-workflow.md、development-guide.md、runbooks/README.md 与 Makefile；形成风险分流和验证映射，check/help 包含 dev-remote-check。依赖 T001 的导航入口。
- [ ] T003 FR-004 FR-005 更新 specs/README.md、docs/plans/README.md、存储登记和任务覆盖模板；登记唯一正文、解释旧状态、分别记录实施/验证/签收，并补齐文档必需入口。依赖 T001、T002。
- [ ] T004 FR-001 FR-002 FR-003 FR-004 FR-005 验证：运行 docs-check、spec-kit-verify、dev-remote-check、whitespace-check，核对 Make 依赖并完成三个场景走查。依赖 T001–T003。
- [ ] T005 FR-004 FR-005 自审最终 diff 与范围，运行 converge 并记录通过、未验证和遗留边界；提交、fetch/rebase/push main，保持全部人工勾选原状。依赖 T004。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已实现 | 通过 | 不适用 | 待用户签收 | 首页、文档导航及 CLI 概览已更新；CLI 枚举与概览全部 34 个命令一致 |
| T002 | 已实现 | 通过 | 不适用 | 待用户签收 | 流程分流、验证映射和 AGENTS 口径一致；make check 的 dry-run 调度一次 dev-remote-check，未包含开发栈 build/up |
| T003 | 已实现 | 通过 | 不适用 | 待用户签收 | 索引、旧状态解释、010 存储登记、任务覆盖模板及文档必需入口已更新；010 的仓库模式命令实测返回 STORAGE=repository |
| T004 | 已执行 | 通过 | 不适用 | 待用户签收 | 见下方本地检查和三个场景走查 |
| T005 | 进行中 | 自审与独立复核通过 | 不适用 | 待用户签收 | 存储登记口径已修正并经复核确认；尚待 converge 与 Git 交付 |

所有勾选保留未签收状态。下方只增补实施与验证事实，不代替用户验收。

### 本地检查（2026-10-31）

- `make docs-check`：189 份 Markdown 的必需入口和相对文件链接通过。
- `make spec-kit-verify`：21 项回归全部通过，上游资产 hash 与覆盖模板兼容检查通过。
- `make dev-remote-check`：12 项回归全部通过，仅本地 Compose 渲染和 mock，不创建容器或连接服务器。
- `make whitespace-check`：通过。当前入口中的 42 个项目 Make 目标均存在；本轮导航中的 18 处标题锚点另行核对通过。
- `make -n check` 与 `make help`：新门禁调度一次且帮助可见；CLI 34 个子命令逐项核对概览通过。

### 三个场景走查

| 场景 | 阅读与流程判断 | 验证组合及边界 |
| --- | --- | --- |
| 修复 Web 既定展示行为的局部缺陷 | README → AGENTS → 流程选择 → 客户端契约；范围明确且不触及高风险边界时使用短流程 | Web 检查；关键交互变化叠加浏览器 smoke，文档变化叠加 docs-check；不由开发指令推导运行发布授权 |
| 修改 Agent 线协议 | README → 工作流 → 协议规范 → 唯一 feature；完整六阶段，保留历史 Schema 并核对协商与兼容性 | api-check、agent-check、文档与 Schema 兼容验证；涉及 Linux 执行路径时叠加对应容器回归和 rust-linux-clippy，重要/高风险改动独立复核 |
| 续接 006 制品交接未验收项 | README → 规格索引 → 存储登记 → 配置外部根后由 wrapper 读取原 tasks；只承接剩余工作 | 登记保留组件已发布、业务现场未验收的边界；当前未配置 Vault，未复读外部正文或复验现场，不新建同名副本、不更改 T011 签收 |

上述为文档阅读路径与规则走查，不声称已实施新的 Web/协议变更或完成外部任务验收。本轮无 Rust、Web、Flutter 生产源码变化，未运行完整产品门禁、Linux 权限容器、浏览器或设备验收；外部规格正文、真实部署和全局 Skill 安装不在本轮验证范围。

### 独立只读复核

复核者 `/root/workflow_review` 按 AGENTS.md 第 9 节检查流程分流、授权、验证映射、存储和签收边界，不修改或提交文件。确认 010 的单次清除外部根命令覆盖同名冲突；另发现存储登记中“仓库仅保留定位说明”的表述未限定 001–009，写入者已修正并明确 010 仓库正文及 Git 提交边界。复核者复查后确认无剩余发现。外部 Vault 正文与真实运行环境未纳入复核。
