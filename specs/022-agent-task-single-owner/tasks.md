# 两阶段部署：准备成功却无制品、发布授权请求失败——任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：已实施，远端门禁验证通过，待上线与用户签收。勾选仅由用户更新。

## 实施与验证

- [ ] T001 只读诊断：正式库任务、事件、制品与 API/Agent journal 时间线，确认三处缺陷与授权失败成因（写入规格背景）。
- [ ] T002 FR-001 Agent 启动所有权；FR-002 重连补发跳过待上传的准备任务。
- [ ] T003 FR-003 发布授权请求跨重连重发；FR-004 心跳写库容错与断线原因日志；FR-005 可安全重试摘要。
- [ ] T004 SC-001 回归测试与变异核对；SC-002 远端全量门禁验证。
- [ ] T005 排障手册与规格登记；本机文档门禁；自审、提交推送。
- [ ] T006 FR-006 日志完整性：断开原因全覆盖、重新排队、门禁失败、重复投递等待、结果延后。
- [ ] T007 SC-003 上线（用户已说“部署正式环境”）与上线后观察；专项压测按用户决定不做。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已完成 | 已验证 | 不适用 | 待用户签收 | 只读核对 6 个失败样本中的 2 个与 1 个成功对照、问题二样本：失败样本 prepare 确认晚于投递 34–50 秒，成功对照 28 秒；`agent_task_events` 1 号为 `progress`、4 号重复 `deploy.started`；API 日志 `agent_event_conflict` 后 Agent 重连，`artifact_prepare_task_inactive` 拒绝上传授权；问题二发布期间 SQLite `database is locked` 与重连同时发生。未改任何线上状态 |
| T002 | 已完成 | 已验证 | 待上线 | 待用户签收 | `agent/src/task_handler.rs`：`StartupOwnership` 覆盖 `prepare()`、`release()`、单阶段执行的启动阶段，进入监控前释放；`on_connected` 补发增加 `!prepare_transfer_pending` |
| T003 | 已完成 | 已验证 | 待上线 | 待用户签收 | `request_release_authorization` 经 `current_outbound` 在 60 秒内等待新连接重发同一请求，返回所用连接；`api/src/agents/websocket.rs` `HeartbeatError`（连接被替代照旧断开，数据库错误连续 3 次才断开）与断开原因日志；`agent/src/connection.rs` 断线日志；摘要见 `api/src/agents/dispatcher.rs`、`api/src/external/mod.rs` |
| T004 | 已完成 | 已验证 | 不适用 | 待用户签收 | 新增 5 个用例：`redelivery_during_slow_startup_waits_for_the_first_owner`、`reconnect_replay_never_reports_prepare_success_before_artifact_upload`、`release_authorization_resends_on_the_next_connection`、`release_authorization_fails_safely_when_no_connection_returns`、`startup_ownership_serializes_one_task_and_releases_its_entry`，外部诊断摘要断言更新。变异核对（qfy-test2）：同时撤掉三处修复后前 4 个新用例失败（305/309 通过），恢复后通过。`make remote-check`：Rust 1018 个全部通过、4 个特权用例跳过，fmt、clippy、OpenAPI 产物检查通过；Web 208 个通过、构建成功。心跳连续失败上限调整后再跑 `rust-check` 1018 个全部通过 |
| T005 | 已完成 | 已验证 | 不适用 | 待用户签收 | `docs/runbooks/deployment-recovery.md` 新增“准备成功却 `prepared_artifact_missing`、发布 `release_authorization_request_failed`（规格 022 修复）”；`specs/README.md` 登记 022；本机门禁见提交记录 |
| T006 | 已完成 | 已验证 | 待上线 | 待用户签收 | `api/src/agents/websocket.rs` 连接循环改为带原因退出，13 种原因、循环结束统一记一条（正常关闭 info、其他 warn）；`requeue_expired_deliveries` 改 `RETURNING` 逐条记录；`fail_deployment_before_dispatch` 记录错误码；Agent `task_redelivered_during_startup`、`prepare_result_deferred_until_upload`。远端 `rust-check` 1018 个全部通过；手册列出完整日志表 |
| T007 | 已完成 | 部分验证 | 已部署 | 待用户签收 | 用户在对话中说“部署正式环境”。五个组件升至 0.3.36（`dad1946e`）使 Agent 修复可随自动升级下发。首次 `make deploy-production` 预检失败：qfy-test2 构建代理 `mihomo-build-proxy` 上游节点拨号失败（探测 `000`），未改服务器；按手册以 `DEPLOY_BUILD_PROXY_URL=` 直连镜像站重跑成功（冷构建 3037 秒：Rust 2084、Web 458）。API/Web 新 PID 410545/410550、`NRestarts=0`、`readyz` 200、公网 200。节点自动升级：生产节点01 11:18:32、预发布环境01 11:18:49 成功；测试环境节点01 首次在 API 重启后连接未就绪时 `upgrade_command_delivery_failed`，60 秒退避后自动重排，11:19:38 成功；三节点均为 0.3.36。新日志已出现 `Agent 控制连接异常关闭`（升级重启所致）。用户决定不做专项压测；构建代理节点需另行修复（需用户授权改服务器配置） |
