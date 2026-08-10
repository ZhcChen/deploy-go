# Agent 原生结构化特权 Release 终审

## 结论

**代码与本地门禁：Go，当前 main 已通过并部署正式主控。** U1-U8 已按权威计划完成；`make privileged-release-check`、`make check`、生产部署契约、Linux cgroup v2 容器测试均通过；正式主控 `deploy.quanxinfu.com` 已授权部署 API/Web/Agent 0.2.0。

**WSL 测试节点：No-Go，仍等待用户新的明确授权。** U9 未执行；本复核未连接、未修改 WSL 节点。

**生产业务节点：No-Go。** 只部署了 Deploy Go 正式主控本身，未连接或修改生产业务节点，未发起 `qfy-voucher-hub` 或任何业务部署。

## 复核范围

- U6：PrivilegedRelease phase 持久化后的 Agent 重启恢复、重复取消、断线续传、唯一终态、授权超时清理。
- U2 阻断：生产 release 专属签名密钥的生成、复用、权限、备份、回滚、systemd 注入与契约测试。
- U7：cgroup v2 缺失时安装失败回滚、Agent/runner/executor 运行态版本配对诊断、0.2.0 fixture 同步。
- U8：聚焦门禁、全仓门禁、OpenAPI/双端客户端一致性与兼容回归。

## 已验证

- `make privileged-release-check` 通过：Linux cgroup v2 容器 4 项（含内置 `deploy-go-agent privileged-release-self-test`）、runner 身份边界 2 项、release authorization/agent protocol/executor/agent/API 聚焦测试、OpenAPI/client 检查、DeploymentFlow 13 项。
- `make check` 通过：cargo workspace 全量测试与 doc-test、`api-openapi-check`、`api-client-check`、admin 94 项、Flutter 51 项、client sensitive 145 文件、生产部署契约、launcher/demo 契约。
- `cargo fmt --all --check` 与 `cargo clippy --workspace --all-targets -- -D warnings` 通过。
- Linux 隔离容器 `agent/tests/install.bats` 14/14 通过，包含“cgroup v2 缺失失败并回滚上一配对版本”和“空控制器失败并回滚”。
- `make deploy-production-check` 通过，契约测试动态读取 API 0.2.0、控制协议 v7。

## 发现与修复

### 正式主控部署演练发现并修复的阻断项

在授权后的正式主控部署演练中发现并修复两个本地门禁无法覆盖的打包/兼容问题：

- **release Dockerfile 缺少工作区依赖**：`api/docker/release/Dockerfile` 与 `agent/docker/release/Dockerfile` 未复制新增的 `release-authorization` crate；API Dockerfile 还缺少 API 运行时嵌入的 `agent/release` schema 与 `agent/install/install.sh`。首次部署在 Docker 构建阶段失败。已补齐 `COPY` 并在 `deploy-production-check` 增加静态契约断言，两个镜像均重新构建通过。
- **历史 v3 manifest 与 0.2.0 schema 不兼容**：服务器上既有 0.1.0 发布物为旧 v3 格式，缺少新增必填字段 `runner_protocol`/`executor_protocol`，导致 0.2.0 API 启动时校验失败并回滚。已将这两个字段改为可选，新 manifest 仍用 `const` 强校验；新增回归测试 `accepts_v3_manifest_without_pairing_protocol_fields`，确保旧发布物可继续共存。

重新部署后正式主控验证通过：`deploy-go-api`/`deploy-go-web` active，`/healthz` 与 `/readyz` 正常，`/etc/deploy-go/release-signing.key` 为 `0440 root:deploy-go`，API 环境已注入 `DEPLOY_GO_RELEASE_SIGNING_KEY_FILE`，Agent 发布目录同时保留 `0.1.0` 与 `0.2.0`；公网 Web 返回 200，未认证 API 返回 401。

### 已修复：生产签名密钥首次生成后的回滚语义

原实现先生成/校验 `release-signing.key`，再建立部署回滚备份；首次部署失败时回滚备份的正是新密钥，无法恢复到“密钥不存在”的初始状态。已把密钥处理移到 `rollback_armed=1` 与备份建立之后：

- 已存在且为普通非空文件时复用，空文件、符号链接或非普通文件拒绝覆盖；
- 首次生成时 `release_signing_key.absent` 已存在，部署失败回滚会移除新密钥；
- 已存在旧密钥时回滚恢复旧文件，权限统一 `0440 root:deploy-go`；
- 契约测试新增行号断言，密钥生成必须发生在回滚备份建立之后。

### 已修复：移动端把 Agent 0.2.0 误报为版本异常

`admin-app/lib/api/contracts.dart` 的 `supportedAgentVersion` 仍为 `0.1.0`，会把新 Agent 标记为“版本异常”。已同步为 `0.2.0`，并通过 `make admin-app-check`（Flutter analyze + 51 项测试）。

### 已修复：本地开发手册缺少两把签名密钥配置

API 0.2.0 服务模式启动即加载终端 capability 与 release 两把签名私钥，但 `docs/runbooks/local-development.md` 未记录。已补充配置表、`0440 root` 普通文件要求、本地生成示例与禁止输出私钥正文的约束。

### 已修复：sysfs 伪文件 size=0 导致 cgroup v2 误判

真实 Ubuntu 与 WSL 安装均报 `cgroup_v2_missing`，但 `/sys/fs/cgroup/cgroup.controllers` 内容非空。根因是安装器用 `[[ -s ]]` 判断控制器，而 sysfs 伪文件 `stat` size 恒为 0，`cat` 有内容时 `test -s` 仍为假。已改为 `grep -q .` 按内容判断，新增静态契约检查防止回退，并增加空控制器回滚 Bats 用例（Linux 容器 14/14 通过）。正式主控已重新部署，线上安装脚本与仓库哈希一致。

### 已修复：executor peer PID 绑定未释放导致 doctor/self-test 被挡

节点安装成功但 `doctor` 报 `EXECUTOR_PROTOCOL`/`PRIVILEGED_RELEASE` 不可用，executor journal 反复 `unauthorized local peer`。根因是 `PeerIdentityRegistry` 把首个连接的 PID 永久钉住：Agent 服务进程存活时，后续一次性 doctor/probe/self-test 进程以不同 PID 连接被拒绝。已改为按连接生命周期持有并释放 PID 绑定（同 PID 引用计数，连接全部关闭后释放），新增 Linux 单元测试覆盖释放、并发拒绝和同 PID 多次连接；`cargo fmt`、executor 聚焦测试通过。该修复需重新构建 0.2.0 发布物并让节点重跑幂等安装。

### 复核无问题项

- executor 的 `VersionProbe`/runner 的 `Version` 请求均不携带执行输入、不改变 sequence、不可被用作命令注入；旧版本端无法识别时诊断降级为 warn，协议与版本检查仍保持 fail/warn 语义。
- `RUNTIME_PAIRING` 将 Agent/runner/executor 运行版本不配对判为 decisive failure，健康且版本一致时通过。
- Agent 安装器在事务激活后执行 cgroup v2 检查，失败走整对回滚；系统目录、unit 校验和既有安全契约未放宽。
- production `install.sh` 不输出私钥正文，API 只注入 `DEPLOY_GO_RELEASE_SIGNING_KEY_FILE` 路径；`ReadOnlyPaths`、`ProtectSystem=strict` 与 API 端 `0440 root` 校验一致。
- OpenAPI/Web/Flutter 生成客户端仅同步 API 版本 0.2.0，无字段漂移；`make api-openapi-check`、`make api-client-check` 通过。
- 旧 launcher、低权限 release 与 v1-v6 兼容路径未改动；`examples/privileged-release-launcher` 与分支部署 demo 契约保持通过。

## 复核维度摘要

- **Correctness**：恢复测试证明重启后只走 `ReleaseOutput`/`ReleaseStatus`，第二次 `ReleaseStart` 会使测试失败；重复 cancel 与断线恢复收敛到唯一终态。
- **Security**：Env/artifact gate 失败时 executor 零调用；release 授权超时清理 pending waiter；签名密钥与终端密钥分离且私钥正文不出安装器、日志、API 响应。
- **Reliability**：签名密钥纳入事务备份/回滚；cgroup 缺失 fail-fast 并整对回滚；版本探测带超时且不可读时仅 warn。
- **API contract**：OpenAPI 0.2.0、控制协议 v7、executor 本机协议 v2、部署目标字段 `privileged_release` 与双端生成客户端一致。
- **Testing**：U6 恢复 4 项、cgroup 缺失回滚、版本配对 fail 路径、production 密钥契约均有自动化覆盖。

## 未决事项

- WSL 测试节点升级需用户新的明确授权（U9），本复核不构成授权。
- 正式主控已生成 `/etc/deploy-go/release-signing.key`（`0440 root:deploy-go`）并部署 0.2.0；生产业务节点仍未授权连接或修改。
- macOS 无法本机证明的 systemd/Bats 动态项，以隔离 Linux 容器结果为准；真实 Linux systemd 节点 doctor/self-test 通过后再汇总 U9 最终结论。
