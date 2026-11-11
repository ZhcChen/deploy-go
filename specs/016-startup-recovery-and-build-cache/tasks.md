# API 启动恢复提速与部署构建缓存任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：T001–T006 已实施并本地验证；T007 上线验证待用户授权部署。全部任务待用户签收。勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-001 `runtime-log` 顺序重放与测试，基准对比。
- [ ] T002 FR-002 API 启动恢复改用顺序重放，现有恢复测试全部通过。
- [ ] T003 FR-003 FR-006 安装器慢启动与崩溃循环区分，并以脚本执行验证场景。
- [ ] T004 FR-004 FR-005 FR-006 BuildKit GC 策略、基础镜像摘要、`COPY --link`，本机真实验证。
- [ ] T005 FR-007 手册、解决记录、登记 016。
- [ ] T006 SC-001 至 SC-004 全量验证、自审 diff、提交推送。
- [ ] T007 SC-005 上线验证：需用户授权部署；记录 API 重启耗时、各阶段耗时与缓存命中。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已实现 | 通过 | 未发布 | 待用户签收 | `runtime-log::replay` 与 `SegmentStore::replay`，新增 3 个测试（与逐页 `read_page` 逐项一致，覆盖多段与淘汰、游标落点、批次边界、字节预算小于一条记录、提前停止、部分写入尾部），存储层共 9 个测试通过。本机 release 基准（同一份数据）：满段 6 万条按页重放 2,226 ms，顺序重放 38 ms（58 倍）；2 万条 250 ms 对 12.6 ms；按页方式记录数翻倍耗时约翻 4 倍，确认二次增长。服务器是 musl 目标，数值与本机不同，需上线后实测 |
| T002 | 已实现 | 通过 | 未发布 | 待用户签收 | `DurableStore::replay` 由文件线程读取，经容量 4 的有界通道交付，批次 500 条或 8MiB；`recover_locked` 改用它，水位线处理逻辑未变。API 现有的 17 个运行日志恢复相关测试全部通过（含 `durable_append_before_database_commit_is_recovered_before_rotation`、`cancelled_request_after_fsync_recovers_before_competing_rotation`、`recovery_upserts_latest_once_per_source…`） |
| T003 | 已实现 | 通过 | 未发布 | 待用户签收 | `wait_for_url` 支持服务名，基础时限 180 秒之后只要服务仍 active 且 `NRestarts` 未增加就继续等到 900 秒。用假 `systemctl`/`curl` 执行 6 个场景：慢启动（第 400 次探测就绪）通过；崩溃循环、进程退出均在 180 秒失败；一直 active 但不就绪在 900 秒失败；上限等于基础时 400 次就绪的也失败；非整数配置被拒绝。契约测试新增 4 条断言 |
| T004 | 已实现 | 通过 | 未发布 | 待用户签收 | BuildKit 完整 GC 策略（缓存挂载保留 720 小时），在本机用真实 BuildKit 创建 builder 并核对 `docker buildx inspect` 输出，策略与预期一致；三个基础镜像按 OCI 索引摘要固定（与部署日志中的摘要一致）；构建阶段 `COPY` 全部 `--link`；契约测试能拦住去掉摘要或去掉 `--link` 的回退。本机真实构建验证 Dockerfile 可用。**对照实验（与线上相同的 `docker-container` 驱动，BuildKit v0.34.2）**：只改 `agent/src` 时，旧 Dockerfile 11 秒并连带重编无关的 `deploy-go-deployer`，新 Dockerfile 1 秒、0 个重编。实验过程踩了三个坑（挂载 id 不入缓存键、必须模拟 `git archive` 的修改时间、不同驱动的 BuildKit 行为不同），已写入解决记录 |
| T005 | 已实现 | 通过 | 不适用 | 待用户签收 | 生产手册新增“构建缓存为什么会失效”“基础镜像摘要”与慢启动排障，更新配置表；新增解决记录（含验证中的三个坑）；登记 016 |
| T006 | 已完成 | make check 全部通过（并行 157 秒；nextest 962 通过、2 跳过）；agent-check 316 通过；docs-check、spec-kit-verify、whitespace-check 通过 | 未发布 | 待用户签收 | 验证中发现我自己的一处错误：收紧部分写入尾部的断言时用了 `unwrap_err`，而 `LogPage` 没有实现 `Debug`，只跑了 `cargo fmt` 没有编译，首次 `make check` 的 rust 组 4 秒失败；已改为不依赖 `Debug` 的写法并确认编译与测试通过 |
| T007 | 未开始 | 未执行 | 未部署 | 待用户签收 | - |
