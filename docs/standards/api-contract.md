---
date: 2026-07-31
topic: api-contract
status: accepted
version: 2
---

# API 通用契约

## 适用范围

本规范约束 `/api/v1` 下所有 HTTP API，并作为 Web 与 Flutter 客户端的共同契约。OpenAPI 必须与本规范及实际 handler 保持一致。

## 标识与时间

- 业务主键使用不可预测的字符串 ID，不对客户端暴露 SQLite 自增序号。
- 时间以 UTC 存储，以 RFC 3339 字符串返回。
- 资源响应包含 `created_at` 和 `updated_at`；需要并发保护的资源额外包含递增 `version`。
- 计数和持续时间使用整数；持续时间字段统一以 `_ms` 结尾。

## 响应与错误

成功响应直接返回资源或集合。创建资源返回 `201`，无正文删除返回 `204`。

```json
{
  "code": "validation_failed",
  "message": "请求参数不符合要求",
  "request_id": "req_01...",
  "details": {
    "field_errors": {
      "name": ["名称不能为空"]
    }
  }
}
```

- `code` 是稳定的机器标识，客户端不得解析 `message` 判断业务分支。
- `message` 不包含内部 SQL、命令、路径、密钥或堆栈。
- 字段校验错误需要结构化表达时放入 `details.field_errors`，不增加顶层错误字段。
- `details` 只承载稳定、脱敏的结构化校验或冲突信息，例如阻止删除凭证的节点最小摘要；不得包含内部错误或敏感值。
- 请求 JSON 无法解析、字段类型错误或包含 schema 未声明字段时统一返回 `422 validation_failed`，不得透传框架原生 rejection 正文。
- 所有响应携带 `X-Request-ID`；请求可提交合法值，否则服务端生成。
- 未认证返回 `401`，功能无权返回 `403`，资源在当前授权范围不可见时返回 `404`。
- 配置版本、幂等内容或资源状态冲突返回 `409`。
- 未知内部错误返回通用 `500`，真实原因只写入已脱敏 tracing。

## 认证与会话

- 浏览器会话使用 `HttpOnly`、`Secure`、`SameSite=Lax` cookie；本地开发可显式关闭 `Secure`。
- 登录成功后轮换 session ID；登出、密码重置和用户停用撤销相关会话。
- 状态变更请求使用同步 token 模式校验 CSRF；登录与初始化接口同时校验 `Origin`。
- `GET /api/v1/setup` 返回 `setup_required` 和 `setup_enabled`，不返回 setup token 或其摘要。
- 客户端恢复已有 Cookie 会话后，通过 `POST /api/v1/auth/csrf` 获取新 CSRF token。该请求不要求旧 token，但必须同时通过有效 session、与 `DEPLOY_GO_ALLOWED_ORIGIN` 完全一致的 `Origin`、`Sec-Fetch-Site: same-origin` 以及 `Sec-Fetch-Mode: cors|same-origin` 校验。
- CSRF refresh 为当前 session 签发新的 token，并保留最多 32 个 session 内有效 token，以支持多个标签页独立恢复。超出上限时淘汰最早签发的 token；登出、session 过期、用户停用或密码重置后全部失效。
- Flutter 使用同一 Cookie 会话协议，从构建配置读取允许 Origin，并显式发送 `Origin` 与 Fetch Metadata；Cookie 和 CSRF token 只进入平台安全存储。
- 密码、session token、CSRF token 和初始化 token 不得进入日志、错误、审计详情或 OpenAPI 示例。

### Agent 身份

- Agent 身份与用户 Cookie 会话完全隔离；Agent 注册、刷新和 WebSocket 端点不接受用户 Cookie，管理端点不接受 Agent token。
- enrollment token 只允许注册预先创建并绑定节点的 Agent，30 分钟内一次性使用；响应和持久化只保留用途隔离摘要，不记录明文。
- Agent 使用独立 rolling refresh token 换取 30 分钟 access token；WebSocket 握手和同连接续期只接受 access token。
- WebSocket access token 使用 `Authorization` header 或认证消息传递，不放入 URL、query、日志、审计详情或 OpenAPI 示例。
- refresh token 每次使用后滚动更新；新凭证经 Agent 确认后撤销旧凭证，确认后的旧 token 重用会撤销该 Agent 凭证族。
- 管理员撤销 Agent 时必须关闭当前连接，并撤销 enrollment、access 和 refresh 凭证。

## 客户端账号契约

- `GET/PATCH /api/v1/auth/profile` 读取或更新当前用户资料；首版只允许修改 `display_name`，不得通过该接口提交 `identity`、状态或应用授权。
- `GET/PUT /api/v1/auth/preferences` 持久化部署失败、部署完成、异常节点、时间格式和日志跟随偏好；更新携带 `version`，冲突返回 `resource_version_conflict`。
- setup 和管理员创建用户可提交可选 `display_name` 与 `email`；登录接受 username 或唯一 email，email 只允许通过管理员创建流程设置。
- `GET /api/v1/users/{id}` 与 `GET /api/v1/users/{id}/applications` 仅管理员可访问，分别用于用户详情与显式应用授权集合。
- profile 和 preferences 更新必须审计，但审计摘要只记录字段名，不记录 token、密码或其他敏感值。

## 分页、排序与筛选

- 列表使用游标分页，参数为 `limit` 和 `after`。
- `limit` 默认 50，最小 1，最大 200。
- 响应包含 `items` 和可空的 `next_cursor`。
- OpenAPI 中每个列表必须引用显式的 `*ListResponse` schema，不允许只声明 `200` 而省略响应 body。
- 每个资源定义稳定默认排序；游标包含完整排序键。
- 排序和筛选字段使用路由白名单，未知字段返回 `validation_failed`。
- 空结果返回 `200` 和空 `items`。

## 幂等与并发

- 部署确认等可重试创建操作要求 `Idempotency-Key`，长度为 16 至 128 个 ASCII 字符。
- 服务端保存调用者、路由、请求摘要、响应资源和有效期。
- 相同调用者、路由和键的相同请求返回原结果；摘要不同返回 `409 idempotency_conflict`。
- 资源更新携带 `version`；版本不一致返回 `409 resource_version_conflict`。
- 部署预览返回 `snapshot_hash`，确认时必须提交；目标配置变化返回 `409 deployment_snapshot_changed`。

## SSE 日志

- 日志端点响应 `text/event-stream`，事件 ID 使用部署内单调递增日志序号。
- 客户端通过 `Last-Event-ID` 或 `after` 续传，两者同时存在时必须一致。
- 服务端先发送已持久化记录，再订阅实时广播，并补查切换窗口避免丢行。
- 心跳使用注释行，不创建业务游标。
- 部署结束后发送终态事件并关闭；连接断开不改变部署状态。
- 过旧或非法游标返回稳定错误，不从任意位置静默重放。

## 审计

- 审计记录是追加式数据，至少包含 actor、action、resource type、resource ID、时间、request ID 和脱敏摘要。
- 用户、SSH 凭证、节点、应用、部署目标、部署操作和系统设置变更必须审计。
- Agent 创建、安装命令重生成、凭证重用阻断、撤销和重新绑定必须审计，但审计中不得出现 token 或完整安装命令。
- 审计摘要不得保存密码、token、私钥、敏感文件内容或未脱敏参数。

## 版本与兼容

- 首版固定 `/api/v1`。
- 每个 OpenAPI operation 必须具有稳定且全局唯一的 `<domain>_<action>` operationId；所有 4xx、502 和 503 响应引用统一 `ErrorResponse`。
- 新增可选响应字段视为兼容变更；删除、改名、改变类型或状态语义必须进入新版本。
- 客户端忽略未知响应字段；服务端默认拒绝未知请求字段，除非 schema 明确允许扩展。
