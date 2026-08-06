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
- `/opt/deploy-go` 由 `root` 管理，`deploy-go` 只能读取和执行；运行数据只写入权限为 `0750 deploy-go:deploy-go` 的 `/var/lib/deploy-go`。
- 每次部署使用独立的本地临时目录，以及 `/var/lib/deploy-go-installer` 下由 `root` 创建的随机 staging；部署参数通过 `0600 root:root` 的 `install.env` 传入，不拼接到 SSH 命令。
- 安装器通过固定锁拒绝并发部署，并在服务重启或健康检查失败时恢复上一版产物、配置和 systemd unit。
- API 默认直接监听 `0.0.0.0:30100`，建议在云安全组或防火墙只放行 `30101`，或为 API 前置 HTTPS 代理后再对外暴露。
- 首次部署会在服务器生成主密钥文件 `/etc/deploy-go/master.key`，权限 `0400 deploy-go:deploy-go`；API unit 通过 `ProtectSystem=strict` 与 `ReadOnlyPaths` 强制只读，不会输出密钥内容。
- 已有主密钥为空、为符号链接或非普通文件时，安装器会停止并要求人工恢复，不会自动生成新密钥覆盖异常状态。
- Web 是纯 HTTP；如对外提供正式访问，应前置 HTTPS 反向代理，并把 `DEPLOY_GO_COOKIE_SECURE=true`。
- 详细步骤、配置项与恢复方式见 `docs/runbooks/systemd-deployment-qfy-test.md`。
