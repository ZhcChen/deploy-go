# Agent 环境标识

## 目标

Agent 增加固定环境标识，默认仅包含四个环境，后续需要时再扩展：

- 开发环境：`dev`
- 测试环境：`test`
- 预发布环境：`staging`
- 生产环境：`prod`

界面使用中文显示，API 与数据库使用英文 code。

## 范围

- 新增 migration `0007_agent_environment.sql`，为 `agents` 增加 `environment` 列并限制枚举。
- API 创建 Agent 时必带环境，列表/详情返回环境。
- Admin 创建 Agent 表单提供环境下拉，列表与详情展示环境。
- 同步 OpenAPI 与生成客户端，补充组件测试和 E2E mock。

## 不在本次范围

- 部署目标（`deployment_targets.environment`）暂不改为固定枚举，后续单独收敛。
- Agent 运行时协议暂不上报环境，仅作为控制端身份标识。

## 验证

- `make api-check`（或 API 聚焦测试与 OpenAPI check）。
- `npm run check --workspace deploy-go-admin`。
- `make admin-test-e2e`。

## 完成记录

- `0007_agent_environment.sql` 已加入 `agents.environment`，约束为
  `dev` / `test` / `staging` / `prod`。
- Agent 创建请求必带环境；列表/详情返回并展示环境；UI 设计源同步为 4 个环境。
- API、Admin、UI、Flutter 客户端生成物与测试均已通过并同步。
