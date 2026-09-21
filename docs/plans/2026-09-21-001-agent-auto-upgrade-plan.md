---
title: Agent 自动串行升级与节点状态实时同步计划
date: 2026-09-21
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# Agent 自动串行升级与节点状态实时同步计划

## Goal Capsule

- **目标：** Deploy Go 正式控制面升级后，能够发现已登记但版本落后的 Linux amd64 Agent，并以可恢复、可审计、全局串行的方式完成配对 Agent、runner 和 root executor 升级；节点列表与详情能实时显示升级进度和阻塞原因。
- **核心方案：** 新增独立的 Agent 升级队列、全局数据库租约和节点维护门禁；Agent 通过 v17 控制协议接收受限升级指令，从固定控制面发布目录下载并校验配对发布物，再由 root executor 执行固定升级操作。管理端使用一条管理员节点状态 WebSocket 接收变更通知，HTTP 快照仍是权威数据源。
- **兼容策略：** v11-v16 Agent 继续连接、心跳和执行已有任务；只有协商到 v17 且声明 `agent_upgrade_v1` 的 Agent 才会接收升级指令。当前 0.3.7 之前的 Agent 不能被远程强制升级，首轮必须人工安装一次支持自动升级的配对版本；未完成 bootstrap 的节点显示明确阻塞状态，不伪造为“等待升级”。
- **架构策略：** 本期只支持 Linux `x86_64`/amd64；非 amd64 节点不占用全局升级租约并显示 `blocked_unsupported_architecture`。不新增 Docker 运行依赖，不把任意 shell、下载地址、路径或 systemd 命令暴露给 Agent 或 executor。
- **安全边界：** 升级授权绑定 Agent、节点、目标版本、manifest 摘要、文件摘要和过期时间；root executor 只接受固定 staging 根目录和固定 `upgrade` operation，原子替换完整配对对象并失败回滚。所有改动只属于 `deploy-go`，严禁修改任何业务应用代码、配置、脚本、仓库内容或发布物。

## Problem Frame

当前正式控制面升级后，Agent 仍需人工逐节点安装新版本。现有 Agent 控制协议 v16 没有升级消息，低权限 Agent 不能替换 `/usr/local/bin`、systemd unit 或 root executor，root executor 也没有 Agent 安装 operation。若简单复用普通部署任务或把安装脚本交给 Agent，会产生三个问题：

1. 旧 Agent 无法理解新任务，严格协议解析会断开连接。
2. 多节点并发替换 Agent、runner 和 executor 会造成服务中断窗口叠加，也会让控制面无法准确判断“正在升级”与“已离线”。
3. 低权限进程不能安全地完成 root 级安装；任意路径、任意命令和任意下载地址会扩大 executor 的 root 权限边界。

因此需要把 Agent 升级作为独立的平台能力：控制面负责版本发现、队列和状态；Agent 负责固定发布物下载、摘要校验和重启前状态落盘；root executor 负责受控安装、服务编排和回滚；重连后的 Agent Hello/升级报告负责最终确认。

## Product Contract

### Requirements

- **R1. 自动发现：** 控制面启动和后台周期扫描当前 Agent 发布目录，发现未撤销、未归档且 Agent 版本低于当前兼容发布版本的节点，按 Agent 与目标版本建立幂等升级任务。
- **R2. 一次 bootstrap：** v11-v16 或未声明 `agent_upgrade_v1` 的 Agent 不接收升级指令；任务状态为 `blocked_bootstrap_required`，并保留当前版本、协议和阻塞原因。已完成 bootstrap 的 v17 Agent 才进入自动队列。
- **R3. 全局串行：** 全局最多一个 Agent 处于实际 `downloading/installing/reconnecting` 升级流程；等待在线、等待空闲的任务不能长期占用实际升级租约。
- **R4. 节点维护门禁：** 升级进入实际执行前建立持久化节点门禁，阻止新的部署、Env 同步、运行探测、节点检查、终端会话和普通 Agent 任务下发；已经运行的业务任务不被强制中断，排队任务在升级完成后继续。
- **R5. 空闲判定：** 只有 Agent 在线且心跳新鲜、无 `agent_tasks` 活动任务、无活动部署/Env/探测租约、无活动终端会话且连接代次仍有效时，才能从等待空闲进入升级。
- **R6. 受控下载：** Agent 只允许从控制面固定的版本化 HTTPS 发布路径下载 manifest、Agent、runner、executor、systemd unit 和 executor config；只接受 Linux amd64 发布物，下载使用临时文件、分块无进展超时、SHA-256 校验和原子落盘。
- **R7. 受控安装：** Agent 只向固定的受管 staging 目录写入下载物，并通过已绑定的 root executor upgrade operation 请求安装；executor 拒绝任意命令、路径、环境变量、远程地址和未签名或错绑定授权。
- **R8. 成对原子回滚：** 升级把 Agent、runner、executor、updater、四个 unit、Agent 非敏感配置和 executor 配置视为一项事务；任何服务健康检查或版本确认失败都恢复上一套完整配对对象和原启用状态。
- **R9. 重连确认：** Agent 自身重启期间不被控制面立即判定为失败；新连接上报目标版本、manifest 摘要和持久化升级报告后，控制面才标记成功并释放维护门禁。旧版本重连、摘要不符或超时则进入失败并保留原因。
- **R10. 状态可见：** Agent API 响应增加可选的独立升级对象，至少区分 `latest`、`blocked_bootstrap_required`、`waiting_upgrade`、`waiting_for_online`、`waiting_for_idle`、`downloading`、`installing`、`reconnecting`、`failed` 和 `blocked_unsupported_architecture`，不能复用节点 `online/offline` 字段。
- **R11. 实时同步：** 管理端建立一条管理员节点状态 WebSocket，服务端只推送带事件序号的节点/升级状态变更通知；连接建立和重连后先用 HTTP 获取快照，断线期间继续使用 HTTP 轮询兜底，不按节点创建多个连接。
- **R12. 运维恢复：** 控制面重启后能恢复队列、租约、门禁和 `reconnecting` 状态；过期 lease 不能让旧 worker 覆盖新任务，升级失败保留可读错误和下一步人工 bootstrap/重试路径。
- **R13. 范围边界：** 不修改业务应用代码、业务应用配置、业务脚本、业务仓库、业务发布物或真实业务节点内容；本期不提供旧 Agent 的无人工 bootstrap，不支持 ARM，不把升级能力实现为通用远程命令执行。

### Acceptance Examples

- **AE1：** 控制面发布目录出现新目标版本，已完成 bootstrap 且当前版本低于目标版本的 Agent 被发现并进入 `waiting_upgrade`；没有同 Agent/目标版本重复任务。
- **AE2：** 两个可升级 Agent 同时等待时，只有一个取得全局租约并进入下载/安装，另一个保持 `waiting_upgrade`；前者成功后后者才继续。
- **AE3：** 节点仍有部署任务、Env 同步、运行探测或终端会话时，升级保持 `waiting_for_idle`，新任务入口全部被门禁拒绝或排队，已有任务不中断。
- **AE4：** v16 Agent 连接正常但没有升级 capability 时显示 `blocked_bootstrap_required`，不会收到未知 v17 消息，也不会因为版本落后被误标为离线。
- **AE5：** Agent 下载中断、manifest 摘要错误、文件 SHA-256 错误或架构不符时，任务进入明确失败/阻塞状态，staging 不污染当前安装。
- **AE6：** root executor 只接受控制面签发且绑定当前 Agent/节点/版本/摘要的升级授权；任意路径、任意命令、错 Agent、过期授权和重复请求均拒绝。
- **AE7：** executor 在替换或重启任一服务失败时恢复完整旧配对；新 Agent 重连后控制面能区分成功确认、旧版本重连和超时失败。
- **AE8：** 管理端列表和详情在同一共享 WS 上收到升级事件后刷新；事件丢失或 WS 断开时重新拉取 HTTP 快照，页面不会把缺失字段显示为“最新”。
- **AE9：** API 重启、worker 进程重复启动和旧 lease 到期不会产生两个活动升级任务，也不会释放仍需要恢复的节点门禁。

## Scope Boundaries

**本期交付：** v17 Agent 控制协议、Agent 升级下载/状态恢复、executor v4 固定升级 operation、SQLite 队列/租约/维护门禁、自动发现 worker、节点/Agent API、管理员节点状态 WebSocket、管理端列表与详情状态展示、amd64 发布物校验、测试、标准与 runbook。

**不在本期：** 业务应用仓库或业务应用代码修改；旧 Agent 的远程首次安装；ARM/aarch64 发布与自动升级；任意远程 shell/脚本/下载 URL；Agent 日志全文采集；跨控制面集群的分布式锁；自动升级业务应用；自动升级真实节点的现场执行；删除历史 Agent 任务或历史部署数据。

## Planning Contract

### Product Contract Preservation

本计划由当前会话需求直接引导，没有可复用的近期 requirements-only Product Contract；以上 R/A/F/AE 是本次实现的完整契约。后续代码实现不得把“自动升级”扩张为业务应用自动发布或通用远程执行。

### Key Technical Decisions

- **KTD1. 升级使用独立协议消息，不复用普通部署任务。** `agent_tasks` 的 ACK/Running/Result 生命周期假设任务进程会返回终态，而 Agent 升级会主动重启自身；独立的 `AgentUpgradeCommand/Ack/Progress/Report` 能明确表达接受、下载、安装和重连确认，也不会让旧 Agent 看到未知 `TaskPayload`。
- **KTD2. v17 保留 v11-v16 兼容窗口。** `PROTOCOL_VERSION` 提升到 17，最低版本继续为 11；v16 Agent 仍以最高共同版本 16 连接。只有协议 17 和 `agent_upgrade_v1` capability 同时成立时才允许下发升级消息。
- **KTD3. 一次 bootstrap 是明确的兼容边界。** 低权限旧 Agent 没有安装 root 文件的权限，不能通过控制面“神奇地”升级；列表和详情必须显示 `blocked_bootstrap_required`，runbook 给出人工安装成对发布物步骤。bootstrap 后的后续版本升级才自动化。
- **KTD4. root executor 通过独立 updater 完成自升级。** executor 协议从 v3 提升为 v4，新增固定 `UpgradeStart/UpgradeAccepted/UpgradeStatus` operation。当前 executor 只负责验签、把 root-owned transaction journal 写入固定目录并启动已经 bootstrap 的 `deploy-go-agent-updater.service`；updater 作为独立 root systemd oneshot 进程负责停止/替换/重启四个服务并恢复事务，因此 executor 自身退出不会丢失升级所有权。请求只包含 job ID、授权和摘要，路径由 executor/updater 本地配置推导，不能由 Agent 注入。
- **KTD5. 升级授权绑定完整输入且复用密钥、隔离 audience。** API 使用现有 release signer 的公钥/私钥，但使用独立 `agent_upgrade` audience、claims 类型和 nonce namespace，避免与特权 release 授权互相消费。claims 精确绑定 `agent_id`、`node_id`、目标版本、manifest digest、每个文件 SHA-256/大小、staging job、协议版本、架构和 deadline。executor 与 updater 重新验证授权、peer uid/gid、Agent 可执行文件和一次性 nonce；bootstrap 版本必须已安装同一公钥配置。
- **KTD6. SQLite 租约与门禁是持久化事实。** `agent_upgrade_jobs` 保存任务状态，`agent_upgrade_leases` 保存单例 global lease，`agent_maintenance_locks` 以 `node_id` 为主键保存 `lease_token` 和单调 `lock_epoch`，root updater journal 保存节点侧升级恢复事实。进程内 mutex/broadcast 只做唤醒和通知，不作为一致性来源。
- **KTD7. 升级采用两阶段候选检查和原子 claim。** `waiting_for_online`/`waiting_for_idle` 只由周期扫描更新，不持有 global lease 或 node lock；worker 在一次 `BEGIN IMMEDIATE` 中重新检查心跳、协议/capability、架构、所有活动任务/会话/租约，然后同时建立 `node_id + lock_epoch` 门禁、写 global lease 和把 job 置为 `downloading`。所有任务创建/派发也在 SQLite 写事务中检查 node lock，因此“检查后被新任务插入”的窗口由数据库写锁消除。
- **KTD8. Agent 状态与 root updater 事务均独立持久化。** Agent 在固定 data root 下通过临时文件、fsync 和原子 rename 保存带 schema/version 的 `upgrade-state.json`；root updater 在 root-owned 固定目录保存 `prepared -> staged -> stopped -> switched -> verified -> committed/rolled_back` checkpoint、旧/新 version slot 和启用状态。控制面重启后 `installing/reconnecting` 进入 recovery barrier，不依据过期 lease 直接让第二个节点开始安装。
- **KTD9. 发布物完整性以 v4 manifest 和文件摘要为准。** Agent 复用现有 HTTPS client、分块下载和 idle timeout 设计，但升级下载逻辑独立于业务 artifact lease；新发布目录只保留 Linux x86_64 配对物和 updater。manifest URL 由控制面版本路由生成，Agent 忽略外部 URL 字段并按版本和固定组件名拼接下载路径。manifest 摘要使用共享 crate 的递归 key 排序 JSON canonicalization，API、Agent、executor/updater 共用 helper/fixture，不能各自直接对未规范化 JSON 做 SHA-256。
- **KTD10. 维护门禁集中复用并覆盖入口。** 将门禁判断抽成 `agent_upgrades` 内部 helper，在 dispatcher 实际 dispatch 核心、WSS Hello 后 reconcile/Env 补偿、Env sync 入队、runtime probe 入队、node check 入队、deployment 创建/恢复和 terminal session 创建处统一检查；任务插入与门禁 claim 必须使用同一 SQLite 写事务。门禁存在时，新的部署/Env/探测/节点检查/终端创建返回稳定 `node_maintenance` 或保持 queued，不能靠前端禁用绕过。
- **KTD11. 管理端 WS 只做失效通知。** 新增独立 `admin-node-status.v1`，使用 Cookie 会话、Origin 和 `csrf.<token>` 子协议，并在连接存活期间复核 session/RBAC；事件只携带脱敏 node/agent ID、`boot_id`、event sequence、node version 和变更类型。浏览器不维护第二套状态模型，收到事件后失效 `nodes`、`agents` 和当前详情查询；断线、会话撤销、权限变化、事件序号缺口、overflow 或 `boot_id` 变化都先重新获取 HTTP 快照，HTTP 轮询始终保留兜底。
- **KTD12. API 兼容优先。** `AgentResponse.agent_upgrade` 为可选对象；旧前端忽略新字段，旧 Agent 连接不受影响。升级枚举不改变 `NodeResponse.status` 或 `AgentResponse.status` 的 `online/offline` 语义。OpenAPI 和生成客户端由项目脚本生成，不手改 generated 文件。
- **KTD13. 控制面单实例假设显式化。** 本期租约保证单个正式控制面实例内全局串行；多控制面 HA、跨数据库锁和 leader election 另行设计，不把进程内状态误称为跨实例一致性。

### Compatibility Matrix

| 层次 | 旧版本 | 本期版本 | 兼容规则 |
|---|---:|---:|---|
| Agent WSS protocol | v11-v16 | v17 | 控制面继续接受 v11-v17；协商到 `<17` 时绝不发送升级消息 |
| Agent executor IPC | v3 | v4 | v3 只执行既有 PTY/release/probe；v4 才声明 `agent_upgrade`；不支持 v3/v4 混合升级 |
| Agent release manifest | schema v3 | schema v4 | 历史 v3 目录仍可下载；当前自动升级目标必须是 schema v4 |
| 发布组件 | Agent/runner/executor | 上述三者 + updater | runner 继续使用既有 Agent runner binary；updater 是固定 root oneshot 组件 |
| Agent capability | 既有 capability | `agent_upgrade_v1` | 只有 v17 + v4 executor capability + amd64 同时满足才进入自动队列 |
| Enrollment | v11-v16 | v17 | enrollment 校验、OpenAPI 和测试上限同步到 17，10/18 仍拒绝 |

版本升级只允许整套成对/成组发布。任何 v3 executor、v4 Agent、v4 manifest 或缺少 updater 的混合状态都只能保持既有任务能力并显示 bootstrap/recovery 阻塞，不能开始自动升级。

### State And Transition Contract

持久化 job 状态和允许迁移固定为：

```text
queued
  -> blocked_bootstrap_required
  -> waiting_for_online
  -> waiting_for_idle
  -> downloading
  -> installing
  -> reconnecting
  -> succeeded
  -> failed
```

`blocked_bootstrap_required` 和 `blocked_unsupported_architecture` 是不可执行阻塞态；失败任务通过管理员只读详情确认后调用 `POST /api/v1/agent-upgrades/{job_id}/retry` 重新进入 `queued`，仅允许 `failed` 任务重试，不得重试 bootstrap/架构阻塞态，不复用旧 lease token。`queued` 只表示已排队，`waiting_for_online`/`waiting_for_idle` 不持有租约，`downloading`/`installing`/`reconnecting` 持有 node lock；`installing`/`reconnecting` 的 lease 过期时进入 recovery barrier，禁止第二 job claim，直到 Agent report 或 updater transaction checkpoint 得出唯一终态。`succeeded`、`failed` 和 `blocked_*` 不得被旧 worker 改回活动态。每次迁移写入 `agent_upgrade_events`，事件 payload 只允许固定枚举、摘要、进度和时间，不写原始响应、路径、token 或 Secret。

状态迁移约束如下：

| 当前状态 | 触发者 | 条件 | 超时/失败 | 租约与门禁 |
|---|---|---|---|---|
| `queued` | scanner/retry | 目标版本存在且无活动 job | 版本不可用则保持 queued | 无 |
| `waiting_for_online` | scanner | Agent 离线或 heartbeat stale | 持续等待，不占 lease | 无 |
| `waiting_for_idle` | scanner | 在线但有活动任务/会话/租约 | 持续等待，不占 lease | 无 |
| `downloading` | atomic claim | 同一事务完成 idle gate、node lock、global lease | 下载失败 -> failed；lease token 不符不得更新 | 有 lock + global lease |
| `installing` | Agent/executor | staging 校验完成，updater transaction accepted | executor/updater 失败 -> rollback 后 failed | 有 lock + recovery barrier |
| `reconnecting` | updater accepted | Agent/控制面连接暂时中断 | 只由 report/checkpoint/人工 recovery 收敛 | lock 保持，禁止新 claim |
| `succeeded` | control plane report | 新版本、manifest digest、capability 和 connection generation 匹配 | 不可回退 | 释放 lock/lease |
| `failed` | control plane/recovery | 有明确稳定错误或 recovery 超时 | 仅管理员 retry 重新进入 queued | 释放 lock；不能释放其他 job 的 lock |
| `blocked_*` | scanner | bootstrap/CPU arch 等不可执行前置不满足 | 只能人工修复前置后新扫描更新 | 无 |

`latest` 是 API 投影状态，不是 job 状态；Agent 实际版本等于当前 release 时立即投影为 `latest`，尚未终结的等待任务由扫描器随后收敛。`upgrade_state_unavailable` 只在数据库读取失败或历史数据缺失时返回，不能作为成功或阻塞的替代。

### 2026-09-21 线上缺陷修复补充

- Agent Hello 上报的实际版本已经等于当前发布版本时，列表和详情必须投影为 `latest`；历史 `failed` job 继续保留为审计事实，不能覆盖当前真实版本。
- 已达到目标版本的失败任务不得再次重试；返回稳定冲突，避免重复安装同一版本。
- 升级后重连报告必须校验“消息来自 Agent 当前连接”，允许当前 connection generation 大于任务下发 generation；旧连接迟到报告不得改变任务。
- ACK 拒绝必须使用 job 的真实 `node_id` 释放 maintenance lock，不能把 `agent_id` 当成节点标识。
- 成功或失败报告真正释放 maintenance lock 后，控制面必须在同一连接内重新执行 Env 补偿和 queued task 派发。
- scanner/claim 不得向已经上报目标版本的 Agent 再次派发同版本活动任务；活动等待任务应按实际版本收敛，历史终态不改写。

对应回归测试至少覆盖：历史失败 + 手工安装后的 `latest` 投影、同版本重试冲突、跨 generation 成功报告、旧 generation 拒绝、ACK 拒绝释放锁、报告释放锁后的任务恢复。

### 2026-09-21 updater staging 路径修复补充

- Agent 下载布局固定为 `<upgrade_root>/<job_id>/manifest.json` 与 `<upgrade_root>/<job_id>/staging/*`；updater 必须从 job 根目录读取 manifest，不得越过 job 目录读取共享的 `<upgrade_root>/manifest.json`。
- 目录布局必须由 updater 单元测试覆盖，防止 detached oneshot 在停服前失败、而控制面长期停留在 `installing`。

### 2026-09-21 手工安装后的旧活动任务收敛补充

- Agent 已通过手工安装达到较新版本且该版本任务已经收敛为 `succeeded` 时，扫描器必须收敛同一 Agent 指向更旧目标版本的 `downloading`、`installing` 或 `reconnecting` 任务。
- 收敛必须复用 job、node、lease token 与 lock epoch 的 CAS 释放路径，同时删除全局租约和节点维护锁；不得直接删除历史任务。
- 归档节点仍不参与扫描和自动升级，旧任务保留 `upgrade_target_superseded` 审计结果。

### 2026-09-21 节点绑定 executor 配置保护补充

- `executor.json.in` 只用于发布清单完整性、摘要和 schema 校验，不得直接安装到节点的 `/etc/deploy-go-agent/executor.json`。
- 自动升级必须保留人工安装阶段已经渲染的 UID/GID、node/agent ID、终端与发布授权公钥等节点绑定配置；只替换 executor 二进制和 unit。
- updater 测试必须确认 `managed_files()` 不包含节点 executor 配置，避免模板占位符导致 executor、runner 与 Agent 无法重启。

节点升级对象建议形状如下，字段均为可选或受控枚举：

```json
{
  "state": "installing",
  "job_id": "upgrade_01...",
  "current_version": "0.3.7",
  "target_version": "0.3.7",
  "phase": "executor_restart",
  "error_code": null,
  "error_summary": null,
  "updated_at": "2026-09-21T00:00:00Z"
}
```

列表接口不返回完整历史事件；详情接口可返回受限的阶段时间和最近错误。不存在升级任务时，已在线且版本等于当前版本显示 `latest`，旧 Agent 显示 `blocked_bootstrap_required`，无法判断时显示 `waiting_upgrade` 或 `upgrade_state_unavailable`，不能默认“最新”。

### Protocol Contract

v17 新增消息使用严格字段和方向校验：

- `AgentUpgradeCommand`（主控 -> Agent）：`job_id`、`target_version`、`manifest_digest`、`authorization`、`deadline_at`、`connection_generation`。
- `AgentUpgradeAck`（Agent -> 主控）：`job_id`、`accepted`、`error_code`；只允许固定错误码。
- `AgentUpgradeProgress`（Agent -> 主控）：`job_id`、`sequence`、`phase`、可选 `downloaded_bytes`/`total_bytes`；不得携带日志正文或路径。
- `AgentUpgradeReport`（Agent -> 主控）：`job_id`、`status`、`target_version`、`manifest_digest`、可选稳定错误码和脱敏摘要；新 Agent 连接完成 Hello 后发送，重复报告幂等。

服务端只通过 `send_generation` 向下发命令，并校验当前连接代次；收到旧连接、未知 job、过期 deadline、重复或回退 sequence 的消息时丢弃或返回稳定协议错误，不改变新连接状态。`validate_for_envelope_version` 对 `<17` 的 `AgentUpgrade*` 消息必须一律返回 false；Agent 协商到 v16 时不发送升级命令。Hello capability 增加 `agent_upgrade_v1`；Hello/HelloAck schema、enrollment handler 上限、方向校验和历史 v11-v16 fixture 必须同步。

### Database Contract

新增 migration `api/migrations/0038_agent_upgrade_queue.sql`，不修改既有 migration。至少包含：

- `agent_upgrade_jobs`：`id`、`agent_id`、`node_id`、`target_version`、`manifest_digest`、`target_architecture`、`status`、`phase`、`current_version`、`connection_generation`、`attempt_count`、`lease_token`、`lease_expires_at`、`error_code`、`error_summary`、`queued_at`、`started_at`、`finished_at`、`updated_at`、`version`；唯一约束保证同一 Agent/目标版本只有一个有效任务。
- `agent_upgrade_leases`：固定 `lease_key='global'`、`job_id`、不可预测 `lease_token`、`expires_at`、`updated_at`，用于全局串行领取。
- `agent_maintenance_locks`：`node_id` 主键、当前 `agent_id` 快照、`job_id`、`lease_token`、单调 `lock_epoch`、`reason`、`created_at`，外键和唯一约束防止同一节点多门禁；释放和更新必须带 node/job/token/epoch CAS。
- `agent_upgrade_events`：`job_id`、单调 sequence、固定 event kind、phase、safe summary、created_at；对 job/sequence 建唯一约束，并设置清理策略。

迁移测试必须验证外键、唯一性、活动 lease 互斥、同一节点门禁互斥、终态保护相关索引和从空库执行。所有 SQL 查询必须遵循项目的参数绑定与 SQLite 事务约定。

## Implementation Units

### U1. Agent v17 控制协议和机器可读 schema

**目标：** 让新 Agent 能安全协商升级能力，同时保证 v11-v16 Agent 不接收未知升级消息。

**涉及文件：**

- `agent-protocol/src/lib.rs`
- `agent-protocol/schema/agent-control.schema.json`
- 新增或更新 `agent-protocol/schema/agent-control-v17.schema.json`
- `agent-protocol/tests/schema_compatibility.rs`
- `api/src/agents/auth.rs`
- `api/tests/agents_api.rs` 或现有 enrollment 测试文件
- `api/openapi/openapi.json`
- `docs/standards/agent-control-protocol.md`
- `api/src/agents/websocket.rs`
- 协议相关 Agent/API 测试

**开发要点：**

- `PROTOCOL_VERSION` 改为 17，`MIN_SUPPORTED_PROTOCOL_VERSION` 保持 11，更新 HelloAck telemetry 校验范围。
- 增加 `AgentUpgradeV1` capability 和四类升级消息；严格 `deny_unknown_fields`、消息方向、当前连接代次、job/sequence 校验。
- 保留 v11 Hello wire shape 与 v12-v16 telemetry 兼容；历史 schema 不重写，只新增 v17 schema/fixture。`validate_for_envelope_version` 对 `<17` 的升级消息一律拒绝，v17 Agent 协商到 v16 时不声明可用升级通道。
- 同步 enrollment 的 `protocol_version` 上限到 17，v11、v16、v17 可注册，10/18 继续拒绝；更新 OpenAPI 和 enrollment handler 测试。
- 统一固定错误码集合，如 `upgrade_unsupported_protocol`、`upgrade_unsupported_architecture`、`upgrade_manifest_invalid`、`upgrade_download_failed`、`upgrade_executor_rejected`、`upgrade_install_failed`、`upgrade_deadline_exceeded`。

**测试场景：**

1. v17 Hello 能序列化 capability，v16 fixture 不出现新字段且仍能协商 v16。
2. v17 command/ack/progress/report 正常 round-trip，未知字段、错误方向、旧 connection generation、重复/回退 sequence 被拒绝。
3. `AgentUpgradeCommand` 不包含任意 URL、命令、路径、环境 map 或 Secret 字段。
4. 协议版本 11、12、16、17 的 HelloAck telemetry 规则分别通过；18 或 10 被拒绝。

### U2. root executor v4 固定升级 operation

**目标：** 让低权限 Agent 能通过可验证的固定协议安装完整配对发布物，不扩大 root 权限。

**涉及文件：**

- `agent-executor/src/protocol.rs`
- `agent-executor/src/main.rs`
- `agent-executor/src/config.rs`
- 新增 `agent-executor/src/upgrade.rs` 或等价模块
- 新增 `agent-updater/Cargo.toml`
- 新增 `agent-updater/src/main.rs`、`agent-updater/src/transaction.rs`
- 新增 `agent-updater/tests/upgrade_lifecycle.rs`
- `agent-executor/tests/protocol.rs`
- `agent-executor/tests/release_admission.rs`
- 新增 `agent-executor/tests/upgrade_admission.rs`
- `agent-executor/tests/upgrade_lifecycle.rs`
- `agent/install/install.sh`
- `agent/install/executor.json.in`
- `agent/install/test-systemd-contract.sh`
- `docs/standards/agent-installation-contract.md`
- `docs/standards/privileged-agent-executor.md`

**开发要点：**

- executor 协议升到 v4，新增固定 `UpgradeStart`、`UpgradeAccepted`、`UpgradeStatus` operation/capability；v3 只保留既有 operation，收到 upgrade 请求直接 fail closed。Agent 启动时 probe executor，只有看到 v4 upgrade capability 才声明 `agent_upgrade_v1`；缺失时继续旧能力，不伪造支持。
- 请求通过现有 release signer 的独立 upgrade audience verifier 验证，claims 精确绑定 node、agent、target version、canonical manifest digest、architecture、每个发布文件 digest/size、job nonce 和 deadline。
- staging 根目录和最终安装目标从 executor/updater 本地受信配置推导；请求不得携带可执行文件、systemd unit 路径、shell、参数、环境变量、任意路径或远程 URL。下载的 `executor.json.in` 只能作为 schema/摘要输入，不能覆盖节点绑定的 `node_id`、`agent_id`、peer uid/gid、socket、允许可执行文件、home/shell、授权公钥或其他安全字段。
- `deploy-go-agent-updater` 是新增 root-owned 固定 binary 和 oneshot unit。executor 在 IPC 请求内写入 root-owned journal 后只启动固定 updater；updater 读取 staging，使用安全 fd/no-follow 重新校验全部摘要和配置白名单，持久化 `prepared/staged/stopped/switched/verified/committed/rolled_back` checkpoint，执行 fsync、版本 slot、固定 systemd service 顺序、健康检查和回滚。逐文件 rename 不被宣称为整套事务原子，崩溃恢复由 updater journal 和稳定旧 slot 负责。
- 新版本不使用 v3/v4 混合配对：v4 Agent + v3 executor、v3 Agent + v4 executor、缺 updater、旧 manifest 任一组合都只保持旧任务能力并阻塞自动升级；安装/回滚测试覆盖每个 checkpoint、kill、磁盘不足、daemon-reload/restart 失败和回滚失败。
- executor 不读取 Agent access/refresh token，不访问外网，不接受 Agent 传入的脚本或环境。

**测试场景：**

1. 合法授权、合法 staging、全量摘要匹配时启动 updater，返回 accepted，updater 事务在重启后继续并报告各阶段。
2. 错 Agent/node/version/manifest/architecture、过期授权、重复 nonce、任意路径、额外文件、符号链接、硬链接、错误 owner/mode 分别拒绝。
3. executor、runner、Agent、updater 任一服务启动失败或健康版本不匹配时，旧二进制、unit、节点绑定配置和 enabled 状态完整恢复；updater 自身重启/崩溃能从 journal 继续或回滚。
4. 非 root 或非授权 peer、并发 upgrade request、超时/取消、executor 重启恢复均保持唯一终态。
5. 旧 v3 executor 对未知 upgrade 请求 fail closed，v4 Agent 与 v3 executor 的混合状态不声明 capability，不影响既有 PTY/release 测试。

### U3. Agent 下载、校验、升级状态恢复和重连报告

**目标：** Agent 在低权限边界内完成安全下载，交给 executor 安装，并能跨自身重启报告结果。

**涉及文件：**

- `agent/src/connection.rs`
- `agent/src/task_handler.rs`
- `agent/src/http_client.rs`
- `agent/src/artifact_transfer.rs`（抽取通用分块/idle timeout 校验，不改变 artifact lease 契约）
- `agent/src/config.rs`
- `agent/src/main.rs`
- 新增 `agent/src/agent_upgrade.rs`
- `agent/src/executor_client.rs` 或当前 executor IPC client 所在模块
- `agent/src/journal.rs`（只增加独立升级状态恢复边界，不改变旧任务格式）
- `agent/src/storage_cleanup.rs`（保护升级状态/transaction staging，按 schema 和保留期清理）
- Agent 单元/集成测试、fixture

**开发要点：**

- 收到 v17 command 后先校验 capability、deadline、版本为控制面当前目标、架构为 `x86_64`，再回复 accepted；拒绝时不开始任何下载。
- 固定拼接 `/api/v1/agent/download/{version}/...`，通过现有 HTTPS client 获取 v4 manifest、Agent/executor/updater 二进制和四个 unit；Agent 忽略 manifest 中的 URL，只按固定组件名拼接路径。manifest 使用共享 canonical JSON helper 校验摘要、版本、协议、架构和固定文件清单，逐文件 SHA-256/size 校验，下载写 `.part` 后原子 rename。
- 复用已有无进展检测：单 chunk idle timeout、有限重试、取消和磁盘空间错误必须进入明确状态，不将无限等待当成在线。
- staging 目录固定在 Agent data root 下的升级工作区，清理只允许本 job 目录；`upgrade-state.json` 使用版本字段、fsync、原子 rename 和损坏恢复；storage cleanup 不删除活动升级目录、不触碰 credentials/secrets，不写凭证、不把访问令牌写入 manifest/staging metadata/log。
- 写入 `upgrade-state.json` 后请求 executor；executor 返回阶段后发送 progress。调用成功后 Agent 主动退出/等待服务重启；新实例读取状态并在 HelloAck 后发送 report。
- report 必须幂等，旧状态、目标摘要不符、恢复文件损坏或 deadline 过期均返回稳定失败；升级完成后安全清理 staging 和状态文件，但保留受控审计摘要。

**测试场景：**

1. v4 manifest、三个二进制、四个 unit、executor config 全部下载和校验成功；无重定向到非 HTTPS、无任意 URL，历史 v3 下载路径仍按旧契约可用。
2. 下载断流、chunk idle timeout、Range 恢复、SHA 不匹配、manifest schema 错误、磁盘不足、取消和 deadline 超时均可收敛并清理临时文件。
3. executor 拒绝时不删除旧状态；Agent 重启后能读取状态并发送 report，不重复下载/安装。
4. 新版本重连且目标版本/manifest digest 匹配时报告 succeeded；旧版本重连或摘要不符时报告 failed。
5. 旧任务 journal、旧 Agent 版本状态、v4 Agent 缺 v4 executor capability 和没有升级状态文件的启动路径不回归。

### U4. SQLite 升级队列、全局租约和维护门禁

**目标：** 提供跨 worker/API 重启可恢复的升级事实和全局串行约束。

**涉及文件：**

- `api/migrations/0038_agent_upgrade_queue.sql`
- 新增 `api/src/agents/upgrades.rs`
- `api/src/db.rs` 或 migration 测试公共模块
- `api/tests/agent_upgrades.rs`
- `api/tests/migrations.rs`
- `api/tests/database_constraints.rs`

**开发要点：**

- 在修改 `api/migrations/0038_agent_upgrade_queue.sql` 前执行 `make setup-git-hooks` 和 `make verify-git-hooks`；建立 job/lease/maintenance/event 四组表、索引、外键和唯一约束；所有时间使用现有 RFC3339 约定，所有安全摘要设长度上限。
- 写 `enqueue_for_current_release`：按已登记 Agent、发布 manifest、架构和 capability 幂等创建/更新任务；不为 revoked/archived Agent 创建可执行任务。
- 写两个阶段的 `refresh_waiting_state` 与 `claim_ready_job`：前者只更新 `waiting_for_online`/`waiting_for_idle`，不持有 lease/lock；后者在 `BEGIN IMMEDIATE` 中重新执行全部 idle gate，再原子建立以 node 为主键的 lock、global lease、lock_epoch 和 connection generation，事务失败不留下半个 lock。
- 所有 lease heartbeat、状态迁移、progress/report、node lock release 都使用 job ID + lease token + lock_epoch 的 CAS；旧 token、终态、错误 job ID 不能更新或释放当前任务。`installing/reconnecting` 进入 recovery barrier 后，即使 global lease 过期也禁止其他 job claim。
- idle gate 至少覆盖 `agent_tasks` active 状态、deployment/env/runtime probe 关系、terminal active 状态、有效 artifact/secret/env lease、Agent 心跳、协议/capability/架构和当前 connection generation。
- API 重启恢复：`downloading` 可在确认无 Agent 侧 active transaction 后回到 waiting；`installing/reconnecting` 一律进入 recovery barrier，由 Agent report、updater journal 查询或人工 recovery 得出唯一终态；过期 delivery lease 与普通部署恢复逻辑分开，不能无条件释放维护门禁。

**测试场景：**

1. 同 Agent/目标版本重复扫描只生成一个 job；不同目标版本生成独立 job。
2. 两个 worker 并发 claim 只有一个在最终 idle gate 事务中得到 global lease，另一个保持 queued/waiting；离线/忙碌候选不占用 global lease。
3. 同节点无法同时建立两个 maintenance lock；lock_epoch/token CAS 防止旧 worker 删除新 job 的门禁，升级成功/失败分别释放自己的锁，控制面崩溃恢复不会无条件释放。
4. active deployment、env sync、runtime probe、terminal session、过期 heartbeat 分别阻止 idle claim；全部完成后才允许 claim。
5. 旧 lease token 的 late update、重复 report、API 重启、数据库 busy/rollback、installing/reconnecting lease expiry 都不破坏唯一终态或全局串行。
6. migration 从空库和当前 schema 升级均通过，约束和索引通过数据库测试。

### U5. 调度器门禁、自动发现 worker 与 Agent WSS 生命周期

**目标：** 把升级流程接入控制面后台循环，同时防止普通任务绕过节点维护门禁。

**涉及文件：**

- `api/src/main.rs`
- `api/src/deployments/runtime.rs`
- `api/src/agents/dispatcher.rs`
- `api/src/agents/websocket.rs`
- `api/src/application_envs/mod.rs`
- `api/src/runtime_probe.rs`
- `api/src/nodes/mod.rs`
- `api/src/terminals/store.rs`
- `api/src/deployments/mod.rs`
- `api/src/lib.rs`
- `api/tests/agent_dispatcher.rs`
- `api/tests/agent_websocket.rs`
- `api/tests/deployment_runtime.rs`

**开发要点：**

- 将 upgrade worker 与部署 worker 放在同一 `shutdown` 生命周期中，但使用独立处理函数、周期和错误边界；升级 worker 不阻塞部署 worker 的状态收敛。
- 启动恢复后周期扫描当前 release，先处理已完成 bootstrap 的 Agent；非 amd64、协议不足或 capability 缺失只更新状态，不占用 global lease。
- 取得 job 后等待/确认当前连接 generation，建立 lock 后使用 `send_generation` 发送命令；Agent 断线进入 reconnecting，不立即失败。
- 收到 Ack/Progress/Report 时调用 U4 的 compare-and-set；在新 Hello 后根据 agent_version、protocol、capabilities 和 upgrade report 完成成功/失败收敛。
- 在实际 `try_dispatch`/任务发送核心和 terminal open 核心做最后一道门禁检查；在 deployment/env/probe/node-check 创建事务中再次检查 node lock。WSS Hello 完成后的 reconcile、`enqueue_pending_env_syncs_for_agent` 和 `dispatch_queued_for_agent` 必须先检查门禁，`installing/reconnecting` 期间只能保持 queued，不得因为 Agent 重连而绕过门禁。
- 门禁存在时，部署/Env/探测/节点检查/终端创建返回稳定 `node_maintenance` 或保持 queued，不能靠前端禁用绕过；所有拒绝/排队动作均不结束已有任务。
- 维护门禁不阻止已运行任务自然结束，不取消现有 deployment 或 terminal；成功/明确失败后触发 queued 任务重新调度。

**测试场景：**

1. 控制面启动、worker 周期、Agent 连接/重连完整流程能创建 job、发送 command、安装后确认并释放 lock。
2. 当前连接代次变化、发送失败、Agent 断线、ACK 拒绝、progress 回退和 report 重复均得到稳定收敛。
3. 普通 deployment、env sync、runtime probe、node check、terminal session 在 maintenance lock 存在时均不能绕过；已活动任务仍可完成。
4. API 重启后升级任务不被当成 deployment failure；新 Agent 重连、WSS Env 补偿和 queued dispatch 在门禁下保持阻塞，新 Agent report 后才能继续，旧 Agent 超时转 failed。
5. v16 Agent 能继续正常心跳/部署/终端路径，且永远不会收到 v17 upgrade message。

### U6. Agent/节点查询 API、管理员 WebSocket 和 OpenAPI

**目标：** 为管理端提供稳定的升级状态查询和变更通知契约。

**涉及文件：**

- `api/src/agents/mod.rs`
- `api/src/agents/upgrades.rs`
- `api/src/agents/auth.rs`
- `api/src/nodes/mod.rs`
- 新增 `api/src/nodes/status_websocket.rs` 或等价模块
- `api/src/lib.rs`
- `api/src/main.rs`（如需注册状态广播/恢复）
- `api/openapi/openapi.json`
- `api/tests/agent_releases.rs`
- `api/tests/nodes_api.rs`
- `api/tests/agent_upgrade_api.rs`
- `api/tests/node_status_websocket.rs`

**开发要点：**

- `AgentResponse` 和管理员可见的 `NodeResponse` 增加 optional `agent_upgrade`，列表/详情使用批量查询或一次受控映射，避免 N+1；普通用户只得到既有节点字段或脱敏状态，保留现有 `status`、协议、版本和架构字段语义。
- 增加管理员只读升级列表/详情 API（建议 `GET /api/v1/agent-upgrades`、`GET /api/v1/agent-upgrades/{job_id}`），以及管理员重试 API `POST /api/v1/agent-upgrades/{job_id}/retry`；重试带 CSRF、审计和版本/CAS 前置条件，只允许 `failed` 且重新生成 attempt/lease，禁止对 `installing`、`reconnecting` 或任意 `blocked_*` 任务重复安装。专用 DTO 严禁返回 authorization、lease token、staging 路径、下载 URL、原始 stderr、堆栈或配置内容，只返回稳定错误码和脱敏摘要。
- 新增管理员节点状态 WS 路由，例如 `/api/v1/nodes/status-stream`；握手必须检查 authenticated session、管理员 RBAC、精确 Origin 和 `deploy-go-node-status.v1, csrf.<token>` 子协议，禁止 token/query 认证。连接存活期间周期复核 session/RBAC，logout、会话撤销或权限降低时主动关闭，复用终端 WS 的生命周期规则。
- 建立服务端有界 broadcast；事件仅含 `boot_id`、`event_sequence`、`node_id`、`agent_id`、`node_version`、`change_type` 和 server timestamp。每次 API 启动生成 boot_id，snapshot marker 携带当前水位；事件断档/overflow/boot_id 变化时发送 resync marker。HTTP 节点快照始终保留低频轮询兜底，不能因 WS 正常就完全停止。
- OpenAPI 对所有新字段/路由标注可选性、管理员权限、错误码和分页边界；运行生成脚本更新客户端，不手改 `admin/src/api/generated/`。

**测试场景：**

1. 管理员可查列表/详情，普通用户不能读取升级任务或管理 WS；不存在 job 按既有不可见语义处理。
2. AgentResponse 无任务、latest、blocked、waiting、failed、installing 的 JSON 与 OpenAPI 一致；长摘要和未知枚举拒绝/截断。
3. WS 无 Cookie、非管理员、Origin 不匹配、CSRF 缺失/错误、错误子协议均拒绝；合法连接收到 snapshot marker 和后续事件，session/RBAC 失效时关闭。
4. 多客户端/同一客户端只产生预期连接；事件序号重复、断档、boot_id 变化、断线、重连、队列溢出都会触发快照刷新，不污染 HTTP 权威状态。
5. `make api-openapi`、`make api-client-generate`、OpenAPI contract 检查通过。

### U7. 管理端共享 WS、节点列表与详情状态展示

**目标：** 让管理员能在列表和详情中看到升级状态，且不引入第二套前端状态源。

**涉及文件：**

- `admin/src/api/contracts.ts`
- 新增 `admin/src/api/node-status-socket.ts`
- `admin/src/providers/` 或现有认证/应用根布局中的共享连接挂载位置
- `admin/src/features/nodes/NodesPage.tsx`
- `admin/src/features/nodes/NodeDetailPage.tsx`
- 节点状态徽标/详情组件及样式文件
- `admin/src/test/NodeStatusSocket.test.ts`
- `admin/src/test/AgentNodeManagement.test.tsx`
- `admin/src/test/AgentManagement.test.tsx`
- `admin/e2e/agent-node-management.spec.ts`
- `admin/e2e/responsive-layout.spec.ts`

**开发要点：**

- Authenticated 管理员在路由上层建立单一 `admin-node-status.v1` WS；退出登录、会话失效或应用卸载关闭连接。
- 收到合法 event 后防抖失效 `['nodes']`、`['agents','node-links']` 和当前 `['node', nodeId]`；WS 正常时可降低 HTTP 轮询频率但不能关闭，断线、会话失效、事件缺口或 overflow 时立即刷新快照并恢复兜底频率。
- 处理重连时先重新请求 HTTP 全量快照，再继续接受增量事件；对 event sequence 做去重，不直接 merge 未验证 payload。
- 列表和详情显示当前 Agent 版本、目标版本、状态、阶段、失败摘要/阻塞原因；升级字段缺失显示“升级状态暂不可用”，不能显示为 latest。
- 不把升级状态写入 localStorage/sessionStorage；遵循现有节点页面布局和中英文/中文文案约定，兼容移动端，不改业务应用页面。

**测试场景：**

1. 一个页面/列表与详情切换只复用一条 WS；登录退出正确建立/关闭。
2. 事件通知能刷新列表和当前详情，重复事件不重复刷新；当前详情切换后仍只刷新当前 query。
3. WS 断开时 HTTP 兜底继续工作，重连后先快照再事件；服务端错误/异常帧不会抛出未处理异常。
4. latest、waiting、installing、reconnecting、failed、blocked 文案和颜色可读，长错误不撑破卡片，移动端不重叠。
5. 现有节点创建、终端、遥测、环境筛选和权限测试不回归；E2E 覆盖列表/详情/响应式布局。

### U8. 发布物、版本升级、安装契约和 runbook

**目标：** 产出可被控制面同步和 Agent 下载的单架构成对版本，并记录 bootstrap、自动升级和回滚操作。

**涉及文件：**

- `api/Cargo.toml`
- `agent/Cargo.toml`
- `agent-executor/Cargo.toml`
- `deploy-go-deployer/Cargo.toml`
- `Cargo.toml`
- `agent/release/manifest.schema.json`
- `agent/release/generate-manifest.sh`
- `agent/release/test-generate-manifest.sh`
- `agent/install/deploy-go-agent-updater.service`
- `scripts/sync-agent-release.sh`
- `scripts/test-sync-agent-release.sh`
- `deploy/production/` 相关构建/安装契约脚本（仅需同步 amd64/v4 校验）
- `docs/runbooks/github-actions-release.md`
- `docs/runbooks/api-migrations.md`
- `docs/runbooks/agent-onboarding.md`
- `docs/runbooks/agent-recovery.md`
- `docs/runbooks/systemd-deployment-production.md`
- `docs/standards/agent-control-protocol.md`
- `docs/standards/agent-installation-contract.md`
- `docs/standards/privileged-agent-executor.md`

**开发要点：**

- 按项目当前补丁版本规则将发布版本提升到 0.3.7，API、Agent、executor、deployer 和新增 updater crate 保持一致；协议 v17、executor v4 和 Agent capability 写入 manifest。
- manifest schema/generator 升到 schema v4，包含 Agent/executor/updater 三个 Linux `x86_64` artifact、Agent/runner/executor/updater 四个 unit、executor config template 和 v17/v4 兼容范围；当前新发布不产生 ARM。历史 schema v3/ARM 目录与下载测试继续只读保留，不能成为自动升级候选。
- 新增 updater 的构建、安装、systemd 静态契约和发布同步校验；executor config template 只作为 bootstrap/schema 输入，自动升级不得覆盖节点绑定安全字段。明确 v3/v4、schema v3/v4、缺 updater 的混合配对拒绝矩阵。
- runbook 明确：先部署控制面并同步当前发布物，再人工 bootstrap 一台/逐台旧 Agent；bootstrap 完成后后续控制面版本走自动串行升级；说明查看状态、暂停/失败恢复和整对回滚。
- 正式控制面部署操作仍使用 `docs/runbooks/systemd-deployment-production.md`，不把业务节点升级和业务应用发布混在一起；本计划不自动触发真实节点升级。

**测试场景：**

1. 新 manifest 只有 x86_64、三个 artifact、四个 unit、文件摘要完整、版本/协议/组件一致；缺 ARM、缺 updater/executor/unit/config 或 v4 不兼容时检查失败，历史 v3/ARM fixture 仍能通过历史下载兼容测试。
2. 安装器静态契约、systemd unit、配对升级/回滚和旧版本恢复测试通过。
3. release sync 在 staging 目录原子替换，控制面启动能发现并拒绝不兼容 manifest。
4. runbook 能从旧版本 bootstrap 到 0.3.7，再验证第二次发布自动串行流程。

### U9. 集成验证、代码复核与正式控制面发布

**目标：** 在不触碰业务应用的前提下完成端到端本地验证、版本提交和用户已授权的正式控制面发布。

**涉及文件：**

- 本轮 U1-U8 变更文件
- `docs/reviews/2026-09-21-001-agent-auto-upgrade-review.md`（若保留复核结论）

**实施要点：**

- 先完成 Rust 协议、Agent、executor、API 单测和管理端测试，再执行发布物/安装契约和 OpenAPI 生成检查。
- 对照 AE1-AE9 记录证据；使用 mock/fixture/隔离本地服务验证升级，不连接业务节点，不修改业务仓库。
- 执行 `git diff --check`、`git diff --cached --check` 和只暂存本轮文件；提交前检查当前工作区既有脏文件未被混入。
- 通过项目代码复核关注协议兼容、授权绑定、迁移并发、门禁覆盖、WS 断线恢复、敏感信息泄露和前端布局。
- 正式控制面部署不属于本地代码 DoD；只有用户在当前对话明确授权具体环境后，才按 `docs/runbooks/systemd-deployment-production.md` 部署 API/Web。部署前确认 qfy-test2 是正式控制面而非业务目标节点，发布物已同步且 amd64 版本一致。部署后只验证控制面健康/ready、发布目录和新 API/WS，不自动向真实 Agent 下发升级。

**测试场景：**

1. 全量或可行聚焦验证通过并记录跳过项及原因；未安装 Docker 的本机不作为构建前提，使用项目既定 qfy-test2 构建/部署路径时另行遵循 runbook。
2. 正式控制面 `/healthz`、`/readyz`、Agent release manifest、节点列表/详情和状态 WS 路由可用。
3. 发现未 bootstrap 的真实节点时只显示阻塞状态，不执行自动安装；用户另行授权后才进行单节点 bootstrap/升级。

## Assumptions And Dependencies

- 正式控制面仍是单实例 SQLite 部署；`BEGIN IMMEDIATE` 能提供本期所需的全局串行语义。
- qfy-test2 能构建 Linux amd64 发布物并提供控制面版本化 HTTPS 下载路径；本机不安装 Docker。
- `release_signer`、Agent 安装发布目录和现有 root executor 配置已可复用，但 upgrade audience/claims 必须与特权 release 的 audience 和 nonce 隔离；现有 bootstrap 安装命令需要先安装包含 updater/v4 的完整配对。
- 现有 Agent WSS 连接已经持久化 protocol、capabilities、architecture 和 last_seen；若历史数据缺字段，统一显示状态不可用或需要 bootstrap，不猜测能力。
- WebSocket 状态广播是进程内最佳努力通知，带 boot_id/水位和 overflow marker；数据库 job/lock 和 HTTP 快照是唯一权威来源；本期不承诺跨 API 实例事件重放，但 HTTP 低频轮询始终保留。
- 自动升级成功后不改变业务应用的 Git 来源、部署契约、Env 文件或发布流程。

## Risks And Mitigations

| 风险 | 影响 | 缓解 |
|---|---|---|
| 旧 Agent 没有升级协议 | 无法自动完成首次升级 | 明确 bootstrap 阻塞状态，保留旧任务能力并在 runbook 给出人工配对安装 |
| Agent 在安装期间断线 | 控制面误判失败或重复安装 | 独立 upgrade-state、job/manifest 幂等、reconnecting 状态和新 Hello 报告确认 |
| executor 替换半套文件 | 节点无法重新连接 | 成对临时文件、静止点快照、服务顺序检查和整套回滚 |
| 门禁覆盖不全 | 升级期间新任务修改节点 | 抽取统一 helper，覆盖 dispatcher、deployment/env/probe/check/terminal 所有创建入口，并用测试矩阵证明 |
| manifest/授权摘要不一致 | 合法升级被拒绝或错误文件被安装 | 明确定义 canonical manifest digest，API/Agent/executor 共用 fixture，逐文件 SHA 校验 |
| WS 丢事件 | UI 状态过期 | WS 只做通知，重连先 HTTP 快照，断线轮询兜底，服务端事件有序号 |
| 新发布只支持 amd64，但历史 ARM 目录仍存在 | 自动升级误选或旧下载回归 | 自动升级候选严格要求 schema v4 + x86_64；历史 ARM 仅保留旧下载兼容，不删除既有 fixture |
| 用户已有脏改动被混入 | 破坏既有工作 | 每个提交只暂存本轮文件，提交前重新核对 `git status` 和 staged diff |

## Verification Contract

### Required checks

```bash
cargo fmt --all -- --check
cargo test -p deploy-go-agent-protocol
cargo test -p deploy-go-agent
cargo test -p deploy-go-agent-executor
cargo test -p deploy-go-api --test agent_upgrades
cargo test -p deploy-go-api --test agent_dispatcher
cargo test -p deploy-go-api --test agent_websocket
cargo test -p deploy-go-api --test nodes_api
cargo test -p deploy-go-deployer
make setup-git-hooks
make verify-git-hooks
make api-client-generate
make api-client-check
make migration-git-guard
make migration-git-guard-staged
make api-check
make api-openapi-check
make agent-manifest-check
make agent-install-check
make agent-runner-isolation-check
make agent-release-sync-check
make admin-check
make admin-test
git diff --check
git diff --cached --check
```

`make api-client-generate` 会更新生成客户端，必须在工作区干净边界内执行并随后运行 `make api-client-check`；不得手改 `admin/src/api/generated/`。新增 migration 前的 hooks 门禁、`migration-git-guard-staged` 和提交前检查不可跳过。上述核心检查如环境缺失导致无法执行，必须记录为阻塞而不是写成“聚焦通过”；前端正式 E2E 按 `AGENTS.md` 的浏览器策略使用项目依赖的 Playwright/隔离运行时，只有在代码影响到节点页面时执行，不连接用户浏览器。

### Quality gates

- v11-v16 Agent 的既有控制、部署、Env、终端和遥测测试无回归；v17 新能力没有任何旧协议未知字段泄露。
- executor 升级 operation 只接受固定 root-owned staging、绑定授权和 amd64 成对发布物；没有任意命令/路径/URL/环境变量。
- 数据库升级 job、global lease、maintenance lock 的并发、恢复和终态保护有测试证据。
- 所有任务创建/派发入口都尊重维护门禁；已有任务可自然完成，升级结束后队列可继续调度。
- API/OpenAPI/前端类型一致；旧页面和普通用户权限不因新增升级字段改变。
- 节点列表/详情在 WS 正常、断开、重连、缺字段、错误状态下均不会误报 latest 或覆盖在线/离线语义。
- `git diff --check`、staged diff 检查、代码复核、安装契约和发布物校验通过。
- 真实控制面部署只修改 Deploy Go 正式控制面服务；不修改业务应用代码、业务节点配置或业务仓库。

## Rollout And Recovery

1. 在当前工作区隔离既有未提交改动，完成 U1-U8 的本地实现和测试；协议、数据库、API 和前端分小步提交并推送 `main`，不创建额外功能分支。
2. 在 qfy-test2 按正式部署 runbook 构建/同步 Linux amd64 的新 API、Agent、runner、executor 和 deployer 发布物；本机不安装 Docker，也不构建 ARM。
3. 部署正式控制面并检查 `/healthz`、`/readyz`、migration、release manifest、节点 API、管理员 WS 和后台 worker 日志。
4. 旧 Agent 首轮保持在线但显示 `blocked_bootstrap_required`；由管理员逐台人工安装支持 v17/`agent_upgrade_v1` 的成对版本。每台 bootstrap 后确认在线、executor 健康、capability 和 `agent_upgrade` 状态。
5. 选择一个非关键节点验证后续版本自动串行：观察 global lease、maintenance lock、状态事件、Agent 重连和成功释放；确认一个节点未完成时第二个节点不进入 install。
6. 若下载或安装失败，优先查看升级 job/event、Agent 脱敏日志和 executor 状态；不手工执行任意安装命令，不删除业务工作目录。executor 失败按成对旧版本回滚，控制面 job 保留 failed 证据。
7. 若新 Agent 无法重连，保持节点维护门禁并按 agent recovery runbook 通过人工复核回滚/重新 bootstrap；只有确认旧/新配对健康后才释放门禁。

## Definition Of Done

- **D1.** 已登记 Agent 能按当前发布版本自动发现，job 幂等且 v16/旧 Agent 明确显示 bootstrap 阻塞。
- **D2.** 任意时刻最多一个 Agent 进入实际升级阶段，global lease、节点门禁、连接代次和 CAS 更新均有测试证明。
- **D3.** Agent 能从固定控制面路径下载并校验 amd64 完整配对发布物，root executor 能授权安装、重启、健康检查和整套回滚。
- **D4.** Agent 重启/控制面重启/WS 断线不产生错误失败或重复安装；成功、旧版本、摘要不符、超时等结果都有可读终态。
- **D5.** 部署、Env、探测、节点检查、终端和普通任务入口均尊重 maintenance lock，已有任务不会被升级强杀。
- **D6.** API、OpenAPI 和管理端节点列表/详情展示升级状态；共享 WS、HTTP 快照和轮询兜底均有测试。
- **D7.** 新版本、manifest、executor 协议和构建脚本只把 Linux amd64 作为自动升级候选；历史 ARM 下载兼容保持不变，发布校验和 runbook 已同步。
- **D8.** 所有代码、测试、文档和部署工具改动仅在 `deploy-go`；没有业务应用代码、配置、脚本、仓库或发布物改动。
- **D9.** 本地验证和代码复核通过；正式控制面部署作为后续明确授权的运行步骤，不自动执行未经单独授权的真实节点升级。

## Execution Status

- [x] U1 Agent v17 控制协议和 schema
- [x] U2 root executor v4 固定升级 operation
- [x] U3 Agent 下载、校验、状态恢复和重连报告
- [x] U4 SQLite 队列、全局租约和维护门禁
- [x] U5 调度器门禁、自动发现 worker 与 Agent WSS 生命周期
- [x] U6 Agent/节点查询 API、管理员 WebSocket 和 OpenAPI
- [x] U7 管理端共享 WS、节点列表与详情
- [x] U8 发布物、版本、安装契约和 runbook
- [x] U9 集成验证、代码复核与正式控制面发布（已部署 qfy-test2；真实节点 Agent 升级仍按节点逐台人工授权）

### 本轮执行记录

- 自动升级实现已按 U1-U8 分阶段提交；当前版本统一为 `0.3.7`，新发布只生成 Linux `x86_64`/amd64 v4 成对发布物。
- 增加下载租约过期收敛和管理员人工恢复 API；人工恢复不会复用旧安装请求，只释放当前任务的门禁并保留失败证据。
- 修复远程生产构建遗漏：Docker release image 和 `deploy/production/deploy.sh` 现在会构建、同步、校验 updater。
- 修复现场自动升级停在 `installing`：executor 不再把 updater 作为自身 cgroup 的子进程直接启动，改为通过 `systemctl --no-block start deploy-go-agent-updater.service` 启动独立 oneshot，避免 updater 停止 `KillMode=control-group` 的 executor 时被一并终止。
- 修复独立 updater unit 被系统只读隔离阻止安装：使用 `ProtectSystem=strict` 建立默认只读边界，仅通过 `ReadWritePaths` 开放配对二进制、unit、executor 配置及升级事务所需的固定目录。
- 已执行 Rust/API、Agent 组件、管理端类型检查与测试、OpenAPI/client 生成校验及安装契约检查；既有 `agent_websocket` 的旧 release fixture 版本缺口仍需在 U9 代码复核中记录或补齐。
- 已在 qfy-test2 使用 amd64 远程构建并部署正式控制面；`deploy-go-api`、`deploy-go-web` active，`/healthz`、`/readyz` 和 v4 Agent manifest 验收通过，未自动触发真实节点升级。
