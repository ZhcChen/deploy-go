---
date: 2026-08-06
topic: privileged-release-launcher
status: accepted
schema_version: 1
---

# 受控发布 launcher 规范

## 目标

本规范约束历史 launcher 兼容实现。平台 release 已固定使用 Agent 原生结构化 `privileged_release`，不再提供目标级开关；需要 Docker、root 或系统级发布操作的历史应用仍可按本文提供应用专属、root 所有、固定路径、固定入口和参数白名单的 launcher，并通过精确 sudo 白名单调用，但 Deploy Go 不会自动选择或回退到 launcher。

管理员 root PTY 和 Agent 原生结构化特权 release 是两个独立 executor operation，遵守 `docs/standards/privileged-agent-executor.md`。现有 launcher 可继续作为历史兼容参考保留；平台 release 固定使用原生 executor，失败时不得自动回退 launcher。

本规范禁止：

- 把 `deploy-go-agent` 加入 Docker 组。
- 给低权限 Agent 的普通任务或业务脚本开放通用 `sudo`、任意 shell 或任意 Docker 参数。
- 在平台或业务脚本中隐式使用 `sudo`。
- launcher 接受 shell 片段、Docker 参数、URL、环境文件内容或任意命令路径。

## 部署形态

launcher 安装后必须满足：

- 路径固定，例如 `/usr/local/sbin/qfy-voucher-hub-release-launcher`。
- 属主为 `root:root`，权限 `0755`，目录链不能被 `deploy-go-agent` 写。
- 入口固定：launcher 只调用自身内部固定的应用发布动作，或调用 root 所有、固定绝对路径的发布入口。
- sudoers 只放行 launcher 的固定绝对路径，不放行通用 shell、`docker`、`bash -c` 或带 `*` 的任意命令。
- sudo 配置使用 `Defaults env_reset`，不放行 `SETENV`，避免业务进程把环境变量注入 launcher。

### sudoers 示例

```text
Defaults env_reset
Defaults!/usr/local/sbin/qfy-voucher-hub-release-launcher !setenv
deploy-go-agent ALL=(root) NOPASSWD: /usr/local/sbin/qfy-voucher-hub-release-launcher --input /var/lib/deploy-go-agent/apps/*
```

命令路径必须精确；`--input` 通配范围必须限定在 Agent 可写且 launcher 可读的任务目录内。

## 调用契约

低权限业务脚本把调用参数写成 JSON 输入文件，再调用：

```bash
sudo -n /usr/local/sbin/qfy-voucher-hub-release-launcher --input "$input_file"
```

输入文件必须位于任务 staging 或 Agent 工作目录内，由业务脚本创建，权限 `0600`，调用后及时删除。

### 输入 Schema

首版只允许以下字段，未知字段必须拒绝：

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `schema_version` | 整数 | 固定 `1` |
| `app_id` | 字符串 | launcher 固定的应用白名单之一 |
| `operation` | 字符串 | 首版固定 `release`，新增操作需独立审查 |
| `task_id` | 字符串 | `^[A-Za-z0-9._-]{1,128}$` |
| `module` | 字符串 | 应用固定模块白名单之一 |
| `release_version` | 字符串 | `^[A-Za-z0-9._-]{1,256}$` |
| `staging_dir` | 字符串 | 绝对路径，realpath 后必须位于允许的任务根内 |

launcher 必须拒绝：

- 额外字段、非字符串字段和格式错误。
- 相对路径、`..`、符号链接逃逸和允许根之外的目录。
- 不存在或不可读的 staging 目录和 manifest。
- 与当前任务不一致的发布版本、commit 或模块。
- 未声明的发布物、路径逃逸和 SHA-256 不一致。

## launcher 行为

launcher 在实施任何特权动作前必须完成全部输入校验，然后：

1. 使用干净环境调用固定入口，只注入 launcher 自行生成的值。
2. 根据 `module` 和 `staging_dir` 内部推导应用目录、Compose 文件、环境文件引用和发布物路径，不接受外部传入。
3. 转发 stdout/stderr，保留底层退出码。
4. 收到 `SIGTERM`/`SIGINT` 后转发给子进程，等待其清理并以明确退出码结束。
5. 写入只追加、root 可写的审计日志，记录调用来源字段，不记录 token、私钥、环境文件内容或完整敏感参数。
6. 不允许 `eval`、二次 shell、`bash -c` 或任何可被输入控制的命令拼接。

## 应用接入检查

应用仓库应提供 launcher 源文件、安装脚本和本地契约测试。检查项：

- `bash -n` 和本地契约测试通过。
- sudoers 文本不含通用 shell、`ALL` 命令或 `docker` 命令。
- launcher 拒绝额外字段、未知应用/模块/操作、路径逃逸、符号链接和环境污染。
- launcher 正确传递底层失败退出码，并在收到信号时结束。
- 生产安装后确认 `ls -l` 属主、路径、sudoers 校验和 `sudo -l` 输出。

可测试参考实现位于 `examples/privileged-release-launcher/`。

## 与 root executor 的关系

- launcher 和 executor 的结构化 release 都面向部署状态机；executor 的 PTY 面向管理员临时维护，三者不能互相冒充。
- 节点不存在 `privileged_execution` 开关；v11 Agent 的 executor 能力由本机配对服务健康状态决定，不授权业务脚本调用终端，也不放宽 launcher sudoers。
- 平台 release 固定使用结构化特权 executor，不开放 PTY；不存在目标级 `privileged_release` 开关或关闭概念。
- executor 不得通过 PTY 代替 launcher 或结构化 release；部署记录、重试、回滚和事件仍以标准业务 Make target 为准。
- 历史 launcher 目标迁移到原生 executor 后固定使用结构化 release；不能改变已创建 deployment 的 snapshot，也不能在一次失败任务中自动重跑另一后端。
