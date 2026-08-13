# 正式环境 systemd 部署

正式域名为 `https://deploy.quanxinfu.com`，`qfy-test` 是本机 SSH config 中指向
Deploy Go 正式控制面服务器的连接别名。业务节点机器（例如 `qfy-prod-1`）不是
Deploy Go 正式控制面，禁止作为 `DEPLOY_HOST` 执行本部署脚本；如确有特殊需求，
必须先获得用户对该节点的明确授权。
部署前先确认 `ssh <alias> 'hostname; systemd-detect-virt'` 的目标身份；
目标不是 Deploy Go 正式控制面时不得继续部署。

## 服务

| systemd 服务 | 端口 | 说明 |
| --- | --- | --- |
| `deploy-go-api` | `30100` | Rust API，SQLite 与 Agent 发布目录在 `/var/lib/deploy-go` |
| `deploy-go-web` | `30101` | Web 静态站点，并把 `/api` 代理到本机 API |

## 使用

默认从当前源码本地构建（Docker 构建 Linux API 与 Agent，`npm` 构建 Web）：

```bash
bash deploy/production/deploy.sh
```

Agent 二进制由部署脚本在本机编译（x86_64 + aarch64）并随 staging 上传，不再依赖 GitHub Release 下载。

正式部署前可先在部署机单独构建并校验 Agent release，不连接服务器：

```bash
make deploy-production-agent-build
```

该命令在本机 Docker 构建 Agent/executor 双架构产物并生成 manifest，输出到
`target/deploy-release/agent`；之后执行 `make deploy-production` 会复用本机构建缓存。
Deploy Go 正式控制面服务器 `qfy-test` 只作为安装目标，不作为构建节点。

使用 GitHub Release 产物获取 API/Web 时：

```bash
DEPLOY_SOURCE=release \
DEPLOY_RELEASE_TAG=v0.1.0 \
bash deploy/production/deploy.sh
```

注意：当前 GitHub Actions 构建发布配置已注释，`release` 模式仅适用于已有 Release 产物的场景。

也可以使用 `make deploy-production`，并通过环境变量覆盖配置。

## 安全边界

- API 和 Web 都运行在低权限 `deploy-go` 用户下，启用 systemd 基础隔离。
- `/opt/deploy-go` 由 `root` 管理，`deploy-go` 只能读取和执行；运行数据只写入权限为 `0750 deploy-go:deploy-go` 的 `/var/lib/deploy-go`。
- 每次部署使用独立的本地临时目录，以及 `/var/lib/deploy-go-installer` 下由 `root` 创建的随机 staging；部署参数通过 `0600 root:root` 的 `install.env` 传入，不拼接到 SSH 命令。
- 安装器通过固定锁拒绝并发部署，并在服务重启或健康检查失败时恢复上一版产物、配置和 systemd unit。
- API 与 Web 默认仅监听服务器 loopback，由现有 HTTPS 反向代理对外提供正式域名。
- 首次部署会在服务器生成主密钥文件 `/etc/deploy-go/master.key`，权限 `0400 deploy-go:deploy-go`；API unit 通过 `ProtectSystem=strict` 与 `ReadOnlyPaths` 强制只读，不会输出密钥内容。
- 已有主密钥为空、为符号链接或非普通文件时，安装器会停止并要求人工恢复，不会自动生成新密钥覆盖异常状态。
- 首次部署还会在服务器独立生成特权发布签名密钥 `/etc/deploy-go/release-signing.key`，权限 `0440 root:deploy-go`；它与终端签名密钥分离，只通过文件路径注入 API，重复部署复用，异常文件拒绝覆盖并纳入安装回滚。
- Web 的服务器内部链路是纯 HTTP，对外统一使用 `https://deploy.quanxinfu.com`，session cookie 强制启用 `Secure`。
- 详细步骤、配置项与恢复方式见 `docs/runbooks/systemd-deployment-production.md`。
