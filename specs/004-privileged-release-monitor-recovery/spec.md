# 特权 Release 终态可靠收敛

状态：规格就绪
来源：`docs/solutions/2026-10-12-privileged-release-result-not-reported.md`、`docs/runbooks/deployment-recovery.md`

## 目标与范围

确保特权 release 已在节点执行后，Agent 与 Deploy Go 能可靠、可恢复地收敛同一个任务的真实终态。输出未完整接收、monitor 暂时失败或 Agent 重启时，不得重复执行 release，也不得把节点端成功直接推断为控制面成功；运维人员应能区分制品下载、release 执行、结果交接和需要恢复的状态。

范围包括 Agent 对既有 durable release job 的监控、输出与结果交接，控制面对 Agent 执行态和终态的投影，以及相应的诊断、恢复手册和自动化验证。不包括业务应用及其脚本/发布物修改、真实节点恢复、人工更改部署终态、重发部署或 production 晋级。除非设计证明现有状态和协议不足，不新增外部 API、Agent wire 字段或数据库 migration。

## 用户场景与验收

### US1：release 结果可靠收敛

- 优先级：P1
- Given：Agent 已为特权 release 建立 durable job，节点脚本完成或 monitor 经历暂时性故障/Agent 重启
- When：Agent 继续对账该任务
- Then：Agent 仅查询和续接同一个 job，先完整持久化连续输出，再持久化并交付与节点事实一致的唯一逻辑终态
- 关键失败或边界场景：输出分页未结束、输出序号缺口、日志或 journal 暂时不可写、outbox 发送中断、executor 尚未进入终态，以及结果重复投递

### US2：运行状态和未决结果可辨识

- 优先级：P1
- Given：release 已越过制品下载并进入远端发布脚本，但控制面尚未收到 TaskResult
- When：运维人员查看部署及目标运行
- Then：目标运行不再错误显示为制品下载；未确认终态时不显示部署成功，并能看到当前运行或明确的恢复/中断诊断
- 关键失败或边界场景：Agent TaskState 晚到或重复、TaskResult 丢失后重放、monitor 不能安全证明节点终态

## 功能要求

- FR-001：release 开始后的恢复必须绑定原有 `job_id` 与 payload digest；恢复、重连和 Agent 重启不得再次启动 release，不得生成替代 job。
- FR-002：Agent 只有在 executor 对应 job 的终态已确认，且所有仍可读取的输出均已连续接收并持久化后，才能将任务收敛为成功或失败；序号缺口和未完成分页不得绕过此门槛。executor 明确报告输出超限且 job 失败时，Agent 必须收敛为带稳定原因的失败，不得因无法补回被丢弃的输出而无限等待。
- FR-003：monitor 的暂时性 IPC、journal、事件或日志交付故障不得静默结束任务监控；系统必须可从 durable 状态续接，并使逻辑终态可幂等重放。
- FR-004：当可解析的 TaskJournal 仍提供可信 task 身份、payload digest 与恢复阶段，但永久序号不一致或关联数据损坏导致结果无法安全证明时，系统必须保留可诊断的恢复信息并报告非成功的明确中断状态；不得伪报成功、推断 release 未执行或自动重跑。若核心 TaskJournal 缺失/不可解析、无法可信取得 payload digest 或游标，则必须报告 keyed-by-task 的明确未知/恢复诊断，不得向 executor 查询、伪造 TaskResult 或标记成功；该核心 journal 损坏场景不承诺自动 terminal 收敛。若本地持久化暂时不可用，monitor 必须保留原恢复阶段并退避重试。
- FR-005：控制面收到能证明远端 release 脚本已开始的有效进度事件后，目标运行必须离开 `artifact_download` 并反映执行中；仅有通用 TaskState `Running` 不足以证明这一阶段，因为它可能先于制品下载。收到有效终态结果后，agent task、目标运行与父 deployment 必须按同一结果收敛，旧状态事件不得使终态回退。
- FR-006：新建特权 release 必须在 Agent durable journal 中持久化非敏感的 deployment/target run 关联标识，并在 Agent 内部诊断中关联 deployment、target run、task 和原 durable job，说明失败阶段、Agent 已持久化序号及可确认的 executor 终态信息。旧 journal 缺少关联标识时应从有效任务 descriptor 回填；若核心 journal 仍可信但 journal 与 descriptor 均缺少/损坏关联标识，必须以 task/job 标识留下明确的关联元数据不可用诊断，并只观测原 job，待其终态后报告非成功中断结果，不得静默跳过或重新启动。若核心 journal 本身缺失/不可解析，则以可得的 task 标识留下未知/恢复诊断，明确列出不可恢复的 digest/游标等字段，不向 executor 发起无 digest 查询或伪造终态。不得向外部诊断公开内部 task/job 标识，亦不得暴露 Env 值、密钥、授权正文、业务输出正文或不必要的内部绝对路径。
- FR-007：不得依据节点健康检查、current release 或 release-job 文件单独将控制面部署标记成功；成功必须来自符合 FR-002 的 Agent 终态结果。
- FR-008：相同 TaskResult 的安全重放不得重复改变 agent task、target run 或 deployment 的终态、版本和更新时间；冲突或迟到的结果不得覆盖已确认终态。

## 成功标准

- SC-001：注入最多 3 次可恢复的 monitor/交付故障后恢复连接，系统从原 job 和最后已持久化序号继续；完整输出不丢失，release 实际启动次数不增加，控制面最终得到唯一逻辑终态。
- SC-002：Agent 在节点 job 终态后重启，重连后能仅靠已有 durable job 收敛准确结果；测试确认没有发送新的 ReleaseStart。
- SC-003：可解析且保有可信 task 身份/digest 的 journal 出现不可恢复序号缺口或关联元数据损坏时不产生 succeeded 终态，并在被分类为永久错误后两个 monitor 周期内产生可查询的明确中断诊断；核心 journal 缺失/不可解析时产生 keyed-by-task 的未知/恢复诊断，不向 executor 发起无 digest 请求，不伪造终态或重新启动。executor 已明确以 `output_limit_exceeded` 失败时，即使输出被截断也必须产生明确失败终态，不得无限循环等待缺失输出。
- SC-004：控制面收到首个有效的 release 脚本开始事件后，目标运行在同一状态更新内离开 `artifact_download`；有效 TaskResult 到达后目标运行和父 deployment 与其结果一致，重复/迟到消息不回退终态。
- SC-005：测试及诊断样本中不存在 Env、密钥、授权正文、业务输出正文或未授权路径信息。
- SC-006：在 TaskResult 投影完成但消息回执提交失败的故障注入下，重放相同结果不重复推进版本/更新时间；同序号冲突结果和终态后的迟到状态均不改变已确认投影。
- SC-007：新特权 release 的 journal 在 ReleaseStart 前持久化 deployment/target run 标识；不含新字段的旧 journal 可加载并从有效 descriptor 回填。当核心 journal 可解析且保有 task 身份、payload digest、恢复阶段与输出游标，但关联字段和 descriptor 缺失/损坏时，reconcile/取消恢复产生带 task/job 标识的明确诊断并只查询原 job；executor 终态后产生非成功中断结果，不静默遗漏、不发起 ReleaseStart，诊断明确标出无法恢复的关联字段且不含敏感正文。核心 journal 本身缺失/不可解析时只验收 Unknown 诊断及“不查询 executor/不伪造终态/不重启”的 fail-closed 行为，不纳入自动 interrupted 收敛。

## 风险与验证要求

此改动影响 Agent、executor durable job、日志 outbox 和 API 部署状态机，属于高风险部署可靠性变更。需要 Agent/executor 与 API 故障注入及端到端 fixture、重复结果/重启恢复测试、敏感信息断言、聚焦 Rust 格式/静态检查，并由独立复核者检查状态机和竞态。可使用本机隔离 Docker 做 Linux 特有验证；不连接真实节点。具体命令与 fixture 见 `plan.md` 和 `quickstart.md`。

## 未决事项

无。默认优先复用现有 TaskResult、durable journal/outbox、executor 状态与 API 字段；只有在 plan 阶段证明它们不足时，才提出兼容性方案并说明 migration/protocol 影响。
