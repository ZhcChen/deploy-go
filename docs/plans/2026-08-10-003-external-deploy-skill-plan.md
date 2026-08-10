# 对外部署 OpenAPI 与 Deploy Go Deployer Skill 计划

计划编号：2026-08-10-003
创建日期：2026-08-10
状态：进行中

## 目标

在 Deploy Go 主控上提供一套**独立对外部署 OpenAPI**，并封装为 Codex skill：

- 仅允许：应用列表、应用详情/可用目标、发起部署、查询部署状态、取消部署。
- 明确禁止：Env 读取/修改，应用/节点/用户/审计/终端等管理面操作，任意部署脚本或命令执行。
- 鉴权使用独立 API Key（Bearer `dgx_...`），不复用浏览器会话、CSRF 或 Agent 授权协议。
- 对外契约独立生成并对外发布：`GET /external/v1/openapi.json`。
- 新增 Rust 可执行二进制 `deploy-go-deployer`，封装上述接口供 Agent/Codex 调用。
- 二进制随正式环境一起构建发布，由 API 提供下载路径：
  `GET /api/v1/deployer/download/{version}/deployer/{arch}` 与 manifest。

## 命名决策

- skill 模块名：`deploy-go-deployer`（目录 `skills/deploy-go-deployer/`）。
  理由：与 `deploy-go-agent`、`deploy-go-api` 的项目命名体系一致；“deployer”表达可执行部署动作的主体。
- Rust workspace crate：`deploy-go-deployer`，二进制名 `deploy-go-deployer`。
- 对外 API 命名空间：`/external/v1/`，内部管理面保持 `/api/v1/`。
- 对外 OpenAPI 产物：`api/openapi/external.json`，运行态 `/external/v1/openapi.json`。

## 安全边界

- 外部 Key 只保存 SHA-256 hash，不保存明文；明文 token 仅在创建时返回一次。
- 每个 Key 通过 `external_api_key_applications` 白名单绑定应用，部署复用共享系统账号的授权检查。
- 外部请求统一走 `ExternalApiKey` extractor，不进入 Cookie/CSRF 路径。
- 对外 OpenAPI 只声明外部路径；任何 `/api/v1`、Env、凭证、用户、节点、审计路径不得出现。
- 部署创建复用内部 preview 校验：参数 schema、目标状态、节点可用性、Env gate、release 策略。
- 部署仍然记录 `requested_by`（共享系统账号）与 `external_api_key_id`，审计可追溯。
- 外部部署创建要求 `Idempotency-Key`，以 Key 为作用域防重放。

## 阶段与执行单元

### U1 数据库与系统账号边界

- 新增 migration `0019_external_deploy_api.sql`：
  - `users.system_account` 标记隐藏系统账号。
  - 插入共享系统账号 `usr_external_api_service`。
  - `external_api_keys`、`external_api_key_applications` 表。
  - `deployments.external_api_key_id` 关联列。
- 登录、session、用户列表过滤 `system_account`，系统账号不能登录/不展示。
- 验证：`cargo test -p deploy-go-api --test migrations --test auth_api --test users_api`。

### U2 外部 API Key 管理（内部管理面）

- 新增 `api/src/external/keys.rs`：
  - `GET/POST /api/v1/external-api-keys`。
  - `GET /api/v1/external-api-keys/{id}`。
  - `POST /api/v1/external-api-keys/{id}/revoke`。
  - `PUT /api/v1/external-api-keys/{id}/applications`。
- Key 创建时同步共享系统账号的应用 grant，保证内部 preview 授权一致。
- 验证：新增 `api/tests/external_api_keys.rs`，审计、token hash、权限同步。

### U3 对外只读端点

- 新增 `api/src/external/mod.rs`：
  - `GET /external/v1/applications`、`GET /external/v1/applications/{id}`。
  - 外部 DTO 不暴露 Env、脚本路径、密钥、节点连接信息。
- 新增 `ExternalApiKey` extractor，校验 Bearer token、有效期并更新 `last_used_at`。
- 验证：新增 `api/tests/external_deploy_api.rs` 的只读部分与越权负向测试。

### U4 对外部署创建、状态与取消

- 重构 `api/src/deployments/mod.rs`：抽取 `create_application_deployment`、
  `create_target_deployment`、`cancel_deployment` 核心函数，内部 handler 与外部共用。
- 外部端点：
  - `POST /external/v1/applications/{id}/deployments`（支持指定 target 或全目标）。
  - `GET /external/v1/deployments/{id}`。
  - `POST /external/v1/deployments/{id}/cancel`。
- `snapshot_hash` 可选：提供则校验，未提供则使用最新 preview 结果。
- 验证：新增外部部署集成测试，覆盖幂等、并发冲突、取消、Env gate。

### U5 对外 OpenAPI 与契约测试

- 独立 `ExternalApiDoc`，生成 `api/openapi/external.json`。
- `main.rs` 的 `openapi` / `openapi-check` 同时维护内部与外部产物。
- `Makefile` 增加 `api-external-openapi` / `api-external-openapi-check`，并入 `api-openapi*`。
- 契约测试：外部文档只含白名单路径、Bearer security、无 `/api/v1`、无 Env 字段。

### U6 Deployer CLI（Rust 可执行二进制）

- 新增 workspace crate `deploy-go-deployer`：
  - `list-apps`、`show-app`、`deploy`、`status`、`cancel`、`openapi`。
  - 支持 `--api-base` / `--api-key` 与环境变量 `DEPLOY_GO_API_BASE_URL`、`DEPLOY_GO_API_KEY`。
  - 嵌入外部 OpenAPI 产物，`openapi` 可输出本地契约。
- 验证：cargo test + `deploy-go-deployer/test-contract.sh`。

### U7 Skill 模块

- 新增 `skills/deploy-go-deployer/SKILL.md`：
  - 使用场景、命令、参数、错误处理、禁止事项。
- 新增 `docs/runbooks/external-deploy-api.md`：Key 管理、调用示例、安全说明。

### U8 发布与下载链路

- API 新增 `api/src/deployer.rs`：读取 `DEPLOY_GO_DEPLOYER_RELEASE_DIR`，
  提供 manifest 与双架构二进制下载路由。
- 新增 `deploy-go-deployer/docker/release/Dockerfile` 本机构建产物。
- `deploy/production/deploy.sh` 与 `install.sh` 构建、校验、原子安装、回滚、注入路径。
- 更新 `deploy/production/test-install-contract.sh` 与 runbook。

### U9 验证与 review

- 运行 `make external-deploy-check`、`make api-openapi-check`、`make check`。
- 执行 simplify 与高风险 review，写入
  `docs/reviews/2026-08-10-external-deploy-api-skill-review.md`。

## 明确不包含（后续另行确认）

- 管理端 Web UI 的 API Key 配置页（本期通过内部管理 API 管理）。
- 外部 OpenAPI 的 Env 读取、SSH、制品上传、Agent 管理。
- 任何真实节点部署或生产环境执行；正式发布须由用户再次明确授权。

## 最终验证清单

- `make external-deploy-check` 通过。
- `make api-openapi-check`、`make api-client-check`、`make check` 通过。
- 对外 OpenAPI 不包含任何 `/api/v1`、Env、管理面操作。
- `deploy-go-deployer --help` 与各子命令可用。
- 生产发布链路契约测试通过，且不输出私钥/API Key 正文。
