# release 准入因 checkout 含符号链接被拒绝，且原因被吞掉

## 现象

2026-10-13 部署 `deployment_01M4SMDXJJ7YGS2QR7GE04N02Z`（test 环境，业务应用 `qfy-voucher-hub`）：

- prepare 阶段成功，5 个模块产物 58.5 MB 上传并校验通过；
- release 阶段下载产物后约 2 秒内失败，`error_code = privileged_release_admission_failed`；
- 控制面只有这个错误码：任务没有进入 executor，节点也没有任何诊断日志，部署日志停在“发布物下载完成，开始校验并解压”。

同一应用、同一 Agent 此前连续成功；产物形状（大小、文件数、manifest 结构）与最近一次成功的完全一致。

## 根因

业务仓库在本次部署的提交范围内新增了 3 个被 git 跟踪的符号链接（`.agents/skills/project-*`，来自当次AI工作入口调整提交）。上一次成功的提交里符号链接数为 0。

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

## 决策与实现（已完成）

按项目铁律“业务应用的问题必须通过本项目解决”，不要求业务仓库删除符号链接，而是在平台侧放宽，且保持特权边界不变：

- **放行范围**：仅 checkout 根部隐藏目录（第一级名称以 `.` 开头，如 `.agents`、`.github`）内的符号链接。
  这类链接是 AI 工具/编辑器配置的常见共享方式，与发布无关。
- **做法是丢弃而不是保留**：这些链接不进入 checkout 摘要，也不被复制进封存 bundle；不跟随、不解析、不校验目标。
  root 进程永远接触不到任何符号链接，所以攻击面比之前更小，而不是更大。
- **仍然拒绝**：非隐藏位置的符号链接、所有硬链接、所有非普通文件（包括隐藏目录里的）。
- **不可被绕过**：Agent 与 executor 共用 `release::checkout_tree_digest`，准入时比较一次、封存复制后再比较一次；
  把已授权的普通文件换成隐藏符号链接会使文件从摘要里消失，因而摘要不一致被拒绝（有两层校验，测试在两层都关闭时才失败）。
- **可见性**：Agent 在部署日志里 `[WARN]` 列出被忽略的链接（仅相对路径），避免脚本悄悄依赖它们。
- **未采用的方案**：全局忽略所有符号链接（源码目录里的链接被静默丢弃可能让脚本悄悄用了默认配置）；
  保留并校验内部相对链接（要在 root 路径上新增链接解析与封存逻辑，风险和复杂度高得多）。

## 顺带解决的无关偶发失败

Linux 容器里 `runner_service::tests::client_launches_only_a_fixed_task_spec` 曾偶发失败
（`launch` 返回 `runner_spawn_failed`，客户端映射成 `PermissionDenied`），在没有符号链接改动的基线上同样出现。

- **根因**：单元测试里 `runner_executable()` 返回 `current_exe()`，即**测试二进制本身**。它把参数
  `runner-stdin <dir>` 当作 libtest 的测试过滤器，跑完 0 个测试就退出且从不读 stdin；父进程随后写 spec 时
  子进程已退出，`write_all` 得到 `BrokenPipe`。是“子进程先退出还是父进程先写入”的竞态，Linux 容器里约 1/3 概率。
  生产里的 runner 是真正的 Agent 二进制，会读 stdin，不受影响。
- **定位方法**：先猜“socket 就绪时序”，改完失败率没有下降（8 次仍失败 3 次），已撤销；之后临时打印
  `write_all` 的具体错误并循环复现，直接看到 `BrokenPipe`。教训：偶发失败要先拿到真实错误再动手，不要靠猜。
- **修复**：仅测试编译的 `TEST_RUNNER_EXECUTABLE` 覆盖点，测试注入一个把 stdin 读到 EOF 的脚本；
  生产构建不含这段代码。修复后 Linux 容器完整套件连续 20 次 0 失败；撤掉注入后 12 次失败 4 次，因果成立。

## 另一个顺带解决的偶发失败：connection 遥测测试

`agent/tests/connection.rs::v12_connection_sends_immediate_telemetry_without_delaying_heartbeat` 在 Linux 容器里单独跑 15 次失败 10 次
（macOS 上不出现），是最早那批“10 个 Linux 容器失败”里留下的最后一个。

- **根因**：测试用 `start_paused = true`（虚拟时间），循环里每轮 `advance(5s)`；而遥测采集在**真实线程**（`spawn_blocking`）里执行，
  外面套的 `timeout(TELEMETRY_COLLECTION_TIMEOUT = 10s)` 用的是**虚拟时间**。容器里线程调度稍慢时，虚拟时间两轮就跳过 10 秒，
  采集超时、遥测任务 `break`，永远等不到 `sample_sequence == 1`。生产用真实时间，不受影响。
- **修复**：先不推进虚拟时间，用真实时间等首份遥测发出（有 10 秒真实时间兜底），再推进虚拟时间等心跳。
  修复后 20 次 0 失败；撤掉后 12 次失败 6 次。

## 教训

- `.map_err(|_| "固定错误码")` 在特权发布链路里等于把现场诊断信息扔掉；错误要带稳定的原因码，
  必要时在失败路径上补一次只读的诊断采集。
- 失败路径上的诊断采集要保持只读、限量、不含内容与绝对路径。
- 测试里混用“虚拟时间”和“真实线程”时，推进虚拟时间前要先等真实线程的结果，否则只是在赌线程调度速度。
- 偶发失败要先量化失败率、拿到真实错误或因果证据，再动手；两个测试的偶发失败（runner stdin、connection 遥测）都是这样定位的。
