# 两阶段部署：准备成功却无制品、发布授权请求失败

状态：规格就绪
来源：keel 项目反馈（2026-11-22 会话，用户转贴）。应用 keel【测试环境】（`app_01M7PE307AB4XSHD7BED5ZZ25E`），目标为两阶段 Git 分支测试目标，构建节点“测试环境节点01”（`agent_01KZAHSQKSYYWGHF5J156857MT`，与控制面同机）。

## 背景与根因（2026-11-22 只读诊断）

证据来自正式库 `agent_tasks`、`agent_task_events`、`deployment_artifacts`（`sqlite3 -readonly`）与 `deploy-go-api`、`deploy-go-agent` 的 journal，未改动任何线上状态。

### 问题一：`prepared_artifact_missing`（6 次）

以 `deployment_01M7VSNPNZPC3Y2AJ4FQR42F26`（准备任务 `task_01M7VSNPV7FTY9EVSBNWVDMADB`）为例，时间为 UTC：

| 时间 | 事件 |
| --- | --- |
| 06:18:03.496 | 控制面投递准备任务，投递租约 30 秒 |
| 06:18:03.660 | runner 写出 `deploy.started`，随后做 Git 检出（runner 内，约 40 秒） |
| 06:18:33 | 投递租约到期仍未收到确认：`requeue_expired_deliveries` 把任务改回 `queued` 并重新投递 |
| 06:18:34.880 | Agent 收到重复投递：`prepare()` 发现 journal 已存在，直接进入 `monitor_prepare_transfer`，把 `deploy.started` 作为 1 号事件发出 |
| 06:18:43.860 | 首次投递的处理者等到 runner 启动 `make`（写出 `process.json`）后才发确认，接着用同一序号 1 发“已接受”状态 |
| 06:18:43.924 | 控制面：`agent_event_conflict`（Agent 事件序号冲突）；按设计关闭该 Agent 的整条控制连接 |
| 06:18:49.78 | runner 结束，`make deploy-go-prepare` 成功 |
| 06:18:57.179 | Agent 重连；`on_connected` 把“已终态、结果未发送”的 journal 直接补发结果——两阶段准备任务因此在**未上传制品**时报告成功（18 号事件 `succeeded`） |
| 06:18:57.558 | 同时恢复的制品上传路径请求上传授权，控制面以 `artifact_prepare_task_inactive` 拒绝（任务已终态） |
| 随后 | 发布门禁找不到本部署的制品：`prepared_artifact_missing` |

其余失败样本（如 `deployment_01M7W2AQ2BN6WF1SHZ1K4PADN5`，确认晚 34 秒）同型；成功对照 `deployment_01M7W2D8K05HM8ESEDAX7FKCNT` 的确认在 28 秒到达，没有触发重新投递。反馈中“确认到开始间隔 7–35 秒、开始到结束 0–2 秒”是因为事件在重连后集中补发。

根因（三处缺陷叠加）：
1. Agent 对准备/发布任务的确认要等 runner 完成 Git 检出并启动脚本之后才发出，检出耗时超过 30 秒投递租约时，控制面必然重新投递。
2. Agent 对同一任务的重复投递没有“单一处理者”约束：首次处理者仍在启动阶段时，重复投递会并行开始转发事件，两者从同一 journal 分配序号，产生序号冲突，导致整条连接被关闭。
3. 重连补发逻辑没有排除“两阶段准备、runner 已结束、制品尚未上传”的任务（对账路径已有该判断 `prepare_transfer_pending`，补发路径漏了），于是把未上传制品的准备任务报告为成功。

### 问题二：`release_authorization_request_failed`

`deployment_01M8180DKZMW1V1SJC2P4S3QWV`：准备阶段同样晚确认（50 秒）并出现一次序号冲突与重连，但这次由正确的上传路径完成，制品 `artifact_01M81827R5G…` 已校验。发布任务 09:05:47 开始，下载与校验后需要向控制面请求发布授权；09:06:16–09:06:29 控制面 SQLite 出现成片慢语句与 `database is locked`（`BEGIN IMMEDIATE` 等待 4.3 秒、5 秒写超时），期间 Agent 控制连接在 09:06:21/09:06:27 重建。授权请求发送时旧连接已关闭，`request_release_authorization` 发送失败即返回 `release_authorization_request_failed`，任务失败。发布授权是 `ReleaseStart` 的前置条件，失败时 executor 尚未开始发布，因此重试 `deployment_01M8184W7S50FK0VJE6JF9WEM1` 成功且不存在重复发布。

与问题一的关系：同属“控制连接中断时，正在进行的任务没有正确接续”，但触发点不同（问题一由重复投递引发序号冲突，问题二由控制面数据库锁竞争期间的断线引发）。控制面在心跳写库失败时会静默断开连接，Agent 也不记录断线，现场只能靠时间吻合推断，这是诊断缺口。

## 目标与范围

1. 准备任务只有在本次执行的制品已登记后才报告成功；任何断线、重连、重复投递都不能让准备任务“无制品成功”。
2. 同一任务在 Agent 内同一时刻只有一个处理者在分配事件序号；控制面重复投递不再引发序号冲突。
3. 发布授权请求遇到断线时，在任务期限内等待重连并在新连接上重发，而不是立即失败。
4. 失败时给出明确结论：属于“发布尚未执行”的失败码，错误摘要与对外诊断写明“可安全重试”，并说明为何不会重复发布。
5. 断线原因可诊断：控制面关闭 Agent 连接时记录原因；心跳写库的瞬时失败不再直接断线。

明确不做：
- 不修改 keel 或任何被部署业务应用；不对线上任务、部署做重试、取消或手改。
- 不调整投递租约时长、不改协议版本与线协议类型（Agent 内部行为与控制面处理逻辑修正即可）。
- 不在本规格内处理 SQLite 锁竞争的来源（需单独定位长写事务），仅降低它对连接的冲击并补诊断。

## 用户场景与验收

### US1：Git 检出慢于投递租约时，准备与发布正常完成

- 优先级：P1
- Given：准备任务 Git 检出耗时超过 30 秒，控制面重新投递同一任务
- When：首次处理者完成启动并确认
- Then：不出现 `agent_event_conflict`；事件序号连续；制品上传完成后才报告成功；发布正常执行
- 关键失败或边界场景：重复投递到达时首次处理者已因断线退出——重复投递接手启动后的监控与上传。

### US2：重连时准备任务不会被报告为无制品成功

- Given：两阶段准备的 runner 已结束、制品未上传，Agent 重连
- Then：补发逻辑跳过该任务，由对账→重新投递→上传路径完成；控制面收到成功结果时本部署已存在已校验制品

### US3：发布授权请求遇到断线时等待重连

- Given：发布任务请求授权时控制连接断开
- When：Agent 在任务期限内重连
- Then：在新连接上以同一 `authorization_id` 重发并继续发布；超出期限仍失败时报告 `release_authorization_request_failed`，摘要写明“发布尚未开始，可安全重试”

### US4：失败结论清楚

- Then：`prepared_artifact_missing`、`release_authorization_request_failed`、`release_authorization_timeout` 的部署摘要与对外诊断提示写明“发布尚未执行，可安全重试，不会重复发布”

## 功能要求

- FR-001：Agent 对准备、发布、单阶段执行任务设“启动所有权”：同一 `task_id` 的处理从收到投递到首次确认与初始状态发送完毕期间互斥；重复投递等待前一处理者完成启动后，再按已有幂等路径（journal 已存在）继续。
- FR-002：Agent 重连补发终态结果时，跳过 `prepare_transfer_pending` 为真的任务（与对账路径一致）。
- FR-003：Agent 发布授权请求发送失败或等待期间连接更换时，等待新连接并重发，总等待受任务期限约束；成功后后续发布流程使用新连接。
- FR-004：控制面心跳写库失败不立即断开连接，记录告警，仍由心跳超时兜底；控制面因回执写库失败断开连接时记录原因；Agent 断线时记录原因。
- FR-005：错误摘要与对外诊断按 US4 修正；`release_authorization_*` 中属于传输类的码单独归为“可安全重试”。

## 成功标准

- SC-001：单元/集成测试复现三处缺陷（重复投递并发、重连补发无制品成功、授权请求断线），修复前失败、修复后通过。
- SC-002：Rust 全量测试与 clippy 通过（`make remote-check` 或等价远端目标）。
- SC-003：上线后由用户授权在测试环境对同一应用连续部署（含 Git 检出慢于 30 秒的情形），统计 `prepared_artifact_missing` 与 `agent_event_conflict` 为 0。上线和连续部署需用户在对话中明确授权，代理不自行执行。

## 风险与验证要求

- Agent 与控制面同时改动：Agent 修复对旧控制面兼容（只改 Agent 内部处理），控制面修复对旧 Agent 兼容（不改协议）。
- 启动所有权不能造成死锁或长期阻塞：只覆盖启动阶段，监控阶段仍由既有 `transfer_lock`/`monitor_lock` 负责；需测试重复投递等待后能接续。
- 发布路径涉及特权 executor：重发授权只发生在 `ReleaseStart` 之前；授权签名无状态，executor 侧对同一授权的重放保护不变；涉及的 Secret 环境租约在投递失败时已有归还逻辑。
- Linux 专项不受影响（不改 cgroup/PTY/身份切换）；Rust 门禁在 qfy-test2 远端执行。

## 未决事项

无。投递租约时长保持 30 秒（修复后晚确认只造成一次无害的重复投递等待）。
