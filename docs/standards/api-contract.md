---
date: 2026-07-31
topic: api-contract
status: accepted
version: 1
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
  "field_errors": {
    "name": ["名称不能为空"]
  }
}
```

- `code` 是稳定的机器标识，客户端不得解析 `message` 判断业务分支。
- `message` 不包含内部 SQL、命令、路径、密钥或堆栈。
- `field_errors` 只在字段校验失败时出现。
- 所有响应携带 `X-Request-ID`；请求可提交合法值，否则服务端生成。
- 未认证返回 `401`，功能无权返回 `403`，资源在当前授权范围不可见时返回 `404`。
- 配置版本、幂等内容或资源状态冲突返回 `409`。
- 未知内部错误返回通用 `500`，真实原因只写入已脱敏 tracing。

## 认证与会话

- 浏览器会话使用 `HttpOnly`、`Secure`、`SameSite=Lax` cookie；本地开发可显式关闭 `Secure`。
- 登录成功后轮换 session ID；登出、密码重置和用户停用撤销相关会话。
- 状态变更请求使用同步 token 模式校验 CSRF；登录与初始化接口同时校验 `Origin`。
- Flutter 可以使用同一 cookie 会话协议，客户端凭证存储由 `admin-app/` 实施计划确定。
- 密码、session token、CSRF token 和初始化 token 不得进入日志、错误、审计详情或 OpenAPI 示例。

## 分页、排序与筛选

- 列表使用游标分页，参数为 `limit` 和 `after`。
- `limit` 默认 50，最小 1，最大 200。
- 响应包含 `items` 和可空的 `next_cursor`。
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
- 审计摘要不得保存密码、token、私钥、敏感文件内容或未脱敏参数。

## 版本与兼容

- 首版固定 `/api/v1`。
- 新增可选响应字段视为兼容变更；删除、改名、改变类型或状态语义必须进入新版本。
- 客户端忽略未知响应字段；服务端默认拒绝未知请求字段，除非 schema 明确允许扩展。
