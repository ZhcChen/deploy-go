---
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan
execution: code
title: External 部署启动前失败诊断能力扩展
date: 2026-09-18
---

## Goal Capsule

**Objective:** 外部部署调用方在部署快速失败、没有目标 `started_at` 或没有 stdout/stderr 时，也能从 External API 判断失败发生在任务生命周期的哪个阶段、由谁返回、对应哪个部署阶段，并获得可安全使用的错误码、摘要和退出码。

**Means:** 在现有部署详情和日志分页能力之上，增加统一的任务生命周期诊断投影，覆盖 `deployment_prepare`、`deployment_execute`、`deployment_release` 任务，并同步 CLI 的状态与诊断查询能力。

**Authority:** 当前用户要求与仓库 `AGENTS.md` 优先；运行和外部 API 语义遵循 `docs/standards/`、`docs/runbooks/`；已完成的 `docs/plans/2026-09-18-002-external-deployment-diagnostics-plan.md` 仅作为已实现基线，不重复实现或覆盖其结论。

**Stop conditions:** 诊断接口能够解释“ACK rejected”“未进入 Running 直接返回 TaskResult”“正常运行后失败”“prepare/release 失败”“控制面超时/恢复”六类状态；权限、敏感信息、响应大小和旧 External 响应兼容性验证通过。正式控制面部署不属于本计划，除非后续用户单独明确授权。

## Product Contract

### Summary

当前 External API 能返回部署汇总状态、部分退出码和日志，但无法解释以下常见结果：部署已失败，`failure_stage=null`、`failure_step=null`、`logs_available=false`、`target_run.started_at=null`。CLI 还丢弃了 API 已返回的诊断字段，且服务端详情查询只关联目标运行对应的一个任务，可能遗漏部署级 `prepare` 或未绑定目标运行的执行任务。

### Problem Frame

`started_at=null` 只能证明没有收到该任务的 `Running` 状态，不能区分 Agent ACK 拒绝、ACK 后 Running 前返回失败、Agent 断连或控制面恢复收敛。没有日志也不能证明没有任务失败，因为 Agent 可能在产生 stdout/stderr 前直接返回 `TaskResult`。调用方因此只能转到控制台或节点日志，无法使用部署 ID 完成第一轮定位。

### Requirements

- **R1 生命周期可见性：** External 诊断结果必须覆盖部署相关的 `prepare`、`execute`、`release` 任务，并区分任务种类、部署阶段、派发/确认/启动/完成时间和最后事件类型；不得要求目标运行一定存在。
- **R2 生命周期来源分类：** `origin` 是“当前任务生命周期事实来源”，不是任意内部状态或必然的失败原因。只允许 `queued`、`dispatching`、`agent_rejected`、`agent_started`、`agent_result`、`agent_timeout`、`control_plane_reconcile`、`unknown`；无法由事实证明时使用 `unknown`，不得把内部枚举直接透出。
- **R3 结构化错误摘要：** 返回经过 External 安全投影的 `error_code`、`summary`、`exit_code`、受控失败阶段/步骤（可空）和是否有日志；优先使用 Agent 明确结果，其次使用已持久化的失败步骤事件，再使用控制面收敛结果。`summary` 最多 512 个 Unicode 字符，去除控制字符和换行，统一脱敏 URL userinfo、命令行凭证、常见 secret/token/private-key、工作目录和节点路径；无法确认安全时返回固定摘要。`error_code` 只允许当前协议定义的安全错误码集合，未知值统一为 `agent_error`，并限制为小写 ASCII 字母、数字、下划线且不超过 64 字符。字段缺失时返回 `null`，不得用“脚本已开始”填充未知状态。
- **R4 新增诊断接口：** 提供 `GET /external/v1/deployments/{id}/diagnostics`，复用现有 External API Key 与应用归属校验，返回部署级诊断和各目标运行诊断；接口只读，不触发重试、恢复、取消或 Agent 通信。
- **R5 详情接口兼容增强：** 现有 `GET /external/v1/deployments/{id}` 可增加受控的 `execution`、`diagnostic` 字段，旧字段含义不变；不返回原始 `task_id`、`result_json`、事件载荷、节点连接 generation、内部连接 ID、任意原始事件名或文件路径。公开 `last_event_kind` 只允许 `dispatched`、`acknowledged`、`ack_rejected`、`started`、`result`、`timeout`、`reconciled`、`unknown`，其中 `ack_rejected` 是由任务状态和 ACK 结果派生的公开值，不是伪造的事件表记录；若无法证明则为 `null`。
- **R6 CLI 可用性：** `deploy-go-deployer status` 必须打印已存在的 `result_summary`、`error_code`、`exit_code`、目标运行时间、日志可用性和最后日志序号；新增 `diagnose` 查询结构化诊断，新增 `logs` 查询已有日志分页接口，并正确处理空日志和启动前失败。
- **R7 权限与错误语义：** 绑定应用的 Key 可查询；未绑定应用、其他应用的部署和不存在的部署保持现有外部不可见行为；非法游标、limit 和无效部署 ID 使用既有错误响应风格。
- **R8 不新增持久化协议：** 优先从现有 `agent_tasks`、`agent_task_events`、`deployment_logs`、`deployments`、`deployment_target_runs` 派生诊断，不修改历史 migration，不要求 Agent 协议新增字段。诊断首版只返回每个部署阶段/目标的当前有效任务，不返回完整重试历史；固定任务数量上限，超出时返回 `truncated=true`。

### Actors

- 外部项目发布工具：通过 External API 查询部署结果并决定是否继续排查。
- Deploy Go API：将内部任务状态和事件投影为稳定、安全的外部诊断契约。
- Agent/Runner：通过既有 ACK、TaskState、TaskResult 和 output 事件提供事实来源。
- 管理员：仍可通过控制面查看完整内部任务与节点日志；本计划不以 External API 替代管理面。

### Key Flows

1. 外部调用方创建部署后调用 `status`，获得汇总状态和基础诊断字段。
2. 部署失败或状态异常时调用 `diagnose`，按部署级任务和目标运行任务查看生命周期与 `origin`。
3. 若 `logs_available=true`，调用现有 `logs` 接口按游标读取已脱敏日志；若无日志，以诊断对象中的 TaskResult/ACK/超时来源完成第一轮定位。
4. 多目标部署中，`prepare` 只归属于部署级；`release` 归属于准确的目标运行；`execute` 按其真实绑定归属，不复制到每个目标。

### Acceptance Examples

| 场景 | 期望诊断 |
|---|---|
| ACK rejected，未进入 Running | `origin=agent_rejected`，有 `acknowledged_at`，`task_started_at=null`，保留白名单化后的 Agent `error_code`，`last_event_kind=ack_rejected` |
| ACK 后直接 TaskResult failed | `origin=agent_result`，`last_event_kind=result`，`task_started_at=null`，保留 `exit_code/error_code/summary` |
| 正常 Running 后脚本失败 | `origin=agent_result`，有 `task_started_at` 和日志游标，保留失败阶段/步骤 |
| prepare 失败 | 部署级任务可见，不能因为没有 `target_run_id` 而丢失 |
| release 失败 | 失败任务归属正确的 `target_run`，多目标时不污染其他目标 |
| Agent 断连或控制面恢复收敛 | `origin=agent_timeout` 或 `control_plane_reconcile`，不伪造脚本已开始 |

### Scope Boundaries

本计划包含 External API DTO/查询、OpenAPI、CLI、测试和 External API runbook 的对应章节。现有日志分页与日志存储作为依赖复用，不重做日志存储；摘要、错误码和事件投影必须使用独立的 External 安全投影，不假设现有日志脱敏可以覆盖结构化字段。不包含 Agent 二进制升级、节点日志采集、任意内部事件透传、完整重试历史、自动重试、控制面 UI 重构、数据库 migration、`docs/runbooks/deployment-recovery.md` 扩展或正式环境发布操作。

### Delivery Priority

- **P0：** U1 的任务归属、origin 确定性分类、安全投影和核心 diagnostics 接口；必须先完成并通过权限/敏感数据测试。
- **P1：** U2 的现有详情兼容增强和 OpenAPI 契约同步。
- **P2：** U3 CLI `diagnose`/`logs` 与 U4 External API runbook 扩展。P2 不得反向改变 P0 的诊断语义。

## Planning Contract

### Key Technical Decisions

- **KTD1：以 `agent_tasks` 为生命周期主索引，事件表为证据补充。** 诊断查询集合固定为 `kind IN ('deployment_prepare','deployment_execute','deployment_release')`，通过 `deployment_id` 找全量任务；`deployment_release` 还必须验证 `target_run_id` 属于同一 deployment。排除 `env_sync`、`git_refs_query` 等其他任务。`deployment_execute` 遵循现有 schema 的 `stage=null,target_run_id=null`，公开层用 `task_kind`/`execution_phase=execute` 表达执行阶段，不伪造 `stage=execute`。
- **KTD2：新增 diagnostics 投影，日志接口职责保持不变。** diagnostics 负责“发生了什么以及在哪一步”；`/logs` 负责已持久化 stdout/stderr，两者不能互相替代。
- **KTD3：无 migration 的派生实现。** 所有新增字段从现有任务、事件、部署和日志记录实时派生，避免旧数据回填与线上 schema 变更；查询失败按现有 API 错误处理，不返回半可信的原始内部数据。
- **KTD4：内部细节白名单化。** 仅暴露稳定的任务 kind、公开阶段、时间、公开事件类别和安全摘要；任务 ID、原始 JSON、连接 generation、节点工作目录、凭证相关载荷、任意内部事件名和任意未验证字符串永不进入 External 响应。公开摘要、错误码、阶段/步骤共用 External 投影策略，不能直接复用日志脱敏函数后就视为安全。
- **KTD5：兼容优先。** 详情接口仅追加可选对象；新增 diagnostics 和 CLI 命令不改变既有 status/logs 路径，CLI 默认人类可读输出保留，`--json` 输出完整受控 JSON。

### High-Level Technical Design

```text
Agent ACK / TaskState / TaskResult / Output
             |
             v
 agent_tasks + agent_task_events + deployment_logs
             |
             +--> 任务归属解析
             |      prepare: deployment_id + stage
             |      execute: deployment_id only (existing single-target task)
             |      release: target_run_id + stage
             |
             +--> 生命周期 origin 分类器
             |      ack -> running -> result/timeout/reconcile
             |
             +--> External 安全投影
                    status detail / diagnostics / logs
```

任务选择规则必须按部署 ID 找出允许集合中的任务，再按 `kind/stage/target_run_id/created_at/id` 稳定排序；首版只返回每个部署阶段/目标的当前有效任务，最多返回固定数量，超出时以 `truncated=true` 表示，不宣称提供完整重试历史。`deployment_prepare` 的公开 stage 为 `prepare`，`deployment_release` 为 `release`，旧单目标 `deployment_execute` 的公开 execution phase 为 `execute`。非法或历史脏的 target 关联不得静默归属某个目标，降级为部署级 `unknown`。诊断分类器应有单一实现，详情接口和 diagnostics 接口复用，确保两处不会给出相互矛盾的 origin。

来源判定采用确定性优先级：

1. 有明确 ACK rejected 结果且无 Running：`agent_rejected`，公开 `last_event_kind=ack_rejected`（派生值）。
2. 有 TaskResult：`agent_result`；即使没有 Running，也不能改称 `agent_started`。
3. 有 Running/started_at：`agent_started`；它表示已经启动，不等价于失败。
4. 有明确超时证据（任务 deadline 已过且任务仍处于 active，或持久化 timeout 结果）：`agent_timeout`。
5. 有明确控制面恢复/重算证据导致的终态：`control_plane_reconcile`。
6. 仍在队列/派发：分别为 `queued`/`dispatching`；其余为 `unknown`。

若多个证据同时存在，以终态事实优先：明确 TaskResult/ACK 结果优先于 Running，明确 timeout/reconcile 只在没有更具体的 Agent 结果时生效；仅凭 `status=interrupted` 不得推断 `control_plane_reconcile`。同一 fixture 重复查询必须输出稳定结果。

建议的受控响应形状：

```json
{
  "deployment_id": "deployment_...",
  "status": "failed",
  "phase": "failed",
  "diagnostic": {
    "origin": "agent_result",
    "error_code": "process_exited",
    "summary": "部署任务返回失败",
    "exit_code": 1,
    "execution_phase": "execute",
    "step": null,
    "logs_available": false
  },
  "tasks": [
    {
      "kind": "deployment_execute",
      "execution_phase": "execute",
      "status": "failed",
      "delivered_at": "...",
      "acknowledged_at": "...",
      "task_started_at": null,
      "task_finished_at": "...",
      "last_event_kind": "result",
      "diagnostic": { "origin": "agent_result", "error_code": "process_exited", "exit_code": 1 }
    }
  ],
  "target_runs": []
}
```

具体字段名称可根据现有 DTO 命名调整，但不得降低 R2/R4/R5 的边界。时间字段必须保持现有 RFC3339 序列化；摘要、错误码、公开事件类别和阶段/步骤必须遵守 KTD4 的白名单与长度限制。诊断响应必须有固定最大任务数和 `truncated` 字段，不允许把所有历史任务或事件原文作为默认响应。

### Assumptions and Dependencies

- 现有 TaskState、TaskResult 和 output 已按既有协议持久化到事件/日志表；ACK rejected 的事实由 `agent_tasks.status/result_json/acknowledged_at` 保存，且 `agent_tasks` 已有 `started_at`、`finished_at` 等字段。
- 现有 External API Key 归属校验可复用，不增加新的认证方式。
- `result_json` 的解析必须依据当前 Agent 协议类型定义，不能在 External 层通过任意 JSON 字段猜测并透传。
- 若某类控制面失败没有可识别事件，分类为 `unknown` 或 `control_plane_reconcile`，并在测试中固定其降级行为。

## Implementation Units

### U1：抽取任务生命周期与诊断分类模型

**Goal:** 在 `api/src/external/mod.rs` 或相邻 External 专用模块中建立内部查询行、任务归属、生命周期和 origin 分类模型，统一详情与 diagnostics 使用的推导逻辑。

**Requirements:** R1、R2、R3、R8；KTD1、KTD3、KTD4。

**Files:** `api/src/external/mod.rs`；必要时复用 `api/src/agents/dispatcher.rs` 的协议解析类型，不修改 migration。

**Approach:** 查询指定 deployment 的全部相关 `agent_tasks`，同时取任务状态时间、stage/kind、result 摘要所需的安全字段、最近事件 kind 和日志存在性。按任务绑定关系归属 deployment/target run；将 ACK rejected、result-before-running、running-after-ack、超时/恢复分别映射到稳定 origin。将原始 payload 解析限制在内部结构体和白名单字段，解析失败时返回可控的 `unknown`。

**Test scenarios:**

- happy path：accepted -> running -> result succeeded/failed 的任务输出完整时间线和 `agent_started/agent_result`。
- edge：ACK 后直接 result、无事件、无 result_json、prepare 无 target_run_id、旧单目标 execute 的 `stage=null/target_run_id=null`、非法 target 关联；覆盖 `queued`、`dispatching`、`agent_started` 三个非终态来源。
- failure：ACK rejected、TaskResult-before-Running、deadline 超时、显式 reconcile 终态分别断言 `agent_rejected`、`agent_result`、`agent_timeout`、`control_plane_reconcile`；仅 `interrupted` 断言为 `unknown`。
- error：非法/损坏 JSON 不导致 External 500，不把原始 JSON 放入响应。
- integration：多目标 prepare/execute/release 任务分别正确归属。

**Verification:** 先添加/扩展 API 测试 fixture，再运行 `cargo test -p deploy-go-api --test external_api` 与相关 dispatcher/deployment protocol 测试。

### U2：新增 External diagnostics 接口并增强详情响应

**Goal:** 对外提供按部署 ID 查询的结构化诊断，且让现有部署详情直接暴露最小必要的执行摘要。

**Requirements:** R3、R4、R5、R7；KTD2、KTD4、KTD5。

**Files:** `api/src/external/mod.rs`、`api/openapi/external.json`、`api/tests/external_api.rs`、`api/tests/external_openapi_contract.rs`。

**Approach:** 注册 `GET /external/v1/deployments/{id}/diagnostics`，复用现有部署所属应用与 Key 校验。返回 deployment-level diagnostic、task summaries 和 target run summaries；仅提供安全字段。现有详情接口复用 U1 的 summary 映射，追加可选 `execution`/`diagnostic`，不改变已有字段和错误语义。新增 OpenAPI schema、路径、安全声明和 examples/字段约束。

**Test scenarios:**

- happy path：成功、运行中、失败部署均返回 200，字段可空且结构稳定；六类终态/启动前来源均有 fixture 断言。
- edge：无日志失败、失败前无 target started、prepare-only 失败、release-only 失败、多个目标和重复/历史脏任务选择。
- error/security：未绑定应用、撤销 Key、inactive application、deployment/application 与 target application 不一致、异常任务归属均保持不可见；无效部署、畸形数据、超长摘要和未知错误码不会泄露内部字段。
- integration：新增路径在 OpenAPI 中可发现，原有详情和 logs 接口仍通过契约测试。

**Verification:** `cargo test -p deploy-go-api --test external_api`、`cargo test -p deploy-go-api --test external_openapi_contract`、`make api-external-openapi`、`make api-external-openapi-check`、`git diff --check`。

### U3：完善 deploy-go-deployer CLI 查询闭环

**Goal:** 使 CLI 能直接展示和消费服务端诊断，不再因人类可读格式丢弃关键字段。

**Requirements:** R6、R7；KTD5。

**Files:** `deploy-go-deployer/src/main.rs`、CLI 相关测试或 fixture 文件（按现有项目测试布局）。

**Approach:** 保持现有 `Status` 命令与 `--json` 行为，补充部署级和目标级 `result_summary/error_code/exit_code/logs_available/last_log_sequence/started_at`。新增 `Diagnose { deployment_id }` 调用 diagnostics；新增 `Logs { deployment_id, after, limit }` 调用现有 logs API，打印序列、公开 stage、stream、时间和内容，空结果明确可读。为避免临时引入新的 HTTP 测试框架，先抽取路径/query 构造和响应格式化为纯函数，单元测试覆盖命令到 URL/参数的映射；CLI 继续复用现有 request/Authorization 实现，必要的端到端请求验证由 API 集成测试承担。API 错误沿用统一 request 错误处理，不显示密钥或内部任务载荷。

**Test scenarios:**

- happy path：status、diagnose、logs 分别解析成功响应并输出关键字段。
- edge：`target_runs=[]`、空日志、`started_at=null`、分页 `next_after`、终态 `terminal=true`。
- error：API 返回 401/404/422、诊断响应字段新增或为空时 CLI 不 panic；`--json` 保留服务端 JSON。
- integration：纯函数断言路径、query 参数和输出格式；Authorization 行为沿用现有 request 测试/代码路径，不为本计划引入新的 HTTP mock 依赖。

**Verification:** 运行 CLI crate 的现有测试/`cargo test -p deploy-go-deployer`，并执行 status/diagnose/logs 的纯函数请求构造与输出测试；不引入真实节点依赖或新的 HTTP mock 框架。

### U4：安全、契约、运行手册与最终复核

**Goal:** 固化外部诊断的安全边界和排障用法，避免实现后仍需要登录管理面才能理解接口。

**Requirements:** R4、R7、R8。

**Files:** `docs/runbooks/external-deploy-api.md`、`api/openapi/external.json`、`docs/reviews/`（如保留复核记录）。

**Approach:** 在外部 API runbook 中增加 status -> diagnostics -> logs 的排障顺序、字段解释、空日志含义、权限边界和示例响应；明确 diagnostics 不能替代管理面/节点日志。执行代码审查关注信息泄露、跨应用访问、错误降级和多目标归属；仅保留本轮相关文件。

**Test scenarios:**

- 安全：典型 token、Bearer、密码键、私钥块、命令行参数和节点路径不会出现在 diagnostics 或 logs 外部响应。
- 契约：OpenAPI schema 与 Rust DTO 一致，新增字段为兼容追加。
- 运维：根据部署 ID 可按文档完成快速失败首轮判断；无日志时文档明确转向结构化诊断和管理面日志。

**Verification:** 运行 External API/OpenAPI 聚焦测试、`cargo fmt --check`、`git diff --check`；按项目规则执行 `make verify-git-hooks`（如本地 hook 已配置），并完成 `$ce-code-review` 或等效复核。不得部署正式控制面作为本计划的自动收尾动作。

## Verification Contract

### Required commands

```bash
cargo fmt --all -- --check
cargo test -p deploy-go-api --test external_api
cargo test -p deploy-go-api --test external_openapi_contract
cargo test -p deploy-go-api --test deploy_event_protocol
cargo test -p deploy-go-api --test agent_dispatcher
cargo test -p deploy-go-deployer
make api-external-openapi
make api-external-openapi-check
git diff --check
```

若某个测试目标在仓库当前构建配置中不存在，应记录实际替代命令和原因，不得把未执行写成通过。测试 fixture 必须覆盖 Acceptance Examples 全部六类，并补充越权与敏感字段断言。

### Quality gates

- `ExternalDeployment` 旧字段序列化和既有 status/logs 行为不回归；新增 diagnostics 路径出现在固定 OpenAPI 路径集合中。
- diagnostics 不暴露原始 Agent 事件、任务 ID、连接信息、路径或凭证。
- 多目标部署的 prepare、execute、release 归属可由断言证明，且遵守现有 `stage`/`target_run_id` schema 约束；六类来源和 `interrupted -> unknown` 可由断言证明。
- CLI status 不再丢弃服务端已有诊断字段，diagnose/logs 可独立运行。
- 没有 migration、没有 Agent 协议破坏性变更、没有真实节点操作。

## Definition of Done

- U1-U4 的代码、契约、测试和文档均完成，且实现严格对应 R1-R8。
- 六类启动前/启动后失败场景均有可重复测试，尤其是 `started_at=null + logs_available=false + exit_code=1`。
- External API Key 的绑定、越权和不存在语义保持一致；敏感信息测试通过。
- 相关格式化、聚焦测试、OpenAPI 生成/契约检查和差异检查通过。
- 现有工作区无关改动未被回滚、格式化覆盖、暂存或提交；本计划本身作为本轮规划产物保留。
- 不遗留试验性查询、重复分类器、未使用 DTO 或无文档的隐藏 CLI 行为。

## Appendix

### 研究依据

- `api/src/external/mod.rs` 当前详情查询按 `target_run_id` 选择任务，已实现的日志接口只查询 `deployment_logs`。
- `api/src/agents/dispatcher.rs` 当前在 ACK rejected、TaskState Running、TaskResult 和终态收敛路径分别更新任务/部署状态；这些路径已提供本计划所需的事实来源。
- `deploy-go-deployer/src/main.rs` 当前 `status` 仅打印部署状态、phase 和 target runs 表格，未打印服务端已有的结构化失败字段。
- `docs/plans/2026-09-18-002-external-deployment-diagnostics-plan.md` 已完成详情失败字段、日志分页和日志脱敏；本计划是其后续补强，不替代其实现记录。

### 复核结论

- 已排除“只增加日志接口”作为完整修复：启动前失败可能没有 stdout/stderr。
- 已排除“仅依赖 `target_run.started_at`”作为失败判断：该字段不区分 ACK rejected、result-before-running 和断连。
- 已排除直接开放 `agent_task_events.payload_json/result_json`：会形成内部协议耦合和敏感信息泄露面。
- 已将部署级任务纳入范围，修正原计划只按 target run 关联的覆盖缺口。
