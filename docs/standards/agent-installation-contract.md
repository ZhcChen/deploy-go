---
date: 2026-08-07
topic: agent-installation-contract
status: accepted
schema_version: 1
---

# Agent 配对安装与发布契约

## 适用范围

本规范约束 `deploy-go-agent` 与 `deploy-go-agent-executor` 的发布清单、节点安装、升级、回滚和卸载。终端授权、PTY 协议和 root 边界以 `docs/standards/privileged-agent-executor.md` 为准。

## 配对发布清单

新安装只接受 `agent/release/manifest.schema.json` 定义的 `schema_version: 2`：

- `agent_version` 与 `executor_version` 必须相同，并与 API 当前发布版本一致。
- `artifacts` 必须恰好包含 `agent`、`executor` 的 Linux `x86_64`、`aarch64` 四个二进制及各自 SHA-256。
- `systemd_units` 必须同时声明 Agent 与 executor unit；`executor_config` 必须声明本机配置模板。
- 所有节点下载 URL 必须为 HTTPS。API 对外服务 manifest 时把 URL 重写到自身版本化下载路由。
- 安装器必须先完成 manifest 结构、版本、架构和所有 checksum 校验，再修改节点文件。

API 可以读取历史 `schema_version: 1` 发布目录，保证版本列表和旧 Agent 下载不因升级中断；v1 不包含 executor，不能被新版安装器用于开启终端能力。

GitHub Actions release workflow 当前保持整体注释禁用，但模板必须能为每种架构成对构建 Agent/executor、配对归档、checksum 与 v2 manifest。正式部署当前通过 `deploy/production/deploy.sh` 在部署机本地构建同样的配对发布目录。

## 身份、进程与 Socket

- 安装器只创建一个专用系统身份 `deploy-go-agent:deploy-go-agent`。联网 Agent 使用该 uid/gid；executor 使用 root。
- executor 配置模板只允许替换 Agent 的数字 uid/gid，shell 固定为 `/bin/sh`。主控和安装命令不能注入 shell、环境变量或 Socket 路径。
- executor 自行创建 `/run/deploy-go-agent/executor.sock` 并设置目录 `0750 root:deploy-go-agent`、Socket `0660 root:deploy-go-agent`。当前不使用 systemd socket activation，不安装 `deploy-go-agent.socket`。
- executor unit 只允许 `AF_UNIX` 并使用 `IPAddressDeny=any`；Agent unit 以 `Wants` 和 `After` 软依赖 executor。executor 失败时 Agent 仍可在线执行普通部署，但不声明 `pty_terminal`。
- executor 先启动、Agent 后启动；停止和卸载时 Agent 先停止、executor 后停止，确保活动 PTY 先失去上游并被清理。

## 原子升级与恢复

安装器把以下对象视为单一事务：两个二进制、两个 unit、Agent 非敏感配置和 executor 本机配置。

1. 下载并校验全部输入。
2. 保留当前对象及原启用状态。
3. 原子替换成对文件并执行 `daemon-reload`。
4. 先验证 executor service 与 Unix Socket，再重启并验证 Agent service。
5. 任一步失败时停止新服务、恢复整对旧对象和启用状态，再按 executor -> Agent 顺序恢复旧服务。

首次安装失败时不留下半套二进制或 unit。已有 Agent 升级失败时必须恢复原有低权限 Agent，使普通部署能力不依赖 executor 成功。安装完成不自动修改数据库中的 `nodes.privileged_execution`。

## 卸载与数据保留

`install.sh --uninstall` 先停止 Agent 再停止 executor，禁用并移除两个服务、两个二进制、executor 配置和运行时 Socket。卸载保留 `credentials.json`、任务 journal、应用工作目录和 secrets，避免未经确认删除业务状态；重新分配或报废节点前应先在主控撤销 Agent 身份。

## 验证门禁

- `make agent-install-check`：安装器语法、Bats（环境存在时）、unit 静态安全契约和 `systemd-analyze verify`（Linux 环境存在时）。
- `make agent-manifest-check`：v2 manifest 生成、四个架构组件、双 unit 和配置模板 checksum。
- `make agent-release-sync-check`：历史 GitHub Release 同步脚本仍按成对发布物执行原子替换。
- `bash deploy/production/test-install-contract.sh`：生产部署本地构建并安装配对发布目录，不在服务器依赖 `jq`。

真实节点安装、升级、卸载、重启或清理仍需当前对话针对具体节点的明确授权。
