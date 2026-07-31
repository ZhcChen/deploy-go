# GitHub Actions API 构建与发布

## 适用范围

本手册用于验证 API release 镜像、触发 GitHub Actions 构建和发布 GitHub Release。构建与发布产物不等于获得连接真实节点或执行真实部署的授权。

## 工作流

| 工作流 | 触发条件 | 行为 |
| --- | --- | --- |
| `CI` | push 到 `main`、Pull Request | 执行 `make check` |
| `Build Release Artifacts` | `v*.*.*` tag、手动触发 | 检查 API，并构建 Linux `x86_64` 与 `arm64` 产物 |

手动触发 `Build Release Artifacts` 时，默认只生成 Actions artifact。只有同时启用 `publish_release` 并提供合法的 `release_tag`，才会创建或更新 GitHub Release。

## 本地构建

前置条件为 Docker Engine，并支持 BuildKit。

```bash
make api-image
```

可覆盖镜像名和目标平台：

```bash
make api-image API_IMAGE=deploy-go-api:test DOCKER_PLATFORM=linux/amd64
```

本地构建只验证镜像，不应向镜像写入主密钥、初始化 token 或数据库。服务运行时必须按 `docs/runbooks/local-development.md` 注入配置；容器内默认监听 `0.0.0.0:8080`，SQLite 默认路径为 `/data/deploy-go.db`。

## Release 产物

每个架构生成以下文件：

- `deploy-go-api-linux-<arch>`：可执行文件。
- `deploy-go-api-linux-<arch>.binary.tar.gz`：可执行文件压缩包。
- `deploy-go-api-linux-<arch>.docker.tar.gz`：可通过 `docker load` 导入的镜像归档。
- `deploy-go-api-linux-<arch>.sha256`：该架构全部产物的 SHA-256。

校验与加载示例：

```bash
sha256sum --check deploy-go-api-linux-x86_64.sha256
gunzip --stdout deploy-go-api-linux-x86_64.docker.tar.gz | docker load
```

## 发布版本

稳定版本使用 `vMAJOR.MINOR.PATCH` tag，预发布版本可使用 `vMAJOR.MINOR.PATCH-suffix`。

```bash
git tag v0.1.0
git push origin v0.1.0
```

tag push 会自动构建并发布 Release。发布说明由 `.github/scripts/generate-release-notes.sh` 根据产物和 Git tag 生成。

## 失败处理

- `api-check` 失败：先在本地运行 `make api-check`，修复后重新提交；不要跳过检查发布。
- 单一架构构建失败：检查对应 runner 的 Docker build 日志；矩阵不会因另一架构失败而提前取消。
- Release 发布失败：构建 artifact 会保留，可在修复权限或 tag 后手动重新运行，并显式填写同一 `release_tag`。
- 镜像启动失败：确认已注入 `DEPLOY_GO_MASTER_KEY_VERSION` 与主密钥文件，并检查 SQLite 挂载目录是否允许 UID `1000` 写入。

## 安全边界

- GitHub Actions 不配置 SSH 私钥、不连接节点、不执行部署脚本。
- 主密钥、setup token 和生产数据库不得作为 workflow artifact 或镜像层的一部分。
- `publish-release` 以外的 job 只有仓库内容读取权限。
