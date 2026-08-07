# 正式环境 systemd 部署

## 适用范围

本手册用于把 Deploy Go API 与 Web 管理端部署到唯一的正式环境服务器，使用 systemd 管理服务。正式域名是 `https://deploy.quanxinfu.com`，`qfy-test` 只是 SSH config 中的服务器别名，不代表测试环境。执行远程部署、重启服务或修改服务器配置前，必须在当前对话中获得针对该节点的明确授权。

## 拓扑

```text
浏览器
  -> https://deploy.quanxinfu.com  HTTPS 反向代理
  -> http://127.0.0.1:30101  deploy-go-web（Python ThreadingHTTPServer）
       / 静态 SPA（BrowserRouter fallback）
       /api/* -> http://127.0.0.1:30100  deploy-go-api
```

| systemd 服务 | 监听 | 数据 |
| --- | --- | --- |
| `deploy-go-api` | `127.0.0.1:30100` | `/var/lib/deploy-go/deploy-go.db`、`/var/lib/deploy-go/agent-releases`、`/var/lib/deploy-go/artifacts` |
| `deploy-go-web` | `127.0.0.1:30101` | `/opt/deploy-go/web` 静态文件 |

## 前置条件

- 正式服务器已配置为本机 SSH alias `qfy-test`，root 可登录。
- 服务器有 Python 3、`curl`、`openssl`、`rsync` 与 systemd；Agent release 校验使用 Python 3，不需要安装 `jq`。
- 本机有 `ssh`、`rsync`、`curl`；`build` 模式还需要 Docker、Node.js 22。
- `DEPLOY_AGENT_SYNC` 默认开启，本机还需 Docker（用于编译 Linux Agent 双架构）。
- `DEPLOY_SOURCE=release` 时，GitHub Release 需已包含对应 tag 的 API 与 Web 产物。

## 部署步骤

### 1. 构建模式（当前源码）

```bash
bash deploy/production/deploy.sh
```

脚本会：

1. 通过 SSH alias `qfy-test` 读取正式服务器架构并确定构建平台。
2. 用 Docker 构建 `deploy-go-api` Linux 二进制，并用 Docker 本机编译 Agent 的 x86_64 与 aarch64 二进制，生成 manifest 与 systemd unit。
3. 执行 `npm ci` 与 Web 生产构建，并扫描敏感内容。
4. 创建本地随机 staging，并在 `/var/lib/deploy-go-installer` 下创建仅 `root` 可写的随机远端 staging。
5. 把部署参数写入 `0600 root:root` 的 `install.env` 后随产物上传，SSH 命令不携带参数值。
6. 取得 `/run/lock/deploy-go-install.lock` 安装锁；已有安装任务时立即停止。
7. 以 `root` 管理 `/opt/deploy-go`，只把运行数据目录交给 `deploy-go` 写入。
8. 备份上一版产物、配置和 unit，再安装主密钥、环境文件和两个 systemd unit。
9. 安装 staging 中本机构建的 Agent release 到 `/var/lib/deploy-go/agent-releases/<版本>`。
10. 启用并重启服务，验证 `/healthz`、`/readyz`、Web 首页和 `/api` 代理；失败时在锁内恢复备份并重启旧服务。

### 2. Release 模式（GitHub Release 获取 API/Web）

先创建并推送 `v0.1.0` tag（当前 `Build Release Artifacts` 已注释，需要已有或手动准备 API/Web Release 产物；Agent 始终由部署机本机构建）：

```bash
DEPLOY_SOURCE=release \
DEPLOY_RELEASE_TAG=v0.1.0 \
bash deploy/production/deploy.sh
```

脚本会校验 API 与 Web 的 SHA-256；Agent 仍由本机 Docker 编译后随 staging 上传。

### 3. 常用配置

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `DEPLOY_HOST` | `qfy-test` | 正式服务器的 SSH alias |
| `DEPLOY_SOURCE` | `build` | `build` 或 `release` |
| `DEPLOY_RELEASE_TAG` | 空 | release 模式必填，必须等于 `v<API 版本>` |
| `DEPLOY_API_PORT` | `30100` | API 本机监听端口 |
| `DEPLOY_API_BIND` | `127.0.0.1` | API 仅本机监听，由 Web 代理访问 |
| `DEPLOY_WEB_PORT` | `30101` | Web 对外端口 |
| `DEPLOY_WEB_BIND` | `127.0.0.1` | Web 仅本机监听，由 HTTPS 反向代理访问 |
| `DEPLOY_GO_COOKIE_SECURE` | `true` | 正式环境 session cookie 强制使用 `Secure` |
| `DEPLOY_GO_MASTER_KEY_VERSION` | `1` | 主密钥版本 |
| `DEPLOY_GO_PUBLIC_BASE_URL` | `https://deploy.quanxinfu.com` | Agent 安装与发布链接的正式 HTTPS origin |
| `DEPLOY_GO_ALLOWED_ORIGIN` | `https://deploy.quanxinfu.com` | API 允许的正式 Web Origin |
| `DEPLOY_AGENT_SYNC` | `1` | 是否在部署机本机构建并上传 Agent release；设为 `0` 可跳过 |
| `DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED` | `true` | 正式环境必须启用跨节点制品通道 |
| `DEPLOY_GO_ARTIFACTS_ROOT` | `/var/lib/deploy-go/artifacts` | 固定在 systemd 可写数据目录内，不允许改到其他目录 |
| `DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES` | `536870912` | 单文件上限 512 MiB |
| `DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES` | `2147483648` | 单次制品总量上限 2 GiB |
| `DEPLOY_GO_ARTIFACT_MAX_FILES` | `256` | 单次制品文件数上限 |
| `DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES` | `8388608` | 上传 chunk 上限 8 MiB |
| `DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS` | `1800` | 未完成上传 lease 有效期 |
| `DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS` | `86400` | 无活动发布引用的已验证制品保留期 |

Python Web 代理以 64 KiB 固定缓冲转发 `Content-Length` 或 chunked 请求，不会按制品总大小缓存请求体。代理拒绝同时携带两种 framing、非法 chunk 和超过 2 GiB 的请求体；最终文件数与文件大小仍由 API manifest 校验。

## 首次初始化

服务启动后访问 `https://deploy.quanxinfu.com`。空库首次访问会进入唯一管理员初始化，完成后 Setup 入口自动关闭。

## Agent 特权终端

主控部署只提供兼容协议和管理入口，不会自动开启任何节点的特权执行。Agent 与 executor 必须按 manifest v2 成对安装，先在非关键节点证明普通部署兼容，再由管理员逐节点启用。

启用、验证、停用和版本回退必须遵循 `docs/runbooks/privileged-agent-terminal.md`。不得把部署主控或升级 Agent 视为启用 root 终端的授权。

## 验证

```bash
systemctl status deploy-go-api deploy-go-web
systemctl is-active deploy-go-api deploy-go-web
curl --fail http://127.0.0.1:30100/readyz
curl --fail http://127.0.0.1:30101/
curl --fail http://127.0.0.1:30101/api/v1/openapi.json
curl --fail https://deploy.quanxinfu.com/
curl --fail https://deploy.quanxinfu.com/api/v1/openapi.json
systemctl show deploy-go-api -p ReadWritePaths -p StateDirectory
sudo -u deploy-go test -w /var/lib/deploy-go/artifacts
du -sh /var/lib/deploy-go/artifacts
```

## 日志与排障

```bash
journalctl -u deploy-go-api --since '30 minutes ago' --no-pager
journalctl -u deploy-go-web --since '30 minutes ago' --no-pager
```

常见问题：

- API 启动失败：查看 `/etc/deploy-go/api.env` 是否只有受控配置、主密钥文件是否 `0400 deploy-go:deploy-go`，以及 unit 是否包含 `ProtectSystem=strict` 和对应的 `ReadOnlyPaths`。
- 制品存储启动失败：确认 `/var/lib/deploy-go/artifacts` 不是符号链接、属于 `deploy-go:deploy-go`，并位于 unit 的 `ReadWritePaths=/var/lib/deploy-go` 内；不要通过放宽到任意系统目录解决。
- 上传经过 Web 代理失败：确认外层 HTTPS 代理允许 request streaming，且没有低于 `DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES` 的 body limit；Deploy Go Python 代理自身保持有界内存并支持 chunked。
- 提示已有安装任务：检查是否确有部署正在执行；不要删除锁文件绕过，确认无安装进程后再重试。
- 主密钥异常：若文件为空、为符号链接或非普通文件，安装器会拒绝继续。应从可信备份恢复原密钥，不能直接重新生成。
- 检测到未完成部署：说明上次安装可能被 `SIGKILL`、掉电或主机重启中断。不要再次部署覆盖现场；根据提示的 `.rollback.*` 目录核对并恢复产物、环境文件和 unit，确认旧服务健康后再移走该目录。
- Web 刷新 404：确认运行的是 `deploy/production/web_server.py`，而不是 `ui/serve.py`。
- `/api` 502：确认 `deploy-go-api` active，且 `web_server.py --api` 指向 `127.0.0.1:30100`。
- Agent 进程正常但节点持续离线：使用 WebSocket Upgrade 请求检查 `/api/v1/agent/control`；生产 `web_server.py` 必须保留 `Connection`、`Upgrade` 和 `Authorization` 并建立双向隧道，不能把控制连接当作普通 HTTP 请求转发。
- Agent 安装命令不可用：需要配置 HTTPS 的 `DEPLOY_GO_PUBLIC_BASE_URL`，并已通过部署脚本安装本机构建的 Agent release。
- API 与 Web 默认仅监听 loopback；正式域名的 HTTPS/WSS 终止与转发由服务器现有反向代理负责。

## 回滚

部署脚本不会覆盖已有数据库和主密钥。安装期间的服务重启或健康检查失败会自动恢复上一版产物、环境文件和 unit。需要人工回滚更早版本时，把旧 API 二进制或旧 Web 目录放回并重启：

```bash
install -m 0550 -o root -g deploy-go /path/to/old-deploy-go-api /opt/deploy-go/api/deploy-go-api
systemctl restart deploy-go-api
systemctl restart deploy-go-web
```

数据库迁移只能前进。如果新版本 migration 已执行，旧二进制可能无法启动；此时应先确认备份与恢复路径，不能直接依赖二进制回滚。

数据库备份不能替代制品目录备份。需要保留仍可重试的历史发布时，应在停止 API 后对 SQLite、`artifacts/objects` 与 `artifacts/quarantine` 做同一时点快照；恢复时保持原路径和所有者，再启动 API 让 reconciliation 核对数据库与文件事实。过期制品属于缓存，不应作为业务应用唯一发布物来源。
