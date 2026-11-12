---
date: 2026-11-12
topic: git-lease-expired-while-queued
spec: specs/019-lease-window-from-delivery
---

# 排队超过 60 秒的任务因 Git 凭证租约过期而失败

2026-11-12 排查他项目反馈“详情页不自动刷新”时，顺带在线上数据里发现一个长期存在的缺陷：同一节点的构建或发布任务排队超过 60 秒，一被派发就以 `secret_lease_lease_not_available` 失败。

## 现象

部署失败，摘要为空，错误码 `secret_lease_lease_not_available`。从任务被派发到失败只有约 0.3 秒，业务脚本一行都没执行。典型场景：两个人几乎同时给生产节点发版，前一个发布跑了 2.5 分钟，后一个的发布任务在队列里等到轮次，立刻失败。

## 根因

1. `create_stage_task` 在创建任务的同一事务里签发 Git 凭证租约，到期时间固定为“创建时刻 + 60 秒”。
2. 同一 Agent 的部署名额被占满时，任务停在 `queued`。调度循环每个周期运行 `expire_secret_leases`，把过期租约改成 `expired`。
3. `try_dispatch` 派发时不触碰租约。Agent 取租约时，`resolve_secret_lease` 要求 `status='issued' AND expires_at>now`，不满足就返回 `lease_not_available`，Agent 把它拼成 `secret_lease_lease_not_available` 并让任务失败。
4. 已有的 `reissue_secret_lease` 只处理“租约响应投递失败”，不处理“从未被取用就过期”。

租约要限制的是“派发之后”的取用窗口，而不是任务排了多久的队。

## 怎么发现的

先看一次失败的完整时间线，再用历史数据验证规律，最后核对代码。

```sql
-- 一次失败任务：创建、派发、开始、结束时间，以及名下租约的到期时间和状态
SELECT t.stage, t.created_at, t.delivered_at, t.started_at, t.finished_at FROM agent_tasks t WHERE t.deployment_id = '<部署 ID>';
SELECT id, status, expires_at FROM git_secret_leases WHERE task_id = '<任务 ID>';

-- 全部历史：按“创建→派发”等待时长分桶，看结果
SELECT stage,
       CASE WHEN w < 30 THEN 'a:<30s' WHEN w < 60 THEN 'b:30-60s' WHEN w < 120 THEN 'c:60-120s' WHEN w < 300 THEN 'd:120-300s' ELSE 'e:>=300s' END AS wait_bucket,
       COALESCE(json_extract(result_json, '$.error_code'), status) AS outcome,
       COUNT(*)
FROM (SELECT *, (julianday(delivered_at) - julianday(created_at)) * 86400.0 AS w
      FROM agent_tasks WHERE kind IN ('deployment_prepare', 'deployment_release') AND delivered_at IS NOT NULL)
GROUP BY 1, 2, 3;
```

线上结果（2026-11-12）：等待不少于 60 秒的 19 个任务（构建 15、发布 4）全部以该错误失败，没有例外；等待不到 30 秒的 3059 个任务只有 2 个因此失败。累计 22 次，最早 2026-09-09，2026-11-05 以后占 11 次。

那 2 个“等待 0 秒却失败”的发布任务总耗时 331 秒和 118 秒：Agent 在取租约前先下载并校验制品，耗时超过了 60 秒。成功任务从派发到取租约的间隔：构建最大 0.4 秒，发布平均 6.2 秒、最大 36.5 秒。

## 修复

任务派发（含重新派发）成功后、任务消息发给 Agent 之前，把它名下从未被取用的 Git 凭证租约重新计时：构建 60 秒、发布 10 分钟，不超过任务截止时间，只延长不缩短。只改控制面；Agent、协议、schema 和 OpenAPI 都不变。

范围刻意收窄，取用规则一概不变：

- 只处理 `granted_at` 为空的租约。`expire_secret_leases` 也会把已取用且到期的租约改成 `expired`，这些不能被复活；
- 复活范围包含 `expired`：线上失败时租约已经被清理成 `expired`，只处理 `issued` 起不到作用；
- 派发被并发门禁拒绝或 Agent 未连接时不改动租约；
- 只在任务当前仍是 `delivered` 时才动。任务终结时 `expire_task_secret_leases` 会主动作废它名下的租约，那些任务不会再被派发，也不能被这里复活。

## 验证中的几个坑

1. **租约的状态不是只有 `issued` 和 `granted`。** 最初的设想是“把还没用的 `issued` 租约续期”，但线上那条租约在任务被派发时早已被清理成 `expired`。变异验证里把复活范围缩回 `issued`，5 个用例失败，才确认这一点必须覆盖。
2. **测试数据要符合表上的约束。** `deployment_prepare` 任务必须关联部署记录并带 `stage='prepare'`，随手插一行会被 CHECK 约束拒绝；按真实数据搭一条最小的“用户、应用、目标、部署”链即可。
3. **用真实函数，不要只测纯函数。** 用例使用真实的 `expire_secret_leases`、`try_dispatch` 和 `resolve_secret_lease`（含主密钥解密），先断言修复前的失败（`lease_not_available`），再断言修复后能取到私钥。

## 经验

- 一个“限时”的凭证，计时起点应该是它被使用的前提成立之时（任务派发），而不是它被创建之时；创建与使用之间存在排队、重试、重连等不确定间隔。
- 同一条链路上的授权要有一致的寿命设计：制品下载租约有效期 30 分钟，Git 租约只有 60 秒，前者允许排队很久，后者却把排队变成了失败。
- 失败只留错误码、摘要为空，用户看到的只是“部署失败”。统计 `error_code` 的历史分布是发现这类问题最快的办法。
