# Flutter API 边界

- `generated/` 是独立的 Dart path package，由根目录 `make api-client-generate` 根据 OpenAPI 生成，禁止手工修改。
- `generated/pubspec.lock` 固定 build_runner 工具链；升级生成器或依赖时应显式评估并同步更新该 lockfile。
- Flutter 业务层负责 Dio/CookieJar、安全存储、CSRF、统一错误、cursor 和 SSE；不得在页面中手写 endpoint 或复制生成模型。
- 会话只能由业务层从安全存储恢复，并通过 Dio CookieJar 发送标准 `Cookie` header；生成流程已移除通用 API-key interceptor，禁止用 `setApiKey('cookieAuth', ...)` 注入会话。
- OpenAPI 变化后同时提交 Web 与 Flutter 生成产物。
