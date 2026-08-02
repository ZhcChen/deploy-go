# 正式客户端行为契约

## 范围

`admin/` 和 `admin-app/` 使用同一份 OpenAPI 接口契约，并以 `test-fixtures/client-behavior.json` 作为错误、cursor 和授权撤销的跨端语义 fixture。fixture 不包含真实凭证、节点地址或部署 secret。

## 一致行为

- `401`：清理本地会话和账号级缓存，返回登录页；不渲染上一会话数据。
- `403`：清理对应受保护资源和日志，显示权限错误；不以客户端隐藏替代 API 授权。
- `409`：保留可排查错误与 Request ID，对 snapshot 冲突要求重新 preview。
- `422`：显示安全的字段校验信息，不回显密码、token 或 secret。
- `500`：显示稳定通用信息和 Request ID，不展示服务端堆栈或内部路径。
- 错误页面应允许复制 Request ID；复制内容只包含 Request ID。
- cursor 分页按资源 ID 去重；筛选变更重建 cursor 链，账号或授权变更清空旧缓存。
- SSE 日志仅按不可信纯文本渲染；授权撤销后清空部署详情、日志和 event ID。
- 未提交表单离开前明确确认；提交期间锁定重复操作，幂等重试复用原 key。

## 可访问性与移动端

- Web 关键流程可仅用键盘完成。确认 Modal 具有可访问名称、首焦点、焦点循环、Escape 关闭和触发器焦点恢复。
- Flutter 关键页面在 200% 系统字体下不裁切主操作，交互控件最小触控目标为 44 logical pixels，图标操作具有 Semantics label。

## 敏感数据

fixture、截图、日志和构建产物不得包含 `protected_values` 代表的真实值。SSH 私钥、主密钥、setup token、Cookie、CSRF token 和脚本 secret 不得进入普通存储、客户端日志或错误提示。
