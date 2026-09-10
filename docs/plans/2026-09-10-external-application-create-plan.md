# 对外部署 API：创建非正式环境应用计划

## 目标

在既有对外部署 API 上补齐「创建应用」，让外部系统与 Codex skill 可以从零完成
建应用 → 登记 Env → 配置部署契约/部署来源 → 发起部署的完整链路：

- 新增 **创建应用**：`POST /external/v1/applications`。
- 只允许创建非正式环境（`dev` / `test` / `staging`）应用；环境为 `prod` 返回
  403 `external_production_environment_forbidden`。
- 创建成功后自动把新应用绑定到调用方 API Key，并同步外部服务用户授权，
  使新应用立即可被同一 Key 查询、配置和部署。
- `deploy-go-deployer` CLI 新增 `create-app` 子命令，Skill 与本机安装同步更新。

正式环境应用只能由管理员在管理面手动创建，对外 API 不提供该能力。

## 范围

### 对外接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| POST | `/external/v1/applications` | 创建非正式环境应用，返回应用详情 |

请求体（`ExternalApplicationCreateRequest`，拒绝未知字段）：

| 字段 | 必需 | 说明 |
| --- | --- | --- |
| `name` | 是 | 应用名称 |
| `slug` | 是 | 应用 slug，全局唯一 |
| `environment` | 是 | 仅 `dev` / `test` / `staging` |
| `description` | 否 | 应用描述 |
| `app_type` / `type_version` | 否 | 默认 `binary` / `1`，与内部创建一致 |
| `tags` | 否 | 应用标签，最多 10 个 |
| `parameter_schema` | 否 | 参数 JSON Schema |
| `verification_config` | 否 | 部署后验证配置 |

响应 201，返回 `ExternalApplicationDetail`（`targets` 为空数组），与
`show-app` 结构一致，便于外部直接进入后续配置流程。

### 复用与重构

内部 `applications::create` 目前把「管理员校验 + CSRF」与业务逻辑写在一起，对外
接口不能复用该鉴权链，因此按既有模式抽出核心函数：

- `applications::create_application(state, actor_id, external_api_key_id, payload, request_id)`。
- `external_keys::bind_application_to_key(...)`：统一「写入 Key 绑定 + 同步外部服务用户授权」，
  由 Key 管理面创建流程与对外创建应用流程共用。
- 内部 handler 只保留 `require_administrator` + `verify_csrf`，行为与响应不变。

审计沿用 `application.create`，对外创建时在 detail 中写入 `external_api_key_id`，
与 `application.update` 保持一致。

### 明确不做

- 不放开正式环境（`prod`）应用创建，也不允许通过 `PATCH` 把环境改成 `prod`（既有规则）。
- 不支持从模板创建（`template_id` 不在对外请求体中）：模板克隆依赖主密钥环与平台
  预置配置，需要模板驱动的应用仍由管理面创建。
- 不提供应用归档/删除、授权、状态修改等管理面能力。
- 不在对外响应中返回内部字段（审计、密钥材料、节点工作根目录）。

## 执行单元

1. **U1 核心函数抽取**：`applications::create_application` + `external_keys::bind_application_to_key`，
   内部 handler 改为调用核心函数，保持现有行为与测试全绿。
2. **U2 对外创建接口**：新增 `POST /external/v1/applications`、请求/响应 DTO、
   非正式环境校验与 Key 自动绑定。
3. **U3 契约与工具链**：重新生成对外 OpenAPI 产物，`deploy-go-deployer` 新增
   `create-app` 子命令，Skill references 与 runbook 同步。
4. **U4 验证**：API 聚焦测试（创建成功 + 自动绑定 + 正式环境 403 + 非法环境 422 +
   slug 冲突 409）、OpenAPI 契约测试、CLI 契约测试、`cargo clippy` 与
   `make external-deploy-check`。

## 验收标准

- 外部 Key 可创建非正式环境应用，创建后同一 Key 能立即 `list-apps` / `show-app` /
  配置 Env 与部署目标 / 发起部署。
- `environment=prod` 返回 403 `external_production_environment_forbidden`；
  非法环境值返回 422；重复 slug 返回 409 `application_slug_exists`。
- 内部管理面创建应用的请求与响应完全不变。
- 对外 OpenAPI、CLI、Skill 文档与本机安装的 skill 与实现一致。
