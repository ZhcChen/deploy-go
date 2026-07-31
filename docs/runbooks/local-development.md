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
| `DEPLOY_GO_ALLOWED_ORIGIN` | `http://localhost` | 初始化与登录请求允许的精确 Origin |
| `DEPLOY_GO_COOKIE_SECURE` | `true` | 是否为 session cookie 添加 `Secure`；仅纯 HTTP 本地开发可设为 `false` |
| `RUST_LOG` | `info` | tracing 过滤级别 |

本地 `.env` 不会自动加载，也不得提交。通过当前 shell 显式导出配置。

首次初始化前至少设置随机的 `DEPLOY_GO_SETUP_TOKEN`。初始化接口成功后移除该变量并重启 API，避免继续保留初始化凭据。

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

## 检查

```bash
make api-check
make ui-check
make check
```

`make api-check` 依次执行 Rust 格式、clippy 和 workspace 测试。`make check` 额外执行 UI 设计源检查。

## 停止与清理

前台运行时发送 Ctrl+C。服务收到 SIGINT 或 SIGTERM 后停止接收新连接，并等待当前 HTTP 请求结束。

本地数据库文件默认为 `deploy-go.db`，同时可能存在 `-shm` 和 `-wal` 文件。只有确认不需要本地数据时才删除这些文件；不得把本地清理命令用于共享或远程环境。
