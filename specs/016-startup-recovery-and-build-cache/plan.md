# API 启动恢复提速与部署构建缓存实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

事故证据：新版 API 启动后约 3 分钟不监听端口，`runtime-log-files` 线程 100% CPU，`gdb` 采样栈在 `read_page` → `read_record` → `serde_json`；日志目录 360MB、11 个满段。代码：`recover_locked` 从序号 0 用 `page(cursor, 500, 8MiB)` 反复读取，`read_page` 每次从段头解析到游标，一个满段（约 6 万条）解析约 360 万次。构建证据（部署日志）：272 个第三方 crate 重编、sccache 0 命中；`docker buildx inspect` 的 `rule#0` 对缓存挂载 48 小时未用即回收；`node:22-alpine` 摘要变化使前端层全部失效。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md)：恢复语义不变；不手改线上状态；部署需用户授权；版本不再变更（0.3.34 尚未上线）。

## 实现方案与文件范围

1. `runtime-log/src/store.rs`、`lib.rs`：`replay` 与 `SegmentStore::replay`，附 4 个测试。
2. `api/src/runtime_logs.rs`：`FileCommand::Replay`、有界通道（容量 4）、批次 500 条或 8MiB；`api/src/runtime_log_ingest.rs`：`recover_locked` 改用重放。
3. `deploy/production/install.sh`：`unit_still_starting` 与带服务名的 `wait_for_url`；`test-install-contract.sh` 增加断言，并以假 `systemctl`/`curl` 本地执行验证场景。
4. `deploy/production/remote-build.sh`：BuildKit 完整 GC 策略（`cache_mount_keep="720h"`）、前端镜像摘要；`deploy/docker/release/Dockerfile`：摘要与 `COPY --link`；契约测试对应断言。
5. 文档：生产手册、解决记录；登记 016。

## 验证计划

存储层新测试与基准对比；API 的 runtime_log 相关测试；`install.sh` 场景脚本；本机真实 BuildKit 验证 GC 策略解析与生效；本机用真实 Dockerfile 构建并做新旧对照实验；`make api-check`、`make agent-check`、`make check`、`make docs-check`、`make spec-kit-verify`、`make whitespace-check`。

## 风险与回退

回退为回滚提交；GC 策略错误只会让构建阶段失败（安装前），不影响线上。上线时 builder 名称变化，首次冷构建；恢复提速需在真实环境实测，服务器是 musl 目标，数值与本机不同。
