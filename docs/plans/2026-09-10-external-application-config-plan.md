# 对外部署 API：应用配置（部署契约与 Env 登记）计划

## 目标

在既有对外部署 OpenAPI 基础上，把「应用配置」补齐到可完整承载一次应用接入：

- 继续拓展编辑应用：元数据、标签、参数 JSON Schema、部署后验证配置（已具备）。
- 新增 **部署契约** 配置：部署目标（节点、执行模式、脚本路径、超时、密钥文件引用、镜像规格）。
- 新增 **部署来源** 配置：固定工作区来源（构建 Agent + 工作区路径）。
- 新增 **登记 Env**：Env 文件登记、更新、删除与元数据查询；对外永不返回明文。

安全边界与既有规则一致：**对外 API 不允许操作正式环境**（应用环境为 `prod` 或目标
为正式环境时返回 403），不允许读取 Env 明文，不允许管理面操作。

> 说明：需求原文写作「非正式环境不可操作」，与上一轮「只能编辑非正式环境应用」的
> 既定规则相反。按上下文与既有安全边界理解为「**正式环境不可操作**」，即非正式环境
> （`dev` / `test` / `staging`）可配置、正式环境（`prod`）返回 403。如需反向放开，
> 必须明确确认后再调整。

## 范围

### 新增对外接口

Env 登记（`external_env_*`）：

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/external/v1/applications/{id}/env-files` | Env 文件元数据与同步状态，不含明文 |
| POST | `/external/v1/applications/{id}/env-files` | 批量登记 Env 文件 |
| PUT | `/external/v1/applications/{id}/env-files/{env_file_id}` | 更新 Env 内容（`expected_version` 乐观锁） |
| DELETE | `/external/v1/applications/{id}/env-files/{env_file_id}` | 删除 Env 文件（墓碑版本 + 同步记录） |

部署契约（`external_deployment_targets_*`）：

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/external/v1/applications/{id}/targets` | 列出部署目标 |
| POST | `/external/v1/applications/{id}/targets` | 新增部署目标 |
| PATCH | `/external/v1/deployment-targets/{target_id}` | 编辑部署目标 |
| PUT | `/external/v1/deployment-targets/{target_id}/status` | 启用/停用部署目标 |

部署来源（`external_application_workspace_source_*`）：

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/external/v1/applications/{id}/workspace-source` | 读取固定工作区来源 |
| PUT | `/external/v1/applications/{id}/workspace-source` | 保存固定工作区来源 |

### 复用与重构

内部 handler 目前把「管理员校验 + CSRF + 重新认证 grant」和业务逻辑写在一起。
对外接口不能复用该鉴权链（API Key 无 session/CSRF/grant），因此把业务逻辑抽成
核心函数，由内部与对外 handler 分别做各自鉴权后调用：

- `application_envs`：登记、更新、删除各抽一个 `*_core`。
- `deployment_targets`：新增、编辑、状态变更各抽一个 `*_core`。
- `application_workspace_sources`：保存抽一个 `*_core`。

核心函数统一接收 `actor_id` 与 `external_api_key_id`，审计记录写入
`external_api_key_id`，与 `applications::update_application` 保持一致。

### 明确不做

- 不提供 Env 明文读取（reveal）或重新认证授权入口。
- 不提供 Git 来源写入（涉及 Git 凭证引用），仅保留既有管理面配置。
- 不提供节点级策略（工作根目录、密钥根目录）修改。
- 不提供正式环境（`prod`）任何写操作。

## 执行单元

1. **U1 核心函数抽取**：`application_envs` 登记/更新/删除核心函数 + 内部 handler 改为调用核心函数，保持现有行为与测试全绿。
2. **U2 Env 对外接口**：新增 4 个 Env 路由、请求/响应 DTO 与路由级鉴权（Key 绑定 + 非正式环境）。
3. **U3 部署目标核心抽取与对外接口**：目标新增/编辑/状态变更核心函数 + 4 个对外路由。
4. **U4 工作区来源核心抽取与对外接口**：保存核心函数 + 2 个对外路由。
5. **U5 契约与工具链**：外部 OpenAPI 重新生成、`deploy-go-deployer` 新增 CLI 子命令、Skill references 与 runbook 同步。
6. **U6 验证**：API 测试（外部接口聚焦用例 + 正式环境 403 用例）、OpenAPI 契约检查、CLI 测试、`cargo clippy`、`make external-deploy-check`。

## 验收标准

- 非正式环境应用可通过对外 API 完成：登记 Env → 配置部署目标/工作区来源 → 发起部署。
- 正式环境应用的上述任一写操作返回 403，错误码明确。
- Env 明文不出现在任何对外响应中。
- 既有内部接口行为不变（内部测试全绿）。
- 外部 OpenAPI 产物与实现一致，`make api-external-openapi-check` 通过。
- CLI 与 Skill 文档覆盖新增命令。

## 进度

- [x] U1 核心函数抽取
- [x] U2 Env 对外接口
- [x] U3 部署目标对外接口
- [x] U4 工作区来源对外接口
- [x] U5 契约与工具链
- [x] U6 验证

## 结果

| 单元 | 结果 |
| --- | --- |
| U1 | `application_envs` 抽出 `register_files_core` / `update_file_core` / `delete_file_core`，既有 5 个测试保持通过 |
| U2 | 新增 4 个 Env 路由：列表、登记、更新、删除；响应只含元数据与同步统计 |
| U3 | `deployment_targets` 抽出 `create_target_core` / `update_target_core` / `update_target_status_core`，新增 4 个对外路由 |
| U4 | `application_workspace_sources` 抽出 `save_workspace_source_core`，新增 2 个对外路由 |
| U5 | 外部 OpenAPI 重新生成；CLI 新增 10 个子命令；Skill references 与 runbook 同步 |
| U6 | `external_api` 15 项、`external_openapi_contract` 6 项、`deployment_targets_api`、`applications_api`、deployer 10 项全部通过；`make external-deploy-check` 通过 |

对外 OpenAPI 现在暴露 11 条路径；`script_path` 从「禁止出现」改为「可配置但受节点
工作根目录校验」，其余内部字段（凭证、终端、审计、密钥材料）继续被契约测试拦截。
