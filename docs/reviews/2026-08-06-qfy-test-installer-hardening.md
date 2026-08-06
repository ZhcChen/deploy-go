# 正式环境安装器安全加固复核

## 范围

- `deploy/production/deploy.sh`
- `deploy/production/install.sh`
- `deploy/production/test-install-contract.sh`
- `Makefile`
- `deploy/production/README.md`
- `docs/runbooks/systemd-deployment-production.md`

## 关键决策

- 本地和远端 staging 每次部署独立创建；远端路径固定在 root 专用的 `/var/lib/deploy-go-installer` 下，不允许调用方指定最终目录。
- 部署参数写入 `0600 root:root` 的 `install.env`，安装器按字段白名单解析，避免 SSH 远端 shell 参数注入。
- `/opt/deploy-go` 由 `root` 管理，`deploy-go` 只读；运行数据保留在 `/var/lib/deploy-go`。
- 主密钥保持 `0400 deploy-go:deploy-go`，API unit 使用 `ProtectSystem=strict` 和 `ReadOnlyPaths` 阻止服务进程修改密钥。
- 安装器使用固定 `flock`，并在服务重启或健康检查失败时恢复上一版产物、环境文件和 systemd unit。

## 验证

- `make deploy-production-check` 通过，包括两次模拟部署的本地/远端随机 staging 与 SSH 配置隔离检查。
- `bash -n` 和 ShellCheck 聚焦检查通过。
- WSL 中重复执行安装器成功，`deploy-go-api`、`deploy-go-web`、`frpc` 均为 `active`。
- WSL 权限符合预期：安装目录 `0750 root:deploy-go`、API `0550 root:deploy-go`、数据目录 `0750 deploy-go:deploy-go`、主密钥 `0400 deploy-go:deploy-go`。
- 使用带 `ProtectSystem=strict` 与 `ReadOnlyPaths` 的 transient systemd unit，以 `deploy-go` 执行 `chmod` 主密钥时返回 `Read-only file system`。
- 把 staging API 替换为不可执行文本并触发健康检查超时后，安装器返回非零；API、Web、环境文件和 API unit 的部署前后 SHA-256 均一致，两个旧服务恢复为 `active`。
- 公网 `https://deploy.quanxinfu.com/` 与 `/api/v1/openapi.json` 返回 HTTP `200`。

## 回滚

安装器会在失败 trap 中恢复部署前的 API、Web、辅助脚本、环境文件和两个 unit，恢复原 enable 状态，执行 `daemon-reload` 后重启旧服务。恢复自身失败时保留 `.rollback.*` 并输出人工恢复路径；下次部署检测到遗留事务会拒绝覆盖。数据库和主密钥不参与版本回滚。
