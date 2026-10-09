# 部署记录应用筛选实施方案

规格：spec.md
状态：就绪

## Technical Context

列表接口仅 limit/after；管理端状态仅过滤当前页。复用 Select 和 useCursorCollection。
无需新依赖、migration 或 Agent 协议变更。

## Constitution Check

遵循 AGENTS.md、docs/runbooks/local-development.md 与 docs/runbooks/systemd-deployment-production.md。
仅修改 deploy-go，main 开发；不修改系统代理。需求目录固定为 specs/003-deployment-application-filter。

## 实现方案与文件范围

- api/src/deployments/mod.rs 增加 application_id/status 可选查询参数，绑定 SQL 条件，保留授权和二元 cursor。
- OpenAPI 与双端客户端通过项目入口生成，不手工修改生成文件。
- form.tsx Select 增加可选 searchable，搜索框在 listbox 外、保留键盘与原非搜索模式。
- DeploymentsPage 获取全部可访问应用；缓存键包含应用/状态；切换重置页码；旧翻页完成不得推进新筛选页码。
- 复用现有测试文件补权限、分页、搜索与切换场景；增加隔离 Playwright 验证。

## 阶段与依赖

接口过滤和契约先行，再接 UI；最后聚焦测试、独立审查、真实浏览器验证、提交与已授权发布。

## 验证与验收

cargo test -p deploy-go-api --test deployments_api；make api-openapi-check api-client-check；
npm --prefix admin run typecheck、lint、test、build；Playwright 隔离路由 fixture 检查下拉与分页。
无筛选保持兼容，越权筛选返回空列表，不增加应用可见性；未知 status 返回 422。

## 运行与恢复

无数据迁移。沿会话已授权正式控制面发布；不发起业务部署。
按正式部署 runbook 恢复上一套控制面/Web 产物即可回滚，新增参数向后兼容。
