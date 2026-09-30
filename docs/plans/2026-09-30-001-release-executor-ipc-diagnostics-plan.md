---
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan
execution: code
title: 特权 Release Executor IPC 失败诊断
date: 2026-09-30
---

## 目标

保留特权 release 启动期间 Agent 与 root executor IPC 的失败类别，使 Agent 日志和 External deployment diagnostics 能区分连接失败、请求帧超限/写入失败、响应超时/关闭/解码失败及非预期响应。不得将尚未确认的生产故障归因于其中任何一种。

## 范围

- 为 IPC 帧错误保留安全的长度和错误类别元数据，不保留请求/响应正文。
- ExecutorClient 对请求和响应错误分类，并映射到稳定错误码。
- Agent 在 release 启动失败时记录 task/job ID、错误类别、IPC 协议版本及适用的帧大小；不记录授权串、Env、请求体或原始 IO 错误文本。
- External API 白名单暴露这些稳定错误码；未知码继续降级为 `agent_error`。
- 修正 doctor 的 executor 协议版本文案，更新 release executor 排障步骤。
- External 对 executor 响应不确定类错误使用明确摘要，提示先核对 durable job、不要盲目重放。

## 实施单元

1. **帧与客户端分类：** 更新 executor frame error 元数据与 Agent client 错误枚举；添加编码、超限、EOF、响应无效及超时测试。
2. **任务结果与日志：** ReleaseStart 失败时记录结构化安全日志并输出稳定错误码；测试响应错误映射，不改变 IPC wire schema。
3. **External 投影与运维文档：** 增加错误码白名单测试，修正 doctor 协议版本提示，记录按 task/job ID 关联 Agent 与 executor 日志的只读排查步骤。

## 验收

- IPC wire schema 和协议版本不变；请求正文、授权数据、Env 明文和原始文件路径不进入日志或 External 响应。
- 单测能区分请求帧超限、executor 不可达、响应超时、EOF、格式错误与非预期响应。
- External 对新增受控码原样返回，对任意未知码仍返回 `agent_error`。
- `cargo fmt --check`、相关 Agent/API 测试和 `git diff --check` 通过。
- 不部署正式控制面、不操作真实节点、不发起重试。

## 非目标

本次不判定 `deployment_01M3RWGJ8SGD7SW96YF175Z5CW` 的生产失败根因，不修改业务应用，不变更 Agent 自动升级策略、权限、systemd、目录或数据库状态。ReleaseStart 请求已发送但响应丢失时，当前 Agent 仍可能把任务终结为失败，即使 executor 已启动 job；本计划只明确暴露该不确定性并禁止文档建议盲目重放，不改变该任务状态机。此项是需独立规划与测试的可靠性风险，不得宣称本轮已修复。

## 执行进度

- [x] IPC 错误分类与帧长度安全元数据。
- [x] Agent release 启动结构化日志及 executor 错误码白名单化。
- [x] External 错误码投影、未知码降级和不确定结果提示。
- [x] doctor 协议版本与当前安装/排障文档同步。
- [x] Agent、executor、External API 聚焦测试与契约测试。
- [!] ReleaseStart 响应丢失后的终态失败/后台 job 状态不确定仍未修复；不得据本计划宣称生产故障根因已修复或执行正式发布。

后续状态机修复由 `docs/plans/2026-09-30-002-privileged-release-start-reconciliation-plan.md` 单独实施和验证；本计划的诊断能力不等于生产故障已确认或已修复。
