# Rust 构建与测试链路优化计划

## 背景

Agent 模块源码复杂度不高，但当前 Rust 构建与测试耗时已经明显影响迭代效率：

- 冷编译 `cargo test -p deploy-go-agent` 约 4m53s；即使增量也要 40s 以上，且全量测试会构建
  `agent/tests/*.rs` 下 19 个独立 integration test 二进制。
- `Cargo.lock` 有 384 个 crate，Agent 依赖链包含 `reqwest`、`tokio`、
  `tokio-tungstenite`、`jsonschema`、`nix` 等重依赖。
- `target/debug` 当前约 146G，`deps` 约 93.8 万文件，`incremental` 约 32.4 万文件、
  3667 个 crate 增量目录；历史构建产物严重膨胀，文件系统扫描与增量写入反而拖慢构建。
- 当前没有 sccache、`cargo nextest`、profile 调优或 target 清理策略，只依赖 Rust 默认
  incremental，缓存收益已经被陈旧产物抵消。

## 目标

- 降低 Rust 冷构建与全量测试的墙钟时间，恢复增量构建收益。
- 控制 `target/` 体积，避免继续膨胀到影响磁盘与文件系统 I/O。
- 保持现有工具链、CI/门禁语义与测试覆盖不变，不改变业务行为。
- 本计划只做构建/测试链路优化，按用户批准逐执行单元推进。

## 设计

### 1. target 清理

- 清理历史陈旧产物，优先清理 `target/debug`，保留 `target/release` 与
  `target/deploy-release` 若仍需复用。
- 清理后执行一次全量 warm-up 构建，再测量冷/热构建基线。
- 后续在 runbook 或 Makefile 增加定期清理指引，避免再次膨胀。

### 2. sccache

- 本地安装 sccache，配置 `RUSTC_WRAPPER=sccache`。
- 让 debug/release、不同 feature 组合、重复构建复用第三方依赖编译产物。
- 不改变测试内容，只改变编译缓存层。

### 3. cargo nextest

- 安装并使用 `cargo nextest run -p deploy-go-agent` 作为日常全量测试入口。
- nextest 并行执行测试、失败隔离与输出更可控；`cargo test` 继续保留用于 CI 门禁。
- 聚焦测试仍可直接使用 `cargo test -p deploy-go-agent --lib <filter>` 或
  `cargo test -p deploy-go-agent --test <integration>`，避免构建全部测试目标。

### 4. profile 与构建配置

- 评估 workspace `[profile.dev]` 与 `[profile.dev.package."*"]` 配置：
  - 控制依赖 debuginfo 大小；
  - 评估增量缓存与 debuginfo 的取舍；
  - 不改 release profile 的优化级别与安全属性。
- 可选：为本地开发与 CI 使用不同 `CARGO_TARGET_DIR`，减少 profile 间互相污染。

## 执行单元

- U1 清理 target 并建立体积/时长基线
- U2 接入 sccache 并验证 debug/release 复用
- U3 接入 cargo nextest 并调整日常测试命令/文档
- U4 profile 调优、Makefile/CI 门禁与 runbook 更新

## 验证

- 清理后 `cargo test -p deploy-go-agent` 全量通过，并记录冷/热构建时长。
- `cargo nextest run -p deploy-go-agent` 通过，失败输出可读。
- `target/debug` 体积明显下降，增量构建不再出现文件系统扫描拖慢。
- `make check` 或等价门禁通过，业务测试与行为无回归。

## 状态

- 2026-08-18 已完成第一阶段正式发布 Docker 构建分层实现与本地验证，并以 `3133569` 提交推送。
- 已修改 API、Agent、Deployer 三个 release Dockerfile：将 workspace manifests 与源码分层，增加
  `cargo fetch --locked`，并为 Cargo registry、git 与按架构隔离的 target 增加命名 BuildKit cache。
- 已增强 `deploy/production/test-install-contract.sh`，覆盖 manifest/source 顺序、命名 cache、
  `sharing=locked` 与 `cargo fetch --locked` 契约；契约测试和 `make deploy-production-check` 已通过。
- 首次 amd64 实构建发现仅复制 manifest 时 Cargo 无法识别隐式 target，已在 fetch 层增加最小占位
  `src/lib.rs` / `src/main.rs`。API、Agent、Deployer 的 amd64 实构建分别约为 4m13s、2m27s、
  1m01s；相同输入二次构建均为 0-1s。
- Agent 与 Deployer 的 arm64 冷构建分别约为 2m02s、40s，确认 target cache 按架构隔离且产物可生成。
- 第二阶段已实现统一双架构产物 builder：构建模式按架构在一次 Cargo 命令中产出所需组件，
  默认从 5 次 Rust builder 收敛为 2 次；release 模式和 Agent build-only 仍按需构建 Agent。
  amd64 四组件统一实构建约 4m34s，对比原先三个独立构建累计约 7m41s，墙钟时间下降约 40%；
  arm64 Agent/executor/deployer 在已有分层缓存下约 24s。契约测试覆盖每架构仅构建一次、产物导出
  和 build-only 不连接远端。sccache、nextest、target 清理与 profile 调优仍按 U1-U4 独立推进。
- 2026-08-18 对正式发布链接器做了隔离 arm64 四组件冷 target A/B：默认 GNU ld 的 Docker/Cargo
  墙钟分别为 233.96s/224.3s；通过 `cc -fuse-ld=lld` 使用 Rust 工具链自带 lld 时分别为
  262.52s/252.3s，退化约 12.2%。因此不引入 lld，也不调整默认 `codegen-units=16`、
  `incremental=false`；稳定态双架构 Agent build-only 三次为 7.67s、2.53s、2.76s，中位数
  2.76s，热路径已接近 Docker 启动与产物抽取开销。
- U4 开始收敛开发/测试 profile：workspace 自有 crate 保留 `debug=1` 的行号级调试信息，第三方
  依赖关闭 debuginfo，降低冷编译、链接和 `target/debug` 体积；release profile 保持默认值，
  不受该配置影响。现有 `target/debug` 只读容量扫描耗时 81.24s，说明历史产物本身已形成明显
  文件系统开销；本轮不主动清理其他会话仍可能复用的 target。同机空 target A/B 构建 Agent
  全部测试二进制：优化 profile 为 21.53s/1,190,440 KiB，Cargo 默认 dev profile 为
  23.59s/1,841,420 KiB，墙钟下降约 8.7%、体积下降约 35.4%；使用优化 profile 的完整 Agent
  测试 18.83s 全部通过。测量环境为 arm64 macOS、Rust/Cargo 1.94.0；两组均使用独立空临时
  `CARGO_TARGET_DIR` 串行运行，默认组通过 Cargo `--config` 恢复 `debug=2`，未清理共享 target。
- 2026-09-05 完成 U1-U3 与 U4 的 release 收口：`cargo clean --profile dev` 实际删除
  1,689,900 个文件、约 305.9GiB；清理后 `target/` 为 842MiB，随后 Agent 测试重建后
  `target/debug` 约 1.2GiB、`target/` 约 2.0GiB。本机安装 sccache 0.17.0 与 cargo-nextest
  0.9.143；Makefile 检测到 sccache 后自动导出 `RUSTC_WRAPPER` 并把 `SCCACHE_CACHE_SIZE`
  默认限制为 20G。新增 `make rust-clean-dev`、`make rust-clean-all`、
  `make rust-target-stats`、`make rust-test-fast`。清理后冷构建 Agent 测试约 20.79s，
  `make rust-test-fast` 热路径 5.47s（180 个测试全部通过）。`Cargo.toml` 增加
  `[profile.release] strip = "debuginfo"`，去掉调试信息但保留符号表；deployer release
  热重链接 1.02s。执行节点 `tasks/`/`apps/deployments/` 清理策略原先未实施；随后按任务
  journal 保留、断线恢复与 root executor 全局预算边界拆分为 Agent 侧受控回收（见下条）。
- 2026-09-05 执行节点本地工作区回收已实现：Agent 新增定时 `StorageCleanup`，任务 journal 默认
  保留 7 天、部署根目录默认保留 30 天、扫描周期默认 1 小时；终态结果成功落库后先立即回收
  checkout/staging/artifact.tar，journal 保留用于断线重放。回收器限定 `data_dir` 内普通路径且
  禁止符号链接祖先，prepare 成功但尚未上传/手动发布的 staging 不会被提前删除。root executor 的
  release job 50 GiB/1 天策略保持独立，不合并为 Agent 侧全局预算硬删。
- 2026-09-05 Docker BuildKit cache 盘点：本机 builder 中 deploy-go 相关 exec cache 约
  1.6GiB（cargo registry 约 541MiB、arm64 target 约 472MiB、amd64 target 约 608MiB），
  另有早期无命名 target cache 约 687MiB。`docker builder prune` 的 `id` filter 匹配的是
  BuildKit record ID 而非 Dockerfile 的共享 cache key，因此在共享 builder 上无法安全地
  按项目自动清理；后续若需要自动回收，应使用独立 builder 或先实现 cache 盘点/门禁。
- 2026-09-11 完成编译耗时测量与 sccache 接入容器链路（本轮）：
  - 测得整仓 Rust 空 target 冷编译墙钟 77.3s（user 507s，并行度 6.6x），第三方依赖占 87%
    CPU 时间；增量编辑路径（api 7.3s、executor 8.8s、agent 15.3s、deployer 2.3s、
    admin 3.9s）已足够快，`admin-app` Flutter debug APK 88s 最慢但不在部署链路。
  - 四个 release Dockerfile（`deploy/docker/release/Dockerfile`、`api/`、`agent/`、
    `deploy-go-deployer/`）统一设置 `RUSTC_WRAPPER=sccache`、`SCCACHE_DIR=/sccache`、
    `SCCACHE_CACHE_SIZE=20G`，安装 `sccache` 并新增 `id=deploy-go-sccache` 的
    `sharing=locked` cache mount，构建结束输出 `sccache --show-stats`。
  - 本机全局 `/Users/chen/.cargo/config.toml` 增加 `[build] rustc-wrapper = "sccache"` 与
    `SCCACHE_CACHE_SIZE = "20G"`，使直接执行的 `cargo` 命令也复用缓存（该文件不属于仓库）。
  - 容器链路实测（linux/arm64，`BUILD_API=1`）：target cache 失效时由 216.5s 降到 89.5s
    （sccache 部分预热）再到 44.2s（预热完成，318 命中 / 0 未命中）；target cache 命中时
    改一行 `api/src/main.rs` 为 34.8s，与未接 sccache 的 39.9s 基线一致，无退化。
    cargo 指纹不含 `RUSTC_WRAPPER`，本机开关 wrapper 实测不触发重编译，故接入 sccache
    本身没有一次性重建成本；3m47s 的冷构建来自 target 缓存本身失效（旧缓存/依赖变更）。
  - 修正 `sed -n '1,8p'` 统计写法：`sccache --show-stats | head` 会因 head 提前关闭管道
    让 sccache panic 并 Aborted，构建日志出现失败假象；已改为读取全部输入的 sed 写法并加
    契约断言。
  - 以正式链路入口复验：`make deploy-production-agent-build` 构建 x86_64 与 aarch64 两套
    Agent/executor 并校验 manifest 通过，制品输出到 `target/deploy-release/agent`。
  - `deploy/production/test-install-contract.sh` 增加 `RUSTC_WRAPPER=sccache` 与
    `id=deploy-go-sccache` 断言；`make deploy-production-check` 通过。
  - 构建性能基线与四层缓存回收边界写入 `docs/runbooks/local-development.md`，
    发布链路说明同步到 `docs/runbooks/systemd-deployment-production.md`。
