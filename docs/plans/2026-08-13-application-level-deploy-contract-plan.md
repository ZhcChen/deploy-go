# 应用级部署契约：参数 Schema 与验证配置上移

## 背景

当前 `parameter_schema` 与 `verification_config` 存储在 `deployment_targets`，导致同一应用多节点部署时必须逐目标重复配置，且容易漂移。两者本质是应用级部署契约，应上移到 `applications`，目标只保留节点相关配置。

## 目标

- `parameter_schema`、`verification_config` 的唯一编辑入口为应用详情/应用配置。
- 目标 API 不再接受这两项；目标响应保留只读的“生效值”，供部署客户端使用。
- 部署 snapshot、preview、confirm 使用应用级契约。
- 迁移已有目标数据到应用层，重复部署不丢失。
- Web/Flutter/OpenAPI/文档同步，回归门禁通过。

## 设计

1. migration `0024_application_deploy_contract.sql`：
   - `applications` 增加 `parameter_schema`、`verification_config`。
   - 从目标回填：同应用多目标不一致时取最近更新目标；无目标时写有效默认值。
   - 受迁移门禁限制（新增 migration 不允许 DROP TABLE/COLUMN），不重建目标表；
     目标旧列保留但弃用，API 不再读写，部署读取一律使用应用级列。
2. API：
   - `SaveApplicationRequest` / `ApplicationResponse` 增加两项并校验。
   - `SaveTargetRequest` 移除两项；`DeploymentTargetResponse` 保留只读生效值（来自应用）。
   - 目标创建/更新时用节点 work_root 校验应用契约；preview 执行时再次校验。
3. Web：
   - 应用详情编辑表单增加“参数 JSON Schema”与“部署后验证配置”。
   - 目标编辑器移除两项；模板向导把两项放到应用创建步骤。
   - 应用详情展示部署契约只读预览。
4. Flutter/生成 client 随 OpenAPI 重新生成；部署页继续使用目标只读生效值，行为不变。
5. 文档同步 `docs/standards/deploy-script-contract.md`、`application-deployment-contract.md` 及相关 runbook。

## 执行单元

- U1 migration 0024 + migration 测试
- U2 API 应用/目标模型与部署读取
- U3 OpenAPI 与双端 client 重新生成
- U4 Web 应用配置与目标编辑器
- U5 模板向导与相关测试
- U6 文档与全量验证

## 验证

- `cargo test -p deploy-go-api`、`cargo clippy --workspace --all-targets -- -D warnings`
- `make api-openapi-check`、`make api-client-check`
- `make admin-check`、`make admin-app-check`
- 部署契约相关测试与迁移测试
- 最终 `make check`
