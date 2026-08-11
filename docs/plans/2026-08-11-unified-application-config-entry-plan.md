---
title: 应用配置唯一入口 UI 收敛实施计划
created_at: 2026-08-11
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: docs/brainstorms/2026-08-11-unified-application-config-entry.md
execution: code
---

# 应用配置唯一入口 UI 收敛实施计划

## Goal Capsule

- **目标**：把管理端运行配置统一到「应用详情 → 应用配置」，并收敛目标编辑器
  与模板向导中的旧「敏感文件引用」入口，消除多入口配置混乱。
- **核心边界**：只做 Web UI 与文档收敛，不新增后端接口、不修改数据库、
  不改变协议；旧 `script` 单脚本模式能力保持兼容。
- **完成条件**：应用详情区块与编辑路由统一为「应用配置」；`two_stage` 目标
  不再展示敏感文件引用；模板向导不再提交旧引用；相关测试与构建通过。

## Product Contract

### Requirements

- **R1**. 应用详情区块标题从「应用 Env」改为「应用配置」，说明保存后自动
  同步到全部目标节点。
- **R2**. 配置编辑路由从 `/apps/:id/env/:envFileId` 改为
  `/apps/:id/config/:envFileId`，旧路由不再保留。
- **R3**. 目标编辑器只在 `script` 单脚本模式显示「敏感文件引用」，并标注为
  旧版；`two_stage` 下隐藏，提交时清空旧引用。
- **R4**. 模板创建向导移除 `secretReferences` 输入与提交字段，模板目标固定
  为 `two_stage`。
- **R5**. 目标详情「环境」改标为「兼容环境标识」，避免与运行配置混淆。
- **R6**. 同步更新管理端测试、Playwright 路径与相关文档措辞。

### Acceptance Examples

- **AE1**. 管理员打开应用详情看到「应用配置」区块，编辑链接指向
  `/config/...`。
- **AE2**. 新建/编辑 `two_stage` 目标时不出现「敏感文件引用」，提交请求
  `secret_file_references` 为空数组。
- **AE3**. 模板向导目标步骤不再出现「敏感文件引用」，请求不包含该字段。
- **AE4**. `script` 单脚本模式仍可配置旧引用，旧 launcher 兼容路径不回归。
- **AE5**. 目标详情展示「兼容环境标识」而不是「环境」。

## Implementation Units

### U1. 应用配置入口

- **Files**
  - `admin/src/features/application-envs/ApplicationEnvSection.tsx`
  - `admin/src/features/application-envs/ApplicationEnvEditorPage.tsx`
  - `admin/src/routes/AppRoutes.tsx`
  - `admin/src/test/ApplicationEnvManagement.test.tsx`
  - `admin/e2e/application-env-management.spec.ts`
- **Approach**
  - 区块标题改为「应用配置」，副标题说明唯一编辑入口与自动同步。
  - 编辑链接与路由改为 `config/:envFileId`。
  - 编辑器不存在提示改为「配置文件不存在」。
- **Verification**
  - `npm test --workspace deploy-go-admin -- --run src/test/ApplicationEnvManagement.test.tsx`
  - 更新 Playwright 路径后执行管理端浏览器 smoke（如环境可用）。

### U2. 目标编辑器收敛

- **Files**
  - `admin/src/features/targets/TargetEditor.tsx`
  - `admin/src/test/ApplicationConfiguration.test.tsx`
- **Approach**
  - 「执行与验证」面板只在 `script` 模式渲染敏感文件引用。
  - `two_stage` 切换时清空草稿引用；提交时非 `script` 模式固定为空数组。
  - 面板说明与字段 hint 标注旧版单脚本专用。
- **Verification**
  - `npm test --workspace deploy-go-admin -- --run src/test/ApplicationConfiguration.test.tsx`

### U3. 模板向导收敛

- **Files**
  - `admin/src/features/templates/CreateFromTemplatePage.tsx`
  - `admin/src/test/TemplateWizard.test.tsx`
- **Approach**
  - 删除 `TargetDraft.secretReferences` 状态、初始值与解析逻辑。
  - 删除目标步骤敏感文件引用输入框；创建请求不再带 `secret_file_references`。
  - 结果页与下一步文案改为「在应用配置登记」。
- **Verification**
  - `npm test --workspace deploy-go-admin -- --run src/test/TemplateWizard.test.tsx`

### U4. 目标详情与文案

- **Files**
  - `admin/src/features/targets/TargetDetailPage.tsx`
  - `admin/src/test/ApplicationConfiguration.test.tsx`
- **Approach**
  - 目标详情「环境」改为「兼容环境标识」。
- **Verification**
  - 管理端 lint / typecheck / 全部单元测试 / build。

### U5. 文档同步

- **Files**
  - `docs/runbooks/application-onboarding.md`
  - `docs/runbooks/application-templates.md`
- **Approach**
  - 将「应用 Env」措辞改为「应用配置」，说明唯一编辑入口。
- **Verification**
  - `git diff --check`

## Scope Boundaries

**本计划范围**

- Web 管理端 UI 收敛、路由、测试与 runbook 措辞。

**明确不做**

- 不新增后端 API 或 migration，不改变 OpenAPI 契约。
- 不移除 `secret_file_references` 数据模型和 `script` 兼容能力。
- 不实现 Web 新建/导入配置文件或非 dotenv 格式编辑。
