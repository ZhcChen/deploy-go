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
