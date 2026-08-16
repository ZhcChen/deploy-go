---
artifact_contract: "ce-handoff/v1"
created_at: "2026-08-14T15:00:00+08:00"
title: "生产 Agent 凭证轮换卡死与离线部署挂起：根因与待修复项"
summary: "卡券系统正式环境 Deploy Go 部署第二阶段未执行，根因是生产 Agent 凭证轮换卡死导致节点离线；平台无告警且部署一直 pending。已定位到 Agent 与服务端两处缺陷，待本地修复和补测。"
keywords: ["deploy-go", "agent", "credential-rotation", "offline", "two-stage", "handoff"]
cwd: "/Users/chen/code/deploy-go"
resume_focus: "按本文第 4 节实现 Agent 凭证恢复、服务端 load_rotation 过期处理、离线部署提示/失败化三项修复并补测试；不要触碰卡券系统正式环境"
repository: "ZhcChen/deploy-go"
branch: "main"
head: "30d3842"
worktree_path: "/Users/chen/code/deploy-go"
---

# 交接：生产 Agent 凭证轮换卡死与离线部署挂起

## 0. 背景与用户约束

用户正在 Deploy Go 上验收卡券系统正式环境部署，观察到“第二阶段（release）没有执行”。
当前用户明确要求：

- **暂时不碰卡券系统正式环境**，不对 `qfy-prod-1` 业务容器、Caddy、Deploy Go 控制面
  做任何运行态修改。
- 接下来把任务交给另一个 AI：优先在 `deploy-go` 仓库内修复代码缺陷并补测试。
- 只有用户再次明确授权后，才能执行远程恢复、部署或切流动作。

上一轮已完成的服务恢复（手动发布 `20260814055912421`，commit `aff48f8a`）仍在线，
worker/api/admin/merchant-admin/consumer-web 5 个模块均 healthy。

## 1. 当前状态

- 仓库：`/Users/chen/code/deploy-go`，分支 `main`，HEAD `30d3842`，工作区干净。
- 问题部署：
  - deployment_id：`deployment_01KZZFRH5Y1GZ1E8P6P5H0J9YX`
  - commit：`d5cd891d3ebe41f7551e36f78f28c0df0fd4d658`
  - release_version：`20260814063610252`
  - prepare task：`succeeded`（06:36:53Z）
  - target_run：`pending`，phase `pending`，没有 release task
  - artifact：`verified`，`expires_at=2026-08-14T07:06:45Z`
- 生产节点/Agent：
  - node：`node_01KZBRNWV9QNS6DJAY0EPNZTEH`（生产节点01），status `offline`
  - agent：`agent_01KZBRNWV91V6RJT2D37SSGHXS`，`last_seen_at=2026-08-14T04:07:00Z`
  - `qfy-prod-1` 上 `deploy-go-agent.service` 已 `systemctl restart`，PID `1554783`，
    服务 active，但控制连接仍无法恢复。

## 2. 根因

第二阶段没执行不是业务脚本问题，而是**目标节点 Agent 离线**：调度只会在节点
`online` 且 Agent 未撤销时创建 release task，离线时部署一直 pending。

Agent 离线原因是**凭证轮换卡死**：

1. `04:08` 左右 Agent 发起 token rotation；rotation 已创建，但 access token 在 WSS
   确认前过期。
2. 之后 Agent 每次 refresh，服务端 `load_rotation` 都返回同一把已过期 access token；
   Agent 用过期 token 连 WSS 收到 401，静默重试，心跳发不出去。
3. 因此 `agents.last_seen_at` 停在 `04:07`，节点显示 offline，release 永远不入队。

### 控制面数据库关键证据（qfy-test）

```text
agent_refresh_credentials（agent_01KZBRNWV91V6RJT2D37SSGHXS）
gen 480 refresh_01KZZ6TFWA01Q53FQSJ628QSVH
  rotation_id=rotation_01KZZ79W6F4GNE99WAH986HFYH
  replaced_by_id=refresh_01KZZ79W7XBSJAFKY5MC326ZHZ
  committed_at=NULL revoked_at=NULL
gen 481 refresh_01KZZ79W7XBSJAFKY5MC326ZHZ
  rotation_id=NULL replaced_by_id=NULL committed_at=NULL revoked_at=NULL

agent_access_sessions
access_01KZZ79W7XNZ1APDEZEN3BPYJX
  refresh_credential_id=refresh_01KZZ79W7XBSJAFKY5MC326ZHZ
  expires_at=2026-08-14T04:38:24Z（已过期）
```

`qfy-prod-1` 的 `/var/lib/deploy-go-agent/credentials.json`：

```text
pending_rotation.rotation_id=rotation_01KZZ79W6F4GNE99WAH986HFYH
pending_rotation.next_refresh_token=存在（对应 gen 481）
pending_rotation.access_token=不存在
```

## 3. 排障期间做过的动作（记录，勿重复）

- `qfy-prod-1` 重启过 `deploy-go-agent`，未恢复。
- 用 gen 481 successor token + 新 rotation_id 做过一次 refresh 验证，控制面因此又创建
  了一组新轮换（gen 482 方向）。这不影响卡券系统业务，但**恢复生产 Agent 时必须先按
  第 6 节处理这个状态**，不能简单把 `pending_rotation` 清掉后复用旧 token。
- 未修改任何业务容器、Caddy、Deploy Go API 代码或数据库。

## 4. 待修复项（本次交接的核心）

### 4.1 Agent 端：凭证轮换自动恢复

文件：`agent/src/token_refresh.rs`，重点 `CredentialAccessProvider::prepare()`。

问题：pending rotation 的 access token 已过期（或缺失）但 `next_refresh_token` 已存在时，
当前逻辑仍用旧 refresh token + 旧 rotation_id 去 refresh，服务端只会返回过期 token，
形成死循环；如果换新 rotation_id 复用已 superseded 的旧 token，服务端会按
`refresh_token_reuse` 吊销整个 credential family。

期望行为：

- `cached_pending_access()` 返回不可用（过期/缺失）时，如果 pending rotation 已持有
  `next_refresh_token`，先执行一次**本地 commit**：把 `refresh_token` 切换为
  `next_refresh_token`、清空 `pending_rotation` 并原子落盘；
- 然后按正常流程创建新 pending rotation，用新 refresh token 发起新 rotation；
- 不得用旧 refresh token 重试，不得静默失败；
- 需要配套单元测试覆盖：过期 pending access + successor token、缺失 access token、
  无 successor token（保持现状）、commit/store 失败路径。

### 4.2 服务端：`load_rotation` 不应返回已过期 access token

文件：`api/src/agents/auth.rs`，`load_rotation()`。

问题：pending rotation 的 access session 已过期时仍原样返回，导致 Agent 无限 401。

期望行为（建议先做最小修复）：

- `load_rotation` 查询时要求 `access.expires_at > now`；
- 过期时返回明确的 `401`/稳定错误码（例如 `expired_pending_rotation`），**不要**
  走 `revoke_family_for_reuse`（否则会把合法 family 吊销）；
- 与 4.1 配合：新 Agent 逻辑会先本地采用 successor token，再开启新 rotation，因此
  服务端只需拒绝过期 pending access，不需要替 Agent 做状态机推进。
- 需要 API 集成/单元测试覆盖：过期 pending rotation 不吊销 family、不返回过期 token。

### 4.3 平台调度/详情：离线 Agent 不应让部署无限 pending

相关文件：

- `api/src/agents/dispatcher.rs`
  - `enqueue_deployment()`：查询强制 `node.status='online'`，离线直接不创建 release。
  - `dispatch_next_deployment()` / `ensure_deployment_task()`：two_stage 调度同样只在
    online 时推进。
- `api/src/deployments/mod.rs`：部署创建与 `release` 阶段状态机。

期望行为（二选一或组合）：

- 至少：目标节点 Agent 离线且部署等待 release 时，在部署详情/事件里产生明确提示
  （如 `agent_offline` / “发布阶段等待节点 Agent 上线”），不要再表现为“第二阶段没执行”。
- 可选：超过一定等待时间后把 target_run/deployment 失败化（`agent_offline`），并支持
  Agent 恢复后重试，而不是永远 pending。
- 需要聚焦测试：两阶段部署在 prepare 成功后目标 Agent 离线时不会丢失状态；Agent 恢复
  后可继续 release；超时失败路径可重试。

## 5. 建议实施顺序与验证

1. 先实现 4.1 Agent 端，补 `deploy-go-agent` 单测。
2. 再实现 4.2 服务端，补 `deploy-go-api` 测试。
3. 最后实现 4.3 调度提示/失败化，补 dispatcher 聚焦测试。
4. 本地验证：
   - `cargo fmt --all --check`
   - `cargo clippy -p deploy-go-api --all-targets -- -D warnings`
   - `cargo clippy -p deploy-go-agent --all-targets -- -D warnings`
   - `cargo test -p deploy-go-agent-protocol -p deploy-go-agent`
   - `cargo test -p deploy-go-api --test artifacts_api --test env_sync_dispatcher`（至少先跑相关测试）
   - `git diff --check`
5. 按 AGENTS.md 小步提交、推送，不在没有用户授权时执行远程部署。

## 6. 后续恢复生产 Agent 的参考路径（当前不执行）

> 未经用户明确授权，不得执行。以下是排障结论，不是操作指令。

由于排障时已用 gen 481 successor token 创建过新 rotation，恢复时不能用旧 gen 480 token
简单重试。推荐恢复路径之一：

1. 用 gen 481 token + 已存在的 probe rotation_id 再调用 refresh，取回 gen 482 方向的新
   refresh token；
2. 将 `qfy-prod-1` credentials.json 本地“提交”到该新 refresh token（保留 agent_id，
   清空 pending_rotation），保持 0600 与 owner；
3. 重启 `deploy-go-agent`，确认 Agent 重新连上 WSS、节点 online、heartbeat 更新；
4. 再决定是否重发 deploy-go 部署或手动处理当前 pending 部署。

更稳妥的方案是先完成 4.1 的 Agent 自动恢复逻辑并发布新 Agent 版本，再让 Agent 自愈。

## 7. 与卡券系统正式环境相关的未完成事项（不属于本次代码修复范围）

- 正式环境 API 目前仍是单容器 + Caddy `reverse_proxy 127.0.0.1:24710`，**尚未做蓝绿首迁**。
- 最新 production 代码对正式 API 强制蓝绿门禁：需要先执行一次
  `make deploy-api-blue-green-production DEPLOY_REMOTE=qfy-prod-1` 完成首次迁移，
  Deploy Go 全量部署的 API 阶段才可能通过。
- 用户当前要求先不碰卡券系统正式环境，因此这些事项仅在用户再次授权后处理。
