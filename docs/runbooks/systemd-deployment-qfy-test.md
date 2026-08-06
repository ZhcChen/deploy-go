# qfy-test systemd 部署

## 适用范围

本手册用于把 Deploy Go API 与 Web 管理端部署到 `qfy-test` 服务器，使用 systemd 管理服务。执行远程部署、重启服务或修改服务器配置前，必须在当前对话中获得针对该节点的明确授权。

## 拓扑

```text
浏览器
  -> http://<qfy-test>:30101  deploy-go-web（Python ThreadingHTTPServer）
       / 静态 SPA（BrowserRouter fallback）
       /api/* -> http://127.0.0.1:30100  deploy-go-api
```

| systemd 服务 | 监听 | 数据 |
| --- | --- | --- |
| `deploy-go-api` | `0.0.0.0:30100` | `/var/lib/deploy-go/deploy-go.db`、`/var/lib/deploy-go/agent-releases` |
| `deploy-go-web` | `0.0.0.0:30101` | `/opt/deploy-go/web` 静态文件 |

## 前置条件

- `qfy-test` 已配置在本机 SSH config 中，root 可登录。
- 服务器有 Python 3、`curl`、`openssl`、`rsync` 与 systemd。
- 本机有 `ssh`、`rsync`、`curl`；`build` 模式还需要 Docker、Node.js 22。
- `DEPLOY_SOURCE=release` 时，GitHub Release 已包含对应 tag 的 API、Web 和 Agent 产物。

## 部署步骤

### 1. 构建模式（当前源码）

```bash
bash deploy/qfy-test/deploy.sh
```

脚本会：

1. 读取 `qfy-test` 架构并确定构建平台。
2. 用 Docker 构建 `deploy-go-api` Linux 二进制。
3. 执行 `npm ci` 与 Web 生产构建，并扫描敏感内容。
4. 创建本地随机 staging，并在 `/var/lib/deploy-go-installer` 下创建仅 `root` 可写的随机远端 staging。
5. 把部署参数写入 `0600 root:root` 的 `install.env` 后随产物上传，SSH 命令不携带参数值。
6. 取得 `/run/lock/deploy-go-install.lock` 安装锁；已有安装任务时立即停止。
7. 以 `root` 管理 `/opt/deploy-go`，只把运行数据目录交给 `deploy-go` 写入。
8. 备份上一版产物、配置和 unit，再安装主密钥、环境文件和两个 systemd unit。
9. 启用并重启服务，验证 `/healthz`、`/readyz`、Web 首页和 `/api` 代理；失败时在锁内恢复备份并重启旧服务。

### 2. Release 模式（GitHub Release）

先创建并推送 `v0.1.0` tag，等待 `Build Release Artifacts` 工作流完成：

```bash
DEPLOY_SOURCE=release \
DEPLOY_RELEASE_TAG=v0.1.0 \
DEPLOY_AGENT_SYNC=1 \
bash deploy/qfy-test/deploy.sh
```

脚本会校验 API 与 Web 的 SHA-256，再把当前 API 版本对应的 Agent release 同步到 `/var/lib/deploy-go/agent-releases`。

### 3. 常用配置

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `DEPLOY_HOST` | `qfy-test` | SSH 目标 |
| `DEPLOY_SOURCE` | `build` | `build` 或 `release` |
| `DEPLOY_RELEASE_TAG` | 空 | release 模式必填，必须等于 `v<API 版本>` |
| `DEPLOY_API_PORT` | `30100` | API 本机监听端口 |
| `DEPLOY_API_BIND` | `0.0.0.0` | API 监听地址；默认对外暴露，可改为 `127.0.0.1` |
| `DEPLOY_WEB_PORT` | `30101` | Web 对外端口 |
| `DEPLOY_GO_COOKIE_SECURE` | `false` | HTTP 部署必须为 `false`；HTTPS 后设为 `true` |
| `DEPLOY_GO_MASTER_KEY_VERSION` | `1` | 主密钥版本 |
| `DEPLOY_GO_PUBLIC_BASE_URL` | 空 | 配置后启用 Agent 安装命令，必须为 HTTPS origin |
| `DEPLOY_GO_ALLOWED_ORIGIN` | 从 SSH hostname 推断 | API 允许的 Web Origin |
| `DEPLOY_AGENT_SYNC` | `release=1`，`build=0` | 是否同步 Agent release |

## 首次初始化

服务启动后访问 `http://<qfy-test>:30101`。空库首次访问会进入唯一管理员初始化，完成后 Setup 入口自动关闭。

## 验证

```bash
systemctl status deploy-go-api deploy-go-web
systemctl is-active deploy-go-api deploy-go-web
curl --fail http://127.0.0.1:30100/readyz
curl --fail http://127.0.0.1:30101/
curl --fail http://127.0.0.1:30101/api/v1/openapi.json
```

## 日志与排障

```bash
journalctl -u deploy-go-api --since '30 minutes ago' --no-pager
journalctl -u deploy-go-web --since '30 minutes ago' --no-pager
```

常见问题：

- API 启动失败：查看 `/etc/deploy-go/api.env` 是否只有受控配置、主密钥文件是否 `0400 deploy-go:deploy-go`，以及 unit 是否包含 `ProtectSystem=strict` 和对应的 `ReadOnlyPaths`。
- 提示已有安装任务：检查是否确有部署正在执行；不要删除锁文件绕过，确认无安装进程后再重试。
- 主密钥异常：若文件为空、为符号链接或非普通文件，安装器会拒绝继续。应从可信备份恢复原密钥，不能直接重新生成。
- 检测到未完成部署：说明上次安装可能被 `SIGKILL`、掉电或主机重启中断。不要再次部署覆盖现场；根据提示的 `.rollback.*` 目录核对并恢复产物、环境文件和 unit，确认旧服务健康后再移走该目录。
- Web 刷新 404：确认运行的是 `deploy/qfy-test/web_server.py`，而不是 `ui/serve.py`。
- `/api` 502：确认 `deploy-go-api` active，且 `web_server.py --api` 指向 `127.0.0.1:30100`。
- Agent 安装命令不可用：需要配置 HTTPS 的 `DEPLOY_GO_PUBLIC_BASE_URL`，并已同步 Agent release。
- API 默认直接监听 `0.0.0.0:30100`，请结合安全组或防火墙限制访问；Agent 接入仍要求 HTTPS/WSS，需要单独配置反向代理。

## 回滚

部署脚本不会覆盖已有数据库和主密钥。安装期间的服务重启或健康检查失败会自动恢复上一版产物、环境文件和 unit。需要人工回滚更早版本时，把旧 API 二进制或旧 Web 目录放回并重启：

```bash
install -m 0550 -o root -g deploy-go /path/to/old-deploy-go-api /opt/deploy-go/api/deploy-go-api
systemctl restart deploy-go-api
systemctl restart deploy-go-web
```

数据库迁移只能前进。如果新版本 migration 已执行，旧二进制可能无法启动；此时应先确认备份与恢复路径，不能直接依赖二进制回滚。
