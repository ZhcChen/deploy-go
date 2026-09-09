# 错误处理

CLI 失败时向 stderr 输出一行诊断，包含 `status`、`code`、`message` 和 `request_id`：

```text
请求失败 status=403 code=external_production_application_forbidden message=对外 API 不允许编辑正式环境应用 request_id=req_...
```

## 常见错误码

| HTTP | code | 含义与处理 |
| --- | --- | --- |
| 401 | `unauthorized` | API Key 缺失、无效、过期或已吊销；停止，让用户重新配置。 |
| 403 | `external_production_deployment_forbidden` | 对外 API 不允许发起正式环境部署；停止，改走管理面。 |
| 403 | `external_production_application_forbidden` | 对外 API 不允许编辑正式环境应用；停止，改走管理面。 |
| 403 | `external_production_environment_forbidden` | 不允许把应用环境改为 `prod`；停止，确认目标环境。 |
| 404 | `not_found` | 资源不存在或当前 Key 无权限；重新执行 `list-apps` / `show-app` 确认。 |
| 409 | `resource_version_conflict` | 应用已被其他请求修改；重新读取 `version` 并让用户确认。 |
| 409 | `application_slug_exists` / `application_identity_exists` | 名称或 slug 冲突；让用户更换后重试。 |
| 409 | `deployment_not_cancelable` / `deployment_snapshot_changed` / `idempotency_conflict` | 部署状态、快照或幂等键冲突；先执行 `status` 确认，不盲目重试。 |
| 422 | `validation_failed` | 参数 Schema、验证配置或字段格式不合法；按 `message` 修正。 |
| 500 | `internal_error` | 服务端异常；记录 `request_id`，不要自动重试写操作。 |

## 重试原则

- 只读命令可以重试一次。
- `deploy`、`update-app`、`cancel` 属于写操作，服务端返回 4xx/5xx 后停止；除非用户确认前一次未成功，否则不自动重试。
- 不通过更换 `target_id`、环境或接口来绕过 403。
- 不输出或记录 `DEPLOY_GO_API_KEY`。
