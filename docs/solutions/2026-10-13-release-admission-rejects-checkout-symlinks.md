# release 准入因 checkout 含符号链接被拒绝，且原因被吞掉

## 现象

2026-10-13 部署 `deployment_01M4SMDXJJ7YGS2QR7GE04N02Z`（test 环境，业务应用 `qfy-voucher-hub`）：

- prepare 阶段成功，5 个模块产物 58.5 MB 上传并校验通过；
- release 阶段下载产物后约 2 秒内失败，`error_code = privileged_release_admission_failed`；
- 控制面只有这个错误码：任务没有进入 executor，节点也没有任何诊断日志，部署日志停在“发布物下载完成，开始校验并解压”。

同一应用、同一 Agent 此前连续成功；产物形状（大小、文件数、manifest 结构）与最近一次成功的完全一致。

## 根因

业务仓库在本次部署的提交范围内新增了 3 个被 git 跟踪的符号链接（`.agents/skills/project-*`，来自提交
“切换主 AI 工作入口为 Claude Code”）。上一次成功的提交里符号链接数为 0。

Agent 在发 `ReleaseStart` 之前会对 checkout 目录计算摘要（`release::directory_digest`，跳过根部 `.git`）。
该函数按 `docs/standards/privileged-agent-executor.md` 的要求**拒绝符号链接、硬链接（nlink≠1）和任何非普通文件**，
因为 checkout 之后会由 root executor 复制并封存。所以拒绝本身是设计内的安全行为，不是回归：
`privileged_release_facts`、`release.rs` 在 0.3.30 → 0.3.31 之间没有改动。

真正属于本项目的缺陷是**可诊断性**：`privileged_release_facts` 的 8 个出口全部是
`.map_err(|_| "privileged_release_admission_failed")`，把“哪一步、哪个文件、什么类型”全部吞掉。

## 排查路径（可复用）

1. 部署记录 → target run 的 `error_code` → 对应 `agent_tasks.result_json` 和 `agent_task_events`，确认失败发生在哪个阶段；
2. 在代码里 grep 该错误码，列出所有产生它的出口，逐一核对与现场时间线是否吻合；
3. 对比同一应用最近成功与本次失败的产物（`deployment_artifacts` 的大小、文件数、manifest）以排除“产物异常”；
4. 对比两次部署所用业务提交的 git 树：`git ls-tree -r <commit> | awk '$1==120000'`（符号链接）、`160000`（子模块）。
   本次差异就是新增的 3 个 `120000` 条目。

## 已做的修复（本项目内）

- `privileged_release_facts` 拆分为稳定错误码：`privileged_release_checkout_unsafe_entry`、
  `privileged_release_checkout_invalid`、`privileged_release_manifest_invalid`、`privileged_release_env_invalid`、
  `privileged_release_env_mismatch`；`privileged_release_admission_failed` 只保留给内部异常；
- 新增 `release::find_unsafe_entries`，只读元数据地列出被拒绝条目（相对路径 + 类型），不读文件内容；
- 准入失败时 Agent 把原因写进部署日志（仅相对路径）并记录 `privileged_release_admission_rejected` 诊断事件；
- 外部 API 公共错误码白名单加入这些错误码，避免被降级成 `agent_error`；
- `docs/runbooks/deployment-recovery.md` 补充处理方式。

这些改动在 Agent 侧，**需要发布新版本 Agent 并升级节点后才会在现场生效**。

## 未决策项

是否放宽“checkout 不得含符号链接”的规则，属于特权执行边界的策略变更，需要单独评估：

- 业务仓库里常见与发布无关的符号链接（AI 工具配置、开发软链），目前会让整次发布失败；
- 若放宽，只应接受**指向 checkout 内部、相对路径、不越界**的符号链接，并在封存 bundle 时按符号链接原样复制，
  同时在摘要里纳入链接目标；任何越界、绝对路径或链式越界必须继续拒绝；
- 这需要同步修改 executor 的封存逻辑、`docs/standards/privileged-agent-executor.md` 与对应测试，并安排独立复核。

在决策之前，业务侧的应对方式是把这些符号链接从仓库移除（或改为不被 git 跟踪的本地文件）。

## 教训

- `.map_err(|_| "固定错误码")` 在特权发布链路里等于把现场诊断信息扔掉；错误要带稳定的原因码，
  必要时在失败路径上补一次只读的诊断采集。
- 失败路径上的诊断采集要保持只读、限量、不含内容与绝对路径。
