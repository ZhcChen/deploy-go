# 卡死任务不再堵塞节点（紧急修复）

状态：规格就绪
来源：2026-11-11 线上事故与用户指令“先赶紧处理”。测试环境节点01 上 `qfy-notification-test` 的发布在 16:59 已失败退出，但任务停在 `canceling` 超过 50 分钟，同节点 `xmst-trade-testing`、`qfy-voucher-hub-testing`、`sc-test` 的部署全部排队。因事故紧急，实现先于本规格落笔，本文记录需求、范围与验证要求。

## 目标与范围

一个收不了尾的部署任务不得拖累同节点其他应用的部署，也不得阻止节点自动升级；并修复导致本次卡死的 executor 输出分页缺陷。

范围：
1. executor 的 `ReleaseOutput` 按字节预算分页。
2. 主控对超期仍无终态的部署任务做兜底中断。
3. 升版到 0.3.34，使节点通过自动升级获得 executor 修复。

明确不做：同节点多应用并发部署（用户已要求，另立规格，需要改 runner broker 的串行门禁并做 Linux 专项验证）；不手工修改线上数据库状态；不修改业务应用。

## 用户场景与验收

### US1：长输出的发布能正常收尾

- 优先级：P1
- Given：发布脚本输出很长的行（单帧落盘约 32KB）
- When：Agent 以每页最多 128 帧读取输出
- Then：每一页都能装进一个协议帧（64KB），全部输出按序读完，任务得到终态
- 边界：单帧本身不超过预算（采集端 8KB 读块）；一页至少返回一帧保证前进。

### US2：卡死的任务超期后释放节点

- 优先级：P1
- Given：已投递的部署任务超过 `deadline_at` 加宽限期仍无终态（含 `canceling`）
- When：主控 worker 轮询
- Then：任务与部署被判为 `interrupted`（`task_deadline_exceeded`），同一 Agent 上排队的部署可以继续派发，节点恢复“空闲”可自动升级
- 边界：宽限期内不处理；排队中的任务不处理；已是终态的不重复处理；兜底自身失败不得让调度循环停摆；`deadline_at` 的两种时间写法都能识别，无法解析的不误判。

## 功能要求

- FR-001：`agent-executor` 的 `ReleaseJobManager::output` 增加单页字节预算（协议帧上限减余量），超出预算即截断本页，至少返回一帧。
- FR-002：`api` 新增 `interrupt_overdue_deployment_tasks`：`deployment_execute/prepare/release` 任务状态为 `delivered/accepted/running/canceling` 且超过 `deadline_at` 5 分钟时，复用既有 `interrupt_task` 投影路径判为中断，错误码 `task_deadline_exceeded`；由 worker 每轮调用，失败只告警。
- FR-003：对外诊断的错误码白名单与超时判定纳入 `task_deadline_exceeded`。
- FR-004：版本升至 0.3.34 并同步 OpenAPI 与双端客户端。
- FR-005：`docs/runbooks/deployment-recovery.md` 记录超期兜底语义与该故障的症状、原因。

## 成功标准

- SC-001：新增 executor 回归测试在修复前失败（一页 1.6MB 超过 64KB）、修复后通过。
- SC-002：新增主控测试覆盖宽限期内不处理、超期中断并释放 Agent、两种时间写法、不重复处理、排队任务不受影响。
- SC-003：`make api-check`、`make agent-check`、`make agent-executor-cgroup-check`、`make api-client-check`、`make check` 通过。
- SC-004：上线后（需用户授权部署）卡死任务被判中断、排队部署继续、节点自动升级到 0.3.34；结果如实记录。

## 风险与验证要求

- 语义变化：此前 Agent 长时间离线的任务会一直等待重连对账，现在超期 5 分钟后判为 `interrupted`，之后 Agent 补报的真实结果被忽略。`interrupted` 的既有含义就是“无法证明结果”，但运维需要知道节点上脚本可能已执行完成。
- 中断后同一 Agent 可能立刻收到下一个部署；若旧任务的进程仍在节点上运行，runner broker 会以 `runner_busy` 拒绝新任务（既有保护），不会并发执行。
- executor 改动不涉及 `cfg(target_os = "linux")` 分支，但 executor 的生命周期测试在 Linux 下依赖 cgroup，仍需运行 `make agent-executor-cgroup-check`。
- 不涉及 migration、线协议 schema 与任务 payload。

## 未决事项

无。
