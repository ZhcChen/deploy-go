# qfy-test systemd 部署

## 服务

| systemd 服务 | 端口 | 说明 |
| --- | --- | --- |
| `deploy-go-api` | `30100` | Rust API，SQLite 与 Agent 发布目录在 `/var/lib/deploy-go` |
| `deploy-go-web` | `30101` | Web 静态站点，并把 `/api` 代理到本机 API |

## 使用

默认从当前源码本地构建（Docker 构建 Linux API，`npm` 构建 Web）：

```bash
bash deploy/qfy-test/deploy.sh
```

使用 GitHub Release 产物并同步当前版本 Agent：

```bash
DEPLOY_SOURCE=release \
DEPLOY_RELEASE_TAG=v0.1.0 \
DEPLOY_AGENT_SYNC=1 \
bash deploy/qfy-test/deploy.sh
```

也可以使用 `make deploy-qfy-test`，并通过环境变量覆盖配置。

## 安全边界

- API 和 Web 都运行在低权限 `deploy-go` 用户下，启用 systemd 基础隔离。
- API 默认直接监听 `0.0.0.0:30100`，建议在云安全组或防火墙只放行 `30101`，或为 API 前置 HTTPS 代理后再对外暴露。
- 首次部署会在服务器生成主密钥文件 `/etc/deploy-go/master.key`，权限 `0640 root:deploy-go`；不会输出密钥内容。
- Web 是纯 HTTP；如对外提供正式访问，应前置 HTTPS 反向代理，并把 `DEPLOY_GO_COOKIE_SECURE=true`。
- 详细步骤、配置项与恢复方式见 `docs/runbooks/systemd-deployment-qfy-test.md`。
