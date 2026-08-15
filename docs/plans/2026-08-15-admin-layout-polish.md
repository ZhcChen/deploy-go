---
title: 控制面高频页面布局优化计划
date: 2026-08-15
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# 控制面高频页面布局优化计划

## 目标

改善控制面管理端在窄宽度和信息密集场景下的可扫描性，解决标题、筛选、操作和表格内容互相挤压的问题，同时保持现有克制的运维后台设计。

## 范围

- 调整共享标题区、筛选栏、操作栏、分页和表格的响应式规则。
- 为部署、节点、应用三个高频列表标记次要列，使小屏优先保留对象、状态和操作。
- 优化概览页的指标和活动列表换行。
- 不修改 API、权限、路由、状态管理或任何部署运行态行为。

## 实施单元

### U1. 共享布局基线

- 文件：`admin/src/styles/index.css`
- 标题、表单操作、详情操作和筛选栏支持换行，文本容器可收缩，按钮保持稳定尺寸。
- 将窄屏筛选控件改为可用宽度，分页控制允许自然换行。
- 测试：在 `390px` 和桌面宽度下检查无页面级横向溢出，主操作始终可见。

### U2. 高频列表的信息优先级

- 文件：`admin/src/features/deployments/DeploymentsPage.tsx`、`admin/src/features/nodes/NodesPage.tsx`、`admin/src/features/applications/ApplicationsPage.tsx`、`admin/src/styles/index.css`
- 使用列语义 class 在小屏隐藏时间、说明、技术详情等次要信息，保留实体、状态和操作列；桌面端不丢失任何字段。
- 测试：现有列表请求、筛选、分页和详情链接继续正常；移动视口仅隐藏展示字段，不影响表格可访问性。

### U3. 概览页密度调整与验收

- 文件：`admin/src/styles/index.css`、`admin/e2e/deployment-flow.spec.ts`
- 指标在中小视口有稳定网格层级，活动状态不覆盖内容；使用隔离 API mock 做桌面和移动截图核验。
- 验证：`npm run lint --workspace deploy-go-admin`、`npm run typecheck --workspace deploy-go-admin`、`npm test --workspace deploy-go-admin`、`npm run build --workspace deploy-go-admin`、`git diff --check`。

## 风险与边界

- 表格仍保留横向滚动作为极端内容的兜底，不通过截断关键状态或操作来强行压缩。
- 仅使用本地 Vite 和 Playwright 的路由 mock；不访问真实控制面或节点。
