---
title: 固定分支自动刷新与预览固化实施计划
date: 2026-08-12
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: code
---

# 固定分支自动刷新与预览固化实施计划

## Goal Capsule

- 部署分支只在应用来源配置阶段固定一次；后续部署由平台自动解析该分支当前
  commit，不再要求用户手动“刷新分支”并依赖 15 分钟 refs 缓存。
- 部署预览由服务端持久化固化；确认时只引用预览时解析到的 commit，分支在
  预览后移动不改变本次部署，重复确认同一快照不能创建多个部署。
- Build Agent 离线、仓库不可达、认证失败或查询超时时，预览/创建部署明确
  失败，不使用过期或离线缓存兜底。

## 关键边界

- 普通用户仍不能输入任意分支、Tag 或 commit；commit 只能来自服务端解析的
  固定分支预览。
- 历史部署与 retry 继续复用原 snapshot 中的固化 commit，不自动跟随分支。
- 旧客户端/外部 API 不带 snapshot_hash 的直接部署仍可用，等价于服务端创建
  预览后立即确认。
- 本计划只改 API、文档与测试，不修改 Agent/executor 本机协议。

## 实施单元

### U1. refs 自动解析内部链路

- `application_sources` 抽出“创建/复用 git_refs_query 任务、签发 secret
  lease、dispatch”的共享 helper，并新增等待 discovery 终态的 helper。
- `resolve_two_stage_source` 改为每次部署预览/创建都自动解析固定分支最新
  SHA；同一 source 的在途查询并发复用；Build Agent 离线/查询失败返回明确
  错误。
- 应用级预览只解析一次来源，所有 target 复用同一个 resolved commit。

### U2. 服务端 preview 固化

- 新增 migration `0023_deployment_previews.sql`：preview 记录包含
  application_id、target_id（单目标）、created_by、snapshot_hash、
  snapshot_json、parameters_json、release_strategy、status、expires_at。
- preview 接口（单目标与应用级）在 two_stage 模式下落库；confirm 通过
  snapshot_hash 加载对应 preview，校验未过期、参数一致、单次确认。
- 直接创建部署（无 snapshot_hash）继续构建最新 preview 并立即创建部署。
- preview 过期由 worker retention 周期清理，避免长期增长。

### U3. 文档与契约同步

- 更新 `docs/standards/git-branch-deployment-contract.md`：固定分支只配置
  一次、每次部署自动解析最新 SHA、确认固化预览 commit、Build Agent 离线
  失败口径。
- 更新 OpenAPI / admin client / 外部部署 runbook；preview 响应补充
  `preview_expires_at`（可选），confirm 错误码补充
  `preview_not_found` / `preview_expired` / `preview_already_confirmed`。

### U4. 测试与门禁

- 新增/更新 API 测试：
  - preview 自动刷新最新 commit（覆盖旧 discovery 过期后自动查询）；
  - preview 后分支移动，confirm 仍固化预览 commit；
  - preview 过期、重复确认、参数不一致被拒绝；
  - Build Agent 离线时 preview 明确失败；
  - 应用级多 target 只创建一次 refs 查询。
- 运行 `make api-check`、`make api-test`，再运行 `make check`。

## 验收

- 应用固定 `deployment_branch=main` 后，连续部署无需手动刷新分支。
- 每次新部署 preview 都显示当前分支最新 SHA，确认后 deployment snapshot
  固定该 SHA。
- retry 继续使用原 commit；历史部署不可变。
- 正式环境部署前本计划 U1-U4 全部完成并通过门禁。
