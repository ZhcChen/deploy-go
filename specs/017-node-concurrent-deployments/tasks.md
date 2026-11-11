# 同节点多应用并发部署任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：已发布到正式环境并在测试环境节点01 开启；真实的多应用同时部署待业务侧触发后观察，待用户签收。勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-003 协议新增能力与常量，schema 与兼容性测试。
- [ ] T002 FR-005 FR-006 FR-010 broker 多槽位、`Capacity` 动作、恢复，单元测试与 Linux 容器测试。
- [ ] T003 FR-007 Agent 启动时查询上限并声明能力。
- [ ] T004 FR-001 迁移（列、测试节点种子、系统设置）与迁移测试。
- [ ] T005 FR-002 节点接口（返回字段、修改接口、审计、乐观版本），OpenAPI 与客户端生成。
- [ ] T006 FR-004 控制面有效上限与投递、等待判断，调度测试。
- [ ] T007 FR-008 管理端节点页与测试。
- [ ] T008 FR-009 标准与手册，登记 017。
- [ ] T009 SC-001 至 SC-004 版本升至 0.3.35，全量验证、自审、提交推送。
- [ ] T010 SC-005 上线与节点开启：需用户授权部署；节点本机开启需另行授权；记录实测结果。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已完成 | 已验证 | 未发布 | 待用户签收 | 新增 `MAX_CONCURRENT_DEPLOYMENTS_PER_NODE`=16 与 `AgentCapability::ConcurrentDeploymentsV1`（`concurrent_deployments_v1`），协议版本仍为 18，schema 已登记。`concurrent_deployments_capability_is_additive_and_optional` 通过：新能力被接受、旧 Hello 仍有效、未知 `concurrent_deployments_v99` 被拒绝、往返一致 |
| T002 | 已完成 | 已验证 | 未发布 | 待用户签收 | runner broker 由单槽位改为受上限约束的 `ActiveTasks`，新增 `Capacity` 动作，重启恢复按上限校验。`runner_service` 单元测试由 9 个增至 16 个（macOS 运行 15 个，另 1 个仅 Linux），新增 7 个覆盖设置解析、上限与重复任务、串行时的清理门禁、并发清理不泄漏槽位、能力查询与旧 broker 降级。Linux 容器 `make agent-runner-isolation-check`：5 通过 0 失败，含 `runner_broker_runs_concurrent_tasks_only_when_enabled`（默认串行；开启后 3 个 runner 并发、各自目录权限独立翻转、单独取消不影响其他、槽位回填）与 `runner_broker_rejects_unrecognized_concurrency_setting`；`make rust-linux-clippy`（`-D warnings`）通过。两项首次运行因容器内下载 1.99.0 工具链 TLS 中断失败，重试通过 |
| T003 | 已完成 | 已验证 | 未发布 | 待用户签收 | Agent 启动时探测 broker 上限，大于 1 才在 Hello 声明能力并记录 `concurrent_deployments_advertised`；`capability_is_advertised_only_when_the_broker_reports_more_than_one`、`capacity_probe_reports_the_configured_limit`、`capacity_probe_degrades_against_a_legacy_broker` 通过 |
| T004 | 已完成 | 已验证 | 未发布 | 待用户签收 | 迁移 `0044_node_concurrent_deployments.sql`（新增列、测试与开发环境节点种子 10、系统设置只升不降且不新建行）；`node_concurrent_deployments_migration_seeds_only_active_test_and_dev_nodes` 与 `node_concurrent_deployments_migration_only_raises_the_platform_concurrency_setting` 通过。2026-11-11 只读核对线上数据：符合种子条件的只有“测试环境节点01”，线上没有 `max_concurrent_deployments` 的系统设置行，迁移不会创建，需在 Web 手动保存 |
| T005 | 已完成 | 已验证 | 未发布 | 待用户签收 | `PUT /api/v1/nodes/{id}/deployment-concurrency`（管理员 + CSRF + 乐观版本 + 审计 `node.deployment_concurrency`），节点响应含 `max_concurrent_deployments`，Agent 响应含 `concurrent_deployments_enabled`；4 个新增接口测试通过；OpenAPI 与 Web client 已重新生成，`api-openapi-check` 与不依赖 Dart 的 client `--force` 检查通过 |
| T006 | 已完成 | 已验证 | 未发布 | 待用户签收 | 有效上限 = Agent 声明能力 ? 节点设置 : 1；投递在单条原子 `UPDATE` 内按有效上限计数，等待判断同口径。`concurrency_tests` 7 个用例通过：默认仍串行、按节点设置并发并回填、设置为 1 保持串行、重投不计自身、accepted/running/canceling 都占槽、按 Agent 独立且超时中断释放槽位、能力缺失或能力 JSON 损坏回退为 1。此前对关键规则做过变异测试，用例能捕获回退 |
| T007 | 已完成 | 已验证 | 未发布 | 待用户签收 | 节点详情页新增“同节点并发部署”区块：显示本机是否开启、设置值与实际并发，管理员可改 1 至 16，普通用户只读，归档节点无入口；新增 7 个组件测试，`make admin-check` 通过（27 个文件 174 个用例，含 lint、类型与构建） |
| T008 | 已完成 | 已验证 | 不适用 | 待用户签收 | 更新 `agent-control-protocol.md`、`agent-installation-contract.md`、`application-deployment-contract.md`、`agent-onboarding.md`（新增“同节点并发部署”：开启步骤、回退与隔离取舍）、`development-guide.md`、`local-development.md`，并登记 017；`make docs-check` 通过 |
| T009 | 已完成 | 已验证 | 不适用 | 待用户签收 | 五个 crate 版本统一为 0.3.35。合并工作区 `make check` 通过（Rust 983 通过/2 跳过，Web 174 个用例，contracts 26s，总墙钟 180s），`whitespace-check` 与 spec-kit 阶段检查通过。提交与推送见 Git 历史 |
| T010 | 已完成 | 部分验证 | 已部署 | 待用户签收 | 用户在对话中分别授权：部署正式环境、上线后在测试环境节点01 开启本机并发、用浏览器把全局并发保存为 10。**上线**：2026-11-11 21:10 部署 `0cf16f6`（0.3.35）；预检通过（服务器身份、构建代理、无活动任务）；整次 191 秒，安装阶段 7 秒；API 21:13:35 启动、约 3 秒就绪，`NRestarts=0`；migration 44 已应用；节点设置为测试环境节点01=10，其余=1；三个在线节点自动升到 0.3.35。**节点开启**：21:15 在测试环境节点01（与控制面同机）的 `/etc/deploy-go-agent/config` 追加 `DEPLOY_GO_RUNNER_CONCURRENT=true`（13 行变 14 行，权限不变），先后重启 runner 与 Agent，重启时该 Agent 无活动任务；日志出现 `runner_broker_capacity max_concurrent_tasks=16` 与 `concurrent_deployments_advertised`，控制面能力列表含 `concurrent_deployments_v1`。**全局并发**：21:16 经 Web 保存为 10，`system_settings.runtime` 新建，其余字段为原默认值。节点页显示：并发部署数 10、本机并发已开启、当前实际并发 10。**未验证**：多个应用在该节点上真实同时部署（需业务侧同时触发，不由 Deploy Go 代为发起）。**既有问题（非本次引入）**：该节点 Agent 每小时记录 69 条“存储目录回收失败 Permission denied”，目录是 deploy-go-runner 属主、权限 0750 的老部署目录（共约 7.2GB）；升级前就存在 |
