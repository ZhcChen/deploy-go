# Runbook 目录说明

本目录保存可以直接执行的运行、部署、迁移、排障和联调手册。

## 使用规则

- 手册应写清适用范围、前置条件、命令、验证方式、失败恢复和安全边界。
- 命令和路径使用仓库相对形式；环境差异通过明确参数或环境变量表达。
- 涉及真实节点、远程脚本或部署操作时，必须遵守 `AGENTS.md` 的远程执行授权规则。
- 实现改变了已有命令、前置条件、验证或恢复步骤时，必须同步更新对应 runbook。
- 历史 plan 只说明当时如何实施，不能代替当前 runbook。

已完成的手册：

- `docs/runbooks/local-development.md`
- `docs/runbooks/api-migrations.md`
- `docs/runbooks/credential-master-key-rotation.md`
- `docs/runbooks/agent-onboarding.md`
- `docs/runbooks/agent-recovery.md`
- `docs/runbooks/ssh-node-onboarding.md`
- `docs/runbooks/deployment-recovery.md`
- `docs/runbooks/github-actions-release.md`
- `docs/runbooks/systemd-deployment-qfy-test.md`

`ssh-node-onboarding.md` 仅记录 legacy 数据退出流程；新节点必须使用 Agent 接入手册。
