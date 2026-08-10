---
artifact_contract: "ce-handoff/v1"
created_at: "2026-08-10T09:18:14Z"
title: "Agent 原生结构化特权 Release 实施交接"
summary: "记录 privileged_release 计划当前实现、验证证据、上线阻断和剩余 U6-U9 工作。"
keywords: ["privileged_release", "agent-v7", "executor-v2", "handoff"]
cwd: "/Users/chen/code/deploy-go"
resume_focus: "完成 U6 恢复竞争测试、生产 release signing key 配置、U8 复核和经授权后的 WSL 灰度。"
repository: "deploy-go"
branch: "main"
head: "ac50d77b839fea0ecfbc8d578d3d44cbcd30c9f1"
worktree_path: "/Users/chen/code/deploy-go"
---

# Agent 原生结构化特权 Release 实施交接

## 当前目标与边界

权威计划是 `docs/plans/2026-08-10-001-agent-native-privileged-release-plan.md`。目标是让两阶段部署的 prepare 继续由低权限 runner 执行，管理员授权目标的 release 由 Agent 内置 root executor 执行固定 `make --no-print-directory deploy-go-release`，不再要求业务节点安装应用 launcher 或 sudoers。

必须保持以下边界：

- 不提供任意 root command、executable、args、env map 或 PTY 复用。
- `privileged_release` 默认关闭且仅管理员可修改，授权事实固定在 deployment snapshot。
- 旧 launcher、低权限 release 和 v6 以下普通任务继续兼容，特权路径失败不得自动降级。
- 不操作生产节点、其他业务项目，不发起 `qfy-voucher-hub` 部署。
- WSL 测试节点升级必须由用户在执行时重新明确授权；本交接不授予远程执行权。

## 仓库状态

- 分支：`main`
- HEAD：`ac50d77 fix: 提前持久化特权发布恢复阶段`
- 交接生成前工作区干净。
- Agent/API/executor 应用版本：`0.2.0`
- Agent 控制协议：v7，兼容下限 v1。
- executor 本机协议：v2；runner broker 协议：v1。

## 已完成实现

### U1-U3：规范、目标授权、快照和控制协议

- 长期规范与运行手册已定义 `privileged_release`、root commit 信任、固定入口、环境白名单、无降级和 WSL 灰度边界。
- migration `api/migrations/0018_privileged_deployment_release.sql` 新增默认关闭字段。
- API、Web、OpenAPI、Web/Flutter 生成客户端、审计、preview/snapshot hash 全链路携带该字段。
- 控制协议 v7 增加 `privileged=true`、独立 `privileged_context`、`privileged_release` capability 和 release authorization 握手。
- API 在创建特权 release task 前检查协议 v7 和 capability；不兼容时明确失败，不创建可执行任务。

### U4-U5：executor v2、授权、bundle、cgroup 与磁盘门禁

- `release-authorization` 使用与 PTY 分离的 signer/audience/nonce。
- executor 离线验证主控短期授权，绑定 deployment/target run/node/agent/snapshot/commit/digest/deadline。
- executor 只执行固定 Make target，`env_clear()` 后仅注入计划中的 9 个平台变量。
- checkout、artifact、manifest、Env 会复制为 root-owned 只读 bundle；拒绝路径逃逸、symlink、hardlink、非普通文件和 digest 不一致。
- durable release job 支持 status/output/cancel、单 job 输出限制、全局 2 GiB 预算、512 MiB 低磁盘水位和 7 天 retention。
- retention 只删除终态 job 的 bundle/output，保留 `state.json` 与 `claims.json`，因此 status 和 nonce 防重放仍有效。
- Linux cgroup v2 测试覆盖忽略 TERM、setsid、double-fork、固定 Make launcher 和 `cgroup.kill`。

### U6：Agent bridge（主体完成，竞争测试仍需补）

- Agent 保留原 artifact/Env/commit admission，再向 API 请求签名授权并调用 executor。
- stdout/stderr、`DEPLOY_GO_EVENT`、退出码和结果进入原 journal/状态机。
- executor frame 按 sequence 原子落盘，Agent 崩溃重放不会重复日志；300 帧分页测试证明终态前会拉完 `128 + 128 + 44` 帧。
- 固定 job ID 为 `release_{task_id}`；reconcile 只 attach/output/status，不应重复 start。
- `ac50d77` 已把 `PrivilegedRelease` phase 和 offset 提前到 `ReleaseStart` 之前持久化，关闭“executor 已启动、journal 仍是旧 phase”的崩溃窗口。

### U7：配对安装、Doctor 和内置 self-test

- installer 同时下发 terminal 与 release 两把公钥，私钥不进入节点。
- manifest v3 明确 Agent/executor `0.2.0`、控制协议范围、runner v1、executor v2；executor v1 manifest 会被拒绝。
- 安装健康检查分别验证 PTY 与 `DeploymentRelease` capability。
- Doctor 分别显示 runner、executor v2、PTY 和结构化特权 release capability。
- 新命令 `sudo -u deploy-go-agent /usr/local/bin/deploy-go-agent privileged-release-self-test` 使用无参数独立 operation。executor 生成固定 Makefile，在 release cgroup 中验证 root UID、环境白名单、事件输出和清理；请求不接受 command/args/path/env。

## 关键提交

- `8755fd4`：增加特权发布授权握手协议。
- `ca65510`：由主控签发特权发布授权。
- `25d2046`：接入 Agent 特权发布执行桥。
- `5e62248`：限制特权发布任务磁盘占用。
- `eebf487`：覆盖特权发布分页日志收敛。
- `9a2ad50`：配对安装特权发布 executor 配置。
- `ac445db`：校验 Agent 特权发布配对协议。
- `1230827`：增加 `make privileged-release-check`。
- `dc789b3`：发布 Agent/executor/API `0.2.0` 配对版本。
- `03c8168`：增加特权发布内置 self-test。
- `ac50d77`：提前持久化特权发布恢复阶段。

## 已有验证证据

`make privileged-release-check` 在 `1230827` 后完整通过，包含：

- release authorization、Agent protocol、executor、Agent 全套测试。
- API target/snapshot/two-stage/dispatcher/audit/OpenAPI 测试。
- OpenAPI 与双端生成 client 漂移检查。
- Web `DeploymentFlow` 13 个测试。
- 隔离 privileged Linux 容器中的 cgroup v2 与 runner 身份测试。

后续增量验证：

- `0.2.0` 版本升级后 API release/node/end-to-end、Agent diagnostics 和 manifest 测试通过。
- self-test 增加后 executor protocol、Agent executor client/diagnostics 通过。
- 最新 Linux cgroup 容器 4 项通过，包含内置 self-test。
- `ac50d77` 后 privileged bridge 2 项、recovery 6 项通过。
- `agent/tests/task_handler.rs` 全文件运行曾有一个旧普通部署测试首次超时；单独精确复跑立即通过。应在最终聚合门禁再次观察，不要隐瞒该时序波动。

## 最高优先级阻断

### 生产部署缺少 release signing key 配置

API `api/src/main.rs` 已强制加载 `DEPLOY_GO_RELEASE_SIGNING_KEY_FILE`，但当前 `deploy/production/` 和相关部署 runbook 尚未生成、安装或注入这把 release 专属私钥。当前 `main` 在生产环境缺少该变量时会启动失败。

下一步必须：

1. 按 terminal signer 的安全模式新增独立 release signing key 文件生成/权限/备份/回滚逻辑。
2. systemd/API 环境只注入文件路径，不输出私钥正文。
3. 部署脚本契约测试覆盖首次生成、重复部署复用、权限和回滚。
4. 更新生产部署 runbook。

完成并验证前不得部署当前 `main`。

## 剩余计划

### 1. 完成 U6 竞争与恢复证明

- 为 `ac50d77` 补专门测试：模拟已持久化 phase 后 Agent 重启，executor 只允许 output/status；收到第二次 `ReleaseStart` 必须失败。
- 覆盖 privileged cancel 重复请求只对应同一 job、断线后从 durable offset 继续、Env/artifact gate 失败 executor 零调用。
- 覆盖 release authorization timeout 后 pending waiter 清理。
- 复核 monitor 与 cancel/reconcile 并发只产生一个终态结果。

### 2. 补齐 U7 安装边界

- 检查 installer 是否明确验证 cgroup v2 缺失并回滚，而不只依赖 executor probe 超时。
- 增加三个服务/二进制版本不一致的 Doctor 或 installer 证据；当前 manifest 已保证下载配对，但运行态版本配对输出仍偏弱。
- 更新 `agent/tests/fixtures/release/0.2.0` 时保持当前 unit/template 与 manifest 同步。

### 3. U8 完成门禁和重要安全复核

- 修复 production release signing key 后重新执行 `make privileged-release-check`。
- 执行 `make check`；如 Flutter/浏览器环境不可用，必须记录具体未执行项，不能用局部测试代替。
- 执行 simplify 与重要安全 code review，至少检查 correctness、security、reliability、API contract 和测试覆盖。
- 将结论写入 `docs/reviews/2026-08-10-agent-native-privileged-release-review.md`。
- 重点审查 `agent/src/task_handler.rs` 体积、reconcile/cancel 竞争、self-test operation 边界、release job retention、签名 key 生命周期和错误日志是否泄密。

### 4. U9 仅在重新授权后执行

- 用户必须在新的对话中明确授权连接和升级具体“测试环境节点（WSL）”。
- 只升级该测试节点，不操作生产节点或其他项目。
- 验证 Agent `0.2.0`、控制协议 v7、runner v1、executor v2、三个服务、在线状态和 `privileged_release` capability。
- 运行内置 self-test，不创建业务 deployment，不触发 `qfy-voucher-hub` prepare/release。
- 失败按 `docs/runbooks/privileged-agent-release.md` 回滚配对版本。

## 最终需向用户报告

- 新 Agent 版本：`0.2.0`；控制协议 v7；executor v2。
- testing 节点是否已升级并在线（当前尚未授权、尚未执行）。
- 部署目标字段：`privileged_release`。
- release 环境变量契约：`DEPLOY_ID`、`DEPLOY_ENVIRONMENT`、`DEPLOY_RELEASE_VERSION`、`DEPLOY_COMMIT_SHA`、`DEPLOY_MODULES`、`DEPLOY_TARGET`、`DEPLOY_ARTIFACT_DIR`、`DEPLOY_ENV_DIR`、`DEPLOY_CANCEL_FILE`。
- 业务仓库不再需要节点 launcher、sudoers 或系统目录安装脚本；仍需提供固定 `make deploy-go-release` 及其业务发布脚本。

## 建议下一位 AI 首轮操作

1. 阅读本交接、权威计划的 U6-U9、`docs/runbooks/privileged-agent-release.md`。
2. 执行 `git status --short`、`git log -8 --oneline`、`git diff --check`，确认 HEAD 和工作区。
3. 先补 `ac50d77` 的“不重复 ReleaseStart”恢复测试。
4. 再修 production release signing key 配置与契约测试。
5. 逐闭环提交推送，不修改原计划正文，不进行任何远程操作。
