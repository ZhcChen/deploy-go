---
title: 分支部署规范与接入 Demo 计划
date: 2026-08-06
status: completed
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: docs-and-fixture
---

# 分支部署规范与接入 Demo 计划

## Goal Capsule

先冻结业务应用使用固定 Git 分支部署的产品与协议边界，并提供不访问真实仓库和节点的可执行 Demo，让后续 API、Agent、管理端和业务项目适配共享同一契约。

## Requirements

- R1. 应用由管理员绑定 Git URL、凭证、构建 Agent和固定部署分支。
- R2. 分支列表由构建 Agent 读取，缓存只用于展示；部署预览必须解析实时分支 commit。
- R3. 部署确认固化完整 ref 和 commit SHA，执行及重试不得跟随分支漂移。
- R4. Agent 负责 Git checkout；业务 Make target 不得更新代码。
- R5. Demo 展示准备、manifest、发布和最小事件标记，不依赖真实 Git、网络、Docker、sudo 或 systemd。
- R6. 聚焦测试覆盖成功路径、非法 commit 和发布物校验失败。

## Implementation Units

### U1. 分支来源规范

新增 `docs/standards/git-branch-deployment-contract.md`，定义 refs 枚举、固定分支策略、预览/确认快照、执行、重试、权限和错误行为，并从两阶段部署规范引用。

### U2. 可执行接入 Demo

新增 `examples/branch-deployment/`，包含 Makefile、准备脚本、发布脚本、示例文件和 README。脚本只写临时或显式传入目录，直接输出 `DEPLOY_GO_EVENT`。

### U3. 契约验证入口

新增 Demo 自测脚本和 `make deploy-contract-demo-check`，校验 Shell 语法、manifest、成功切换、非法输入和篡改阻断，并接入全仓 `make check`。

## Verification

- `make deploy-contract-demo-check`
- `jq empty docs/standards/*.json`
- `git diff --check`

## Scope Boundary

本轮不实现 API、数据库、Agent Git 执行器、管理端页面、真实 artifact 传输或 `qfy-voucher-hub` 改造，不连接任何真实节点。

## Completion

已完成分支部署规范、可执行准备/发布 Demo、manifest 和事件示例，以及成功、重复发布、非法 commit 与发布物篡改测试。验证全程使用临时目录，未访问真实 Git 或节点。
