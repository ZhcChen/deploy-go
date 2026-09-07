---
title: 应用列表运行状态异步探测实施计划
date: 2026-09-07
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: 会话确认（D1-D3 推荐方向）
execution: code
---

# 应用列表运行状态异步探测实施计划

## Goal Capsule

应用列表的「运行状态」目前由最近一次 `deployments.status` 推导，只能反映
最近一次发布结果，不能反映服务当前是否可达。目标是改为：列表首次加载只返回
最近已知状态，随后前端对当前页应用并发发起异步运行时探测；探测完成后列表
无刷新更新为真实运行状态。

## 现状与证据

1. `api/src/applications/mod.rs::ApplicationRow::into_response` 使用
   `latest_deployment_status` / `latest_deployment_finished_at` 推导
   `runtime_state` / `runtime_checked_at`：
   - `succeeded -> running`
   - `failed -> failed`
   - `queued/running/canceling -> checking`
   - 无部署 -> `unknown`
   - 归档 -> `archived`
2. `application_runtime_statuses` 表与 `agent_tasks.runtime_status_id`
   仍保留在 `api/migrations/0022_application_manifest_and_runtime_status.sql`，
   但代码已随 `30d3842` 移除，当前未被读取。
3. `verification_config` 支持 `http` / `tcp` / `command` 三种形状，字段在
   `api/src/execution_spec.rs::validate_verification_config` 校验，当前只做
   形状校验，实际健康检查由业务 release 脚本负责并回传
   `deploy.verification.*` 事件（见
   `docs/standards/application-deployment-json.md`）。
4. 关键语义缺口：
   - `http` 类型没有 `host` / `port`，平台无法知道探测哪个端口。
   - `command` 的 `path` 必须位于节点 `work_root` 内，且平台不允许任意
     shell；由控制面自行执行会破坏现有安全边界。
   - `qfy-voucher-hub` 这类业务应用是一个应用一次发布多个模块
     （worker/api/3 个 Web），单个应用级 `verification_config` 不足以表达
     所有模块的当前状态。
5. 已有 Agent 控制协议为 v14，任务 payload 无 runtime probe 类型；executor
   本机协议为 v3。历史 v9 RuntimeStatusProbe 与 executor v3 RuntimeStatus
   已删除，可参考实现提交：`116bb75`、`b157f99`、`42ebabf`。

## 关键决策（已确认）

### D1：HTTP 探测的端口来源

候选：

1. `verification_config.http` 增加可选 `port` 字段；优先使用该值。
2. 未填 `port` 时使用目标 `image_spec.host_port`（只覆盖镜像直连应用）。
3. 两者都缺失时不做真实 HTTP 探测，回退到最近部署验证状态并显示
   `未配置探测端口`。

已确认：D1 = 1 + 2 + 3。要求新增配置时能显式填 `port`，历史配置不受破坏。

### D2：探测能力放在 Agent 普通任务层还是 executor 层

HTTP/TCP 探测只访问 `127.0.0.1:<port>`，无需 root：

已确认：Agent 普通任务层新增结构化 `runtime_probe` 任务，不新增 executor root
operation。能力通过 Agent 控制协议 v15 的 `runtime_probe_v1` capability
声明，未升级 Agent 不接收该任务。

`command` 探测首版不开放；仍以最近部署验证事件作为该类型应用的状态来源。
以后若要支持 command，应单独评审并走固定 release-check 授权，而不是给普通
任务开放任意命令。

### D3：多模块应用的语义

单个 Deploy Go 应用若一次部署多个模块，应用列表的「运行状态」应表达什么？

候选：

1. 应用级「最近一次发布的全部模块验证结果」：不新增真实探测，只异步刷新
   最近验证事件汇总。
2. 平台级探测只适用于单目标/单入口应用；多模块应用先保持最近发布验证结果，
   后续由业务 `deploy-go.yaml` 提供模块级 health check 配置再逐模块探测。
3. 多模块应用拆成多个应用后各自独立探测。

已确认：D3 = 2。避免用单个 verification_config 冒充整个业务系统状态。

## 实施单元（在 D1-D3 确认后按顺序执行）

### U1. 验证配置补丁与文档（已完成）

- `execution_spec::validate_verification_config` 对 `http` 允许可选 `port`；
  port 必须 1-65535。
- `docs/standards/application-deployment-json.md` 增加 `port` 字段说明和
  多模块边界。
- 单元测试覆盖缺省 port、越界 port 与 image_spec 回填逻辑。

### U2. Agent 控制协议 v15 runtime_probe（已完成）

- `agent-protocol`：
  - `PROTOCOL_VERSION = 15`
  - `AgentCapability::RuntimeProbeV1`
  - `TaskPayload::RuntimeProbe(RuntimeProbeTask)`
  - payload 只携带 runtime_status_id、probe 类型、`127.0.0.1` 固定目标、
    port/path/expected_status/timeout。
- 新增不可变 `agent-control-v14.schema.json` 快照，latest schema 升 v15；
  双向 schema/version 测试同步。
- Agent：
  - main 声明 `runtime_probe_v1`；
  - task_handler 新增结构化 probe 执行（HTTP GET / TCP connect），只允许
    localhost，拒绝任意 host、URL、命令与 env；
  - journal/ack/state/result 复用现有任务状态机。

### U3. API 状态表与批量探测入口（已完成）

- 读取 `application_runtime_statuses` 最新记录作为 `runtime_state`，
  缺失时回退最近部署推导。
- 新增批量 POST：
  `POST /api/v1/applications/runtime-probes`
  - body：`application_ids`（上限当前页，建议 20）
  - 对每个有权访问的应用/启用目标创建 pending 记录并去重
  - 目标离线、Agent 无 capability 等不可达情况收敛为 `failed` 并保存原因
- dispatcher 在 `try_dispatch` / Agent 重连路径中加入 runtime probe 能力门禁；
  stale pending 超时收敛。
- OpenAPI、Web/Flutter client、测试同步。

### U4. Admin 列表异步刷新（已完成）

- 列表加载后对当前页应用调用批量探测入口，显示「检测中」。
- 使用轮询或短延迟 refetch 更新 `runtime_state` / `runtime_checked_at`；
  不阻塞首屏列表。
- Tooltip 区分「部署验证时间」与「平台运行探测时间」。
- 测试覆盖首屏、探测中、成功、失败、Agent 离线与归档不探测。

### U5. 文档、复核与门禁（已完成）

- 更新 `agent-control-protocol.md`、`privileged-agent-executor.md`（如无
  executor 变更仅澄清边界）、`application-deployment-json.md`、
  `api-migrations.md` 与 `application-onboarding.md`。
- 执行聚焦测试、`make api-openapi-check`、`make api-client-check`、
  admin 测试与 `make check`。
- 复核记录写入 `docs/reviews/2026-09-07-application-runtime-status-async-probe-review.md`。

## Verification Contract

| Gate | Command |
|---|---|
| 协议 | `cargo test -p deploy-go-agent-protocol` |
| Agent | `cargo test -p deploy-go-agent` |
| API | `cargo test -p deploy-go-api --test migrations --test applications_api --test runtime_probe_api --test agent_dispatcher --test openapi_contract` |
| OpenAPI/client | `make api-openapi-check && make api-client-check` |
| Admin | `npm run check --workspace deploy-go-admin` |
| 全量 | `make check` |
| Diff | `git diff --check` |

## Definition of Done

- 应用列表「运行状态」不再只由最近部署状态推导；列表加载后会并发发起异步
  真实探测并回填结果。
- 探测只访问 `127.0.0.1` 固定地址，不引入任意 URL/SSRF/命令执行。
- 未升级 Agent、离线节点、目标不可用、配置缺端口均有稳定展示状态与原因。
- 历史协议 schema 不可变；v15 新能力通过 capability 门禁，不影响现有
  v11-v14 Agent 的部署/PTY/Env 链路。
- 不连接或修改任何真实节点；本地实现、测试与文档完成后再由用户授权验收。
