# 同节点多应用并发部署

状态：规格就绪
来源：用户需求（2026-11-11）：测试环境有多个项目共同开发，部署频繁，同时部署的概率很大；不同应用应当能同时部署，不应在同一个节点上排队；并发数量调到 10；生产节点保持原样。分析与证据见 [Obsidian 待办登记](../../docs/solutions/2026-11-11-api-startup-log-replay-and-build-cache.md) 之前的排查：`api/src/agents/dispatcher.rs` 的按 Agent 串行投递，以及 `agent/src/runner_service.rs` 的单槽位 runner broker（提交 `aa2153b` 起有意为之）。

## 目标与范围

让同一个节点（Agent）上不同应用的部署可以同时运行，数量按节点配置；默认保持现状（同一节点一次一个），只在节点本机显式开启并且控制面为该节点设置了大于 1 的数量时才并发。

范围：
1. 控制面：节点新增“同节点并发部署数”设置（1 至 16，默认 1），投递时按该数量与 Agent 声明的能力共同限制同一 Agent 的活动部署任务数。
2. Agent：新增能力 `concurrent_deployments_v1`；runner broker 由单槽位改为受上限约束的多槽位，上限由节点本机配置决定（默认 1，开启后为 16）。
3. 管理端：节点页显示并可修改并发数，显示该节点本机是否已开启。
4. 迁移：测试与开发环境的节点默认设为 10；全平台总并发（系统设置）低于 10 时提到 10。
5. 标准、手册同步，说明隔离取舍与开启步骤。

明确不做：
- 不放开同一应用同一部署目标的互斥（同一个应用连发两次仍然一个一个来）。
- 不为并发任务分配独立的运行身份（见“风险”，作为已知限制记录，不在本规格内实现）。
- 不改变 release 阶段（root executor 的 release job 本来就按任务独立运行）。
- 不修改 Agent 线协议版本；不修改业务应用。
- 不在节点上自动开启并发；节点本机开启需要运维步骤并单独授权。

## 用户场景与验收

### US1：测试节点上不同应用同时部署

- 优先级：P1
- Given：测试节点已在本机开启并发，控制面为该节点设置的并发数为 10，全平台总并发不小于 10
- When：同一时间对该节点上 5 个不同应用发起部署
- Then：5 个部署同时进入运行，不再排队等待前一个结束
- 边界：同一个应用的同一目标再次发起时仍排队；超过并发数的部署排队，有部署结束就补上一个。

### US2：生产与预发布节点不受影响

- 优先级：P1
- Given：节点本机未开启并发（默认），或控制面并发数为 1
- When：同一节点上有多个应用部署
- Then：行为与现状完全一致：同一节点一次只运行一个，其余排队
- 边界：即使控制面把某个未开启并发的节点设为 10，实际仍是 1（能力缺失）；旧版本 Agent 同样按 1。

### US3：卡死的任务仍会释放名额

- 优先级：P1
- Given：某个任务超期无终态
- When：超期兜底（015）把它判为中断
- Then：它占用的并发名额被释放，同节点其他部署继续

### US4：管理员能看懂并调整

- 优先级：P2
- Given：管理员打开节点详情页
- When：查看并修改并发数
- Then：看到当前值、允许范围（1 至 16）、本机是否已开启并发；节点本机未开启时给出明确提示，修改被记录审计，并发数的修改需要 CSRF 并使用乐观版本

## 功能要求

- FR-001：数据库新增 `nodes.max_concurrent_deployments`（整数，默认 1，约束 1 至 16）。迁移把环境为 `dev` 或 `test` 的、未撤销未归档的 Agent 所属节点设为 10；若系统设置中的 `max_concurrent_deployments` 存在且小于 10，则提到 10；其余节点与设置不变。
- FR-002：节点接口返回该设置；新增管理员接口修改它（范围 1 至 16、需要 CSRF、校验节点版本、写审计、响应不缓存）；节点归档后不可修改；OpenAPI 与双端客户端同步生成。
- FR-003：`agent-protocol` 新增 `AgentCapability::ConcurrentDeploymentsV1`（`concurrent_deployments_v1`）与常量 `MAX_CONCURRENT_DEPLOYMENTS_PER_NODE = 16`；同步 JSON schema 与兼容性测试；不修改 `PROTOCOL_VERSION`。
- FR-004：控制面对同一 Agent 的部署类任务（`deployment_execute`、`deployment_prepare`、`deployment_release`）的有效并发上限为：Agent 声明了 `concurrent_deployments_v1` 时取节点设置，否则为 1。投递改为“同一 Agent 上处于 `delivered`、`accepted`、`running`、`canceling` 的其他部署类任务数小于上限”，判断与更新在单条语句内原子完成；排队等待判断使用同一规则。
- FR-005：broker 的活动任务由单个槽位改为集合，上限由本机配置 `DEPLOY_GO_RUNNER_CONCURRENT`（默认关闭；开启后上限为 16，关闭时为 1）决定；启动超过上限、同一任务重复启动返回 `runner_busy`；取消只作用于集合内的任务；清理只在目标任务不活动时才被 `runner_busy` 拒绝（不再因为别的任务活动而拒绝）；重启恢复所有可信的活动 runner，数量超过上限时 fail-closed 拒绝启动；每个任务目录的权限切换互相独立。
- FR-006：broker 本机协议新增 `Capacity` 请求动作，返回当前上限；旧 Agent 不会发送，旧 broker 对未知动作返回 `invalid_request`，两个方向都兼容。
- FR-007：Agent 启动时向 broker 查询上限，大于 1 才在 Hello 中声明 `concurrent_deployments_v1`；查询失败或为 1 时不声明。
- FR-008：管理端节点页展示与修改并发数，展示 Agent 是否声明了并发能力；未声明时说明需要在节点本机开启；前端测试覆盖。
- FR-009：更新标准 `agent-installation-contract.md`、`application-deployment-contract.md` 与手册，说明默认值、开启步骤、隔离取舍、回退方法。
- FR-010：Linux 容器专项验证覆盖 broker 的多槽位、上限、恢复与权限切换。

## 成功标准

- SC-001：未开启并发的节点，所有现有测试与行为保持不变（broker 上限 1 时第二个任务得到 `runner_busy`，控制面同一 Agent 一次一个）。
- SC-002：开启并发并设置为 N 的节点，同一 Agent 上最多同时有 N 个部署类任务处于活动状态，第 N+1 个排队，结束一个补上一个（控制面测试覆盖 N=1、3 以及能力缺失）。
- SC-003：broker 多槽位测试在 Linux 容器内通过：N 个并发启动成功、第 N+1 个 `runner_busy`、每个任务目录权限独立切换与回收、重启恢复多个活动 runner、超过上限 fail-closed。
- SC-004：`make api-check`、`make agent-check`、`make admin-check`、`make admin-app-check`、`make api-client-check`、`make agent-runner-isolation-check`、`make agent-executor-cgroup-check`、`make check` 通过。
- SC-005：上线（需用户授权部署）后，生产与预发布节点行为不变；测试节点在本机开启并发（另需授权）后，同时发起的多个不同应用部署能同时运行，结果如实记录。

## 风险与验证要求

- **隔离取舍（已知限制）**：当前所有 runner 共用同一个运行身份，任务目录只在任务活动期间对 runner 组开放。并发后，多个准备阶段任务同时活动，它们的任务目录（含该任务落盘的 Git 部署密钥与 Env 副本）会同时对所有 runner 进程可读，一个应用的准备脚本理论上能读到另一个同时运行的应用的这些内容。发布阶段由 root executor 执行，本来就能读取节点上的全部内容，不因并发改变。因此该能力默认关闭，仅在受信任的测试节点由本机显式开启；生产与预发布不得开启，除非先实现按任务独立运行身份。这一点必须写入标准与手册。
- 串行门禁是 broker 的安全不变量，改动必须保持“fail-closed、恢复只信任进程身份、不由 Agent 进程内锁替代”，由 Linux 容器专项验证覆盖。
- 控制面与 Agent 的上限必须一致：节点设置的上限不能超过本机开启后 broker 的上限；开启后 broker 上限固定为 16，等于节点设置的最大值，不会不匹配；关闭时控制面凭能力缺失按 1 处理。
- 迁移改动的是线上数据（节点默认值、系统设置），需要核对迁移后的实际值，并记录。
- 涉及 `cfg(target_os = "linux")` 代码，必须用 Docker 目标验证，不能以 macOS 通过为准。
- 版本联动：agent、agent-executor、agent-updater 成对发布，升至 0.3.35；控制面先于节点部署（既有顺序）。

## 未决事项

无。节点本机开启并发（修改 `/etc/deploy-go-agent/config` 并重启 runner 与 Agent）属于对测试节点所在服务器的运行态操作，需要在上线后单独授权，不包含在本规格的实施授权内。
