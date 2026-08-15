---
title: 控制面工作区宽度优化计划
date: 2026-08-15
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# 控制面工作区宽度优化计划

## 目标

消除节点详情及相似信息密集页面在宽屏下未使用可用工作区宽度的问题，使运维数据、编辑器和部署创建流程能够在侧栏之外充分展开。

## 范围

- 移除通用详情页、Env 编辑器、模板向导和部署创建页的无业务必要宽度上限。
- 保留系统设置、个人资料和单列表单的阅读宽度约束，避免短字段在宽屏下过度拉伸。
- 以节点详情作为回归入口，验证详情页宽度与工作区一致；既有移动端响应式规则继续生效。
- 不修改路由、API、表单字段、业务状态或正式环境运行态。

## 实施与验证

1. 在 `admin/src/styles/index.css` 建立“信息密集工作区占满内容列”的共享规则，并移除冗余例外。
2. 在 `admin/e2e/responsive-layout.spec.ts` 增加节点详情宽屏断言，确认详情内容宽度接近页面内容列且小屏无溢出。
3. 执行 lint、typecheck、Vitest、Playwright、build 与 `git diff --check`；使用隔离 API mock 的桌面/移动截图复核。
