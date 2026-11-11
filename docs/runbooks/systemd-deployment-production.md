# 正式环境 systemd 部署

## 适用范围

本手册用于把 Deploy Go API 与 Web 管理端部署到唯一的正式环境服务器，使用 systemd 管理服务。正式域名是 `https://deploy.quanxinfu.com`。`qfy-test2` 与 `qfy-test` 是本机 SSH config 中指向同一台 Deploy Go 正式环境服务器的两个连接别名：`qfy-test2` 为内网直连别名，是默认部署入口；`qfy-test` 为公网入口别名，不在内网时通过 `DEPLOY_HOST=qfy-test` 覆盖使用。业务节点机器（例如 `qfy-prod-1`）不是 Deploy Go 正式环境，禁止作为 `DEPLOY_HOST` 执行本部署脚本；如确有特殊需求，必须先获得用户对该节点的明确授权。执行远程部署、重启服务或修改服务器配置前，必须在当前对话中获得针对该节点的明确授权。

## 拓扑

日志容量、组件集中诊断和回滚保护见 `docs/runbooks/log-retention.md`。正式安装器为控制面创建独立 `0700` 运行日志目录，节点成对升级后启用四组件诊断，不重发业务部署。

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

- 正式环境服务器已配置为本机 SSH alias `qfy-test2`（内网直连）和
  `qfy-test`（公网入口回退），root 可登录。
- 部署前先通过 `ssh qfy-test2 'hostname; systemd-detect-virt'` 确认目标节点身份；
  目标必须是 Deploy Go 正式环境（`qfy-test2`/`qfy-test` 为同一台服务器）；
  不得把业务节点（如 `qfy-prod-1`）作为部署目标。
- 公网域名 `deploy.quanxinfu.com` 必须解析到该服务器；DNS 未切换前，Caddy 无法签发 HTTPS 证书。
- 服务器有 Python 3、`curl`、`openssl`、`rsync`、`jq`、Docker/buildx 与 systemd；Agent release 校验使用 Python 3。
- 本机有 `ssh`、`rsync`、`curl`、`git`；默认远程 `build` 模式不需要本机 Docker、Rust 或 Node.js。
- `qfy-test2` 需要具备 Docker/buildx、Node 22 容器拉取能力、足够构建磁盘和跨架构构建能力。
- 远程构建源码位于 `/var/lib/deploy-go-builder/build.<随机值>`，控制面运行目录和数据目录不作为构建目录。

## 部署步骤

### 0. 远程构建并部署（默认）

默认流程不需要预构建：`make deploy-production` 会在 `qfy-test2` 上远程构建并安装。如需在**不连接服务器**的前提下，先在本机用 Docker 构建并校验 Agent/executor release（显式的兼容入口，需要本机 Docker），使用：

```bash
make deploy-production-agent-build
```

真正部署（会 SSH 到正式环境、上传源码快照、重启 `deploy-go-api` / `deploy-go-web`，执行前必须获得当前对话的明确授权）：

```bash
make deploy-production
```

脚本使用 `git archive HEAD` 生成干净源码快照，上传到 qfy-test2 的随机独立目录，
然后远程构建 API、Agent/executor、deployer 和 Web。构建成功后回收产物，再进入现有
安装 staging、备份、健康检查和失败回滚流程。构建失败只清理构建目录，不影响运行中的控制面。

统一 Rust release Dockerfile 在远程构建机先复制 workspace manifests 并执行 `cargo fetch --locked`，
再复制源码。Cargo registry 与 git checkout 使用跨组件命名 cache，编译产物使用
`deploy-go-rust-target-${TARGETARCH}` 按架构隔离，另有 `deploy-go-sccache` 缓存挂载配合
`RUSTC_WRAPPER=sccache` 复用单次编译产物；构建模式下 API、Agent、executor 与
deployer 按架构合并到同一次 Cargo 构建，默认从五次 builder 收敛为两次。普通源码变更
不会使依赖下载层失效，修改
`Cargo.toml`、`Cargo.lock` 或 `rust-toolchain.toml` 会按预期重新执行依赖准备。target
缓存失效时（改 Dockerfile 构建层、切架构、清理 BuildKit cache）sccache 仍可把重建墙钟
从 3m47s 压到 46s；构建日志末尾的 `sccache --show-stats` 输出用于确认命中情况，命中率为
0 说明缓存挂载没有生效。远程构建缓存排查基线见 `docs/runbooks/local-development.md`。

### 1. 构建模式（当前源码）

```bash
bash deploy/production/deploy.sh
```

脚本会：

1. 通过 SSH alias `qfy-test2`（不在内网时改为 `qfy-test`）读取正式环境服务器架构并确定构建平台。
2. 将 `HEAD` 的 Git 归档快照上传到 `/var/lib/deploy-go-builder/build.<随机值>`，不包含 dirty worktree 和未跟踪文件。
3. 在远程构建目录用 Docker/buildx 构建 API、Agent、executor 与 deployer，并用 Node 22 容器构建 Web。
4. 校验远程产物 commit、版本、架构、manifest、SHA-256 和 Web 敏感内容扫描结果。
5. 回收产物后清理远程构建目录；再创建本地随机 staging，并在 `/var/lib/deploy-go-installer` 下创建仅 `root` 可写的安装 staging。
6. 把部署参数写入 `0600 root:root` 的 `install.env` 后随产物上传，SSH 命令不携带参数值。
7. 取得 `/run/lock/deploy-go-install.lock` 安装锁；已有安装任务时立即停止。
8. 以 `root` 管理 `/opt/deploy-go`，只把运行数据目录交给 `deploy-go` 写入。
9. 备份上一版产物、配置、unit 和特权发布签名密钥，再安装主密钥、签名密钥、环境文件和两个 systemd unit。
10. 安装 staging 中远程构建的 Agent release 到 `/var/lib/deploy-go/agent-releases/<版本>`。
11. 启用并重启服务，验证 `/healthz`、`/readyz`、Web 首页和 `/api` 代理；失败时在锁内恢复备份并重启旧服务。

### 2. 常用配置

变量分两层：本机运行 `make deploy-production` 时设置 `DEPLOY_*`（如 `DEPLOY_API_PORT`、`DEPLOY_WEB_PORT`）；`deploy.sh` 把它们写入 `0600` 的 `install.env` 并转换为服务端使用的 `DEPLOY_GO_*`（如 `DEPLOY_GO_API_PORT`），安装器 `install.sh` 只读取后者。因此不要在本机手工设置 `DEPLOY_GO_API_*` / `DEPLOY_GO_WEB_*`；在服务器上直接运行 `install.sh` 排障时才需要使用 `DEPLOY_GO_*` 名称。`DEPLOY_GO_COOKIE_SECURE` 等少数变量两层同名。

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `DEPLOY_HOST` | `qfy-test2` | Deploy Go 正式环境的 SSH alias；内网默认，不在内网时设为 `qfy-test`，禁止改为业务节点 |
| `DEPLOY_BUILD_MODE` | `remote` | `remote` 在构建服务器构建；`local` 为显式本机构建兼容模式 |
| `DEPLOY_BUILD_HOST` | `DEPLOY_HOST` | 远程构建 SSH alias，默认与正式环境同机 |
| `DEPLOY_BUILD_CLEANUP` | `1` | 远程构建前清理本项目旧 builder 与泄漏镜像（只匹配 `deploy-go-production*` 与 `deploy-go-rust-release-remote-<sha>-x86_64`），结束后按保留 6GB / 上限 20GB 修剪 BuildKit 缓存；`0` 跳过 |
| `DEPLOY_BUILD_PROXY_URL` | `http://127.0.0.1:17890` | 构建机本机的 mihomo 构建代理（见下方“构建代理”）；远程构建前会先探测可用性，不可用立即失败。设为空字符串则直连。仅影响构建，不写入任何全局环境变量。网络参数变化使用独立 builder，避免复用旧代理配置 |
| `DEPLOY_BUILD_REGISTRY_MIRROR` | 使用代理时为空，否则 `docker.1ms.run,docker.m.daocloud.io` | BuildKit（docker-container 驱动）不继承 daemon 的 `registry-mirrors`，由此写入 builder 的 `buildkitd.toml`；逗号分隔、按序尝试，均失败才回退 Docker Hub；显式设为空字符串则不配。变更后自动换用新 builder |
| 构建依赖国内源（Dockerfile 内置） | apk=`mirrors.aliyun.com`、crates=`rsproxy.cn`、npm=`registry.npmmirror.com` | 构建机访问 Alpine 官方源、crates.io、npmjs 极慢（实测约 16KB/s），故依赖走国内镜像；Rust 镜像用 `--build-arg APK_MIRROR`/`CARGO_MIRROR_REGISTRY`/`RUSTUP_MIRROR` 覆盖，`APK_MIRROR` 设为空则用官方源。源替换不改变 `Cargo.lock` 校验和 |
| `DEPLOY_HEALTH_WAIT_SECONDS` | `180` | 安装器等待 API/Web 健康检查通过的基础时限，超时即安装失败，失败后按上文第 11 步在锁内恢复备份。崩溃重启、进程退出等明确失败按它判定 |
| `DEPLOY_HEALTH_MAX_WAIT_SECONDS` | `900` | 仅对 API：超过基础时限后，只要 `deploy-go-api` 仍处于 active 且自等待开始后没有崩溃重启，就继续等待到此上限。慢启动不是失败：曾有一次部署因 API 启动恢复耗时约 197 秒被误判失败并回滚（见 [解决记录](../solutions/2026-11-11-api-startup-log-replay-and-build-cache.md)） |
| `DEPLOY_API_PORT` | `30100` | API 本机监听端口 |
| `DEPLOY_API_BIND` | `127.0.0.1` | API 仅本机监听，由 Web 代理访问 |
| `DEPLOY_WEB_PORT` | `30101` | Web 对外端口 |
| `DEPLOY_WEB_BIND` | `127.0.0.1` | Web 仅本机监听，由 HTTPS 反向代理访问 |
| `DEPLOY_GO_COOKIE_SECURE` | `true` | 正式环境 session cookie 强制使用 `Secure` |
| `DEPLOY_GO_MASTER_KEY_VERSION` | `1` | 主密钥版本 |
| `DEPLOY_GO_PUBLIC_BASE_URL` | `https://deploy.quanxinfu.com` | Agent 安装与发布链接的正式 HTTPS origin |
| `DEPLOY_GO_ALLOWED_ORIGIN` | `https://deploy.quanxinfu.com` | API 允许的正式 Web Origin |
| `DEPLOY_AGENT_SYNC` | `1` | 是否构建并上传 Agent release；设为 `0` 可跳过 |
| `DEPLOY_AGENT_BUILD_ONLY` | `0` | 设为 `1` 时只在本机构建 Agent release 并退出，不连接服务器 |
| `DEPLOY_AGENT_OUTPUT_DIR` | `target/deploy-release/agent` | `local` 模式下的本机 Agent 构建输出目录 |
| `DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED` | `true` | 正式环境必须启用跨节点制品通道 |
| `DEPLOY_GO_ARTIFACTS_ROOT` | `/var/lib/deploy-go/artifacts` | 固定在 systemd 可写数据目录内，不允许改到其他目录 |
| `DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES` | `536870912` | 单文件上限 512 MiB |
| `DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES` | `2147483648` | 单次制品总量上限 2 GiB |
| `DEPLOY_GO_ARTIFACT_MAX_FILES` | `256` | 单次制品文件数上限 |
| `DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES` | `8388608` | 上传 chunk 上限 8 MiB |
| `DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS` | `1800` | 未完成上传 lease 有效期 |
| `DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS` | `86400` | 无活动发布引用的已验证制品保留期 |

Python Web 代理以 64 KiB 固定缓冲转发 `Content-Length` 或 chunked 请求，不会按制品总大小缓存请求体。代理拒绝同时携带两种 framing、非法 chunk 和超过 2 GiB 的请求体；最终文件数与文件大小仍由 API manifest 校验。

### 构建代理

构建机（qfy-test2）直连 Docker Hub、Alpine、crates.io、npm 极慢（Docker Hub 被 DNS 污染，Alpine 官方源约 16KB/s）。
构建时经本机的 mihomo 代理访问，实测拉 Rust 基础镜像的最大层约 5.9MB/s（镜像站约 100KB/s）。

- 服务：`mihomo-build-proxy.service`，用户 `mihomo-proxy`，二进制 `/opt/mihomo-build-proxy/mihomo`，
  配置 `/etc/mihomo-build-proxy/config.yaml`（`0640 root:mihomo-proxy`，含节点密码，不得进入日志或仓库）。
- 只监听 `127.0.0.1:17890`，不设置 `HTTP_PROXY` 等全局环境变量，不改 docker daemon（改 daemon 需要重启全部业务容器）；
  只有部署脚本经 `DEPLOY_BUILD_PROXY_URL` 注入 BuildKit builder 的构建才使用它，不影响同机其他应用。
- 配置只含一个节点，模式为 `MATCH,PROXY`；订阅里的其他节点不上传。

代理没有 SLA。部署脚本会在构建前经代理访问 `https://registry-1.docker.io/v2/`（返回 401 即通），失败会立即退出并提示本节。

检查：

```bash
ssh qfy-test2 'systemctl is-active mihomo-build-proxy; curl -s -x http://127.0.0.1:17890 -o /dev/null -m 15 -w "%{http_code}\n" https://registry-1.docker.io/v2/'
```

更换节点（代理失效或订阅过期时）：

1. 在本机 Clash Party 更新订阅，并选定要使用的节点。
2. 从 Clash Party 的 `work/config.yaml` 取出该节点，生成只含这一个节点的最小配置（权限 `0600`，用后删除；不打印密钥）：
   `mixed-port: 17890`、`bind-address: 127.0.0.1`、`allow-lan: false`、`external-controller: ""`、`geo-auto-update: false`、
   国内 DNS、`proxies: [节点]`、`proxy-groups: [PROXY(select)]`、`rules: [MATCH,PROXY]`。
3. 先确认构建机能连通节点（TCP 连接测试），再 `scp` 到临时文件，`install -m 0640 -o root -g mihomo-proxy` 覆盖
   `/etc/mihomo-build-proxy/config.yaml`，然后 `systemctl restart mihomo-build-proxy` 并运行上面的检查命令。
4. 删除本机与服务器上的临时配置文件。

升级内核：从 MetaCubeX/mihomo 官方 release 下载 `mihomo-linux-amd64-compatible-*.gz`，核对 release 元数据中的 sha256 后替换二进制并重启服务。

### 公网入口长连接参数

正式环境公网入口由 `qfy-sc-test` 上的 nginx 承担，位置是
`/etc/nginx/conf.d/qfy-443-frp-web.conf`。只对 Deploy Go 的
`deploy.quanxinfu.com`（Web/API 代理）和
`deploy-api.quanxinfu.com`（独立 API 入口）两个 server block 增加：

```nginx
proxy_buffering off;
proxy_request_buffering off;
proxy_read_timeout 3600s;
proxy_send_timeout 3600s;
client_body_timeout 3600s;

# 必须继续保留 WebSocket Upgrade 转发
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection $connection_upgrade;
```

这些参数只影响 Deploy Go 的控制面入口，不要加到全局 snippet 或其他业务站点。
修改前先备份对应 nginx 配置文件，然后执行 `nginx -t` 和
`systemctl reload nginx`；`client_body_timeout` 表示两次收到请求体之间的间隔，
不是整个上传的总超时，Agent 侧仍需用无进展超时处理“连接存在但字节不前进”的情况。

### Agent 发布物下载无进展超时

Agent 0.3.7 在 `agent/src/artifact_transfer.rs` 中使用
`DEFAULT_DOWNLOAD_READ_IDLE_TIMEOUT = 120s` 作为发布物下载的“无新数据”静默窗口，
不是整个下载的总超时，也不是等待 120 秒后固定重试一次：

1. 每收到一个 HTTP body chunk 就重新计时；只要持续有字节，下载超过 120 秒也不会失败。
2. 连续 120 秒没有收到新 chunk，判定当前 TCP/HTTP 流卡死，主动断开本次请求。
3. Agent 根据本地 `.part` 文件已写长度发起 `Range: bytes=<offset>-` 续传。
4. 同一任务在 3 次中断后各续传一次，第 4 次中断时返回传输错误；最终仍校验归档摘要。

这个阈值针对的是公网/FRP 链路“连接未断但字节不前进”的现象，用来替代原先主要依赖
reqwest 总超时造成的长时间不可观察等待。当前值是硬编码常量；公网偶发抖动会重新计
时，因此不建议把阈值降到个位秒。

## 首次初始化

服务启动后访问 `https://deploy.quanxinfu.com`。空库首次访问会进入唯一管理员初始化，完成后 Setup 入口自动关闭。

## Agent 原生特权发布签名密钥

`deploy/production/install.sh` 在首次部署时独立生成 `/etc/deploy-go/release-signing.key`（base64 编码的 32 字节 Ed25519 seed），与 `/etc/deploy-go/terminal-signing.key` 分离。生成后保持 `0440 root:deploy-go`，重复部署只复用不覆盖；API 环境只写入文件路径 `DEPLOY_GO_RELEASE_SIGNING_KEY_FILE`，systemd unit 通过 `ReadOnlyPaths` 只读注入，安装器、日志和安装命令不输出私钥正文。

密钥纳入安装事务备份与回滚：已存在且为普通非空文件时复用；为空、符号链接或非普通文件时安装器停止并要求人工恢复，不能重新生成覆盖。执行器只接收安装命令携带的对应公钥，不接触私钥。

## Agent 特权终端与自动升级

控制面支持 v11-v18 协商，最低兼容版本保持 v11。v11 Agent 在控制面升级期间继续 heartbeat、部署、PTY 和 Env 任务，但不提供节点遥测；协商到 v12 及以上的 Agent 按 `hello_ack` 下发的 10 秒间隔发送 telemetry。v18 新任务启用持久化日志回执，旧任务继续原交付语义。控制面升级或回滚不得主动断开仍兼容的 v11 及以上 Agent。

发布顺序固定为先部署包含完整 manifest v4 发布物的控制面。已经人工 bootstrap 到 v17、声明 `agent_upgrade_v1`、使用 executor v4 且架构为 Linux `x86_64` 的正常节点，会由控制面按全局串行队列自动成对升级 Agent、runner broker、executor 和 updater。归档、已撤销、不在线或有活动任务的节点不会直接进入安装；恢复为正常且重新满足条件后再参与调度。任意时刻只能有一个全局升级租约和一个对应节点维护锁，任务终态后必须释放两者。

旧版节点首次 bootstrap、升级失败后的人工恢复、节点停用和版本回退仍是单独的真实节点操作，必须获得对应授权并遵循 `docs/runbooks/agent-recovery.md` 与 `docs/runbooks/privileged-agent-terminal.md`。不得把部署正式环境视为任意操作业务节点的授权；自动升级只适用于节点已主动接入并声明的固定升级能力。

## 验证

```bash
systemctl status deploy-go-api deploy-go-web
systemctl is-active deploy-go-api deploy-go-web
curl --fail http://127.0.0.1:30100/readyz
curl --fail http://127.0.0.1:30101/
curl --fail http://127.0.0.1:30101/api/v1/openapi.json
curl --fail https://deploy.quanxinfu.com/
curl --fail https://deploy.quanxinfu.com/api/v1/openapi.json
curl --fail https://deploy.quanxinfu.com/external/v1/openapi.json
systemctl show deploy-go-api -p ReadWritePaths -p StateDirectory
sudo -u deploy-go test -w /var/lib/deploy-go/artifacts
du -sh /var/lib/deploy-go/artifacts
```

internal OpenAPI 应包含 `GET /api/v1/nodes/{id}/telemetry`，external OpenAPI 不包含该路径。具备节点读取权限的会话可在节点详情确认 `capability`、`freshness` 和最近样本；遥测为空、预热或 stale 不是部署失败条件。生产 migration、控制面部署和逐节点 Agent 升级均需分别获得当前对话的明确授权。

## 日志与排障

**部署日志与耗时**：`make deploy-production` 的完整输出同时写入本机忽略目录 `.data/deploy-logs/deploy-<时间>-<提交短哈希>-<进程号>.log`，保留最近 20 份（`DEPLOY_LOG_KEEP` 可调，`DEPLOY_LOG_DIR` 可改目录）；目录不可写时只提示并继续部署。日志里有 `DEPLOY_TIMING stage=<阶段> seconds=<秒>` 行（阶段为 `preflight`、`source-upload`、`remote-build`、`artifact-fetch`、`install`，本机构建为 `local-build`，另有 `total`），成功、失败和被中断都会输出；`remote-build.sh` 另输出 `REMOTE_BUILD_TIMING rust_seconds=`、`web_seconds=`。部署慢时先看这些行定位是哪一阶段。日志不含 `install.env` 内容；不要把日志发到公开渠道，其中有服务器地址与构建输出。

```bash
journalctl -u deploy-go-api --since '30 minutes ago' --no-pager
journalctl -u deploy-go-web --since '30 minutes ago' --no-pager
```

常见问题：

- API 启动失败：查看 `/etc/deploy-go/api.env` 是否只有受控配置、主密钥文件是否 `0400 deploy-go:deploy-go`，以及 unit 是否包含 `ProtectSystem=strict` 和对应的 `ReadOnlyPaths`。
- 制品存储启动失败：确认 `/var/lib/deploy-go/artifacts` 不是符号链接、属于 `deploy-go:deploy-go`，并位于 unit 的 `ReadWritePaths=/var/lib/deploy-go` 内；不要通过放宽到任意系统目录解决。
- 上传经过 Web 代理失败：确认外层 HTTPS 代理允许 request streaming，且没有低于 `DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES` 的 body limit；Deploy Go Python 代理自身保持有界内存并支持 chunked。
- 发布物下载卡住：确认 Agent 日志已输出“正在下载发布物”，且公网 nginx 对
  `deploy.quanxinfu.com` 与 `deploy-api.quanxinfu.com` 保留了流式代理参数；正常
  链路会由 Agent 在 120 秒无进展后自动 Range 续传，而不是一直转圈。
- 提示已有安装任务：检查是否确有部署正在执行；不要删除锁文件绕过，确认无安装进程后再重试。
- 远程构建慢或卡在 crates.io index：检查 qfy-test2 的 Docker/buildx、代理、磁盘和
  `/var/lib/deploy-go-builder` 权限；不要把构建目录放入 `/opt/deploy-go` 或
  `/var/lib/deploy-go`。清理 BuildKit cache 后会发生一次 amd64 冷构建；正式环境和当前
  Agent 发布物只支持 Linux `x86_64`，不再执行 arm64 QEMU 构建。需要本机兼容构建时才显式设置
  `DEPLOY_BUILD_MODE=local`。
- 部署启动就失败并提示“构建代理 … 不可用”：`deploy.sh` 在上传源码前经构建代理探测 Docker Hub 没有通过，
  说明代理节点失效，不会改动服务器。按上文“构建代理”一节检查服务并更换节点；临时直连可设 `DEPLOY_BUILD_PROXY_URL=`（空字符串，此时回退到镜像站，速度很慢）。
- 构建没有报错但长时间停在某一步（例如拉基础镜像、`apk add`、`cargo fetch`、`npm ci`）：通常是构建机访问国外源极慢
  （此前出现过 Docker Hub 约 100KB/s、Alpine 官方源约 16KB/s）。先确认代理是否在用：BuildKit 容器环境里应有
  `HTTPS_PROXY=http://127.0.0.1:17890`（`docker inspect` 该 builder 容器）；再按“构建代理”一节的检查命令测代理速度。
  基础镜像升级或清理 BuildKit 缓存后的第一次部署会重新下载全部层，会明显更慢，属预期。
- 远程构建在 `resolve image config for docker-image://docker.io/...` 阶段报 `i/o timeout`：说明
  构建服务器访问不到 Docker Hub（直连被 DNS 污染；先确认上面的构建代理是否可用）。该失败发生在构建阶段，
  不会改动服务器。可用 `ssh qfy-test2 'curl -s -m6 -o /dev/null -w "%{http_code}\n" https://registry-1.docker.io/v2/'`
  确认（`401` 为可达，`000` 为不可达）。本机能访问 Docker Hub 时，改用 `DEPLOY_BUILD_MODE=local make deploy-production`
  在本机构建后上传；Apple Silicon 本机构建 amd64 走模拟，耗时明显更长。网络恢复后可回到默认的远程构建。
- 安装阶段停在“等待健康检查”：先看 API 是不是还在启动。`systemctl show -p ActiveState,NRestarts deploy-go-api` 为 `active` 且 `NRestarts` 没增加说明进程活着，慢启动会等到 `DEPLOY_HEALTH_MAX_WAIT_SECONDS`；`ss -ltn | grep 30100` 没有监听说明还没完成启动恢复。0.3.33 及以前，API 启动恢复要把全部中心日志逐条重放，且每页都从段头重新解析（二次复杂度），360MB 日志约 197 秒，期间 `runtime-log-files` 线程占满一个核且端口不监听；0.3.34 起改为顺序重放，通常几秒。
- 主密钥异常：若文件为空、为符号链接或非普通文件，安装器会拒绝继续。应从可信备份恢复原密钥，不能直接重新生成。
- 检测到未完成部署：说明上次安装可能被 `SIGKILL`、掉电或主机重启中断。不要再次部署覆盖现场；根据提示的 `.rollback.*` 目录核对并恢复产物、环境文件和 unit，确认旧服务健康后再移走该目录。
- Web 刷新 404：确认运行的是 `deploy/production/web_server.py`。
- `/api` 502：确认 `deploy-go-api` active，且 `web_server.py --api` 指向 `127.0.0.1:30100`。
- Agent 进程正常但节点持续离线：使用 WebSocket Upgrade 请求检查 `/api/v1/agent/control`；生产 `web_server.py` 必须保留 `Connection`、`Upgrade` 和 `Authorization` 并建立双向隧道，不能把控制连接当作普通 HTTP 请求转发。
- Agent 安装命令不可用：需要配置 HTTPS 的 `DEPLOY_GO_PUBLIC_BASE_URL`，并已通过部署脚本安装远程构建的 Agent release。
- API 与 Web 默认仅监听 loopback；正式域名的 HTTPS/WSS 终止与转发由服务器现有反向代理负责。

## 回滚

部署脚本不会覆盖已有数据库和主密钥。安装期间的服务重启或健康检查失败会自动恢复上一版产物、环境文件和 unit。需要人工回滚更早版本时，把旧 API 二进制或旧 Web 目录放回并重启：

```bash
install -m 0550 -o root -g deploy-go /path/to/old-deploy-go-api /opt/deploy-go/api/deploy-go-api
systemctl restart deploy-go-api
systemctl restart deploy-go-web
```

数据库迁移只能前进。如果节点遥测 migration 已执行，旧二进制可能无法启动；此时应先确认备份与恢复路径，不能直接依赖二进制回滚。回滚控制面前还应确认 Agent 会按旧控制面支持的协议范围协商降级，降到 v11 时停止 telemetry，而不是要求先修改真实节点。

系统设置（`system_settings` 里 `runtime` 这一行）会随版本增加字段。管理员在新版本里保存过设置后，再回滚到不认识新字段的旧二进制，旧版读取该 JSON 会失败（设置页和依赖设置的 worker 会报错）。人工回滚前先确认目标版本是否认识当前设置里的全部字段（登录会话时长改动新增了 `session_idle_days`、`session_absolute_days`，更早还加过 `max_total_log_bytes`）；不认识时，先对数据库做备份，再用 `sqlite3` 把 `runtime` 行删除让旧版回落到默认值，之后在设置页重新保存。登录会话（`sessions`）的 `absolute_expires_at` 列是新增列，旧二进制会忽略它，不影响登录。

数据库备份不能替代制品目录备份。需要保留仍可重试的历史发布时，应在停止 API 后对 SQLite、`artifacts/objects` 与 `artifacts/quarantine` 做同一时点快照；恢复时保持原路径和所有者，再启动 API 让 reconciliation 核对数据库与文件事实。过期制品属于缓存，不应作为业务应用唯一发布物来源。

## 构建缓存与磁盘

- 远程构建用 `--target artifacts --output type=local` 导出产物，不再 `--load` 镜像，每次提交不再泄漏约 90MB 镜像。
- BuildKit GC 配置：保留 6GB、上限 20GB；缓存挂载（cargo registry、target、sccache、npm）约 1.9GB，**保留 30 天**（`remote-build.sh` 的 `cache_mount_keep`）。BuildKit 默认策略是 48 小时未使用就回收缓存挂载，部署间隔超过两天就会变成全量冷编译，见下一节。
- 本机实测（arm64）：冷构建 565 秒；仅改一个 Rust 文件 227 秒，缓存总量 5.41GB → 5.56GB，不随构建次数线性增长。
- 一次性清理旧 builder 与泄漏镜像属于对构建机的运行态操作，需用户明确授权。

### 构建缓存为什么会失效

2026-11-11 部署 0.3.34 时 Rust 构建 269 秒、前端 31 秒，部署日志（`.data/deploy-logs/`）显示 cargo 重新编译了 272 个第三方 crate，`sccache` 的结果是 0 命中、387 未命中。原因有三个，按影响排序：

1. **缓存挂载被 BuildKit 默认策略回收**。`docker buildx inspect <builder>` 的 `GC Policy rule#0` 是：对 `type==exec.cachemount`（cargo target、sccache、npm 缓存）超过 48 小时未使用就回收（缓存总量超过 488MiB 时）。上一次部署是 7 天前，缓存挂载已不存在，只剩 7 天前的普通层记录。现在 `remote-build.sh` 显式给出完整的 GC 策略，把这条规则的保留时间改为 30 天（其余规则与默认一致）。
2. **前端基础镜像的浮动标签被上游重新发布**。`node:22-alpine` 两次部署之间指向了不同的镜像（`fe4f6d74…` 变成 `0e175475…`），前端所有层失效，`npm ci` 重新下载 457 个包（15 秒），再加重新拉取镜像与元数据查询。现在基础镜像按摘要固定，见下一节。
3. **升版本号**。每个 crate 的 `version` 和 `Cargo.lock` 都变化，`CARGO_PKG_VERSION` 参与 cargo 指纹，所有工作区 crate 必须重编。这是升版固有的成本，与缓存无关；第三方依赖不受影响。

此外，构建阶段的 `COPY` 全部使用 `--link`：每个输入独立成层，前面的输入变化不再连带使后面的层失效、文件被重新复制并更新修改时间，从而让 cargo 误判没变的 crate 需要重编。

**怎么判断缓存是否生效**：看部署日志里的三个信号。`#<n> CACHED` 越多越好；`Compiling` 行里只应出现本次改动涉及的工作区 crate，大量 `Compiling <第三方 crate>` 说明缓存挂载丢了；构建末尾 `sccache --show-stats` 的 `Cache hits` 不应长期为 0。也可以直接看缓存挂载记录的最后使用时间：

```bash
ssh qfy-test2 'docker buildx du --builder <builder 名> --filter type=exec.cachemount'
```

**注意**：BuildKit 配置只在创建 builder 时生效，builder 名称随配置摘要变化。因此第一次带着新 GC 策略的部署会新建 builder，那一次仍是冷构建，之后才受益。

### 基础镜像摘要

`deploy/docker/release/Dockerfile` 的 `rust` 与 `alpine`、`remote-build.sh` 内联前端 Dockerfile 的 `node`，都按 `@sha256:` 摘要固定。好处有两个：浮动标签被重新发布时不会让缓存整体失效；构建输入不会被镜像站或标签悄悄替换。代价是上游的安全更新不会自动进来，需要有意识地更新：

```bash
docker buildx imagetools inspect docker.io/library/node:22-alpine --format '{{.Manifest.Digest}}'
docker buildx imagetools inspect docker.io/library/rust:1.99.0-alpine --format '{{.Manifest.Digest}}'
docker buildx imagetools inspect docker.io/library/alpine:3.22 --format '{{.Manifest.Digest}}'
```

取回的是 OCI 镜像索引摘要（多平台），把新值同时写入 Dockerfile 与 `remote-build.sh`，运行 `make deploy-production-check`（它会拒绝未固定摘要的 `FROM`）。更新后的第一次部署会重新拉取镜像并使依赖层失效，属预期。建议随 Rust 工具链升级（`rust-toolchain.toml`）一起更新，安全公告出现时按需更新。
