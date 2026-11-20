# 本机编辑、qfy-test2 构建测试

状态：规格就绪
来源：用户需求（2026-11-20 会话）：“赶紧按照 Obsidian 的规范，去 qfy-test2 进行本地开发环境的运行和测试，本机现在 CPU 100% 有点撑不住了”。规范原文：Obsidian `QFY/开发规范/本机编码与远端开发验证`、`QFY/开发规范/Docker 开发环境与共享网络`；资源事实源 `~/ai-space/server/dev-environments/REGISTRY.md`。

## 背景

- 规范要求：本机编辑、格式化与编辑器即时检查；构建、自动化单测与集成测试默认在 qfy-test2 执行。源码以快照增量同步到 `/opt/qfy-dev/<项目>/dev/<分支键>/source`，在一次性工具容器里运行，只接入既有 `dev-shared-network`，不发布宿主端口，依赖缓存可重建，证据回传，同一分支加锁串行。
- 现状：Deploy Go 只有一套远端 API/Web 开发栈（`docs/runbooks/remote-development.md`，用于完整装配与浏览器验收），构建与测试全部在本机执行。`make check` 的 Rust 部分本机实测约 70 秒测试运行，加上编译，改一个 API 源文件后全量约 3 分钟，占满 18 核；与其他项目同时开发时本机负载持续 30 以上。
- 同机已有 qfy-finance、qfy-cgb、qfy-malls、keel、cc-auth-go 等按同一规范接入，通用协议 `remote_sync.py`（协议版本 1）已在多个项目中使用。

## 目标与范围

为 Deploy Go 接入规范规定的远端构建与测试入口，复用通用协议，不另写一套。

范围：
1. 原样复制通用协议 `scripts/dev/remote_sync.py`（自 qfy-finance，正文逐字节一致，只改文件头说明）；新增合同 `scripts/dev/remote-test.json`、薄入口 `scripts/dev/remote-test.py`、运行适配 `scripts/dev/deploy_go_remote_runtime.py`。
2. 工具镜像 `deploy/dev-tools/Dockerfile.rust`、`Dockerfile.node`，基础镜像按摘要固定，镜像标签为 Dockerfile 内容摘要。
3. 登记命令：`rust-check`、`rust-fmt`、`rust-clippy`、`rust-test [nextest 参数]`、`web-check`、`web-test [文件]`；Makefile 入口 `remote-test`、`remote-dry-run`、`remote-status`、`remote-verify`、`remote-check`。
4. 本地回归 `scripts/dev/test_remote_test.py`（并入 `make dev-remote-check`）；运行手册、工程指南、AGENTS 同步；资源登记。

5. （首次远端运行后追加）远端在 Linux 上执行了 macOS 跳过的用例，暴露两处测试环境适配问题，按测试原有语义修正，不改任何断言：
   - `agent/tests/executor.rs`、`agent/tests/image_release.rs` 把临时根硬编码为“当前目录/target/tmp”，源码只读挂载时写入失败（8 个用例）；改用 Cargo 为集成测试提供的 `CARGO_TARGET_TMPDIR`（编译输出目录下的 tmp，绝对路径、无符号链接），本机仍落在 `target/tmp`。
   - `agent-executor/tests/release_lifecycle.rs` 有 3 个用例缺少同文件其余用例都有的 `linux_cgroup_unavailable()` 跳过守卫，在没有 cgroup 权限的 Linux 上启动发布子进程失败；补上守卫后，它们仍在设置 `DEPLOY_GO_RUN_CGROUP_V2_TEST=1` 的特权容器（`make agent-executor-cgroup-check`）与 macOS 上执行。

明确不做：
- 不改 Rust、Web 的产品代码与断言；测试代码只做上面第 5 项的环境适配。不改协议、migration、OpenAPI。
- 需要特权的 Linux 专项（cgroup、runner 身份隔离、特权终端与发布）不迁移：它们需要 `--privileged` 等能力，仍按原目标在本机隔离容器执行。
- 文档、Spec Kit、空白、敏感模式、API client 漂移等轻量检查（依赖 `.git` 或耗时不到几秒）仍在本机。
- 不更新、不停止、不接管已有的远端 API/Web 开发栈；不删卷、不 prune、不改 daemon、不建网络；不碰正式环境。
- 测试效率优化（依赖编译优化、OpenAPI 检查合并、文档测试空跑等）另行处理，不混在本规格。

## 用户场景与验收

### US1：本机只编辑，构建测试在远端

- 优先级：P1
- Given：本机 CPU 紧张
- When：执行 `make remote-check` 或 `make remote-test REMOTE_COMMAND="rust-test -p deploy-go-api --test platform"`
- Then：本机只生成快照并同步（秒级），编译与测试在 qfy-test2 的一次性容器内执行，结束后日志、结果与证据回传本机 `tmp/remote-dev/<run_id>/`，退出码与远端测试一致
- 关键失败或边界场景：未提交的工作区也可测（开发快照，证据标记 dirty）；同一分支同时只能有一个运行；快照期间源码变化则中止。

### US2：远端运行不影响正式环境与其他项目

- 优先级：P1
- Given：qfy-test2 同时承载 Deploy Go 正式环境与其他项目
- When：远端执行 Rust 全量编译测试
- Then：容器受 CPU、内存、进程数硬上限约束，调度权重低于默认；只接入 `dev-shared-network`，无宿主端口，无 Docker 卷；运行结束容器即删除；正式 API/Web 不重启

### US3：结果可信

- Then：工具链与 `rust-toolchain.toml` 完全一致；`--locked` 防止锁文件漂移；没有运行任何用例、或找不到测试汇总行时按失败处理；特权用例因环境变量门控而跳过，不会在普通容器误跑。

## 功能要求

- FR-001：通用协议原样复制，正文与来源逐字节一致；合同固定 `qfy-test2`、`/opt/qfy-dev/deploy-go`；`CLAUDE.md` 符号链接与 `.data/` 不同步；`examples/templates/*/.env.example`（被 `include_str!` 编进 API）通过 `include_tracked` 同步。
- FR-002：命令严格白名单；`rust-test` 只接受选择用例的参数（`-p`、`--test`、`--lib`、`-E` 等），拒绝 `--config`、`--target-dir`、`--manifest-path`、`-Z`、绝对路径、`..` 与 shell 元字符。
- FR-003：源码只读挂载；Cargo 注册表、git 与 target 缓存放在 `/opt/qfy-dev/deploy-go/cache/rust/<键>/`，键为工具镜像与架构（不按 `Cargo.lock` 划分，cargo 按指纹增量判断）；npm 依赖按锁文件摘要缓存。
- FR-004：Rust 容器 6 CPU、12 GiB；Node 容器 4 CPU、6 GiB；`--cpu-shares 512`；`--pids-limit 4096`；`no-new-privileges`；日志 local 10m×3；标签 `qfy.dev.{project,instance,branch,run,role}`。
- FR-005：`rust-check` = fmt → clippy → nextest 全 workspace → 文档测试（workspace 有文档代码块时才执行）。`web-check` = lint → typecheck → vitest → vite build。
- FR-006：Makefile 入口与帮助；手册写清用法、资源、证据位置与本机仍需执行的检查。
- FR-007：资源登记写入 `REGISTRY.md`。

## 成功标准

- SC-001：`make remote-check` 在 qfy-test2 通过，用例数与本机一致（Rust 全量，Web 全量）。
- SC-002：执行期间本机 CPU 占用仅限快照与同步（秒级）。
- SC-003：远端运行前后正式 API/Web 的 PID、健康不变；运行结束无残留容器；未新建网络或卷。
- SC-004：`make dev-remote-check`、`make docs-check`、`make whitespace-check`、`make spec-kit-verify` 通过。

## 风险

- qfy-test2 负载：当前多个项目同时迁移，负载常在 20 至 40（20 核）。以硬上限与低调度权重保证正式服务优先；首次冷编译耗时较长，属预期。
- 缓存占用磁盘：target 可达数十 GB；登记上限与清理方式（清理需另行授权，只删本项目缓存目录）。
- Linux 与 macOS 差异：远端会执行 macOS 上被跳过的 `cfg(target_os = "linux")` 非特权用例，可能暴露此前未覆盖的问题；这是收益，按真实问题处理，不放宽断言。
- 远端测试输出在运行结束后一次性回传，运行中不实时显示；需要观察时用 `make remote-status`。

## 未决事项

无。以下为默认选择，用户可调整：Rust 6 CPU/12 GiB、Node 4 CPU/6 GiB、调度权重 512。
