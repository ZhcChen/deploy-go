# 部署记录应用筛选任务

输入：spec.md、plan.md
状态：已完成

## 实施

- [x] T001 FR-002/004：列表接口过滤与权限分页回归，文件 api/src/deployments/mod.rs、api/tests/deployments_api.rs。
- [x] T002 FR-002/004：依赖 T001，标准生成 OpenAPI/双端客户端并验证漂移。
- [x] T003 FR-001/003/005：可搜索 Select、应用选项加载与错误提示，文件 admin/src/components/form.tsx、admin/src/styles/index.css、DeploymentsPage.tsx。
- [x] T004 FR-002：依赖 T002/T003，请求条件与缓存隔离、切换页码/过期翻页保护。

## 验证与交付

- [x] T005 FR-001–005 验证：API、组件与页面回归、前端静态检查/构建、隔离 Playwright。
- [x] T006 独立复核最终 diff，修正发现并执行 analyze/converge 语义核对。
- [x] T007 检查 diff、提交推送，按会话授权发布正式控制面并只读验收。

## 验收证据

- 部署 API 9/9；前端 lint/typecheck/build 和 165/165 测试通过；输入法修正后的聚焦测试 2/2。
- 隔离 Playwright 10/10，包含 1280px/390px 搜索与联合筛选、axe、原部署流程与响应式布局。
- OpenAPI 与双端客户端标准生成/漂移检查通过；本机 Dart 启动阻塞，临时 Docker Dart 工具完成生成，未修改系统 SDK。
- 独立审查发现并修复中文输入法误选；已补真实匹配项 composition、旧翻页竞态、首屏/后续页加载失败恢复及普通用户 cursor 回归。
- analyze/converge 结构和语义核对：FR-001–005/SC-001 实现齐全，无追加实现任务。详见 docs/reviews/2026-10-09-deployment-application-filter.md。
- 实现提交 50942fc 已推送 main，正式控制面安装成功。API/Web active、healthz/readyz 通过；线上 OpenAPI 包含 application_id/status，公网引用新资源，服务器 JS SHA-256 与本机构建一致。
