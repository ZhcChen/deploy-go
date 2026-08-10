# Agent 原生结构化特权 Release 终审

## 结论

**代码与本地门禁：Go，可以提交 main。** U1-U8 已按权威计划完成；`make privileged-release-check`、`make check`、生产部署契约、Linux cgroup v2 容器测试均通过。

**WSL 测试节点：No-Go，仍等待用户新的明确授权。** 本复核未连接、未修改任何真实节点；U9 不在本轮执行范围。

**生产节点：No-Go。** 当前 main 已补齐 release 签名私钥生成与注入的阻断项，但未获得部署授权，不执行正式部署，也不发起 qfy-voucher-hub 或任何业务部署。

## 复核范围

- U6：PrivilegedRelease phase 持久化后的 Agent 重启恢复、重复取消、断线续传、唯一终态、授权超时清理。
- U2 阻断：生产 release 专属签名密钥的生成、复用、权限、备份、回滚、systemd 注入与契约测试。
- U7：cgroup v2 缺失时安装失败回滚、Agent/runner/executor 运行态版本配对诊断、0.2.0 fixture 同步。
- U8：聚焦门禁、全仓门禁、OpenAPI/双端客户端一致性与兼容回归。

## 已验证

- `make privileged-release-check` 通过：Linux cgroup v2 容器 4 项（含内置 `deploy-go-agent privileged-release-self-test`）、runner 身份边界 2 项、release authorization/agent protocol/executor/agent/API 聚焦测试、OpenAPI/client 检查、DeploymentFlow 13 项。
- `make check` 通过：cargo workspace 全量测试与 doc-test、`api-openapi-check`、`api-client-check`、admin 94 项、Flutter 51 项、client sensitive 145 文件、生产部署契约、launcher/demo 契约。
- `cargo fmt --all --check` 与 `cargo clippy --workspace --all-targets -- -D warnings` 通过。
- Linux 隔离容器 `agent/tests/install.bats` 13/13 通过，包含“cgroup v2 缺失失败并回滚上一配对版本”。
- `make deploy-production-check` 通过，契约测试动态读取 API 0.2.0、控制协议 v7。

## 发现与修复

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
- 生产环境尚未实际生成 `/etc/deploy-go/release-signing.key`；未授权不执行部署。
- macOS 无法本机证明的 systemd/Bats 动态项，以隔离 Linux 容器结果为准；真实 Linux systemd 首装/升级演练保留到 U9 授权后执行。
