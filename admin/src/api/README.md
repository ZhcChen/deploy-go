# Web API 边界

- `generated/` 由根目录 `make api-client-generate` 根据 `api/openapi/openapi.json` 生成，禁止手工修改。
- 业务代码通过本目录的适配层处理 Cookie、CSRF、统一错误、cursor 和 SSE，不直接散落 endpoint 或字段模型。
- OpenAPI 变化后先运行生成命令，再提交契约和双端生成产物。
