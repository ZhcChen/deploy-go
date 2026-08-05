# 本地开发环境

## 适用范围

本手册用于启动和验证本地 Rust API。它不授权连接真实节点或执行远程部署脚本。

## 前置条件

- Rust 1.94.0，包含 rustfmt 和 clippy。
- Node.js 22、Java 21、Python 3、Flutter 3.41.5（Dart 3.11.3）。
- Web E2E 首次执行前运行 `npx playwright install chromium`；CI 使用相同的 Chromium 工具链。
- SQLite 由 SQLx 内置依赖提供，不要求单独启动数据库服务。

## 配置

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `DEPLOY_GO_BIND_ADDR` | `127.0.0.1:30100` | API 监听地址 |
| `DEPLOY_GO_DATABASE_URL` | `sqlite://deploy-go.db` | SQLite URL |
| `DEPLOY_GO_ALLOWED_ORIGIN` | `http://localhost` | 单个允许的精确 Origin；未设置复数变量时使用，并供 Flutter 构建配置使用 |
| `DEPLOY_GO_ALLOWED_ORIGINS` | 未设置 | API 允许的逗号分隔 Origin 白名单；与单数变量同时设置时拒绝启动 |
| `DEPLOY_GO_COOKIE_SECURE` | `true` | 是否为 session cookie 添加 `Secure`；仅纯 HTTP 本地开发可设为 `false` |
| `DEPLOY_GO_PUBLIC_BASE_URL` | 未设置 | 生成 Agent 安装命令使用的可信 HTTPS origin；与 manifest 路径同时设置 |
| `DEPLOY_GO_AGENT_MANIFEST_PATH` | 未设置 | 当前主控兼容 manifest 的本地绝对路径；其父目录必须包含同一版本的 Agent 二进制与 systemd unit，供 API 下载路由使用 |
| `DEPLOY_GO_MASTER_KEY_VERSION` | 无 | 当前 SSH 凭证主密钥的正整数版本，服务模式必填 |
| `DEPLOY_GO_MASTER_KEY` | 无 | Base64 编码的 32 字节当前主密钥，与 `_FILE` 二选一 |
| `DEPLOY_GO_MASTER_KEY_FILE` | 无 | 保存当前主密钥的 `0600` 普通文件路径，与直接值二选一 |
| `DEPLOY_GO_PREVIOUS_MASTER_KEY_VERSION` | 无 | 轮换期间的上一版本；必须与上一主密钥同时提供 |
| `DEPLOY_GO_PREVIOUS_MASTER_KEY` | 无 | Base64 编码的上一主密钥，与 `_FILE` 二选一 |
| `DEPLOY_GO_PREVIOUS_MASTER_KEY_FILE` | 无 | 保存上一主密钥的 `0600` 普通文件路径 |
| `RUST_LOG` | `info` | tracing 过滤级别 |

本地 `.env` 不会自动加载，也不得提交。通过当前 shell 显式导出配置。

全新实例在空库状态下首次访问即可初始化唯一管理员；初始化成功后 `POST /api/v1/setup` 自动关闭，不会再次开放。

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

## Web 管理端

```bash
npm ci
make api-run
```

API 保持运行，在第二个终端执行：

```bash
make admin
make admin-test
make admin-check
make admin-build
make admin-test-e2e
```

`make api-run` 默认允许 `http://127.0.0.1:30101` 作为浏览器 Origin，并为本地纯 HTTP 联调关闭 session cookie 的 `Secure` 属性。`make admin` 默认在该地址启动 Vite 开发服务器，并将 `/api` 代理到 `http://127.0.0.1:30100`。可通过 `ADMIN_PORT=30103` 和 `ADMIN_API_PROXY_TARGET=http://127.0.0.1:30104` 覆盖；Vite 使用 `strictPort`，端口被占用时会直接报错，不会静默切换端口。修改 Web 端口时必须同步设置 API Origin，例如先运行 `make api-run DEPLOY_GO_ALLOWED_ORIGIN=http://127.0.0.1:30103`，再运行 `make admin ADMIN_PORT=30103`。

API 需要允许多个管理端时使用复数变量，例如：

```bash
make api-run DEPLOY_GO_ALLOWED_ORIGINS='http://127.0.0.1:30101,http://localhost:30101'
```

每项会去除首尾空白、标准化并去重，只接受不含凭证、路径、查询和 fragment 的 `http(s)` Origin；空项、通配符和 `*` 均会导致服务拒绝启动。Web 或 Flutter 客户端仍只发送自身的一个 Origin，Flutter 的 `DEPLOY_GO_ALLOWED_ORIGIN` 必须是 API 白名单中的成员。

`make admin-test` 使用 MSW fixture，不启动 API、不连接节点；`make admin-test-e2e` 使用 Playwright 路由 fixture 和隔离的本地 Vite，不执行真实部署。`make admin` 本身不启用 mock：需要交互联调时必须显式启动本地 API，页面操作只会在用户主动提交后调用 API。`DEPLOY_GO_COOKIE_SECURE=false` 只允许用于本地纯 HTTP 联调。

正式 Web 是纯客户端 SPA，使用 `BrowserRouter`，不启用 React Router RSC Mode、server action 或服务端运行时。当前 `react-router-dom@7.18.2` 的 npm high advisory 仅影响 RSC Mode；在升级到上游修复版本前不得开启这些服务端能力。

## Flutter 管理端

Flutter 管理端要求 Flutter 3.41.5（Dart 3.11.3）。依赖、检查和启动命令：

```bash
make admin-app-get
make admin-app-check
make admin-app-build
export DEPLOY_GO_API_BASE_URL=http://127.0.0.1:30100
export DEPLOY_GO_ALLOWED_ORIGIN=http://127.0.0.1:30101
make admin-app
```

`DEPLOY_GO_API_BASE_URL` 与 `DEPLOY_GO_ALLOWED_ORIGIN` 通过 `--dart-define` 编译进入当前本地构建；后者必须是 API Origin 允许列表中的成员。Android Emulator 访问宿主机时可将 API 地址改为 `http://10.0.2.2:30100`，但 Origin 仍使用 API 明确允许的值。不要把 Cookie、CSRF token 或主密钥放入 `--dart-define`。

App 使用 Dio/CookieJar 发送 HttpOnly session Cookie，CookieJar backend 与 CSRF token 都只写入 Android Keystore/iOS Keychain。Android 最低 API 24 且禁用应用备份；iOS 使用仅限当前设备的首次解锁 Keychain accessibility。恢复进程后先读取 Cookie，再调用 `POST /api/v1/auth/csrf` 更新 CSRF token；401 会清除本地会话并返回登录。

`make admin-app-check` 和 `make admin-app-test` 只使用内存 fixture 和隔离安全存储，不连接 API 或节点。`make admin-app-build` 默认构建 Android debug APK，产物位于 `admin-app/build/app/outputs/flutter-apk/app-debug.apk`；该产物仅用于本地验证，不代表生产签名发布包。

设备级安全存储与关键导航 smoke：

```bash
flutter devices
make admin-app-test-integration DEVICE_ID=<device-id>
```

该入口执行安全存储、关键导航和部署生命周期三组 smoke。分别在 Android Emulator 与 iOS Simulator 执行；测试只使用隔离安全存储值和内存业务 fixture，不连接 API 或真实节点。未提供 `DEVICE_ID` 时命令会直接给出用法并退出。

服务模式仍需配置主密钥，以读取和清理 migration 保留的 legacy SSH 凭证，并保护 Agent token 状态。可使用 `openssl rand -base64 32` 生成主密钥；不得把输出写入仓库、命令历史或普通日志。`make api-migrate` 不读取主密钥。

要在本地生成 Agent 安装命令，必须同时提供 `DEPLOY_GO_PUBLIC_BASE_URL` 与 `DEPLOY_GO_AGENT_MANIFEST_PATH`。实际节点接入和故障恢复分别遵循 `docs/runbooks/agent-onboarding.md` 与 `docs/runbooks/agent-recovery.md`；普通本地测试不需要连接 Agent。

## 启动

```bash
make api-run
```

验证：

```bash
curl --fail http://127.0.0.1:30100/healthz
curl --fail http://127.0.0.1:30100/readyz
curl --fail http://127.0.0.1:30100/api/v1/openapi.json
```

`healthz` 只证明进程可响应。`readyz` 同时执行 SQLite 查询，数据库不可用时返回 `503`。

API 启动后同时运行进程内部署 worker。worker 只把 SQLite 中的 queued 任务投递给在线 Agent；同一目标串行执行，全局并发由系统设置控制，不存在 SSH fallback。服务重启的状态语义见 `docs/runbooks/deployment-recovery.md`。

## 检查

```bash
make api-check
make api-openapi-check
make ui-check
make ui-test
make client-sensitive-check
make check
```

`make api-check` 依次执行 Rust 格式、clippy、workspace 测试和 OpenAPI 漂移检查。修改 API 契约后运行 `make api-openapi` 更新 `api/openapi/openapi.json`。`make check` 聚合 API、UI 静态检查、双端生成漂移、Web、Flutter 和客户端敏感模式扫描；需要浏览器或设备的 `make ui-test`、`make admin-test-e2e`、`make admin-app-test-integration` 保持为显式入口。

## 干净环境复演

```bash
npm ci
make admin-app-get
make api-client-check
make admin-test
make admin-build
make admin-app-test
make admin-app-build
make client-sensitive-check
make check
```

这些命令均不连接真实节点。不要把真实节点地址、Agent token、legacy SSH 凭证、Cookie、CSRF token、主密钥或脚本 secret 写入 fixture、构建参数或日志。

## 停止与清理

前台运行时发送 Ctrl+C。服务收到 SIGINT 或 SIGTERM 后停止接收新连接，并等待当前 HTTP 请求结束。

本地数据库文件默认为 `deploy-go.db`，同时可能存在 `-shm` 和 `-wal` 文件。只有确认不需要本地数据时才删除这些文件；不得把本地清理命令用于共享或远程环境。
