# Runbook 目录说明

本目录保存可以直接执行的运行、部署、迁移、排障和联调手册。按任务查找的总入口是 [`docs/README.md`](../README.md)；本文只维护手册分类和写作边界。

## 按场景查找

| 场景 | 手册 |
| --- | --- |
| 开发与验证 | [`development-guide.md`](development-guide.md)、[`local-development.md`](local-development.md)、[`remote-development.md`](remote-development.md)、[`spec-kit-workflow.md`](spec-kit-workflow.md) |
| 业务应用接入 | [`application-onboarding.md`](application-onboarding.md)、[`application-templates.md`](application-templates.md)、[`external-deploy-api.md`](external-deploy-api.md) |
| Agent 与节点 | [`agent-onboarding.md`](agent-onboarding.md)、[`agent-recovery.md`](agent-recovery.md)、[`privileged-agent-release.md`](privileged-agent-release.md)、[`privileged-agent-terminal.md`](privileged-agent-terminal.md) |
| 部署恢复与日志 | [`deployment-recovery.md`](deployment-recovery.md)、[`log-retention.md`](log-retention.md) |
| 数据与密钥 | [`api-migrations.md`](api-migrations.md)、[`credential-master-key-rotation.md`](credential-master-key-rotation.md) |
| 正式环境 | [`systemd-deployment-production.md`](systemd-deployment-production.md) |
| Legacy SSH | [`ssh-node-onboarding.md`](ssh-node-onboarding.md)；仅记录旧数据退出流程，新节点必须使用 Agent 接入手册 |

## 编写和维护规则

- 手册必须写清适用范围、前置条件、命令、验证方式、失败恢复和安全边界。
- 命令和路径使用仓库相对形式；环境差异通过明确参数或环境变量表达。
- 涉及真实节点、远程脚本或正式部署时，必须遵守 `AGENTS.md` 的远程执行授权规则。
- 实现改变了已有命令、前置条件、验证或恢复步骤时，同一改动必须同步更新对应 runbook。
- 历史 plan 只说明当时如何实施，不能代替当前 runbook；发现历史文档与当前实现漂移时，先更新当前手册，再在历史文档中留下指向。
- 提交前运行 `make docs-check`，确保入口文件存在且 Markdown 相对链接没有断链或越界。
