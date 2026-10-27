# 部署执行、取消与恢复

## 适用范围

本手册用于排查 Agent 部署队列、取消、日志续传和 API/Agent 重启恢复。操作真实节点前必须获得当前对话中针对具体节点和动作的明确授权。

## 状态语义

| 状态 | 含义 | 恢复处理 |
| --- | --- | --- |
| `queued` | 已持久化，等待在线 Agent 和目标锁 | 保持排队，worker 重新领取 |
| `running` | Agent 已 ACK 且脚本正在运行 | 等待 Agent reconcile；无法证明时进入 `interrupted` |
| `canceling` | 已下发结构化取消任务，终态未确认 | 等待 Agent 结果；无法证明时进入 `interrupted` |
| `succeeded` / `failed` / `canceled` | 已确认终态 | 保持终态 |
| `interrupted` | 进程身份或最终结果无法证明 | 核实后人工 retry，不自动重跑 |

`interrupted` 不代表脚本已停止、失败或回滚。平台不接管应用回滚。

两阶段部署（`execution_mode=two_stage`）在外层 `status` 之上还有 `phase`：

| phase | 含义 | 恢复处理 |
| --- | --- | --- |
| `queued` | 尚未创建 prepare task | worker 按 `(deployment_id, stage='prepare')` 创建并投递 |
| `preparing` | prepare task 正在执行或已成功但 release 未创建 | 只重放 prepare，不重复执行已完成阶段 |
| `awaiting_release` | 手动模式 prepare 已完成，等待 Env 就绪和管理员放行 | 核对制品与 Env；不得手工改 phase 或创建 task |
| `deploying` | release task 正在执行 | 等待 release reconcile；无法证明时进入 `interrupted` |
| `verifying` | release 已输出验证事件，等待终态 | 等待 release 最终结果 |

`phase` 只反映当前阶段，不替代外层 `status`。UI 不得只从日志推导阶段状态。

## 正常观察与 SSE 续传

1. 查看 deployment 的 `status`、`phase`、`exit_code` 和 `protocol_complete`。
2. SSE `/api/v1/deployments/{id}/logs` 断线后使用最后 sequence 作为 `Last-Event-ID` 或 `after`。
3. API 先补发 SQLite 中游标后的日志，再推送新事件；终态发送 `terminal`。
4. `stream-error` 保留游标后重连；`authorization-revoked` 要求重新认证或获取授权。

Agent 输出按任务内 sequence 去重。日志表使用 deployment 全局 `sequence` 排序，SSE 日志事件额外携带 `stage`、`task_id` 和 `task_sequence`，便于按阶段分组展示。migration `0011` 之前的旧日志迁移后保持 `task_id=NULL`、`task_sequence=sequence`。达到日志预算时记录诊断但不泄露 secret；日志保留期结束后只清理输出，不删除 deployment 历史。

## 两阶段 stage 任务恢复

两阶段部署在 `agent_tasks` 中对应两条任务：`stage='prepare'` 与 `stage='release'`，数据库以 `(deployment_id, stage)` 唯一约束防止重复创建。

1. 先按部署和阶段定位任务，不要只按 `deployment_id` 查单条：

```sql
SELECT id, stage, status, last_sequence, finished_at
FROM agent_tasks
WHERE deployment_id = ? ORDER BY stage;
```

2. 自动模式只有 prepare 已持久化 `succeeded` 且发布物校验通过后，worker 才会创建 release；release 失败不反向改写 prepare 终态。
3. 手动模式 prepare 成功后保持 `awaiting_release`。先在应用详情核对 Env 当前版本全部同步成功，再从部署详情执行“开始发布”；不要直接修改数据库或手工创建 release task。
4. API 重启后按数据库阶段事实继续：自动模式 prepare 已成功则直接创建/投递 release；手动模式继续等待显式放行；均不重复执行 prepare。两个阶段任一处于不确定状态时进入 `interrupted` 并人工核实。
5. 恢复时保留两个 stage 的日志与终态；不能手工删除某一阶段 task 来解除部署锁。
6. 取消作用于当前活动 stage 或等待门禁，并阻止后续 stage 创建；取消后遗留任务 staging 由 Agent 随任务清理。

Agent 在 prepare 执行或制品上传期间断线时，重连对账会重新挂接任务；若 prepare 进程已结束但发布物尚未登记，Agent 会请求主控重新下发同一 prepare 任务以恢复上传，不会直接判定 prepare 成功。控制面 SQLite 写事务使用 `BEGIN IMMEDIATE`，避免 WAL 下先读后写触发 `SQLITE_BUSY_SNAPSHOT` 导致事件落库失败并断开 Agent。

## 特权发布缺少制品下载授权

`privileged_release_artifact_required` 表示特权 release 缺少 `artifact_download`；Agent 在发布脚本启动前拒单，不能据此判断迁移、重启或版本切换已发生。旧版对外接口可能将它折叠为 `agent_error`，修复后的白名单保留具体码，未知错误仍隐藏。

Git 和 workspace 两阶段部署即使同节点也必须完成上传、verified 校验及下载授权。控制面须启用 `DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED`；名称中的跨节点不表示同节点可以跳过。未启用时，预览/确认返回 `cross_node_artifacts_disabled`，调度兜底将未开始的阶段安全失败。镜像路径沿用既有制品流程。

| 错误码 | 含义与核查方向 |
| --- | --- |
| `prepared_artifact_required` | 尚未投递的历史 prepare 没有上传授权，不能安全投递 |
| `prepared_artifact_missing` / `prepared_artifact_expired` / `prepared_artifact_failed` / `prepared_artifact_deleting` / `prepared_artifact_unavailable` | prepare 的可信制品缺失、过期或状态不允许发布 |
| `release_artifact_binding_invalid` | release 制品摘要、Agent、目标运行或下载 lease 绑定无效，或制品/授权已过期 |
| `privileged_release_artifact_required` | 特权 release 缺下载授权，属于平台交接契约故障 |

控制面保留历史 payload 和 digest：缺下载字段的 queued/delivered release 收敛为失败；accepted/running/canceling 不因本次门禁被批量改判。已投递 prepare 即使 ACK 丢失也保留恢复路径；动态制品和 lease 检查只在 release 首次投递前执行，避免已下载任务重投时因 lease 到期被误伤。门禁失败若在收敛中遭遇 API 重启，worker 根据持久错误码继续收敛，不重复启动 release。多目标只阻断未调度目标，保留其他正在运行的目标。

排查保留部署 ID、阶段、错误码与版本/配置事实，不导出 lease、凭证、Env 或完整 payload。核对控制面与 Agent 版本、传输开关和 verified 记录；没有现场证据时不能断言是版本不匹配还是传输配置。不要修改业务代码、手改数据库、从 staging 补造制品或盲目重试。修复发布和新的业务验证部署均须分别获得具体授权；旧失败记录不复活。

本地验证：`cargo test -p deploy-go-api --lib artifact_handoff_tests`、`cargo test -p deploy-go-api --test two_stage_deployment --test agent_end_to_end`、`make api-check`、`make privileged-release-check`。本修复无 migration、线协议或 Agent 执行代码变更，回退使用独立代码 revert 和正式部署手册，保留任务/制品/journal。

## prepare 构建成功但发布物交接失败

`exit_code=0` 只表示构建脚本成功，不代表 prepare 已完成。`artifact_transfer_failed`、`artifact_transfer_timeout`、`artifact_prepare_failed`、`artifact_authorization_failed` 等结果表示后续打包、授权或 HTTP 交接失败；控制面不会因此创建 release。

同一 deployment 的 `diagnose` 可查看任务摘要中的 `stage`、`category`、`http_status`、`attempts`、`elapsed_ms`、`confirmed_offset`。无法取得的字段显示未知；`cause` 表示恢复查询前的原始上传失败。`connect` 不能单独证明 DNS/TLS 故障；`offset=0` 不能证明服务端没有收到字节。旧节点无详细结果时返回“传输详情未记录”。API 仅公开白名单摘要，不公开原始 URL、正文、凭证或节点内部路径。

Agent 在同一 lease/归档内有限恢复：init/进度查询/finalize 每轮至多 3 次，PUT 每轮 1 次（401 有限凭证刷新除外），上传至多 3 轮恢复查询。进度未确认时不盲传；finalize 响应丢失重复同一 finalize 请求，因为普通 GET 不支持 verified 查询。只有 verified 且长度匹配才成功；永久拒绝、内容冲突、取消和任务 deadline 均停止自动恢复。

优先保留同一部署的状态、诊断、分页日志和时间边界，不更换幂等键盲目重发，不复活失败任务。需要授权节点只读核查时，用 deployment ID/时间关联 Agent journal 的 `prepared artifact transfer failed` 与 `artifact upload completed`。后者仅记录总上传耗时；具体错误阶段耗时来自诊断。缺调用方时间线时不能把调用前准备耗时归因于平台。

本地回归：

```bash
cargo test -p deploy-go-agent --test artifact_transfer --test two_stage --lib
cargo test -p deploy-go-api --test artifacts_api --test external_api --test external_openapi_contract --test two_stage_deployment
cargo test -p deploy-go-deployer
```

回滚使用本轮代码提交的独立 revert；运行版本按正式部署手册备份恢复。无需数据库迁移，不删除任务、制品或 journal。发布后须核对构建 Agent 已更新；仅更新控制面不会改变旧 Agent 的上传恢复行为。

## 取消任务

- queued deployment 可在数据库中直接转为 `canceled`，不投递 Agent。
- 已投递任务通过版本化 `TaskCancel` 指定 task ID，不传任意 shell 或信号命令文本。
- Agent 只终止自己 durable runner 记录且进程身份可验证的进程组。
- 无法确认进程归属或取消结果时进入 `interrupted`。

不要手工修改 SQLite 状态或删除 task/journal 来解除锁。

## 特权发布瞬时 executor 故障与取消恢复

- 特权 release 启动后由 Agent 侧 monitor 持续调用 executor v4 `ReleaseOutput`/`ReleaseStatus`；瞬时连接失败、超时或非预期响应不会直接放弃，默认 250ms 后重试，直到唯一终态。
- Agent 重启后从持久化 `PrivilegedRelease` phase 恢复，只续传输出和状态，不重复 `ReleaseStart`；重复 cancel 幂等，最终只产生一次 `TaskResult`。
- cancel 到达时即使 monitor 尚未恢复或已退出，Agent 也会在发送 `ReleaseCancel` 后重新接管 monitor，补齐终态；不应停留在 `canceling` 等待外部干预。
- 若页面仍停留在 `canceling` 且 executor 日志显示 job 已结束，先核对 Agent/executor 是否成对 0.3.5、executor Socket 与权限、`ReleaseStatus` 日志，再等待 Agent reconcile；不得手工改数据库状态或删除 task/journal。
- API dispatcher 对跨节点部署也会排除同一 target 已有 `running`/`canceling` 的 queued 部署，避免创建 prepare 后撞 `deployments_one_execution_owner_per_target` 唯一索引并锁死后续部署。

### 节点发布已完成但控制面仍显示下载中/运行中

节点 executor 的 release-job `succeeded`、业务健康检查通过、current release 已切换，只能证明节点侧脚本和切换完成，不能代替 Deploy Go 收到并投影 Agent 的 `TaskResult`。官方终态以 `agent_tasks` 结果、target run 和 deployment 状态一致收敛为准；未确认前不得手工标成功、取消/重发部署或晋级 production。

只读排查顺序：

1. 按 deployment ID 查询 `agent_tasks` 的 `stage/status/last_sequence/finished_at/result_json`、对应 `deployment_target_runs` 的 `status/phase/updated_at`，以及 `agent_task_event_receipts` 最后连续 committed 序号；不要读取 Env 或密钥。
2. Agent 主日志用 deployment/target run 标识关联结构化 `privileged_release` 诊断，再以日志中的内部 task/job 标识核对受控 journal 与 executor job；外部诊断不公开 task/job 标识。
3. 只有 Agent 生成并发送 `stage=release` 的 `deploy.started` 外层边界事件（业务脚本不得输出该事件）后，才把 `downloading/artifact_download` 推进为 `running/release`；该事件在 monitor 挂接到真实 release job 之后才产生。通用 `TaskState::Running` 可能先于制品下载，不代表 release 脚本已开始。
4. Agent 已观察 executor 终态而控制面仍未收敛时，先看最终 TaskResult 是否存在及对应 receipt 是否 committed。相同终态结果可安全重放以补齐未完成投影；不同内容或终态后的新结果应被拒绝，不应人工改数据库。
5. 输出超限以 `output_limit_exceeded` 作为稳定失败原因；Agent 先发送所有仍可读取的连续帧（可能多页），再投递失败结果。序号冲突、不可解释的截断、终态 job 宣告的最后序号长期（约 13 秒）追不平，或历史 durable 输出帧丢失/损坏，都应以 `interrupted` 收敛（错误码分别为 `release_output_sequence_conflict`、`release_output_truncated_unexpectedly`、`release_output_persistence_failed`），不得推断成功，也不得当作临时故障无限重试。

descriptor 缺失/损坏时，Agent 只有在核心 journal 能证明原 task ID、payload digest、PrivilegedRelease phase 和游标时，才允许对原 executor job 做只读观察；无法重建 release 事件时 terminal 结果必须是 `interrupted`。核心 journal 缺失/不可解析则报告 `Unknown`，不查询 executor、不合成结果。恢复期间不得再次发送新的 ReleaseStart。

### 特权 ReleaseStart IPC 失败

若 External diagnostics 返回 `executor_*` 或 `release_*` 错误码，不要据此推断业务脚本已启动。`executor_*` 表示本机 IPC 失败；`release_task_*`、`release_artifact_*`、`release_secret_environment_*`、`release_deadline_*`、`release_snapshot_*` 和多数 `release_authorization_*` 表示控制面授权/门禁阶段失败，executor 可能尚未收到 ReleaseStart。新版本 Agent 会在 `deploy-go-agent` journal 记录 `privileged release executor IPC request failed`，包含 `deployment_id`、`target_run_id`、`target_id`、内部 `task_id`/`job_id`、稳定错误码、期望 executor 协议版本、响应存在时的实际版本、适用时的帧长度和 `io_kind`；不记录请求帧、签名授权、Env 或原始 IO 错误文本。

只读关联日志：

```bash
journalctl -u deploy-go-agent -u deploy-go-agent-executor \
  --since '30 minutes ago' --no-pager -o cat
```

External diagnostics 不公开内部 `task_id`。用其 deployment/target 标识与错误阶段、时间范围关联 Agent journal，再用日志中的 `task_id`/`job_id=release_<task_id>` 对照 executor journal。结合错误码判断：

| 错误码 | 可确认的 IPC 事实 | 下一步只读核查 |
| --- | --- | --- |
| `executor_unavailable` | Agent 无法连接 executor Socket | 查 executor unit 状态、Socket 是否存在及 journal；不要直接改权限 |
| `executor_request_too_large` | 请求序列化长度超过日志中的上限 | 核对目标模块数、受控 Env 变量数量与请求预算；不要读取或输出 Env 值 |
| `executor_request_write_failed` | 请求帧写入失败 | 结合 `io_kind` 和两侧同一时间点日志核对 Socket 生命周期 |
| `executor_request_write_failed` / `executor_response_timeout` / `executor_response_closed` / `executor_response_truncated` | 请求可能已部分或完整写入，但 Agent 未收到完整响应；job 可能已启动 | 当前 Agent 会用完全相同的 ReleaseStart 请求（job ID 与 payload digest 不变）重试一次；无论重试收到错误还是再次失联，都不能推翻第一次请求的不确定性。Agent 保留 `PrivilegedRelease` journal phase 并进入 durable monitor。核对 executor 日志与对应 job 持久状态，不要清理、重启或重发部署 |
| `executor_response_invalid` / `executor_response_too_large` / `executor_response_unexpected` | 收到的响应不符合当前 IPC 响应合同 | 核对 Agent/executor 是否成对安装、协议版本是否一致，并保留两侧 journal |
| `release_authorization_*` / `release_task_*` / `release_artifact_*` / `release_secret_environment_*` / `release_deadline_*` / `release_snapshot_*` | 控制面拒绝授权或发布门禁；多数情况下 Agent 尚未发送 ReleaseStart | 按错误码核对任务是否仍活动、目标/快照/制品/Env 门禁及控制面签名配置 |
| `privileged_release_checkout_unsafe_entry` | Agent 在发 ReleaseStart 之前，对 checkout 做摘要时发现 root 执行器不接受的条目：符号链接、硬链接文件或非普通文件（标准见 `docs/standards/privileged-agent-executor.md`）。部署日志会列出前几个被拒绝条目的相对路径和类型 | 这是业务仓库内容问题：`.git` 与**仓库根部隐藏目录（如 `.agents`、`.claude`）内的符号链接**不会导致失败（它们被忽略，不进入发布 bundle，部署日志会以 `[WARN] 已忽略…` 列出）；其余位置的符号链接、硬链接和非普通文件仍会被拒绝，请把部署日志列出的条目从仓库移除后重新部署。不要尝试在节点上手工删除文件。0.3.31 及更早版本的 Agent 没有隐藏目录例外，遇到此类链接也会以此错误码失败 |
| `privileged_release_manifest_invalid` / `privileged_release_env_invalid` / `privileged_release_env_mismatch` / `privileged_release_checkout_invalid` | 准入前的本地校验失败：发布物 manifest 不可读或格式错误、Env 文件缺失或摘要与控制面不一致、checkout 路径不合法 | 核对该任务的发布物是否完整下载、Env 是否在部署期间被改动；仍无法定位时结合节点 `privileged_release_admission_rejected` 诊断事件（含 `reason`） |
| `privileged_release_admission_failed` | 旧版 Agent（0.3.31 及更早）对上述所有准入失败使用的笼统错误码，不含原因；新版 Agent 只在极少数内部错误时才出现 | 升级 Agent 后复现可得到具体错误码；旧版无法区分原因 |
| `release_authorization_replayed` / `release_job_conflict` | executor 检测到授权重放或 job ID 与 digest 冲突；Start 重试中的冲突本身不能证明首次 Start 未在并发 admission 中成功 | Agent 继续查询同一 job；若 Output/Status 确认 digest 冲突则将当前 task 以 `release_job_conflict` 终结。核对两侧 journal 和 durable job，不要盲目重发部署 |
| `release_path_*` / `release_unsafe_file` / `release_digest_mismatch` / `release_storage_*` / `release_spawn_failed` / `release_recovery_blocked` | executor 返回 admission/job 错误；若它来自首次 Start 的有效拒绝则可以失败，若前一次 Start 已不确定则先由 durable monitor 收敛 | 按稳定错误码检查路径校验、发布物完整性、存储预算或任务启动结果 |

响应丢失类错误码不证明 executor 没有启动 job。当前 Agent 会用同一 ReleaseStart 请求最多重试一次；executor 若已持久化相同 job，会返回现有 job 而不会再次启动。第二次 Start 返回错误也不能否定首次仍在 admission/启动的请求。若仍未确认，Agent 保留 `PrivilegedRelease` phase 并持续通过 `ReleaseOutput`/`ReleaseStatus` monitor 对账，不会立即发出失败终态；读取 IPC 错误和非终态状态时使用最高 5 秒的退避间隔。先从 Agent journal 取得 `job_id`，只读检查默认 job 状态文件：

```bash
sudo jq '{job_id,state,pid,exit_code,reason,last_sequence,updated_at}' \
  "/var/lib/deploy-go-agent-executor/release-jobs/<job_id>/state.json"
```

若日志出现 `terminal state could not be persisted`，说明启动已失败但终态落盘也失败；状态文件可能仍为 `Sealing`。admission 前存储失败则可能尚无 job 文件。这两类情况仍需人工只读核对存储与进程，Agent 不会根据 not-found 推定业务未执行。取消中的任务会反复对同一 job 发送 cancel，覆盖任务尚未建立时的取消丢失；监控只接受协议版本和 job ID 匹配的响应。

若使用了非默认 `release_jobs_dir`，先从 executor 本机配置确认受管目录。状态为 `running`/`sealing` 或文件不存在时都不要手工重放部署：前者表示 job 可能仍在执行，后者也可能是 executor 尚未完成 admission/sealing，不能单独证明未启动。保留现场并结合 Agent/executor journal 核对；不要删除状态、杀进程或重启服务。Agent 自动进行的一次同 job 幂等重试不等于创建新部署，也不得由人工额外重放。

旧 Agent 仍可能把这些情况折叠成 `privileged_release_executor_protocol`，External diagnostics 会将未知错误码显示为 `agent_error`。若无上述结构化 Agent 日志，只能确认诊断信息不足；不能据此断定 executor 版本冲突或某个 IPC 分支已发生。持续无法确认 durable job 时，修复现场前不要清除 release job、任务 journal 或重启服务。

## API 与 Agent 重启

计划重启前记录活动 deployment、task ID 和最后日志游标。API 重启后节点先离线，Agent 重连并以新 connection generation 对账。Agent 重启后从受保护 journal 恢复 payload digest、进程 start-time、日志偏移和完成标记。

只有 task ID、digest 和进程身份一致时继续跟踪；不确定结果进入 `interrupted`。核实不存在冲突执行后使用 retry API 创建新 deployment，不复用或删除原记录。详细 Agent 故障步骤见 `docs/runbooks/agent-recovery.md`。

API 收到 SIGTERM 后先停止 HTTP 接入，再通知内部署 worker 退出并等待完成；不通过直接 abort 留下新的领取循环。启动时 worker 会恢复过期 delivery lease，并核对制品数据库与受控目录。

## 制品与 Env 恢复

1. 不手工删除 `artifacts/quarantine`、`artifacts/objects` 或修改 `deployment_artifacts` 状态。worker 启动及每小时执行 reconciliation：过期上传失败化、缺失 object 失败化、无活动 lease/run 的过期制品清理、孤儿文件清理。
2. 下载中的 object 有进程内 pin，当前下载结束前不会被清理；API 异常退出后 pin 消失，但数据库中的活动 target run/lease 仍阻止误删。无法证明引用关系时先停止 API并保留现场。
3. Agent 重连后 Env 只补偿应用当前版本，不重放已经被替代的明文版本。`pending`/`syncing` 可等待收敛；`failed` 由管理员按目标重试，成功节点不得重复下发。
4. release 报 `env_gate_failed` 时先在 Web 核对该目标的 `actual_version`、脱敏错误码和节点在线状态；不要绕过门禁或把 Env 内容写入部署参数。
5. Env 删除使用 tombstone 和同一 no-follow 路径。节点离线时删除保持待同步，重连后只执行当前删除事实。
6. `awaiting_release` 的制品受 deployment 引用保护，不因普通 TTL 清理。若页面提示制品缺失或 Env 未就绪，保留现场并修复同步问题；不得通过修改 `expires_at`、`phase` 或同步台账绕过门禁。

### 制品上传 finalize 失败

`POST /api/v1/agent/artifact-leases/{id}/upload/finalize` 返回 500 时，先按 request ID 查询 API 日志，确认 `phase`、`artifact_id`、`lease_id`、`attempt` 和脱敏的数据库错误类别。重点区分 `begin`、`promote_object`、`consume_lease`、`verify_artifact`、`bind_target_runs`、`commit` 与 `commit_state_unknown`，不能仅凭 500 推断为 SQLite 锁竞争。

只读核查制品和 lease 的数据库事实：

```sql
SELECT lease.id, lease.status AS lease_status, lease.expires_at,
       artifact.id AS artifact_id, artifact.status AS artifact_status,
       artifact.storage_key, artifact.upload_offset, artifact.upload_size
FROM artifact_leases lease
JOIN deployment_artifacts artifact ON artifact.id = lease.artifact_id
WHERE lease.id = ?;
```

同时核对制品对象是否存在、大小是否等于 `upload_size`，并按其 SHA-256 与 `storage_key` 比对。对象已存在且数据库为 `verified` 且 `storage_key` 等于摘要时，允许客户端重试 finalize；接口应幂等返回成功。对象存在但数据库仍为 `uploading` 时，不得直接把数据库改成 `verified`，应保留 request 日志并由受控重试或 reconciliation 收敛。对象存在而数据库为 `failed` 或无 `storage_key` 时，不重试原 lease，也不手工删除对象，交由 reconciliation 按数据库事实处理。

若日志为 `commit_state_unknown`，禁止自动重试原事务、删除 object、移动 quarantine 文件或手工修改 SQLite。保留数据库、`-wal`、`-shm`、制品目录和完整 request 日志，待数据库可读后重新执行上面的只读核查：确认已提交则保留 object；确认未提交且没有引用再由受控 reconciliation 清理；仍无法确认则继续保留现场并升级处理。任何补偿 rename 失败都必须保留对象并记录告警，不能以请求成功或静默清理掩盖不一致。

修复版本的正式验收应记录 finalize 成功率和延迟、SQLite `BUSY/LOCKED` 次数、`commit_state_unknown` 次数、补偿失败数、孤儿对象数及异常 artifact 状态数。发布后观察窗口内若出现提交结果未知增加、对象与数据库状态持续不收敛、或 finalize 失败率高于基线，应停止继续发布并保留回滚证据。

## SQLite 备份与恢复

SQLite 使用 WAL。优先停止 API 后备份；在线备份必须使用 SQLite backup API 或经过验证的一致性工具，不能只复制主 `.db`。

恢复顺序：

1. 停止 API，保留当前数据库、`-wal` 和 `-shm` 作为证据。
2. 恢复同一时点的一致性备份并运行 `make api-migrate`。
3. 启动 API并确认 `/readyz`、migration 和审计正常。
4. 等待 Agent 重连和 reconcile，再核对 queued/running/canceling；不手工猜测终态。

## 本地验证

```bash
cargo test -p deploy-go-api --test deployment_runtime --test deployment_recovery
cargo test -p deploy-go-api --test agent_dispatcher --test agent_end_to_end
cargo test -p deploy-go-api --test two_stage_deployment --test deploy_event_protocol
cargo test -p deploy-go-api --test artifacts_api --test env_sync_dispatcher
cargo test -p deploy-go-agent --test privileged_release_recovery
cargo test -p deploy-go-api --lib cross_node_dispatch_skips_queued_deployment_when_target_already_active
make api-check
```

这些测试不需要 OpenSSH 客户端、SSH 私钥或真实节点。
