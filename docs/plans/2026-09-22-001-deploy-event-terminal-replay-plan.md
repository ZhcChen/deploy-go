---
title: 部署事件终态 marker 幂等重放修复
date: 2026-09-22
status: implemented
---

# 部署事件终态 marker 幂等重放修复

## 问题与目标

正式部署 `deployment_01M336BTW9XBZZWGXNCSYKX6RM` 的业务脚本在 `api.remote.seed` 失败后，连续输出了两次字段完全相同的 `deploy.step.failed`。Agent 接受第一次后关闭 active step，又把第二次判定为 `step_mismatch`，最终用 `deploy_event_protocol_conflict` 覆盖了原始执行失败。

目标是在不修改业务应用的前提下，使 Agent 对紧邻的、字段完全一致的 step 失败终态重放保持幂等，并继续拒绝字段变化、跨 step、跨 module 或成功/失败互换的冲突 marker。

## 执行单元

1. 在 `MarkerState` 中记录最近一次已接受的 step 终态 marker；开始新 step 时清空该记录。
2. `process_line` 在状态迁移前识别完全相同的 `deploy.step.failed` 重放，并返回 `Ok(None)`，避免重复持久化事件。
3. 增加回归测试，覆盖完全相同失败 marker、字段变化、其他 step 和成功后失败四类场景。
4. 运行 Agent marker 与 two-stage 聚焦测试、格式检查和差异检查。

## 完成标准

- replay7 对应的重复 marker 序列不再产生协议冲突，任务仍按非零退出码失败并保留真实 stdout。
- 非完全相同或非紧邻的终态 marker 仍产生 `step_mismatch`。
- 不修改任何业务应用代码、配置、脚本、仓库或发布物。

## 执行记录

- Agent 已将字段完全一致且紧邻重放的 `deploy.step.failed` 视为幂等重复，不重复生成 progress 事件。
- 字段变化、跨 step 及成功/失败互换仍返回 `step_mismatch`。
- 已通过 `cargo test -p deploy-go-agent --test deploy_events`、`cargo test -p deploy-go-agent --test two_stage`、`cargo fmt --all -- --check` 和 `git diff --check`。
