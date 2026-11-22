# 两阶段部署：准备成功却无制品、发布授权请求失败——实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

- 投递：控制面 `try_dispatch` 置 `delivered`、投递租约 30 秒；`requeue_expired_deliveries` 把租约到期仍未确认的任务改回 `queued` 并重新投递（`api/src/agents/dispatcher.rs`）。
- Agent 确认时机：`prepare()` → `execute_prepare` → `execute_spec` → `spawn_spec` 等待 runner 写出 `process.json` 后才返回，随后发确认；runner 在写 `process.json` 之前完成 Git 检出（`agent/src/runner.rs`）。检出超过 30 秒就必然重新投递。
- 重复投递：`prepare()` 发现 journal 已存在，立即进入 `monitor_prepare_transfer` 转发事件（推进 `last_sequence`、`events_sent`）；首次处理者此时仍在 `spawn_spec` 中，返回前用启动前的 journal 副本 `store`，把两个计数覆盖回 0。随后首次处理者用序号 1 发“已接受”，与已提交的 1 号进度事件内容不同 → 控制面 `agent_event_conflict` → `run_connection` 关闭连接。
- 重连：`on_connected` 补发“终态且 `result_sequence` 为空”的任务结果，未排除两阶段准备（对账路径用 `prepare_transfer_pending` 排除了）。
- 发布授权：`request_release_authorization` 只在当前连接上发送一次，发送失败即 `release_authorization_request_failed`；Agent 的发送端按连接创建（`agent/src/connection.rs`），断线后旧发送端永久关闭。
- 控制面连接：`record_heartbeat` 任一错误（含 SQLite 锁超时）都会断开；回执生成失败也静默断开。

## Constitution Check

只改 deploy-go 自身代码；不改线协议类型、schema、migration、OpenAPI；不改业务应用；诊断只读访问正式库与 journal（`sqlite3 -readonly`、`journalctl`），未改线上状态。上线需用户“部署正式环境”授权，测试环境连续部署验证需用户另行授权。

## 实现与文件范围

1. `agent/src/task_handler.rs`
   - `StartupOwnership`/`startup_ownership(task_id)`：按任务的异步互斥，`dispatch()` 在进入 `prepare()`、`release()` 与单阶段执行前取得；首次确认与初始状态发完后、进入监控前释放；重复投递路径一进入即释放（之后由既有 `transfer_lock`/`monitor_lock` 保证单一处理者）。释放时无等待者则移除锁表项。
   - `on_connected` 补发跳过 `prepare_transfer_pending` 为真的任务。
   - `current_outbound`（watch）：`on_connected` 更新；`request_release_authorization` 发送失败或等待期间连接关闭时，经 `next_connection` 在 60 秒预算内等待新连接，以同一请求重发；连接仍在却超时保持原 `release_authorization_timeout` 语义；成功后返回所用连接，`start_privileged_release` 后续使用该连接。
2. `agent/src/connection.rs`：断线与连接失败记录 `control_connection_closed`/`control_connection_failed`（错误为固定文案，无敏感信息）。
3. `api/src/agents/websocket.rs`：`HeartbeatError::{Superseded, Database}`；数据库错误记录告警并保持连接，连续 3 次才断开；连接被替代/吊销照旧断开；心跳超时、令牌过期、回执失败断开时记录原因。
4. `api/src/agents/dispatcher.rs`、`api/src/external/mod.rs`：`prepared_artifact_missing`、`release_authorization_request_failed`、`release_authorization_timeout` 的摘要写明“发布尚未执行，可安全重试，不会重复发布”。
5. 测试：`agent/tests/two_stage.rs` 重复投递等待首次处理者；`agent/src/task_handler.rs` 重连补发、授权重发与预算耗尽、启动所有权；`api/tests/external/external_api.rs` 摘要断言。
6. 文档：`docs/runbooks/deployment-recovery.md` 新增排障小节；`specs/README.md` 登记 022。

## 验证计划

- 远端：`make remote-test REMOTE_COMMAND="rust-test -p deploy-go-agent"`；变异核对（撤掉三处修复后新测试必须失败）；`make remote-check`（fmt、clippy、Rust 全量、OpenAPI 漂移、Web 全量）。
- 本机：`make docs-check`、`make whitespace-check`、`make spec-kit-verify`、`git diff --check`。
- 上线后（需授权）：测试环境同一应用连续部署，统计 `agent_event_conflict` 与 `prepared_artifact_missing`；查 Agent 日志 `control_connection_*` 与授权重发记录。
