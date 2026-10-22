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
| 403 | `external_production_application_forbidden` | 对外 API 不允许编辑正式环境应用，也不允许修改其 Env、部署目标与部署来源；停止，改走管理面。 |
| 403 | `external_production_environment_forbidden` | 不允许把应用环境改为或创建为 `prod`；停止，确认目标环境，正式环境应用改走管理面。 |
| 403 | `external_production_deployment_forbidden` | 应用存在正式环境部署目标时禁止对外部署；停止，确认目标归属。 |
| 403 | `external_application_reactivation_forbidden` | 应用不是由此 Key 归档的（管理员或其他 Key 归档），不能经对外 API 重新启用；停止，让用户联系管理员。 |
| 404 | `not_found` | 资源不存在或当前 Key 无权限；重新执行 `list-apps` / `show-app` 确认。 |
| 404 | `application_workspace_source_not_configured` | 应用尚未配置固定工作区来源；确认是要新增还是改走 Git 来源。 |
| 409 | `resource_version_conflict` | 应用已被其他请求修改；重新读取 `version` 并让用户确认。 |
| 409 | `preview_expired` / `preview_already_confirmed` / `deployment_snapshot_changed` | 预览已过期、已被确认过，或发布版本、来源在预览后变化；重新 `preview-deploy`，不要重用旧 `snapshot_hash`。 |
| 409 | `application_archived` | 应用已归档，不能预览或部署；用户确认后先 `set-app-status --status active`。 |
| 409 | `source_not_configured` / `source_archived` | 应用尚未配置 Git 来源或来源已归档；先 `set-git-source`。 |
| 409 | `git_ref_discovery_missing` / `git_ref_discovery_expired` | 当前来源版本没有可用的分支发现结果或结果已过期；重新执行 `refresh-git-branches --wait`。 |
| 409 | `git_branch_not_found` | 分支不在最近一次发现结果中；让用户从发现列表中选择。 |
| 409 | `git_credential_unavailable` | 引用的 Git 凭证已停用（不存在时为 404）；用 `list-git-credentials` 重新选择。 |
| 409 | `git_ref_discovery_timeout` / `git_authentication_failed` / `git_repository_unreachable` | 分支发现超时或失败（部署时自动解析分支失败返回 409；`refresh-git-branches` 的发现记录以 `error_code` 呈现）；报告原因码，让用户检查凭证、仓库授权与构建节点后再重试。 |
| 409 | `env_file_already_registered` | 同名 Env 文件已登记；改用更新命令，不要重复登记。 |
| 409 | `env_file_referenced_by_image_target` | Env 文件被镜像目标引用；先移除目标 `image_spec` 中的引用再删除。 |
| 409 | `node_not_deployable` / `agent_offline` / `agent_protocol_unsupported` | 目标节点或构建 Agent 不可用；停止写操作，让用户先恢复节点。 |
| 409 | `application_slug_exists` / `application_identity_exists` | 名称或 slug 冲突；让用户更换后重试。 |
| 409 | `deployment_not_cancelable` / `deployment_snapshot_changed` / `idempotency_conflict` | 部署状态、快照或幂等键冲突；先执行 `status` 确认，不盲目重试。 |
| 422 | `validation_failed` | 参数 Schema、验证配置或字段格式不合法；按 `message` 修正。保存 Git 来源时若提示“对外 API 不接受该仓库地址”，说明地址协议不是 ssh/https、https 带了用户信息或主机是本机/内网 IP 字面量；改用域名地址或让管理员在管理面配置，不要尝试变形绕过。 |
| 500 | `internal_error` | 服务端异常；记录 `request_id`，不要自动重试写操作。 |

## 构建后的发布物交接失败

部署诊断中的 `artifact_transfer_failed` / `artifact_transfer_timeout` 表示构建完成后的上传失败或超时；`artifact_prepare_failed` 表示打包/校验失败，`artifact_authorization_failed` / `artifact_authorization_timeout` 表示上传授权失败，`artifact_transfer_unavailable` / `artifact_transfer_deadline_exceeded` 表示传输未配置或预算已耗尽。

即使脚本 `exit_code=0`，prepare 仍可能失败且不会进入 release。使用同一 ID 的 `diagnose` 查看任务 summary：其中 stage/category、HTTP 状态、attempts、elapsed_ms、confirmed_offset 及原始失败 cause 只包含安全白名单信息。未知字段或旧 Agent 缺详情时保留“未确认”，不能根据 offset0 推断未发送字节，也不能根据 connect 推断 DNS/TLS。不要尝试读取节点 journal、内部管理 API 或重发部署规避错误。

## 写操作重试原则

- 只读命令可以重试一次。
- `deploy`、`update-app`、`cancel` 以及 Env、部署目标、工作区来源、Git 来源（`set-git-source`、`refresh-git-branches`、`set-git-branch`）的写命令都属于写操作，服务端返回 4xx/5xx 后停止；除非用户确认前一次未成功，否则不自动重试。
- 不通过更换 `target_id`、环境或接口来绕过 403。
- 不输出或记录 `DEPLOY_GO_API_KEY`。
