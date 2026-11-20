# 本机编辑、qfy-test2 构建测试任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：已实施，远端与本地门禁验证通过，待用户签收。勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-001 复制通用协议，新增合同与薄入口。
- [ ] T002 FR-002 至 FR-005 运行适配器与工具镜像。
- [ ] T003 FR-006 Makefile 入口、`.gitignore`、本地回归测试。
- [ ] T004 SC-001 至 SC-003 远端首次运行验证：预览、工具镜像构建、`make remote-check`，核对用例数、正式环境与残留资源。
- [ ] T005 FR-006 FR-007 手册、工程指南、AGENTS 同步，登记 021 与 REGISTRY。
- [ ] T006 SC-004 本地门禁验证、自审、提交推送。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已完成 | 已验证 | 不适用 | 待用户签收 | 通用协议 `scripts/dev/remote_sync.py` 自 qfy-finance 复制，文件头之后的正文与来源逐字节一致（`diff` 核对）；合同 `remote-test.json` 固定 `qfy-test2`、`/opt/qfy-dev/deploy-go`，排除 `/CLAUDE.md`（符号链接）与 `/.data`，放行 3 个 `examples/templates/*/.env.example`；快照 1063 个文件约 9.6 MiB、无未跟踪文件；薄入口 `remote-test.py` |
| T002 | 已完成 | 已验证 | 不适用 | 待用户签收 | 适配器：命令白名单（`rust-check`、`rust-fmt`、`rust-clippy`、`rust-test`、`web-check`、`web-test`）；工具镜像 `deploy-go-dev-tools-rust:d19da7d4…`（`rust:1.99.0-bookworm@sha256:114c7a44…`，clippy、rustfmt、cargo-nextest 0.9.143，首次构建约 2.5 分钟）与 `deploy-go-dev-tools-node:3e517131…`（`node:22-alpine@sha256:1314fb30…`）；容器 6 CPU/12 GiB 与 4 CPU/6 GiB、`--cpu-shares 512`、`no-new-privileges`、只接 `dev-shared-network`、源码只读。首轮远端运行发现并修正：①协议把同步文件的修改时间统一设为 1，Cargo 按修改时间判断，改过的文件不会重编，第二轮跑出的是旧代码的结果——新增按内容摘要同步的 `runtime/rust-tree`，只改写变化的文件并刷新修改时间；②`tsc -b` 写 `tsconfig.{app,node}.tsbuildinfo` 到只读源码——逐个映射可写文件；③Vitest 彩色输出导致汇总行匹配不到而误判失败——解析前去除控制符并设 `NO_COLOR`。另补 `rust-check` 的 OpenAPI 产物漂移检查，与本机 `make api-check` 对齐 |
| T003 | 已完成 | 已验证 | 不适用 | 待用户签收 | Makefile 新增 `remote-test`、`remote-dry-run`、`remote-status`、`remote-verify`、`remote-check` 与帮助行（拆成独立目标，docs-check 才能识别）；`.gitignore` 增加 `/tmp/`（本机结果目录）；`scripts/dev/test_remote_test.py` 13 个回归（白名单接受与拒绝、合同映射与排除、模板放行、构建目录只改写变化文件且保持未变文件修改时间、清单路径越界、TS 子项目全部映射、彩色汇总行解析、镜像摘要固定、工具链与 `rust-toolchain.toml` 一致、资源上限），并入 `make dev-remote-check` |
| T004 | 已完成 | 部分验证 | 不适用 | 待用户签收 | 远端执行：`remote-dry-run` 同步 1.5 秒；`rust-check` 共跑 5 轮，前 4 轮依次暴露问题——8 个 Agent 用例把临时目录写在只读源码下（改用 `CARGO_TARGET_TMPDIR`）、3 个 executor 用例缺少同文件其他用例都有的 cgroup 跳过守卫（补上）、修改时间问题（见 T002）、`runtime_log_ingest` 重开存储时与旧写入线程退出存在竞态（新增 `reopen()` 等待锁释放，替换 4 处 `yield_now()`/固定睡眠写法）；断言均未改动。最终 `make remote-check` 端到端 1 分 53 秒、本机 CPU 约 2.5 秒：Rust 1013 个用例全部通过、4 个特权用例跳过（本机 macOS 为 991 个，Linux 多 22 个），fmt、clippy、OpenAPI 产物检查通过，nextest 67 秒；Web 31 个文件 208 个用例、构建成功。正式 `deploy-go-api`/`deploy-go-web` 前后 PID 2487950/2487956 不变、`NRestarts=0`、`readyz` 200；运行后无残留容器，网络 33、卷 6 不变。远端占用约 4.8 GB。**未验证**：改动的 4 个测试文件没有在本机 macOS 重跑（用户要求减轻本机 CPU；从代码看 macOS 行为不变）；`make agent-executor-cgroup-check`（特权容器）未跑，补的守卫与同文件其他用例完全一致 |
| T005 | 已完成 | 已验证 | 不适用 | 待用户签收 | `docs/runbooks/remote-development.md` 新增“远端构建与测试”（用法、命令对照、工作方式、资源与缓存、仍在本机执行的检查）并更新“环境选择与授权”；`docs/runbooks/development-guide.md` 增加“在哪里执行”；`AGENTS.md` 3.1 与 4 节更新默认执行位置与等价目标；登记 021；`~/ai-space/server/dev-environments/REGISTRY.md` 新增条目 |
| T006 | 已完成 | 已验证 | 不适用 | 待用户签收 | 本机门禁见提交记录：`make docs-check`、`make whitespace-check`、`make spec-kit-verify`、`make dev-remote-check`、`cargo fmt --all --check`；Rust 与 Web 门禁以远端 `make remote-check` 为准（见 T004）。提交与推送见 Git 历史 |
