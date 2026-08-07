---
title: 部署详情多视图与二级返回导航 - Plan
date: 2026-08-07
type: feature
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-brainstorm
execution: code
---

# 部署详情多视图与二级返回导航 - Plan

## Goal Capsule

- **目标：** 把部署详情重构为可快速判断进度、查看事实和排查日志的三视图工作区，并统一 Web 二级页面的返回导航。
- **产品权威：** 本文决定部署详情的信息架构、流程视图状态语义、视图导航和返回控件行为；既有部署脚本与 Agent 协议继续作为执行事实权威。
- **开放阻塞：** 无产品决策阻塞；规划阶段需确认现有标准化事件是否足以生成流程视图，缺失时定义最小只读契约。

---

## Product Contract

### Summary

部署详情提供“流程、详情、日志”三个可链接视图，默认用 GitHub Actions 风格的阶段主线回答“部署执行到哪里”。
所有 Web 二级页面使用一致的返回控件，主内容使用更紧凑且稳定的固定间距。

### Problem Frame

当前部署详情把元数据、逐节点状态、阶段任务和完整日志纵向堆叠在一个页面中，操作者需要滚动并自行拼接执行顺序。
现有文字式返回链接在页面层级中辨识度较弱，不同二级页面也缺少统一的父级导航语言。

### Requirements

**视图与导航**

- R1. 部署详情必须提供“流程、详情、日志”三个 Tab，并默认打开“流程”。
- R2. 当前视图必须写入 `view` URL 查询参数，支持刷新、前进后退和分享链接后保持视图；缺失或非法值回退到“流程”。
- R3. 切换 Tab 不得改变部署事实或触发部署操作，运行中部署仍持续刷新必要状态。
- R4. “详情”必须承载部署元数据、逐节点状态和阶段任务摘要；“日志”必须保留现有实时日志、筛选、复制、连接控制和安全纯文本渲染能力。

**流程视图**

- R5. 流程默认以 `预检 → prepare → release → 验证` 的横向主线展示，移动端按相同顺序改为纵向排列。
- R6. 每个主阶段节点必须支持展开模块与步骤，不得默认平铺全部步骤。
- R7. 流程节点必须显示未执行、执行中、成功、失败四种 GitHub Actions 风格状态角标，并同时提供非颜色状态文案。
- R8. 取消和执行中断使用失败视觉状态，但文案必须保留“已取消”或“执行中断”的真实语义。
- R9. 部署失败时流程视图必须自动展开失败阶段，并提供切换到日志视图且定位相关阶段的操作，不得自动把用户从流程视图跳走。
- R10. 流程必须来源于平台持久化的标准化部署事件或等价执行事实，不得通过解析自然语言日志推断步骤状态。
- R11. 单脚本、两阶段、无模块事件、尚未执行和协议不完整的部署都必须有可理解的降级流程，不得显示虚假的成功步骤。

**二级页面与布局**

- R12. 所有 Web 二级页面必须统一使用“返回箭头图标 + 父级名称”的返回控件，整个控件可点击并具有明确无障碍名称。
- R13. 返回目标必须使用确定的父级路由，不依赖浏览器历史，以保证深链访问仍回到正确列表或详情页。
- R14. Web 主内容桌面内边距固定为 `32px`，窄屏使用适合触控和内容宽度的较小固定间距，不再随宽屏继续放大。

### Key Decisions

- **采用横向阶段流作为流程主视图。** (session-settled: user-directed — chosen over vertical timeline and stage-by-node matrix: it matches the GitHub Actions mental model and keeps the primary deployment path easiest to scan) Governs R5, R6.
- **流程使用两级信息层次。** (session-settled: user-directed — chosen over module-first and fully flattened steps: the main stages remain readable while details stay available on demand) Governs R5, R6.
- **流程是部署详情默认视图。** (session-settled: user-directed — chosen over details-first and logs-first: progress is the first question operators need answered) Governs R1, R2.
- **取消和中断复用失败视觉状态。** (session-settled: user-directed — chosen over an unexecuted state or a fifth status: four status shapes remain easy to learn while text preserves the exact outcome) Governs R7, R8.
- **所有二级页面统一返回控件。** (session-settled: user-directed — chosen over deployment-only replacement: consistent parent navigation reduces relearning across the admin) Governs R12, R13.
- **Tab 状态进入 URL。** (session-settled: user-directed — chosen over component-only state: refresh and shared troubleshooting links retain context) Governs R2.

### Actors

- **部署操作人员：** 查看当前进度、展开阶段、定位失败并切换到相关日志。
- **管理员：** 在相同详情页执行取消、发布和失败目标重试等受权操作。
- **Deploy Go API 与 Agent：** 提供持久化执行事实和标准化事件，前端只负责组织与展示。

### Key Flows

- F1. 查看运行中部署
  - **Trigger:** 用户打开部署详情或从部署列表进入。
  - **Steps:** 页面进入流程视图；显示主阶段状态；运行中事实更新；用户按需展开当前阶段。
  - **Outcome:** 用户无需阅读原始日志即可判断当前执行位置。
  - **Covered by:** R1-R7, R10-R11.
- F2. 定位失败
  - **Trigger:** 部署或步骤进入失败、取消或中断状态。
  - **Steps:** 对应阶段显示失败角标并自动展开；用户点击“查看相关日志”；页面切换到日志视图并定位对应阶段。
  - **Outcome:** 用户保留流程上下文并能一步到达诊断证据。
  - **Covered by:** R7-R10.
- F3. 分享指定视图
  - **Trigger:** 用户复制带 `view` 参数的部署详情 URL。
  - **Steps:** 另一会话打开链接；权限校验完成；页面恢复指定视图。
  - **Outcome:** 协作双方查看相同上下文。
  - **Covered by:** R2-R4.
- F4. 从二级页面返回
  - **Trigger:** 用户从列表、详情或深链进入任一 Web 二级页面。
  - **Steps:** 页面显示统一返回控件；用户点击后进入该页面定义的父级路由。
  - **Outcome:** 返回行为不依赖不可预测的浏览器历史。
  - **Covered by:** R12-R13.

### Acceptance Examples

- AE1. **Covers R1-R6:** 给定一个正在执行 release 的两阶段部署，打开详情时默认选中“流程”，release 节点显示执行中并可展开当前模块与步骤。
- AE2. **Covers R7-R9:** 给定一个步骤失败的部署，失败阶段使用红色叉号且自动展开，点击“查看相关日志”后 URL 变为 `?view=logs` 并定位该阶段。
- AE3. **Covers R8:** 给定已取消或执行中断的部署，节点使用失败视觉，但可见文案分别为“已取消”或“执行中断”。
- AE4. **Covers R2:** 给定 `?view=details` 或 `?view=logs`，刷新后仍显示对应 Tab；给定非法值则显示流程视图。
- AE5. **Covers R10-R11:** 给定没有完整结构化步骤的历史部署，流程显示可证明的阶段事实与协议不完整提示，不从普通 stdout/stderr 猜测步骤。
- AE6. **Covers R12-R14:** 从深链打开应用、节点、部署或设置二级页时，返回控件进入确定父级；桌面内容边距为 `32px`，390px 宽度无页面级横向溢出。

### Scope Boundaries

- 本轮不改变业务应用 `DEPLOY_GO_EVENT` 输出规范、Agent 执行方式或部署状态机。
- 本轮不引入流程编辑器、DAG 编排、步骤重跑或任意节点命令执行。
- 阶段 × 节点矩阵不作为首版主视图；多节点事实继续由详情视图承载。
- Admin App 不在本轮范围内，后续可复用相同状态语义但需要独立移动端设计。

---

## Planning Contract

### Product Contract Preservation

Product Contract unchanged.

### Existing Patterns

- `api/src/agents/dispatcher.rs` 已把 Agent `task_progress` 标准化事件写入 `deployment_events`；本轮直接读取该事实，不复制日志解析逻辑。
- `api/src/deployments/mod.rs` 已统一部署权限校验、详情响应、日志 SSE 与 cursor 风格；新事件查询沿用相同授权和错误语义。
- `admin/src/features/deployments/DeploymentDetailPage.tsx` 已按终态控制轮询，并集中处理 403 撤权、取消、重试和手动 release；Tab 重构保留这些操作入口。
- `admin/src/features/deployments/DeploymentLogPanel.tsx` 已提供实时日志、阶段分组、筛选和复制；日志视图复用该组件，不创建第二套日志状态机。
- `.back-link` 已被多个二级页面复用；本轮提升为正式组件并逐页替换，避免继续复制样式与文案。

### Key Technical Decisions

- KTD1. **新增分页只读部署事件接口。** API 从 `deployment_events` 返回结构化事件，使用稳定 cursor 和有界 page size；Admin 在流程视图按需加载并在运行中增量刷新。Governs R3, R9-R11.
- KTD2. **流程聚合保持在 Admin 的纯函数层。** API 返回持久化事实，前端把事件映射为阶段、模块和步骤视图模型；映射函数无 React 状态，便于覆盖历史部署和异常序列。Governs R5-R11.
- KTD3. **日志组件仅在日志 Tab 挂载。** 切入日志时由既有 SSE 从持久化窗口恢复，切出后释放连接；部署详情轮询和流程事件轮询独立维持当前事实。Governs R3-R4.
- KTD4. **Tab 由 URLSearchParams 驱动。** 切换使用站内导航更新 `view`，保留部署路径和其他合法查询参数；非法值规范化为默认流程。Governs R1-R3.
- KTD5. **返回导航使用单一 `BackLink` 组件。** 组件接收确定父级路由和父级名称，渲染箭头图标、文本和统一可点击区域；错误态返回操作也复用同一语义。Governs R12-R13.
- KTD6. **不新增 migration。** `deployment_events` 已保存事件名、状态、payload 与时间，当前索引支持按部署读取；若实现期查询计划证明现有索引不足，只能新增更高版本 migration，禁止修改历史 migration。

### Event Projection

- 事件接口只暴露流程展示需要的白名单字段：事件 ID、事件名、状态、阶段、模块、模块名称、步骤 ID、步骤名称、失败阶段、消息和时间；无关 payload 字段不透传给客户端。
- 阶段状态遵循可证明事实：任一失败事件优先为失败；存在 started 且无终态为执行中；存在对应 succeeded 为成功；没有事件为未执行。
- `prepare` 和 `release` 由事件的 `stage` 区分；`verification` 事件汇总到验证节点；预检节点汇总最早可证明的 preflight 生命周期。
- 模块和步骤只在其父阶段展开后生成；未知事件保留为协议不完整提示，不生成虚假节点。
- 取消或中断导致缺少脚本失败事件时，依据部署和阶段任务终态把当前未完成节点投影为失败，并保留真实终态文案。

### Dependencies and Sequencing

1. 先完成事件只读 API、OpenAPI 与生成客户端，给流程视图建立稳定输入。
2. 再实现纯聚合模型与流程组件，然后接入部署详情 Tab 和 URL。
3. 返回控件和全局间距可以在流程功能稳定后独立替换，但必须在同一轮 E2E 中验证全部二级路由。
4. 最后执行视觉复核、无障碍检查和窄屏回归；不在实现过程中连接真实节点或发起真实部署。

### Risks

- **历史事件不完整：** 只展示可证明事实，并显式标记协议不完整；不回退到自然语言日志推断。
- **运行中事件分页重复或遗漏：** cursor 使用稳定排序键，客户端按事件 ID 去重；终态后停止增量轮询。
- **失败定位与日志粒度不一致：** 首版按 `prepare`、`release` 等现有日志阶段定位，不承诺精确滚动到单条脚本日志。
- **全局返回组件范围漂移：** 只替换已有确定父级的 Web 二级页面，不重构路由树或列表页。

---

## Implementation Units

### U1. 部署事件只读契约

- **Goal:** 让授权用户按稳定顺序读取一个部署的标准化流程事件。
- **Files:** `api/src/deployments/mod.rs`、`api/src/openapi.rs`、`api/tests/deploy_event_protocol.rs`、`admin/src/api/generated/`。
- **Patterns:** 复用部署详情的授权检查、部署日志的 cursor/错误响应模式和 OpenAPI 生成流程；查询只选择白名单字段并安全解析 `payload_json`。
- **Test Scenarios:** 管理员和获授权普通用户可读取；无权用户返回 403；不存在部署返回 404；分页无重复遗漏；畸形或未知 payload 降级而不导致 500；响应不泄露未声明字段。
- **Verification:** `cargo test -p deploy-go-api --test deploy_event_protocol`、`make api-openapi-check`、`make api-client-check`。
- **Dependencies:** 无。

### U2. 流程投影模型与组件

- **Goal:** 把部署详情、阶段任务和标准化事件投影为两级四态流程，并提供阶段展开和失败日志入口。
- **Files:** 新增 `admin/src/features/deployments/deployment-flow.ts`、新增 `admin/src/features/deployments/DeploymentFlowPanel.tsx`、`admin/src/features/deployments/status.ts`、`admin/src/styles/index.css`、`admin/src/test/DeploymentFlow.test.tsx`。
- **Patterns:** 状态聚合写为纯函数；图标使用 lucide；状态同时由图形、颜色和文本表达；展开按钮使用 `aria-expanded` 与稳定关联区域。
- **Test Scenarios:** 两阶段成功；prepare 运行中；release 步骤失败并自动展开；取消和中断映射失败视觉但保留文案；未执行阶段；单脚本降级；无事件和协议不完整；未知事件忽略；移动端顺序不变。
- **Verification:** `npm test --workspace deploy-go-admin -- DeploymentFlow.test.tsx`、`npm run typecheck --workspace deploy-go-admin`。
- **Dependencies:** U1。

### U3. 部署详情三视图与日志定位

- **Goal:** 用 URL 驱动的流程、详情、日志 Tab 重构部署详情，默认流程并保留现有部署操作。
- **Files:** `admin/src/features/deployments/DeploymentDetailPage.tsx`、`admin/src/features/deployments/DeploymentLogPanel.tsx`、`admin/src/features/deployments/api.ts`、`admin/src/styles/index.css`、`admin/src/test/DeploymentFlow.test.tsx`、`admin/e2e/deployment-flow.spec.ts`。
- **Patterns:** 复用现有 segmented/tab 控件视觉语言，但使用 `tablist`、`tab`、`tabpanel` 语义；URL 是选中状态权威；日志定位通过阶段筛选或稳定锚点实现。
- **Test Scenarios:** 无参数默认流程；三个合法参数深链；非法参数回退；浏览器前进后退恢复；运行中流程刷新；切入日志建立 SSE、切出释放；失败按钮切到日志并定位阶段；取消、重试和 release 操作仍可用；403 撤权清理状态。
- **Verification:** `npm test --workspace deploy-go-admin -- DeploymentFlow.test.tsx`、`npm run test:e2e --workspace deploy-go-admin -- deployment-flow.spec.ts`。
- **Dependencies:** U1、U2。

### U4. 全局二级返回控件与固定内容间距

- **Goal:** 统一所有 Web 二级页面的父级返回导航，并收紧主内容留白。
- **Files:** 新增 `admin/src/components/BackLink.tsx`、`admin/src/features/applications/ApplicationDetailPage.tsx`、`admin/src/features/application-envs/ApplicationEnvEditorPage.tsx`、`admin/src/features/deployments/NewDeploymentPage.tsx`、`admin/src/features/deployments/DeploymentDetailPage.tsx`、`admin/src/features/nodes/NodeDetailPage.tsx`、`admin/src/features/targets/TargetDetailPage.tsx`、`admin/src/features/users/UserDetailPage.tsx`、`admin/src/styles/index.css`、`admin/src/test/AppRoutes.test.tsx`、相关 E2E 文件。
- **Patterns:** 组件使用 `Link` 和 `ArrowLeft`，可见父级名称与 `aria-label` 一致；父级目标由调用页明确传入，不读取 history。
- **Test Scenarios:** 每个二级页深链打开后返回正确父级；键盘焦点可见；可点击区域满足控制尺寸；桌面边距固定 `32px`；390px 窄屏无横向溢出；错误态返回路径一致。
- **Verification:** `npm test --workspace deploy-go-admin -- AppRoutes.test.tsx`、`npm run test:e2e --workspace deploy-go-admin`。
- **Dependencies:** 可与 U1-U2 并行，集成验证依赖 U3。

### U5. 全量复核与文档同步

- **Goal:** 验证产品合同、API 契约、视觉行为和既有部署主流程一致。
- **Files:** `docs/plans/2026-08-07-004-feature-deployment-detail-views-plan.md`；仅在命令或行为发生变化时更新相关 `docs/runbooks/`。
- **Patterns:** 使用本地 fixture 和浏览器 mock 验证，不把功能验证当作真实节点部署授权。
- **Test Scenarios:** 成功、运行、失败、取消、中断、协议不完整、多节点、单脚本与两阶段；桌面及 390px 视口截图；axe smoke；日志纯文本安全回归。
- **Verification:** `make api-check`、`make admin-check`、`make admin-test-e2e`、`git diff --check`。
- **Dependencies:** U1-U4。

---

## Verification Contract

| Layer | Command | Covers | Done Signal |
| --- | --- | --- | --- |
| API focused | `cargo test -p deploy-go-api --test deploy_event_protocol` | U1 | 权限、分页、解析和白名单场景通过 |
| Generated contract | `make api-openapi-check && make api-client-check` | U1 | OpenAPI 与生成客户端无漂移 |
| Admin unit | `npm test --workspace deploy-go-admin` | U2-U4 | 流程投影、Tab、返回导航回归通过 |
| Admin static/build | `npm run check --workspace deploy-go-admin` | U2-U4 | lint、typecheck、unit、production build 通过 |
| Browser E2E | `make admin-test-e2e` | U3-U5 | 三视图、部署操作、窄屏和无障碍 smoke 通过 |
| Final diff | `git diff --check` | U1-U5 | 无空白错误或范围外改动 |

---

## Definition of Done

- R1-R14 均由实现单元和自动化场景覆盖，流程默认视图可从结构化事实解释成功与失败。
- 部署详情三个 Tab 可深链、刷新和前进后退，日志能力与现有安全边界无回归。
- 所有已有 Web 二级页面使用统一返回控件，桌面内容边距固定为 `32px`，窄屏无页面级溢出。
- API、Admin 完整检查和浏览器 E2E 通过，视觉截图确认流程节点、角标、展开区域和操作控件无重叠。
- 工作区只包含本计划相关改动；完成后按项目规则小闭环提交并推送，不自动部署正式环境。
