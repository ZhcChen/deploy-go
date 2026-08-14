---
title: Agent 凭证轮换恢复与离线发布可见性 - Plan
date: 2026-08-14
type: bugfix
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: code
---

# Agent 凭证轮换恢复与离线发布可见性 - Plan

## Goal Capsule

- **目标：** 修复 Agent 在未确认轮换的 access token 失效后无法自愈的问题，并让两阶段部署等待离线 Agent 时具备明确、可操作的状态说明。
- **事实来源：** `docs/handoffs/2026-08-14-agent-credential-rotation-and-offline-deployment-handoff.md` 的本地排障证据；当前用户指令优先于该交接材料。
- **范围边界：** 只修改本仓库代码与本地测试；不连接、重启、部署、迁移或变更任何真实节点、业务容器、Caddy 或 Deploy Go 控制面。

## Requirements

- R1. 当 pending rotation 的缓存 access token 缺失或失效且已有 `next_refresh_token` 时，Agent 必须先原子落盘本地 commit，再用 successor token 建立新的 rotation。
- R2. 本地 commit 失败时不得调用 refresh；没有 successor token 时不得改变既有 pending 状态或复用策略。
- R3. 服务端查询 pending rotation 时不得返回已过期 access token，并以稳定的认证错误拒绝请求，不得吊销合法 refresh credential family。
- R4. 两阶段部署在 prepare 成功后因目标 Agent 离线无法创建 release task 时，部署详情或事件必须明确说明 release 正在等待 Agent 上线。
- R5. Agent 恢复在线后，原有 pending target run 能继续进入 release，且离线等待不会丢失 prepare 成功的状态。

## Implementation Units

### U1. Agent 本地轮换恢复

- **Files:** `agent/src/token_refresh.rs`、`agent/tests/token_refresh.rs`。
- **Decision:** 仅在 `cached_pending_access()` 无法提供可用 access 且 pending 具有 successor token 时本地提交。该提交复用现有 `CredentialStore::store()` 原子落盘语义；提交成功后在同次 `prepare()` 内创建全新 rotation。不得将 superseded 的旧 refresh token 发送给服务端。
- **Test scenarios:** 过期 access 加 successor、缺失 access 加 successor、无 successor 保持旧 pending 并按既有 refresh 行为、store 失败时不调用 refresher 且内存与磁盘状态不前进。
- **Verification:** `cargo test -p deploy-go-agent --test token_refresh`，再按最终验证矩阵执行 Agent package tests 与 clippy。

### U2. 服务端过期 rotation 拒绝

- **Files:** `api/src/agents/auth.rs` 及现有 Agent auth 测试文件。
- **Decision:** `load_rotation()` 只接受 `access.expires_at > now` 的记录。过期 rotation 走专门、稳定的 401 错误路径，绝不调用 refresh reuse 检测或 family revoke 路径。
- **Test scenarios:** 构造过期 pending rotation；断言响应为稳定 401/错误码、响应不含 access token、对应 refresh credential family 未被 revoke。
- **Verification:** 对应 API 聚焦测试与 `cargo clippy -p deploy-go-api --all-targets -- -D warnings`。

### U3. 离线 release 等待可见性

- **Files:** `api/src/agents/dispatcher.rs`、`api/src/deployments/mod.rs` 及现有 dispatcher/deployment API 测试文件。
- **Decision:** 保持 Agent 离线时不创建 release task 的执行安全边界，同时写入或投影可解释的 `agent_offline` 发布等待事实；不在本轮引入未经验证的超时失败化或自动重试状态机。
- **Test scenarios:** prepare 成功而目标离线时仍保留 target run 与 prepare 成功事实并可见等待提示；Agent 回到 online 后可创建原 release task；不为离线 Agent 错误创建 task。
- **Verification:** 对应 dispatcher 测试及用户指定的 API integration tests。

## Sequencing And Delivery

1. 完成 U1，运行聚焦测试、格式检查和 Agent 静态检查后提交。
2. 完成 U2，运行聚焦 API 测试和静态检查后提交。
3. 完成 U3，运行调度聚焦测试后执行完整指定验证矩阵并提交。
4. 每次提交只暂存本轮相关代码、测试与本计划；提交后执行 `git fetch origin`、`git rebase origin/main`、`git push origin main`，不使用强推。

## Verification Contract

| Layer | Command |
| --- | --- |
| Format | `cargo fmt --all --check` |
| API lint | `cargo clippy -p deploy-go-api --all-targets -- -D warnings` |
| Agent lint | `cargo clippy -p deploy-go-agent --all-targets -- -D warnings` |
| Agent tests | `cargo test -p deploy-go-agent-protocol -p deploy-go-agent` |
| API focused | `cargo test -p deploy-go-api --test artifacts_api --test env_sync_dispatcher` |
| Diff | `git diff --check` |

## Progress

- [x] U1 Agent 本地轮换恢复：已覆盖过期/缺失 access、无 successor 与本地落盘失败；`cargo test -p deploy-go-agent --test token_refresh` 和 `cargo test -p deploy-go-agent --lib` 通过。
- [ ] U2 服务端过期 rotation 拒绝
- [ ] U3 离线 release 等待可见性
