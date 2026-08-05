# GitHub Actions 构建与发布

## 适用范围

本手册用于验证 API、Web、Android 构建，触发 GitHub Actions 并发布 GitHub Release。构建与发布产物不等于获得连接真实节点或执行真实部署的授权。

## 工作流

| 工作流 | 触发条件 | 行为 |
| --- | --- | --- |
| `CI / Check workspace` | push 到 `main`、Pull Request | 执行 API、UI 静态检查、双端生成漂移、Web/Flutter 检查和敏感扫描 |
| `CI / UI preview E2E` | push 到 `main`、Pull Request | 使用隔离 Chromium 执行 UI 预览交互回归 |
| `CI / Web E2E` | push 到 `main`、Pull Request | 使用隔离 Chromium 执行 Web 键盘、axe 和业务 smoke，并扫描构建/测试产物 |
| `CI / Android 15 smoke` | push 到 `main`、Pull Request | 在 API 35 Emulator 执行 Flutter 集成 smoke |
| `CI / iOS Simulator secure session smoke` | push 到 `main`、Pull Request | 在可用 iPhone Simulator 执行安全会话 smoke，不读取签名材料 |
| `Build Release Artifacts` | `v*.*.*` tag、手动触发 | 构建 API、Agent 双架构、Web、Android 验证产物并生成 manifest 与统一 checksum |

手动触发 `Build Release Artifacts` 时，默认只生成 Actions artifact。只有同时启用 `publish_release` 并提供合法的 `release_tag`，才会创建或更新 GitHub Release。
手动发布还要求该 tag 已存在，且 tag commit 与本次 workflow dispatch 选择的 ref 完全一致；不允许把当前分支构建物覆盖到其他 commit 的 Release。

## 本地 API 构建

前置条件为 Docker Engine，并支持 BuildKit。

```bash
make api-image
```

可覆盖镜像名和目标平台：

```bash
make api-image API_IMAGE=deploy-go-api:test DOCKER_PLATFORM=linux/amd64
```

本地构建只验证镜像，不应向镜像写入主密钥、初始化 token 或数据库。服务运行时必须按 `docs/runbooks/local-development.md` 注入配置；容器内默认监听 `0.0.0.0:30100`，SQLite 默认路径为 `/data/deploy-go.db`。

## 本地客户端 dry-run

```bash
npm ci
make admin-build
make admin-app-build

cd admin-app
flutter build appbundle --release \
  --dart-define=DEPLOY_GO_API_BASE_URL=https://deploy-go.invalid \
  --dart-define=DEPLOY_GO_ALLOWED_ORIGIN=https://deploy-go.invalid
cd ..
```

`release` build type 未配置 signingConfig，AAB 必须保持未签名。使用 ZIP 结构检查签名块，避免依赖 `jarsigner` 的本地化输出：

```bash
if unzip -Z1 admin-app/build/app/outputs/bundle/release/app-release.aab \
  | grep -Eiq '^META-INF/[^/]+\.(SF|RSA|DSA|EC)$'; then
  echo 'AAB 包含签名块' >&2
  exit 1
fi
```

本地 AAB 构建需要完整 Android SDK command-line tools、已接受许可证及 Flutter 指定的 NDK。任何一项缺失都应修复本机 toolchain，不得改回 debug signing 规避构建失败。

## Release 产物

每个架构生成以下文件：

- `deploy-go-api-linux-<arch>`：可执行文件。
- `deploy-go-api-linux-<arch>.binary.tar.gz`：可执行文件压缩包。
- `deploy-go-api-linux-<arch>.docker.tar.gz`：可通过 `docker load` 导入的镜像归档。
- `deploy-go-api-linux-<arch>.sha256`：该架构全部产物的 SHA-256。

Agent 使用静态链接的 Linux musl 产物：

- `deploy-go-agent-linux-x86_64`、`deploy-go-agent-linux-aarch64`：安装器直接下载的二进制。
- `deploy-go-agent-linux-<arch>.tar.gz`：对应二进制归档。
- `deploy-go-agent-linux-<arch>.sha256`：对应二进制与归档的校验清单。
- `deploy-go-agent-manifest.json`：包含 Agent semver、控制协议范围、systemd unit 和两种架构二进制的 HTTPS URL 与 SHA-256。
- `deploy-go-agent.service`：以低权限 `deploy-go-agent` 用户运行的受限 systemd unit。
- `install.sh`：幂等安装器；首次接入、同身份修复、显式重新绑定和失败回滚使用同一脚本。

客户端和统一校验产物：

- `deploy-go-admin-web.tar.gz`：Web 静态目录归档，解压后入口为 `index.html`。
- `deploy-go-admin-android-debug.apk`：可侧载的调试验证包，不用于生产分发。
- `deploy-go-admin-android-release-unsigned.aab`：未签名的 release AAB 构建输入，必须由后续受控发布流程签名。
- `SHA256SUMS`：以上客户端产物及全部 API 产物的统一 SHA-256 清单。

校验与加载示例：

```bash
sha256sum --check deploy-go-api-linux-x86_64.sha256
sha256sum --check SHA256SUMS
sha256sum --check deploy-go-agent-linux-x86_64.sha256
sha256sum --check deploy-go-agent-linux-aarch64.sha256
gunzip --stdout deploy-go-api-linux-x86_64.docker.tar.gz | docker load
tar -tzf deploy-go-admin-web.tar.gz
unzip -t deploy-go-admin-android-debug.apk
unzip -t deploy-go-admin-android-release-unsigned.aab
```

## Agent 发布配置

API 不根据请求 `Host` 推导安装地址。需要生成 Agent 安装命令的环境必须成组配置：

```bash
export DEPLOY_GO_PUBLIC_BASE_URL=https://deploy.example.com
export DEPLOY_GO_AGENT_MANIFEST_URL=https://github.com/ZhcChen/deploy-go/releases/download/v0.1.0/deploy-go-agent-manifest.json
export DEPLOY_GO_AGENT_MANIFEST_PATH=/etc/deploy-go/deploy-go-agent-manifest.json
```

`DEPLOY_GO_AGENT_MANIFEST_PATH` 是从同一 Release 下载并只读挂载给 API 的 manifest 快照；`DEPLOY_GO_AGENT_MANIFEST_URL` 是节点安装器访问的公网地址。API 启动时校验快照 JSON Schema 和控制协议兼容范围，不兼容时拒绝启动；三项都不配置时 API 可运行，但创建 Agent 或重新生成安装命令会返回 `agent_installation_unavailable`。

管理员创建 Agent 后执行响应中的 `install_command`。同一 Agent ID 重跑时保留有效本地凭证，只更新或修复二进制；不同 Agent ID 会拒绝覆盖。Agent 被撤销后，通过 `POST /api/v1/agents/{agent_id}/install-command` 生成显式重新绑定命令，新 enrollment 成功前不会恢复身份。

## 发布版本

稳定版本使用 `vMAJOR.MINOR.PATCH` tag，预发布版本可使用 `vMAJOR.MINOR.PATCH-suffix`。

```bash
git tag v0.1.0
git push origin v0.1.0
```

tag push 会自动构建并发布 Release。发布说明由 `.github/scripts/generate-release-notes.sh` 根据产物和 Git tag 生成。

## 失败处理

- `api-check` 失败：先在本地运行 `make api-check`，修复后重新提交；不要跳过检查发布。
- `UI preview E2E` 或 `Web E2E` 失败：分别运行 `make ui-test`、`make admin-test-e2e`，并检查敏感扫描结果。
- Android/iOS smoke 失败：使用 `flutter devices` 获取设备 ID，再运行 `make admin-app-test-integration DEVICE_ID=<id>`；fixture 不连接 API 或节点。
- 单一架构构建失败：检查对应 runner 的 Docker build 日志；矩阵不会因另一架构失败而提前取消。
- Agent manifest 失败：确认 tag 与 `agent/Cargo.toml` 版本一致，并核对两个 musl 二进制和 systemd unit 都存在；不得手工跳过 checksum 或协议范围校验。
- AAB 显示已签名：发布边界被破坏，停止发布并检查 `admin-app/android/app/build.gradle.kts`，不得上传调试签名 AAB。
- 客户端 artifact 扫描失败：不得通过删除规则或跳过解包处理，应定位构建输入中的受保护值并重新构建。
- 本地 AAB strip 失败：运行 `flutter doctor -v` 并确认 NDK 中存在 `llvm-strip`；该问题属于本地 Android toolchain，不应通过修改签名配置解决。
- Release 发布失败：构建 artifact 会保留，可在修复权限或 tag 后手动重新运行，并显式填写同一 `release_tag`。
- 镜像启动失败：确认已注入 `DEPLOY_GO_MASTER_KEY_VERSION` 与主密钥文件，并检查 SQLite 挂载目录是否允许 UID `1000` 写入。

## 安全边界

- GitHub Actions 不配置 SSH 私钥、不连接节点、不执行部署脚本。
- Agent release 只包含公开二进制、安装器、unit、manifest 和 checksum，不包含 enrollment、access 或 refresh token。
- 主密钥、setup token 和生产数据库不得作为 workflow artifact 或镜像层的一部分。
- Android workflow 不读取 keystore、签名密码、provisioning profile 或其他签名材料；iOS 不构建签名发布物。
- Web、APK 和 AAB 在上传前解包扫描；统一 release bundle 再次解包扫描并校验 `SHA256SUMS`。
- `publish-release` 以外的 job 只有仓库内容读取权限。
