---
date: 2026-08-06
topic: git-branch-deployment-contract
status: accepted
schema_version: 1
---

# Git 分支部署规范

## 目标

本规范定义业务应用固定使用一个远程 Git 分支部署时的配置、引用发现、部署预览、确认、执行和重试语义。Tag 部署不属于本规范，后续使用独立规范扩展。

## 应用配置

只有管理员可以配置或修改：

- `repository_url`：Git 仓库地址。
- `git_credential_id`：可选的 Git 凭证引用。
- `build_agent_id`：执行引用发现和准备任务的 Agent。
- `source_policy`：固定为 `branch`。
- `deployment_branch`：不带 `refs/heads/` 前缀的远程分支名。

保存前必须由选定构建 Agent 完成仓库连通性检查，并确认所选分支当前存在。修改 Git URL、凭证或构建 Agent 后，已有分支列表和所选分支验证结果立即失效，必须重新获取。

普通用户可以查看当前部署分支，但不能临时输入或切换 Git URL、分支、Tag 或 commit。

## 分支发现

分支列表由构建 Agent 使用参数化 Git 进程读取，语义等同于：

```text
git ls-remote --heads <repository_url>
```

要求：

- Git 命令必须通过参数数组启动，不把 URL、凭证或分支拼接进 shell。
- 只接受 `refs/heads/` 命名空间，向界面返回去掉该前缀后的分支名和对象 ID。
- 不返回远程符号引用、Tag、提交历史或凭证信息。
- 分支名必须满足 Git ref 规则；空名称、控制字符、`..`、`@{`、反斜杠和以点或斜杠结尾的名称必须拒绝。
- 返回结果按分支名稳定排序，支持服务端分页或有界数量；界面提供搜索和手动刷新。
- 列表缓存只用于选择体验，不能作为部署时 commit 的事实来源。
- 每次生成部署预览或直接创建部署时，主控必须自动触发一次新的 refs 查询并
  等待构建 Agent 返回结果；同一来源的在途查询可以复用，但不能用已过期或旧
  缓存结果替代部署时解析。
- Agent 离线、凭证无效、仓库不可达、超时和无分支必须返回可区分错误。

## 分支选择

首版每个应用只允许固定一个 `deployment_branch`。这样普通用户不能通过选择未审查分支绕过发布门禁。

管理员保存时提交短分支名，主控规范化为：

```text
refs/heads/<deployment_branch>
```

主控必须验证该名称来自当前 Git URL、凭证和构建 Agent 组合的一次成功发现结果，并在保存时再次精确解析。发现结果过期或分支已经消失时拒绝保存。

## 部署预览与确认

部署预览必须通过构建 Agent 精确解析配置分支，并展示：

- 仓库标识。
- 分支短名称和完整 ref。
- 当前 commit SHA。
- commit 摘要；作者和提交时间属于可选展示信息，不能参与执行裁决。
- 模块、环境、发布版本和目标节点。

每次生成预览时，主控自动触发一次分支发现并解析固定分支当前最新 SHA；
普通用户无需在部署前手动“刷新分支”。

确认部署时保存不可变快照：

```text
source_policy=branch
requested_ref=refs/heads/main
resolved_commit_sha=<full sha>
```

确认后的部署永远指向 `resolved_commit_sha`。分支在确认后新增提交、删除或 force-push，不能静默改变该部署；无法再取得固化 commit 时准备阶段明确失败，不能退回到分支最新提交。

主控必须持久化预览快照。确认时只引用预览中已固化的 commit，不重新解析
分支；分支在预览后移动到新 commit 不改变本次部署，需要最新代码时重新生成
预览。预览有明确有效期，过期后必须重新生成；同一预览只能确认一次，重复
确认同一快照不能创建多个部署。管理员修改来源或重新固定分支后，该应用下
尚未确认的 active 预览必须立即失效，确认时要求重新生成预览；已确认或历史
部署继续使用原快照，不受影响。

## Agent checkout

准备任务只携带结构化 Git 字段和固化 commit，不携带任意 Git 参数或 shell：

```text
repository_url
credential_reference
requested_ref
resolved_commit_sha
workspace_root
```

Agent 必须：

1. 在任务独占工作区初始化或复用受控 object cache。
2. fetch 指定完整 ref，不执行无边界的远程引用同步。
3. 验证固化 commit 可以从该 fetch 结果取得。
4. checkout `resolved_commit_sha` 为 detached HEAD。
5. 确认 `HEAD` 精确等于固化 commit，工作区满足干净策略。
6. 再执行 `make --no-print-directory deploy-go-prepare`。

业务 target 禁止执行 `git pull`、切换分支或根据分支名重新选择 commit。Git 凭证只提供给受控 Git 子进程，不传给 Make target。

## 重试与审计

- 重试复用原 `requested_ref` 和 `resolved_commit_sha`，不重新读取分支最新值。
- 需要部署分支新提交时必须创建新的部署预览和部署记录。
- 重试直接复用历史部署 snapshot，不需要也不允许通过新的 refs 查询改变
  原 commit。
- 审计记录保存应用、分支、commit、请求人、确认时间、Git 配置版本和构建 Agent。
- 删除应用分支配置不能改写历史部署快照。

## 错误口径

至少区分：

- `git_repository_unreachable`
- `git_authentication_failed`
- `git_branch_not_found`
- `git_ref_invalid`
- `git_ref_discovery_expired`
- `git_ref_discovery_timeout`
- `preview_not_found`
- `preview_expired`
- `preview_already_confirmed`
- `git_commit_unavailable`
- `git_checkout_mismatch`
- `git_workspace_dirty`

错误正文不得包含凭证、带凭证 URL、环境文件或 Git helper 输出的敏感内容。

## 与两阶段部署的关系

本规范只负责把应用配置的浮动分支安全收敛为一次部署的不可变 commit。后续准备、manifest、传输和发布行为继续遵守 `docs/standards/application-deployment-contract.md`。
