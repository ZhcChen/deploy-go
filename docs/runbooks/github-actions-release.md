# GitHub Actions 构建与发布

> 当前状态：`.github/workflows/release-artifacts.yml` 仅在推送 `v*.*.*` 新 tag 时构建并发布对应提交的产物。分支 push 和手动操作均不会触发发布构建。

## 适用范围

本手册用于验证 API、Web、Android 构建，触发 GitHub Actions 并发布 GitHub Release。构建与发布产物不等于获得连接真实节点或执行真实部署的授权。

## 工作流

| 工作流 | 触发条件 | 行为 |
| --- | --- | --- |
| `CI` | 仅 GitHub 页面手动触发 | 执行 workspace、UI/Web E2E 与移动端 smoke，不由分支 push 自动运行 |
| `Build Release Artifacts` | 推送 `v*.*.*` tag | 从 tag 指向的提交构建 API、Web、Android 和 Agent/executor 双架构产物，生成 checksum、v3 manifest 并发布 GitHub Release |

`Build Release Artifacts` 不提供 `workflow_dispatch`。构建来源固定为触发事件中的 tag commit，不允许从分支或手动选择 ref 后覆盖已有 Release。

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

Agent 配对组件使用静态链接的 Linux musl 产物：

- `deploy-go-agent-linux-x86_64`、`deploy-go-agent-linux-aarch64`：安装器直接下载的二进制。
- `deploy-go-agent-executor-linux-x86_64`、`deploy-go-agent-executor-linux-aarch64`：本机 root executor，PTY 子进程提供完整 root 登录能力。
- `deploy-go-agent-pair-linux-<arch>.tar.gz`：同架构 Agent/executor 配对归档。
- `deploy-go-agent-pair-linux-<arch>.sha256`：两个二进制与配对归档的校验清单。
- `deploy-go-agent-manifest.json`：`schema_version: 3`，包含 Agent/executor 相同 semver、控制协议范围、三个 unit、executor 配置模板和四个二进制的 HTTPS URL 与 SHA-256。
- `deploy-go-agent.service`：以低权限 `deploy-go-agent` 用户运行的受限 systemd unit。
- `deploy-go-agent-runner.service`：以 root 运行、只接受本机 Agent 固定任务启动请求并降权到 `deploy-go-runner` 的 broker unit。
- `deploy-go-agent-executor.service`：以 root 运行并允许 PTY 子进程联网和管理主机的 executor unit。
- `executor.json.in`：由安装器写入 Agent uid/gid 与目标机 root home/login shell 的本机配置模板。
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
sha256sum --check deploy-go-agent-pair-linux-x86_64.sha256
sha256sum --check deploy-go-agent-pair-linux-aarch64.sha256
gunzip --stdout deploy-go-api-linux-x86_64.docker.tar.gz | docker load
tar -tzf deploy-go-admin-web.tar.gz
unzip -t deploy-go-admin-android-debug.apk
unzip -t deploy-go-admin-android-release-unsigned.aab
```

## Agent 发布配置

API 不根据请求 `Host` 推导安装地址，并由 API 自身提供 Agent 发布物下载。Agent 与 API 使用相同版本号；部署时由部署端从 GitHub Release 下载当前 API 版本对应的 Agent 发布物到 API 发布目录。需要生成 Agent 安装命令的环境必须成组配置：

```bash
export DEPLOY_GO_PUBLIC_BASE_URL=https://deploy.example.com
```

Agent 发布目录固定为 `/var/lib/deploy-go/agent-releases`，不再通过环境变量配置。每个版本一个子目录，目录名与 manifest 中的 `agent_version` 一致。部署端完成同步后结构如下：

```text
/var/lib/deploy-go/agent-releases/
├── 0.1.0/
│   ├── deploy-go-agent-manifest.json
│   ├── deploy-go-agent-linux-x86_64
│   ├── deploy-go-agent-linux-aarch64
│   ├── deploy-go-agent-executor-linux-x86_64
│   ├── deploy-go-agent-executor-linux-aarch64
│   ├── deploy-go-agent.service
│   ├── deploy-go-agent-executor.service
│   └── executor.json.in
└── 0.2.0/
    └── ...
```

部署端使用以下命令从 GitHub Release 同步当前版本：

```bash
make agent-release-sync \
  DEPLOY_GO_AGENT_VERSION=0.1.0
```

脚本固定写入 `/var/lib/deploy-go/agent-releases`，从 `https://github.com/{repository}/releases/download/v{version}` 下载 manifest、双架构 Linux 二进制和 systemd unit，先写入 staging 目录并校验 manifest 版本、控制协议范围、SHA-256 与 systemd 安全项，再原子替换到发布目录。未显式设置 `DEPLOY_GO_AGENT_VERSION` 时，脚本从 `api/Cargo.toml` 读取版本（Agent 与 API 版本不一致会直接失败），因此也可以省略该变量。

API 启动时扫描固定发布目录，逐版本校验 manifest JSON Schema 和控制协议兼容范围，不兼容时拒绝启动；`DEPLOY_GO_PUBLIC_BASE_URL` 未配置时 API 可运行，但创建 Agent 或重新生成安装命令会返回 `agent_installation_unavailable`。

管理后台在“设置 → Agent 版本”中查看已同步版本并清理历史版本。清理只删除发布目录中的对应版本目录，不删除数据库数据；当前 API 版本对应的发布物禁止清理。

Docker 部署时应把宿主发布目录 bind mount 到容器内相同路径，并确保 API 运行用户对目录有读写权限，否则清理历史版本会失败。

API 会将安装命令中的 manifest 地址指向自身，并按版本提供下载：

- `https://deploy.example.com/api/v1/agent/download/0_1_0/manifest.json`
- `https://deploy.example.com/api/v1/agent/download/0_1_0/agent/x86_64`
- `https://deploy.example.com/api/v1/agent/download/0_1_0/agent/aarch64`
- `https://deploy.example.com/api/v1/agent/download/0_1_0/executor/x86_64`
- `https://deploy.example.com/api/v1/agent/download/0_1_0/executor/aarch64`
- `https://deploy.example.com/api/v1/agent/download/0_1_0/systemd-unit/agent`
- `https://deploy.example.com/api/v1/agent/download/0_1_0/systemd-unit/executor`
- `https://deploy.example.com/api/v1/agent/download/0_1_0/executor-config`

版本路径使用下划线形式，同时也接受点分版本 `0.1.0`。安装器仍先下载 manifest，再按架构下载二进制并校验 SHA-256，不再依赖 GitHub Release 作为节点下载源。

管理员创建 Agent 后执行响应中的 `install_command`。同一 Agent ID 重跑时保留有效本地凭证，只更新或修复二进制；不同 Agent ID 会拒绝覆盖。Agent 被撤销后，通过 `POST /api/v1/agents/{agent_id}/install-command` 生成显式重新绑定命令，新 enrollment 成功前不会恢复身份。

## 发布版本

稳定版本使用 `vMAJOR.MINOR.PATCH` tag，预发布版本可使用 `vMAJOR.MINOR.PATCH-suffix`。
发布前必须保持 `api/Cargo.toml`、`agent/Cargo.toml` 与 `agent-executor/Cargo.toml` 的版本号一致；release workflow 会同时校验三者与 tag，不一致时构建失败。

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
- Release 发布失败：先修复失败原因并递增项目版本，再创建和推送新的版本 tag；发布 workflow 不提供分支或手动触发入口。
- 镜像启动失败：确认已注入 `DEPLOY_GO_MASTER_KEY_VERSION` 与主密钥文件，并检查 SQLite 挂载目录是否允许 UID `1000` 写入。

## 安全边界

- GitHub Actions 不配置 SSH 私钥、不连接节点、不执行部署脚本。
- Agent release 只包含公开二进制、安装器、unit、manifest 和 checksum，不包含 enrollment、access 或 refresh token。
- 主密钥和生产数据库不得作为 workflow artifact 或镜像层的一部分。
- Android workflow 不读取 keystore、签名密码、provisioning profile 或其他签名材料；iOS 不构建签名发布物。
- Web、APK 和 AAB 在上传前解包扫描；统一 release bundle 再次解包扫描并校验 `SHA256SUMS`。
- `publish-release` 以外的 job 只有仓库内容读取权限。
