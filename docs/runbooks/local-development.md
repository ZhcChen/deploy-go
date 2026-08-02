# 本地开发环境

## 适用范围

本手册用于启动和验证本地 Rust API。它不授权连接真实节点或执行远程部署脚本。

## 前置条件

- Rust 1.94.0，包含 rustfmt 和 clippy。
- Node.js 与 Python 3，用于既有 UI 设计源检查。
- SQLite 由 SQLx 内置依赖提供，不要求单独启动数据库服务。

## 配置

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `DEPLOY_GO_BIND_ADDR` | `127.0.0.1:8080` | API 监听地址 |
| `DEPLOY_GO_DATABASE_URL` | `sqlite://deploy-go.db` | SQLite URL |
| `DEPLOY_GO_SETUP_TOKEN` | 未设置 | 一次性管理员初始化 token；完成初始化后应移除并重启服务 |
| `DEPLOY_GO_ALLOWED_ORIGIN` | `http://localhost` | 初始化、登录与 CSRF refresh 请求允许的精确 Origin；Flutter 构建配置使用同一值 |
| `DEPLOY_GO_COOKIE_SECURE` | `true` | 是否为 session cookie 添加 `Secure`；仅纯 HTTP 本地开发可设为 `false` |
| `DEPLOY_GO_MASTER_KEY_VERSION` | 无 | 当前 SSH 凭证主密钥的正整数版本，服务模式必填 |
| `DEPLOY_GO_MASTER_KEY` | 无 | Base64 编码的 32 字节当前主密钥，与 `_FILE` 二选一 |
| `DEPLOY_GO_MASTER_KEY_FILE` | 无 | 保存当前主密钥的 `0600` 普通文件路径，与直接值二选一 |
| `DEPLOY_GO_PREVIOUS_MASTER_KEY_VERSION` | 无 | 轮换期间的上一版本；必须与上一主密钥同时提供 |
| `DEPLOY_GO_PREVIOUS_MASTER_KEY` | 无 | Base64 编码的上一主密钥，与 `_FILE` 二选一 |
| `DEPLOY_GO_PREVIOUS_MASTER_KEY_FILE` | 无 | 保存上一主密钥的 `0600` 普通文件路径 |
| `RUST_LOG` | `info` | tracing 过滤级别 |

本地 `.env` 不会自动加载，也不得提交。通过当前 shell 显式导出配置。

首次初始化前至少设置随机的 `DEPLOY_GO_SETUP_TOKEN`。初始化接口成功后移除该变量并重启 API，避免继续保留初始化凭据。

Web 和 Flutter 恢复 Cookie 会话后调用 `POST /api/v1/auth/csrf` 签发新的 CSRF token。请求必须显式发送允许的 `Origin`、`Sec-Fetch-Site: same-origin` 与 `Sec-Fetch-Mode: cors`；不得把返回 token 写入日志、普通首选项或 fixture。

## 双端 API client

`api/openapi/openapi.json` 是 Web 与 Flutter 唯一的 API 代码生成输入。首次生成前需要安装 Node.js 22 或更高版本、Java 21、Flutter 3.41.5，并在仓库根目录执行：

```bash
npm ci
make api-client-generate
```

生成结果分别位于 `admin/src/api/generated/` 和 `admin-app/lib/api/generated/`，禁止手工修改。OpenAPI 变化时必须同时提交两端生成结果；提交前执行：

```bash
make api-openapi-check
make api-client-check
```

`make api-client-check` 会在临时目录重新生成并逐文件比较，不修改工作区。若失败，先运行 `make api-client-generate`，不要直接修补 generated 文件。

服务模式必须配置 SSH 凭证主密钥。可使用 `openssl rand -base64 32` 生成主密钥；不得把输出写入仓库、命令历史或普通日志。`make api-migrate` 不读取主密钥。

## 启动

```bash
make api-run
```

验证：

```bash
curl --fail http://127.0.0.1:8080/healthz
curl --fail http://127.0.0.1:8080/readyz
curl --fail http://127.0.0.1:8080/api/v1/openapi.json
```

`healthz` 只证明进程可响应。`readyz` 同时执行 SQLite 查询，数据库不可用时返回 `503`。

API 启动后同时运行进程内部署 worker。worker 只领取 SQLite 中的 queued 任务；同一目标串行执行，全局并发由系统设置控制。服务重启的状态语义见 `docs/runbooks/deployment-recovery.md`。

## 检查

```bash
make api-check
make api-openapi-check
make ui-check
make check
```

`make api-check` 依次执行 Rust 格式、clippy、workspace 测试和 OpenAPI 漂移检查。修改 API 契约后运行 `make api-openapi` 更新 `api/openapi/openapi.json`。`make check` 额外执行 UI 设计源检查。

## 停止与清理

前台运行时发送 Ctrl+C。服务收到 SIGINT 或 SIGTERM 后停止接收新连接，并等待当前 HTTP 请求结束。

本地数据库文件默认为 `deploy-go.db`，同时可能存在 `-shm` 和 `-wal` 文件。只有确认不需要本地数据时才删除这些文件；不得把本地清理命令用于共享或远程环境。
