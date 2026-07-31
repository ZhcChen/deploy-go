---
title: Compound Engineering 工作流迁移计划
status: completed
date: 2026-07-31
---

# Compound Engineering 工作流迁移计划

## 目标

- 将仓库工作流统一为官方 Compound Engineering（CE）工作流。
- 保留现有正式文档，清理已经被 CE skill 替代的手工提示词和模板。
- 补齐适合轻部署平台的文档权威、运行手册和远程执行授权边界。

## 范围

1. 调整 `AGENTS.md` 的工作流、产物、执行、Review、Compound 和 Git 规则。
2. 新增 `docs/standards/document-authority.md` 与 `docs/runbooks/README.md`。
3. 删除 `docs/prompts/` 和四个工作流 `TEMPLATE.md`。
4. 更新 `README.md` 中的工作流入口。

## 非目标

- 不修改产品需求、UI 设计源或部署脚本契约。
- 不引入功能分支、worktree、PR 或自动合并流程。
- 不执行远程节点连接、部署或脚本运行。

## 验证

- 仓库有效规则不再引用 `docs/prompts/` 或工作流 `TEMPLATE.md`。
- `AGENTS.md` 明确 CE 六阶段循环和远程执行授权边界。
- `git diff --check` 与 `make ui-check` 通过。

## 完成结果

- 仓库已切换为官方 CE 六阶段工作流，并保留小任务直做与 Bug 调试旁路。
- 已补齐文档权威规则、runbook 目录和远程脚本执行授权边界。
- 旧手工提示词和工作流模板已删除，现有正式工作流文档保持不变。
- 复核结论见 `docs/reviews/2026-07-31-compound-engineering-workflow.md`。
