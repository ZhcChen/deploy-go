---
title: 对外部署来源可观测性
date: 2026-10-08
status: in_progress
---

# 目标与边界

外部调用方通过 `show-app` 确认配置分支，通过部署详情确认该次发布实际固定的提交。只修改 Deploy Go；不修改业务应用、不发起业务部署、不增加数据库迁移或远端 Git 查询。

# 执行单元

- U1：应用详情新增 `sources.git` 与 `sources.workspace` 可空摘要，包含状态、来源版本、构建 Agent/节点标识与名称；Git 包含配置分支、验证时间和物化策略。两种配置独立返回，目标 `two_stage` 使用 Git，`two_stage_script` 使用工作区，其他模式不据此判断来源。不返回仓库凭证、租约或私钥。
- U2：部署详情新增 `source_policy`、`deployment_branch`、`resolved_commit_sha`、`release_version`，只读取该次快照白名单字段；工作区摘要不得冒充 Git SHA。同步 CLI 人类可读输出、JSON、OpenAPI、skill 和 runbook。
- U3：聚焦验证、独立审查、提交推送、版本更新与正式控制面部署；沿用本机 Docker 构建，不依赖 10808 代理。

# 验证与完成条件

- Git verified/draft/archived、未配置、工作区和双来源返回正确；普通读取不新增任务；未授权应用不可读。
- 部署快照字段与当前配置分离，历史非 Git 部署返回空 Git 字段。
- CLI 旧响应兼容，文本与 JSON 可确认分支；OpenAPI 与 skill 一致且不包含凭证。
- API/CLI 聚焦测试、生成物一致性、格式和 diff 检查通过；正式环境健康与下载契约验收，记录提交与版本。
