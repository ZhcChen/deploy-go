---
title: Agent Git 物化执行身份与 checkout 所有权修复计划
date: 2026-09-20
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: code
---

# Agent Git 物化执行身份与 checkout 所有权修复计划

## 实施状态（2026-10-13 对照代码核实）

**状态：核心已实现并已发布；R4、诊断与 Linux 多 UID 测试已于 2026-10-13 补做（待发布），仍有少量待办。** 实现提交 `58af9fb`（2026-09-20，已包含在线上 0.3.32 中）。

- **已实现**：prepare 的 Git clone/fetch/checkout 只由 `deploy-go-runner` 在 runner 内执行一次（`runner.rs` 的 `run_spec`）；
  `execute_prepare` 只做路径校验、祖先符号链接检查和 `git::reset_checkout_for_runner` 清理，失败返回独立错误码 `git_checkout_cleanup_failed`；
  全仓库没有使用 `safe.directory`；full/sparse 都由 runner 执行；artifact/workspace 不触发 Git，跨节点 Git release 只有一个物化责任；
  不涉及业务应用。原先的问题描述“存在两次物化”已不再成立。
- **与计划不一致，已按现状裁定**：prepare **总是先整体删除旧 checkout 再由 runner 重建**，所以“复用 runner 自有的合法 checkout”
  不会在 prepare 中发生（AE3 改为“prepare 不复用”）；比计划保守，不影响正确性。
- **本次核实后补做（2026-10-13）**：
  - R4：对已存在的仓库，运行 Git 之前先检查 checkout 目录与 `.git` 的属主，不是当前执行身份就返回独立错误
    `git_checkout_ownership_untrusted`；Git 自己报的 `dubious ownership` 归到同一个错误码。三类目录里，
    “可清理的半成品（无 `.git`）”由既有逻辑清理，“属主不可信”现在有独立错误，“runner 自有的合法仓库”照常 fetch。
  - 新发现并修复的误分类：任何 `permission denied` 都被报成 `git_authentication_failed`，现在文件系统权限错误单独为
    `git_checkout_permission_denied`，只有 SSH 认证特征（`publickey` 等）才是认证失败。
  - U4 诊断：失败时附带执行身份 `euid/egid` 与 checkout、`.git`、父目录的 `uid/gid/mode`（只有数字元数据）；Git 的原始输出
    经过脱敏（去掉 URL 里的用户信息、转义控制字符、限长）再进入诊断；Agent 清理失败的日志同样带上这些元数据。
  - Linux 多 UID 测试：`runner_identity_linux.rs` 新增用例，用真实的不同 UID 验证属主检测，并锁住 Git 真实的
    `dubious ownership` 文案；经变异验证（去掉预检与分类后该用例失败）。
  - 外部 API 的公共错误码白名单加入 `git_*` 稳定错误码，否则外部客户端只会看到 `agent_error`。
- **仍未实现（待办）**：
  - “仅限 `apps/deployments`”：代码只限定在 `work_root` 之内。要么落实，要么把文字改为“限定在 work_root 内”。
  - 特权 release（Agent 以 `deploy-go-agent` 用户物化）没有 prepare 那样的残留清理，KTD6 要求的单独复核没有记录。


## Goal Capsule

- **目标：** 修复两阶段 Git 部署中 Agent 与 runner 重复物化同一 checkout，导致 Linux `dubious ownership`、权限不一致和残留目录无法复用的问题。
- **核心方案：** prepare 阶段由 `deploy-go-runner` 单一负责 Git clone/fetch/checkout；Agent 负责任务输入校验、路径安全检查和受管残留目录清理，不再先创建 Git checkout。
- **兼容策略：** 不修改业务应用、业务仓库或业务发布物；不改变 full/sparse 物化策略、固定 commit、prepare/release 协议语义；旧任务失败后通过新任务重试，不改写已持久化任务快照。
- **安全边界：** 不使用全局或通配 `safe.directory`；不通过放宽目录权限掩盖执行身份错误；清理只允许受管 `apps/deployments` 路径。
- **上线边界：** 代码、测试、文档和 Agent 发布物属于本项目；正式控制面部署和节点 Agent 升级需另行获得明确授权，不能由本计划自动执行。

## Problem Frame

当前 Git prepare 链路存在两次物化：

1. `Executor::execute_prepare` 在 `agent/src/executor.rs` 中先以 `deploy-go-agent` 执行 Git checkout。
2. 生成的 `RunnerSpec` 仍携带 `repository_url`、Git 凭证和物化策略。
3. runner broker 再以 `deploy-go-runner` 启动 runner；`agent/src/runner.rs` 根据 `repository_url` 对相同 checkout 再次执行 fetch/checkout。
4. 两个用户交替创建或复用同一个目录，Git 的 ownership protection 或后续文件写权限检查失败。

本次反馈中的 `fatal: detected dubious ownership in repository` 已证明错误分类修复有效，但没有消除执行责任重复。节点上已存在的 runner-owned、agent-owned 或半成品目录还会使单次字段修改无法恢复历史状态。

## Product Contract

### Requirements

- R1. prepare 的 Git 来源只执行一次 clone/fetch/checkout，执行用户固定为 `deploy-go-runner`。
- R2. prepare runner 创建的 checkout、其中的 `.git`、工作树和必要父目录具有 runner 可持续读写的所有权与组权限；不得依赖 `safe.directory` 绕过 ownership protection。
- R3. Agent 在启动 runner 前必须验证 checkout 路径位于受管工作根目录、路径祖先无符号链接，并按受管规则清理可识别的历史 checkout 残留；清理失败必须返回独立稳定错误。
- R4. runner 复用目录时必须区分：runner 自有合法 Git 仓库、可安全清理的非 Git/半成品目录、所有权或权限不可信目录；不得把三者统一报告为“不是 Git 仓库”。
- R5. full 和 sparse 两种 Git 物化策略均由 runner 执行；缺省策略仍为 full，显式 sparse 的路径校验、partial clone 和工作区校验保持现有语义。
- R6. Git prepare 失败仍返回认证、网络、超时、commit 不可用、目录清理和 sparse 相关的具体错误分类，并保留脱敏诊断。
- R7. release 行为保持现有分模式契约：artifact/workspace artifact 不触发 Git；跨节点 Git release 只保留一个物化责任；同一部署的 prepare/release 不因本修复隐式改变来源 commit 或 materialization snapshot。
- R8. 现有持久化 `RunnerSpec`、journal 和部署 snapshot 可继续解析；新代码不修改历史任务内容。旧失败任务通过新任务重试完成恢复。
- R9. 不修改任何业务应用代码、配置、脚本、仓库内容或发布物。

### Acceptance Examples

- AE1. 新 prepare Git 任务中，Agent 不先创建 checkout；runner 以 `deploy-go-runner` 创建并完成 full checkout，业务 `make deploy-go-prepare` 成功。
- AE2. 新 prepare sparse 任务中，runner 完成 partial clone、sparse checkout、固定 commit 校验和干净工作树校验，未声明的文档目录不进入工作区。
- AE3. runner-owned 合法 checkout 可被同一 runner 用户复用；不会出现 `dubious ownership`。
- AE4. agent-owned、runner-owned 半成品、缺少 `.git` 的目录在受管范围内可安全恢复；无法删除时返回 `git_checkout_cleanup_failed`，不伪装为认证或仓库错误。
- AE5. checkout 目录存在但 owner/group/mode 不符合策略时，不使用 `safe.directory=*`；系统能明确记录 ownership/permission 诊断并采取受管清理或失败。
- AE6. artifact/workspace release 不因 prepare 责任调整而执行额外 Git；跨节点 Git release 仍只执行一次 Git 物化并完成 release。
- AE7. 旧任务 spec 含 `repository_url` 时仍能解析、恢复和收尾；新 prepare 任务生成的 spec 不会让同一 checkout 被 Agent 和 runner 重复物化。
- AE8. Git 凭证只按既有 runner credential copy 机制提供给 runner；任务终止后 `git-key` 和 `runner-git-key` 均清理。

## Scope Boundaries

**本期交付：** Agent executor、runner、Git 物化与清理逻辑；协议结构如需调整仅限兼容性所需的现有字段语义；相关单元测试、集成测试、安装/恢复 runbook 和诊断说明。

**不在本期：** `--depth 1`、新的 Git ref 管道、业务应用改造、仓库历史重写、节点批量升级、数据库迁移、全局 Git 配置修改和 `safe.directory` 白名单扩大。

## Technical Decisions

- KTD1. **runner 单一物化：** runner 已有受限 Git 凭证副本、业务脚本执行身份和 checkout 写入责任；让它从创建开始拥有工作树，比让 Agent 创建后再交给 runner 更符合现有权限模型。
- KTD2. **Agent 不创建 prepare checkout：** `execute_prepare` 保留 Git 来源、commit、物化策略和路径的结构化校验，但把 Git 执行延后给 runner；prepare RunnerSpec 继续携带 repository、credential 和 materialization 输入供 runner 完成唯一一次物化，Agent executor 不再提前调用 Git。
- KTD3. **清理与物化分离：** Agent 可在受管目录中清理历史目录，但不通过 chown 抢占新 checkout 的 owner；runner 负责创建新 checkout，避免两个用户轮流改变 owner。
- KTD4. **不使用 safe.directory 绕过：** ownership mismatch 是权限契约错误。精确 `safe.directory` 只能绕过 Git 检查，不能保证业务脚本写权限，也会削弱安全边界。
- KTD5. **旧任务兼容优先解析、非旧任务重跑：** 持久化 spec 保持反序列化兼容；已在运行或已失败任务不在线改写，升级后通过新 deployment/retry 验证。
- KTD6. **release 单独复核：** prepare 的责任调整不能顺带改变 release。Artifact/workspace artifact 继续由现有物化函数生成；跨节点 Git release 必须确认 Agent/runner/root executor 中只有一个真正执行 Git 的路径。

## Implementation Units

### U1. 明确 prepare 物化责任并调整 RunnerSpec

**目标：** 消除 prepare 阶段 Agent executor 与 runner 的重复 Git 操作，同时保留完整的来源校验、凭证隔离、materialization 和错误诊断能力。

**涉及文件：**

- `agent/src/executor.rs`
- `agent/src/runner.rs`
- `agent/src/runner_service.rs`
- `agent/src/task_handler.rs`（仅在 release/prepare 调度边界需要同步时）
- 对应 Agent 测试模块

**实施要点：**

- 在 `execute_prepare` 中保留 `validate_git_source`、`validate_two_stage_paths` 和物化策略校验。
- 移除 Agent executor 对 prepare checkout 的实际 Git clone/fetch/checkout，避免其创建目录或 `.git`。
- 保留 prepare RunnerSpec 的 repository、credential、commit 和 materialization 输入，并明确这些字段只供 runner 的唯一物化步骤消费；不要破坏 event context、环境变量和 commit 摘要。
- runner 只在明确允许的阶段和来源模式执行 Git；禁止因历史字段存在而对 artifact/workspace artifact 执行 Git。
- 保持 prepare 失败时 runner 能将具体 Git 错误写入 completion/journal，并由 Agent 正确回传。

**测试场景：**

- prepare spec 生成不会调用 Agent Git 函数。
- runner 收到 Git prepare spec 后只执行一次物化。
- workspace prepare 不触发 Git，Git prepare 仍要求合法 repository URL 和 commit SHA。
- Git 错误码及 diagnostics 不回退为泛化目录错误。

### U2. 受管 checkout 残留清理与权限策略

**目标：** 处理旧版本产生的 agent-owned、runner-owned、非 Git 半成品和失败 clone 目录，避免重试复用时再次触发 ownership 或权限错误。

**涉及文件：**

- `agent/src/git.rs`
- `agent/src/executor.rs`
- `agent/src/storage_cleanup.rs`
- `agent/src/dir_guard.rs`（仅在现有目录守卫需要复用时）

**实施要点：**

- 在 Git 物化前校验 checkout 位于允许的 `work_root/apps/deployments` 受管范围，拒绝符号链接祖先和路径逃逸。
- 对缺少 `.git` 的目录继续执行已有受管恢复；对 ownership 不可信的 Git 目录不得直接 fetch，按可验证的受管清理策略处理。
- 清理失败返回独立 `git_checkout_cleanup_failed`，诊断包含稳定错误码和脱敏权限事实，不泄露凭证或完整敏感路径。
- 不由 Agent 对新 checkout 执行 chown；新目录由 runner 创建并继承正确 owner/group/mode。
- cleanup 逻辑必须能删除 runner 创建的终态 checkout，且不能删除活动任务、非受管目录或其他部署目录。

**测试场景：**

- 合法 runner-owned checkout 复用成功。
- agent-owned checkout 可由受管清理逻辑回收后重新创建。
- 缺少 `.git`、普通文件替代目录、符号链接和路径越界分别返回预期错误。
- 无权限删除时保留现场并返回 cleanup 错误，不误报为 Git repository invalid。
- storage cleanup 不删除活动任务或仍有后续 release 的 checkout。

### U3. release 路径与协议兼容复核

**目标：** 确认 prepare 修复不会破坏 artifact、workspace artifact、同节点 release 和跨节点 Git release。

**涉及文件：**

- `agent/src/task_handler.rs`
- `agent/src/executor.rs`
- `agent/src/runner.rs`
- `agent/src/runner_service.rs`
- `agent/src/storage_cleanup.rs`
- `docs/runbooks/application-onboarding.md`
- `docs/runbooks/agent-recovery.md`

**实施要点：**

- 对每种 `ReleaseCheckoutMode` 画出实际唯一物化者，并在代码中保持单一执行路径。
- artifact/workspace artifact 只使用发布物 materialization，不读取 Git 凭证或 repository URL。
- 跨节点 Git release 保留 materialization policy、commit SHA 和 credential lease 的 snapshot 语义；确认 Agent、runner 和 root executor 不会重复 checkout。
- 保持旧字段可反序列化；必要时增加内部阶段标识或 capability，而不是复用协议版本表达新语义。
- 同步恢复手册，说明新版本部署后应创建新部署验证，旧失败部署不修改快照。

**测试场景：**

- prepare -> same-node release 使用正确 checkout/artifact 生命周期。
- cross-node Git release 只执行一次 checkout。
- artifact 和 workspace artifact release 在无 Git 凭证时成功。
- 旧 RunnerSpec fixture 可读取、恢复并正常终止。
- 新旧 Agent capability/协议协商不会把旧节点误判为支持新行为。

### U4. 诊断、测试矩阵与发布验收

**目标：** 让线上问题能区分执行用户、owner、权限、Git 认证和网络失败，并提供可重复的验证闭环。

**涉及文件：**

- `agent/src/git.rs` 测试
- `agent/src/executor.rs` 测试
- `agent/src/runner.rs`/`runner_service.rs` 测试
- `agent/src/storage_cleanup.rs` 测试
- `docs/runbooks/agent-recovery.md`
- 必要时 `docs/standards/agent-installation-contract.md`

**实施要点：**

- 增加执行身份、owner/group/mode、物化阶段和 cleanup 结果的非敏感诊断字段。
- 不记录私钥、token、Env 明文、完整内部敏感路径或原始 credential 内容。
- 在 Linux 隔离测试中使用不同 UID/GID 验证 runner 创建目录、Agent 清理目录和 Git ownership protection。
- 验证安装发布物仍保持 Agent、runner broker、executor 成对版本；不在本计划中自动升级真实节点。

**测试场景：**

- Agent 全量单元测试和 Git 集成测试。
- runner isolation/安装契约测试。
- prepare/release 状态机、reconcile、retry 和 cleanup 回归测试。
- `cargo fmt --check`、`git diff --check` 及相关 workspace 检查。
- 测试节点上仅在另行授权后执行一条新部署验证；验证前确认节点 Agent 已安装包含本修复的成对发布物。

## Verification Contract

### Required checks

1. `cargo test -p deploy-go-agent`
2. Agent Git、runner isolation、storage cleanup 相关聚焦测试
3. `cargo fmt --check`
4. `git diff --check`
5. `make agent-install-check`
6. `make agent-runner-isolation-check`（Linux/容器条件满足时）
7. 对照本计划 AE1-AE8 逐项记录结果

### Runtime verification, separately authorized

- 控制面升级或发布不自动包含在代码实现中。
- 节点 Agent 安装、重启、清理和真实部署必须由当前对话对具体节点单独授权。
- 线上验证应检查：prepare 的 checkout owner、runner 身份、Git error code、runner stderr/completion、业务脚本是否实际启动，以及 cleanup 后目录状态。
- 失败时保留任务 ID、deployment ID、错误码和脱敏权限事实，不修改业务项目代码或业务仓库。

## Rollout And Recovery

1. 本地完成 U1-U4 和聚焦测试。
2. 生成仅 `linux/x86_64` 的 Agent/runner/executor 配对发布物，并执行发布物校验。
3. 先在隔离环境验证新 prepare；确认没有重复 Git 物化后，再按用户授权部署控制面。
4. 由节点管理员逐个升级 Agent；未升级节点继续按旧实现运行，不通过控制面向其下发新假设。
5. 选择一个非关键 Git 应用进行新部署验证，确认其 `commit_sha`、来源策略和 release_version 关系未被改变。
6. 失败时优先回滚成对 Agent 发布物并保留任务目录；不手工设置全局 Git safe.directory，不删除业务工作目录。

## Definition Of Done

- D1. prepare Git checkout 只有一个物化执行者，且创建者为 `deploy-go-runner`。
- D2. 旧 owner/半成品目录不会再次导致 `dubious ownership` 或泛化目录错误；无法安全恢复时有稳定 cleanup 错误。
- D3. full、sparse、workspace、artifact、same-node release 和 cross-node Git release 的既有契约均有测试证明。
- D4. 旧任务和旧 spec 可解析，新的部署不会修改历史 snapshot；协议和发布物兼容矩阵已更新。
- D5. 诊断和 runbook 能指导定位执行用户、目录 owner/group/mode、凭证、网络和 cleanup 问题。
- D6. 所有改动只在 `deploy-go` 仓库，未触碰任何业务应用代码、配置、脚本、仓库或发布物。
- D7. 必要本地验证通过；真实控制面部署和节点升级仅在明确授权后执行并有回滚记录。

## Active Execution Units

- [x] U1 prepare 物化责任与 RunnerSpec（已核实，见“实施状态”）
- [ ] U2 checkout 残留清理与权限策略
- [ ] U3 release/协议兼容复核
- [ ] U4 诊断、测试矩阵与发布验收
