---
title: Git 来源稀疏物化与部署契约计划
date: 2026-09-18
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# Git 来源稀疏物化与部署契约计划

## 实施状态（2026-10-13 对照代码核实）

**状态：大部分已实现并已发布，有缺口。** 核实方式是逐条读代码，不是看提交信息；线上 11 个应用来源目前没有一个启用 sparse。

- **已实现并发布**：提交 `a9dd627`（API 来源策略与校验、migration 0037、snapshot 固化）、`4415e8a`（协议与 dispatcher 门禁，capability `git_sparse_checkout_v1`）、
  `36b6968`（Agent partial clone + sparse checkout）、`56ec7f7`（管理端配置）、`8157194`（恢复 full 的历史行为）、`58af9fb`（检出统一由 runner 执行）。
- **本次核实后修复**：non-cone 的 sparse-checkout 里不含斜杠的名字（如 `api`、`Makefile`）不锚定，会匹配任意层级的同名路径，
  物化范围超出声明（用真实 Git 复现：声明 `api` 也物化了 `docs/api`、`src/api`、`vendor/pkg/api`）。Agent 现在统一加前导 `/` 锚定到仓库根，
  并有真实 Git 回归测试；同时特权 release 跨节点检出不再把具体错误码压成 `git_checkout_failed`。
- **与计划不一致，已按现状裁定**：
  - full 模式**不使用** partial clone（`8157194` 有意恢复历史行为）；R7/R8 里“full 允许回退”不再适用，full 就是完整克隆。
  - 目录不会被规范化为 `dir/**`（API 无法判断路径是文件还是目录）；目录需显式写 `/**`，裸目录名经锚定后同样只匹配根部目录。
  - 没有 `source_materialization_invalid` / `source_materialization_paths_required` / `git_sparse_checkout_failed`：API 用通用 validation，
    `sparse-checkout set` 失败映射为 `git_sparse_checkout_unavailable`。
- **计划过时**：协议版本（当前 v18，v16 schema 已冻结）；检出不再由 `execute_prepare` 里的 Agent 执行，而是 runner 在业务脚本前执行（见 2026-09-20-002 计划）。
- **仍未覆盖的测试**（待办）：v16 schema/payload 往返与旧 Agent 拒绝、migration 0037 断言、snapshot/跨节点 release 的 sparse 集成断言、
  非法保存不递增 `source_version`、旧预览失效、sparse 不回退 full、符号链接越界、管理端 sparse 用例。


## Goal Capsule

- **目标：** 允许管理员在 Git 来源配置中声明构建所需的目录和文件；Deploy Go Agent 使用 partial clone + sparse checkout，只下载并物化构建需要的工作区内容，减少文档、截图、备份和其他非构建资料带来的网络、磁盘和 checkout 成本。
- **兼容策略：** 没有 `source_materialization` 配置时默认 `full`，现有 Git 部署行为不变；只有显式配置 `mode: "sparse"` 才改变检出范围。
- **核心边界：** Git clone、fetch、checkout、凭证、固定 commit 校验和工作区完整性仍由 Agent 管理；业务 `deploy-go-prepare` / `deploy-go-release` 脚本不拉代码、不切换 ref、不决定源码范围。
- **不改变的事实：** 固化 commit、两阶段状态机、发布物校验、Env 同步、workspace 来源和 release 权限链路保持原语义。
- **落地方式：** 先完成契约、快照、协议和 Agent 的闭环，再补管理端配置与文档；未获得当前对话针对具体环境的明确授权前，不执行真实节点部署。

## Problem Frame

当前 Git 两阶段部署首次 clone 使用 `--no-checkout`，随后由 Agent checkout 固化 commit。即使采用 `--filter=blob:none`，完整 checkout 仍会按需下载整个目标树；当项目源码只有数 MB、`docs/`、截图、备份和临时资料却持续膨胀时，prepare 仍承担不必要的网络传输、磁盘占用和 checkout 时间。

用户需要一个可审计、可复现且向后兼容的选择机制：默认仍完整检出；应用明确声明构建输入后，Agent 才按 allowlist 进行 sparse checkout。配置必须随来源版本和 deployment snapshot 固化，不能由部署请求参数或业务脚本临时决定。

## Product Contract

### Requirements

- R1. Git 来源支持可选 `source_materialization` 配置，形状为 `{ "mode": "full" | "sparse", "paths": [...] }`。
- R2. 未配置该字段等价于 `mode: "full", paths: []`；已有来源、已有快照和旧 Agent 不因新增能力改变行为。
- R3. `full` 模式完整检出；`sparse` 模式只物化 `paths` allowlist。空路径列表在 `sparse` 模式下是配置错误，不能默认为 full 或空工作区。
- R4. allowlist 只接受相对 POSIX 路径和受限目录模式：普通文件路径、目录路径，以及以 `/**` 结尾的目录递归模式。目录路径在规范化时转换为递归目录规则；拒绝绝对路径、`.`、`..`、反斜杠、控制字符、重复项、空项和路径逃逸；实现不得把任意 shell 或任意 Git 命令交给用户。
- R5. 来源配置保存、修改和校验时执行结构化验证；物化策略变化必须使旧分支发现/预览失效并递增 `source_version`。
- R6. deployment snapshot 必须保存规范化后的物化策略、`materialization_policy_version`（代码定义的策略格式版本）和来源 `source_version`（已有来源配置版本）；确认、重试和 prepare/release 使用同一份策略，不重新读取当前应用来源。
- R7. Agent 首次 Git 检出采用 partial clone，并在 `sparse` 模式执行 sparse checkout；随后必须校验 detached `HEAD` 等于固化 commit，并执行既有干净工作区校验。
- R8. partial clone 或 filter 不可用时，`full` 模式允许回退现有全量 clone；`sparse` 模式不得静默回退 full，必须返回稳定的配置/检出错误。
- R9. 跨节点 Git release 必须携带并执行与 prepare 相同的物化策略；同节点 release 复用 prepare 工作区时不得再次改变工作区。
- R10. workspace 来源不使用 Git sparse checkout，继续执行现有 workspace snapshot 与 `workspace_artifact` 逻辑。
- R11. 管理端在 Git 来源配置页提供 full/sparse 选择和路径编辑；普通用户只读最终来源策略，不能在部署时修改。
- R12. 文档、API/OpenAPI、Agent 协议 schema、Rust/TypeScript/Dart 生成模型和测试必须同步更新。
- R13. sparse 工作区校验必须处理 Git tracked symlink：允许目标仍在 checkout 根目录内且符合物化范围的链接；解析后越出 checkout 根目录的链接直接失败，不能让业务脚本通过链接读取工作区外内容。

### Key Flows

- F1. **配置 Git 物化策略：** 管理员编辑 Git 来源，默认显示 full；选择 sparse 后录入 allowlist，服务端验证并保存来源版本，旧 refs/预览失效。
- F2. **预览与快照：** 预览读取已验证来源的策略，生成包含规范化策略和策略版本的 snapshot；部署确认后 immutable snapshot 成为唯一执行输入。
- F3. **prepare：** Agent 从 snapshot 收到策略，partial clone/fetch，按 full 或 sparse 物化固化 commit，校验工作区后执行固定 `deploy-go-prepare`。
- F4. **release：** 同节点继续使用 prepare 工作区；跨节点按同一策略检出固化 commit，再校验发布物并执行固定 `deploy-go-release`。
- F5. **失败与兼容：** 未配置策略的旧部署走 full；sparse 缺少入口文件、策略非法、commit 不可用或 filter 不支持时明确失败，不能隐式扩大源码范围。

### Acceptance Examples

- AE1. 老应用和缺少 `source_materialization` 的来源生成 full snapshot，旧 Agent 可按原 payload 行为完成部署。
- AE2. `{ "mode": "sparse", "paths": ["Makefile", "api/**", "scripts/**"] }` 只物化这些路径；`docs/` 和未声明的大文件不出现在 checkout 工作区。
- AE3. sparse 配置包含 `/tmp/x`、`../secret`、`api\\x`、空字符串或重复路径时，API 返回 validation 错误且不递增来源版本。
- AE4. sparse 配置 `paths: []` 或缺少 `paths` 时拒绝保存；full 配置即使 paths 为空也合法。
- AE5. 分支移动或来源策略修改后，旧预览不能确认；重试使用原 snapshot 中的策略和 commit。
- AE6. partial clone/filter 失败时，full 模式可按既有回退逻辑继续；sparse 模式返回 `git_sparse_checkout_unavailable` 或等价稳定错误，不执行 full checkout。
- AE7. sparse workspace 缺少固定 prepare 入口时，prepare 在业务脚本启动前明确失败，并保留 Git/策略诊断。
- AE8. 跨节点 release 使用与 prepare 相同的策略；workspace 来源和 image/artifact release 不携带 Git 物化策略。
- AE9. 新增一个未声明的大文档不会进入 sparse 工作区；新增构建所需路径但未更新契约时，部署明确失败而不是“碰巧”成功。
- AE10. 管理端保存、刷新、取消编辑和普通用户只读显示均保持现有权限、CSRF、并发版本和未保存保护。

### Scope Boundaries

**本期交付**

- Git 来源配置与数据库保存；部署预览/snapshot 固化；Agent prepare 和跨节点 release 的 Git sparse checkout；full 默认兼容；协议/schema/API/Web/测试/文档同步。

**Deferred for later**

- `--depth 1` 与 `requested_ref` 管道；Git LFS 专项优化；构建缓存和对象复用；按模块自动推导 allowlist；远程服务端仓库裁剪；Tag 来源策略。

**Outside this product's identity**

- 让业务两阶段脚本自行 clone、过滤或切换 Git；将 arbitrary shell/Git pattern 暴露给用户；自动分析构建依赖并修改 allowlist；workspace 来源强行套用 Git sparse checkout；部署时临时传入源码路径名单。

## Planning Contract

### Product Contract Preservation

本计划根据当前会话已确认的兼容决策和现有 `git-branch-deployment-contract` 建立。既有 Git 两阶段契约的 Agent 管 Git、固定 commit、两阶段脚本边界和 workspace 独立语义保持不变；本计划只新增可选源码物化能力。

### Key Technical Decisions

- KTD1. **来源级 allowlist，不使用 ignore list：** 使用 `source_materialization.mode` 和 `paths` 表达构建输入。allowlist 更容易证明构建输入完整，也避免新增长文档意外进入工作区；忽略名单在仓库增长后容易产生反向遗漏。
- KTD2. **显式模式，缺省 full：** 服务端/API/Agent 对缺省字段统一按 full 解释；不会根据 paths 是否存在猜测模式。`sparse` 必须显式且 paths 非空。
- KTD3. **策略放在 Git 来源，不放参数 Schema：** 物化策略描述源码获取边界，不是业务脚本参数或部署后验证规则；保存到 `application_sources`，并在 snapshot 中冻结。
- KTD4. **Git 仍由 Agent 管理：** Agent 执行 clone/fetch、sparse-checkout、detached checkout、SHA 校验、凭证隔离、超时、取消和审计；两阶段脚本只消费工作区。
- KTD5. **受限路径语法：** 首版支持 literal file、literal directory 和 `literal/**`；literal directory 规范化为递归目录规则，不接受任意 Git sparse pattern。API 和 Agent 双侧都验证规范化结果，避免不同实现对 glob 的解释不一致。
- KTD6. **sparse 不自动回退 full：** 回退会掩盖契约漏配并可能把敏感/大体积资料暴露给构建；只允许 full 模式保留现有 filter 失败回退。
- KTD7. **协议向后兼容：** 当前协议为 v15；sparse payload 使用下一协议版本和独立 capability（实施时按发布流程落为 v16），旧 v15 Agent 不能接收 sparse 字段。full/缺省任务不序列化 sparse-only 字段，因此仍可发送给现有 v11-v15 Agent；不复用历史协议版本号。
- KTD8. **同一策略跨阶段复用：** prepare 与跨节点 release 都从同一 deployment snapshot 生成 payload；不能只优化 Build Agent 而让 Target Agent 全量检出。
- KTD9. **入口完整性由执行阶段证明：** 平台不猜测 Makefile、语言或框架依赖；Agent/脚本在 sparse 工作区缺少入口时失败，诊断包含缺失路径和策略摘要但不泄露凭证。
- KTD10. **prepare 的源码物化前置：** 当前 `agent/src/executor.rs::execute_prepare` 只校验 Git 来源并创建 runner spec，`agent/src/runner.rs::run_spec` 现有 Git checkout 条件只覆盖 release；本功能必须把 Git prepare checkout 明确放到业务 `make deploy-go-prepare` 之前，由 Agent 完成，而不是依赖旧 checkout 目录或业务脚本偶然创建源码。
- KTD11. **符号链接边界：** Git tracked symlink 不是普通文件；checkout 后按 dirfd/no-follow 或等价路径解析检查其目标，允许 checkout 根内链接，拒绝越界链接并返回 `git_sparse_checkout_invalid`，避免 allowlist 只检查 Git 名称而漏掉实际访问边界。

### High-Level Technical Design

```text
Git source config
  -> validate + normalize policy
  -> source_version++ / invalidate refs
  -> preview snapshot {policy, policy_version, commit}
  -> prepare/release payload
  -> Agent: partial clone/fetch
  -> Agent: full checkout OR sparse-checkout paths
  -> HEAD/clean-worktree validation
  -> fixed deploy-go-prepare / deploy-go-release
```

建议的持久化形状：

```json
{
  "mode": "sparse",
  "paths": ["Makefile", "api/**", "scripts/**"]
}
```

`application_sources` 保存规范化 JSON；部署 snapshot 保存同样的不可变值、来源 `source_version` 和 `materialization_policy_version`。其中前者用于判断来源配置是否变化，后者用于判断 Agent 是否理解策略格式。未配置值在读取和写入边界都归一化为：

```json
{"mode":"full","paths":[]}
```

Agent 的 Git 单元按以下方向实现：首次目录不存在时 `clone --filter=blob:none --no-checkout`；已有目录执行 `fetch --prune origin`；sparse 模式初始化 non-cone sparse checkout，通过受限路径清单写入规则，再 checkout 固化 SHA；full 模式保持现有全量 checkout 语义。实际命令参数、Git 版本兼容和已有目录清理策略留给实施阶段依据测试 fixture 调整。

### Data Model and API Direction

- `api/migrations/`：为 `application_sources` 增加 `source_materialization_json TEXT NOT NULL DEFAULT '{"mode":"full","paths":[]}'`，使用新 migration，不改历史 migration；复用已有 `source_version`，不新增重复的配置版本列。
- `api/src/application_sources/mod.rs`：扩展 `SaveSourceRequest`、`ApplicationSourceResponse`、`SourceViewRow` 和来源查询/保存 SQL；统一校验/规范化，修改策略时复用现有 source version 递增和 refs 失效流程；管理员可写，普通用户继续只读。
- `api/src/deployments/mod.rs`：将来源策略带入 two-stage preview/deployment snapshot；所有从 snapshot 生成 prepare/release payload 的路径都读取同一策略，workspace/image 分支明确不携带 Git 策略。
- `api/src/agents/dispatcher.rs`：生成 Git prepare 和跨节点 Git release payload 时附带策略；增加 sparse capability/protocol 门禁和稳定错误收敛，full 缺省任务可兼容旧 Agent。
- `agent-protocol/src/lib.rs`、`agent-protocol/schema/`：增加 `SourceMaterialization` 类型、可选 `source_materialization` payload 字段、`GitSparseCheckoutV1` capability 和 v16 schema；当前仓库只有 generic latest schema，没有 v15 独立文件，协议升级时先冻结现有 `agent-control.schema.json` 为 `agent-control-v15.schema.json`，再把 latest schema 更新为 v16，保留既有 v11/v12/v14 历史 schema。
- `agent/src/git.rs`：实现策略解析、路径验证、partial clone、sparse checkout、固定 SHA、干净工作区和稳定错误映射；不让业务 runner 自己执行 Git。
- `agent/src/runner.rs`、`agent/src/executor.rs`、`agent/src/task_handler.rs`：把策略从协议 payload 传入 runner/git；保留身份等待预算、credential lease 和 workspace 分支；release 只在 Git checkout 模式使用策略。
- `admin/src/features/applications/ApplicationSourceSection.tsx` 及 generated API models：增加 full/sparse 控件、路径编辑、错误提示和只读摘要；重新生成或按项目既有方式同步 OpenAPI 客户端模型。
- `docs/standards/git-branch-deployment-contract.md`、`docs/standards/agent-control-protocol.md`、`docs/standards/application-deployment-json.md`：补充来源级物化契约、snapshot 规则、协议门禁和不适用范围；明确它不是 parameter schema/verification config 字段。

### Development-Level Specification

#### Canonical data shapes

API、数据库读取模型和 deployment snapshot 使用同一语义，字段名统一为 `source_materialization`；数据库仅负责保存 canonical JSON：

```json
{
  "mode": "full",
  "paths": []
}
```

```json
{
  "mode": "sparse",
  "paths": ["Makefile", "api/**", "scripts/**"]
}
```

- `mode` 只能是 `full` 或 `sparse`。
- `paths` 在 API response 中始终返回数组；省略配置读取为 `full`/空数组。
- canonicalization：去除路径首尾空白；将目录 `api` 规范化为 `api/**`；按字典序排序；重复项在规范化后拒绝，而不是静默去重。
- 路径限制：单项、总项数、总字节数使用与当前 API payload 限制同等级的硬上限；实施时抽成命名常量并在 API/Agent/schema 测试中引用，避免只在一侧限制。
- `materialization_policy_version` 是策略格式版本常量，首版为 `1`，写入 snapshot；`source_version` 是来源配置变更版本，两者不能互换。

#### API and persistence flow

1. `PUT /api/v1/applications/{application_id}/source` 的 `SaveSourceRequest` 增加可选 `source_materialization`；缺省或 `null` 归一化为 full，未知字段仍由 `deny_unknown_fields` 拒绝。
2. `validate_source_materialization` 返回 canonical typed value 和 canonical JSON；保存前先校验 Git URL、credential、build agent 和物化策略，任一失败都不打开写事务。
3. source update SQL 同时写 `source_materialization_json`，复用现有 `source_version=source_version+1`、清空 `deployment_branch/branch_verified_at`、设为 draft 的原子更新；创建来源写入 full 默认值。
4. `show` response 始终返回结构化 `source_materialization`，旧数据库行通过默认 JSON 反序列化为 full。
5. 分支刷新只绑定 source version；策略变化不能继续使用旧 discovery。若当前实现的版本失效条件只由 URL/credential/agent 触发，应将 canonical policy digest/version 加入同一失效条件，而不是新增旁路状态。
6. `ApplicationSourceResponse` 不必暴露内部 `source_version`；管理端并发编辑继续使用现有 `version`，`source_version` 只用于 discovery、snapshot 和部署一致性。

#### Snapshot and dispatch flow

在 `api/src/deployments/mod.rs`：

- `TwoStageSourceInfo` 和 `VerifiedSourceRow` 增加 typed source materialization 与 source version；`resolve_two_stage_source` 从 verified source 一次读取，禁止 preview 后重新查询策略。
- `build_two_stage_preview` 的 `source` 对象增加 `source_materialization` 和 `materialization_policy_version`；snapshot hash 自动覆盖这两个字段。
- `build_two_stage_script_preview` 不增加 Git 字段；image preview 也不增加 Git 物化字段。
- 所有从 snapshot 生成 `DeploymentPrepareTask` / `DeploymentReleaseTask` 的路径统一读取 `snapshot.source.source_materialization`。同节点 release 可以携带策略但 Agent 不重复 checkout；跨节点 Git release 必须携带策略。
- full 策略序列化时省略 `source_materialization` 字段以保持旧 v15 Agent 的 `deny_unknown_fields` 兼容；sparse 策略必须先通过 `git_sparse_checkout_compatibility`（协议 >= 16 且 capability `git_sparse_checkout_v1`）再建 task。
- compatibility failure 复用现有 `agent_protocol_unsupported` / `agent_capability_unavailable` 稳定错误码，summary 明确要求升级 sparse capability；不得留下 queued task 等待旧 Agent 升级；与现有 workspace/runtime probe compatibility 收敛方式一致。

#### Protocol shape and compatibility

在 `agent-protocol/src/lib.rs`：

```rust
// Directional shape; exact derives and enum naming follow existing protocol style.
struct SourceMaterialization {
    mode: SourceMaterializationMode,
    paths: Vec<String>,
}
```

- 新增 `SourceMaterializationMode::{Full,Sparse}` 和 `AgentCapability::GitSparseCheckoutV1`。
- `DeploymentPrepareTask` 和跨节点 Git `DeploymentReleaseTask` 增加 `#[serde(default, skip_serializing_if = "Option::is_none")] source_materialization`；API 只在 sparse payload 设置该字段，full 任务为 `None`。
- bump `PROTOCOL_VERSION` 到 16，新增 v16 schema；不得修改 v11-v15 schema 的语义。Hello capability、Agent `main.rs` capability 列表、API compatibility query 和 stale-task reconciliation SQL 同步识别新 capability。
- v16 Agent 仍接受 full payload；v15 及更早 Agent 不接受 sparse payload，控制面在 dispatch 前拒绝。
- 协议单测至少覆盖：full payload 无字段可被旧 fixture 解析；sparse payload 只能由 v16 解析；未知 mode/path 规则和 unknown field 被拒绝；协议 schema compatibility fixtures 保持历史版本通过。

#### Agent Git execution

在 `agent/src/git.rs` 扩展现有 `checkout_commit`，把物化策略作为 typed 参数传入，不在 runner 中拼接 Git 参数：

1. 先验证 SHA、策略和 checkout root；策略非法直接返回 `InvalidMaterialization`。
2. 目录不存在：执行 `git clone --no-checkout --filter=blob:none`。full 模式 filter 失败后按现有逻辑清理目录并回退一次全量 clone；sparse 模式 filter/partial clone 失败返回 `SparseCheckoutUnavailable`，不得执行全量回退。
3. 目录已存在：执行现有 `fetch --prune origin`；确认仓库仍是受控 checkout，避免把普通目录当 Git 仓库继续复用。
4. full 模式在 checkout 前禁用遗留 sparse config（若存在），再 detached checkout 固定 SHA。
5. sparse 模式执行 `git sparse-checkout init --no-cone`，通过 stdin 设置 canonical patterns；不把路径作为 shell 字符串拼接，不允许用户控制额外 Git 参数。目录 canonical `api/**` 后再写入规则。
6. detached checkout 固定 SHA 后继续现有 `rev-parse HEAD`、`status --porcelain` 检查；必要时增加 `git ls-files`/工作区边界校验，确保未声明路径没有被物化。
7. 错误映射至少稳定区分：策略非法、partial/sparse 能力不可用、commit 不可用、认证失败、仓库不可达、脏工作区；日志只记录脱敏诊断和策略摘要，不记录私钥、lease 内容或 Env。

在 `agent/src/executor.rs` / `agent/src/runner.rs`：

- `TwoStageRunnerSpec` 传递 typed policy；`execute_prepare` 只对 `SourcePolicy::Branch` 传 Git policy，workspace 仍走 snapshot_workspace。
- 修复当前 prepare checkout 责任缺口：`execute_prepare` 在启动 runner 前调用 Git materialization（或抽取一个由 prepare/release 共用的 Agent checkout preflight），确保 `deploy-go-prepare` 的 `-C checkout_dir` 一定指向本次固化 commit 的工作区；checkout 失败时不得启动业务脚本。
- 保持 release runner 的 checkout 逻辑只用于跨节点 Git release；同节点 release 继续消费 prepare 已建立的 checkout，不能在 release 前重新 full/sparse checkout。
- `execute_release` 仅在 `ReleaseCheckoutMode::Git` 且存在 repository URL 时执行 checkout policy；artifact/workspace_artifact 不执行 Git。
- 保留现有 `identity_wait_budget`，不要为 sparse 功能重新引入固定 5 秒身份窗口；新增测试确保 clone 延迟时 process identity 仍可等待。
- prepare 在业务 `make deploy-go-prepare` 前完成 sparse checkout；固定入口缺失由 make/入口校验产生明确失败，不能先运行脚本再补路径。

#### Admin UI behavior

在 `ApplicationSourceSection.tsx`：

- draft 增加 `sourceMaterialization`，旧 response 缺失时初始化 full。
- full/sparse 使用单选或 segmented control；选择 sparse 后显示多行路径编辑器，按“一行一个路径”转换为 API paths；显示示例和当前 canonical 结果，但不把帮助文字当作配置值。
- 保存按钮提交结构化 paths；前端只做即时格式提示，服务端仍是最终校验；非法/空 sparse 响应显示稳定错误并保留用户输入。
- 只读详情展示模式和路径摘要；普通用户不显示编辑入口。编辑 source 时策略变化与仓库/凭证变化一样触发 dirty、并发 version 和未保存离开保护。
- OpenAPI source request/response、TypeScript generated models 和实际 API mock 必须同一批更新；不得只手工修改某个 generated model。

#### Error codes and observability

计划使用以下稳定错误码，实施时沿用项目既有 `ApiError` / `GitError` 映射：

| 层 | 错误码 | 触发条件 |
| --- | --- | --- |
| API | `source_materialization_invalid` | mode、路径、限制或未知字段非法 |
| API | `source_materialization_paths_required` | sparse 缺少或拥有空 paths |
| API/dispatcher | `agent_protocol_unsupported` / `agent_capability_unavailable` | 目标 Agent 不支持 v16/capability |
| Agent | `git_sparse_checkout_invalid` | Agent 二次校验 payload 失败 |
| Agent | `git_sparse_checkout_unavailable` | partial clone 或 sparse-checkout 能力不可用 |
| Agent | `git_sparse_checkout_failed` | 受限规则写入或 sparse 初始化执行失败 |
| Agent | `git_commit_unavailable` | 固化 SHA 无法 checkout 或 HEAD 不匹配 |
| Agent | `git_dirty_worktree` | checkout 后工作区不干净 |
| Agent | `git_authentication_failed` / `git_repository_unreachable` / `git_timeout` | 沿用现有 Git 认证、网络和超时分类 |

错误 details 只能包含 request/task/deployment 标识、脱敏 repository URL、策略摘要和建议动作；不得包含 credential、private key、token、Env 明文或任意完整内部路径。

### Implementation Units

#### U1. 物化策略模型、校验与来源 API

**目标：** 让 Git 来源可以安全保存显式 full/sparse 策略，并对旧数据默认 full。

**涉及路径：**

- `api/migrations/<next>_git_source_materialization.sql`
- `api/src/application_sources/mod.rs`
- `api/tests/application_sources_api.rs`
- `api/tests/migrations.rs`
- `api/tests/database_constraints.rs`

**测试场景：**

- 缺省字段、显式 full、显式 sparse 的序列化/反序列化和默认值。
- 合法根文件、目录、`dir/**`，规范化排序和重复项拒绝。
- 绝对路径、`..`、`.`、反斜杠、控制字符、空项、过长路径、过多路径和非法 glob 拒绝。
- sparse 空 paths 拒绝；full 空 paths 接受。
- 修改策略递增 source version、清除已固定分支/验证状态并使旧 refs 结果不可继续使用。
- 旧 migration 数据读取为 full，migration 门禁、SQLx migration 测试通过。

**开发步骤：**

1. 先检查当前 migration 最大编号并创建唯一的新 migration；执行 `make setup-git-hooks` / `make verify-git-hooks` 后再修改 migration。
2. 增加 `source_materialization_json` 默认值；不要修改 `0008_git_branch_two_stage_deployment.sql` 或其他已应用 migration。
3. 新建 typed policy 解析/规范化 helper，确保 save、show、deployment resolve 共用同一 canonical 逻辑，避免 API 和 snapshot 各自解释路径。
4. 更新来源查询的所有 SELECT/INSERT/UPDATE 绑定和 OpenAPI annotations；补齐 `application_sources_api.rs` 旧行兼容 fixture。

#### U2. Snapshot、预览和 Agent payload 传递

**目标：** 保证 preview、confirm、retry、prepare 和跨节点 release 使用同一个不可变策略。

**涉及路径：**

- `api/src/deployments/mod.rs`
- `api/src/agents/dispatcher.rs`
- `api/tests/deployments_api.rs`
- `api/tests/agent_end_to_end.rs`
- `api/tests/agent_dispatcher.rs`（若现有测试文件承载 dispatcher 场景）

**测试场景：**

- Git 来源策略进入 preview snapshot 和 snapshot hash。
- 来源修改后旧 preview 失效；重试仍使用原策略和原 commit。
- 同节点 Git release、跨节点 Git release 分别验证策略传递；workspace/image/artifact 任务不携带 Git 策略。
- sparse Agent capability/protocol 不满足时返回稳定错误，不创建不可执行任务；full 任务保持兼容。
- payload digest、幂等键、恢复/重连流程包含策略影响但不泄露凭证或本地路径边界之外的信息。

**开发步骤：**

1. 扩展 `TwoStageSourceInfo`、`VerifiedSourceRow` 和来源 SQL，使 preview 阶段只读取一次 verified source 的 canonical policy。
2. 在 Git `source_snapshot` 写入 `source_materialization`、`materialization_policy_version` 和已有 `source_version`，让 `digest_json` 自动覆盖策略。
3. 找全所有 `DeploymentPrepareTask` / `DeploymentReleaseTask` 构造点，按 `source_policy`、`checkout_mode` 和 cross-node 条件传递策略；不得从应用当前表重新读取。
4. 增加 dispatcher 的 sparse compatibility helper，并同步 queued-task compatibility/reconciliation 查询，避免不兼容任务永久排队。
5. 先固定 payload JSON fixture，再让协议 crate 和 Agent 实现跟进，避免 API 与 Agent 各自形成不同字段名。

#### U3. Agent Git partial/sparse checkout

**目标：** 在 Agent 内完成真正的 Git 物化，保护固定 commit、凭证和工作区边界。

**涉及路径：**

- `agent-protocol/src/lib.rs`
- `agent-protocol/schema/agent-control-v15.schema.json`（从当前 latest 冻结的历史 schema）
- `agent-protocol/schema/agent-control-v16.schema.json`（升级后的 latest schema）
- `agent/src/git.rs`
- `agent/src/runner.rs`
- `agent/src/executor.rs`
- `agent/src/task_handler.rs`
- `agent/src/storage_cleanup.rs`
- `agent/tests/two_stage.rs`
- `agent/tests/git.rs`
- `agent/src/runner_service.rs`（现有 runner fixture/spec 测试）
- `agent-executor/src/protocol.rs`、`agent-executor/tests/protocol.rs`（若协议 mirror 需要同步）

**测试场景：**

- full/缺省策略保持现有 clone、fetch、checkout、HEAD 和 dirty worktree 行为。
- sparse 模式只物化 allowlist，根文件和递归目录均可用。
- partial clone filter 失败：full 回退成功；sparse 明确失败且不执行 full 回退。
- 固化 SHA 不在分支 tip、已有 checkout 复用、fetch 后 checkout、重复 prepare/retry 均保持 SHA 校验。
- sparse 目录缺少 `Makefile`/固定入口时在脚本执行前失败；错误码稳定且不泄露 credential 内容。
- 取消、超时、Git 凭证 lease 清理、任务重启恢复和现有身份等待窗口不回归。
- 路径规则在 Agent 双侧复验，拒绝路径逃逸、符号链接造成的工作区越界和未知策略字段。

**开发步骤：**

1. 先把当前 generic latest schema 冻结为 v15 历史 schema，再在 `agent-protocol` 定义 policy/capability、v16 schema 和历史 schema compatibility fixtures，再修改 executor/runner 的 spec。
2. 将 `checkout_commit` 扩展为接收 policy，保留现有 public helper 的默认 full 包装（如现有测试/调用需要），降低非 Git 两阶段调用影响。
3. 把 clone/fetch、sparse init/set、checkout、HEAD/status 的命令构造拆成可单测的纯 helper；实际命令通过现有 `run_command` 执行并复用凭证注入。
4. 对 full 模式明确 disable sparse；对 sparse 模式明确禁止 filter 失败后的 full fallback；两种模式都在最后执行 detached SHA 和 clean worktree 验证。
5. 在 runner/executor/task handler 逐层传递 policy；每层只对 Branch/Git checkout 生效，Workspace/Artifact 路径保持现有 payload 和校验。
6. 用临时本地 Git repository fixture 验证大文档 blob 未进入 sparse 工作区，并验证已有 checkout、fetch、取消、超时和身份等待路径。

#### U4. 管理端 Git 来源配置与只读展示

**目标：** 提供管理员可理解的 full/sparse 配置入口，同时保持已有来源配置流程。

**涉及路径：**

- `admin/src/features/applications/ApplicationSourceSection.tsx`
- `admin/src/features/applications/ApplicationSourceSection.test.tsx`（如不存在则沿用 `admin/src/test/GitSourceConfiguration.test.tsx`）
- `admin/src/test/GitSourceConfiguration.test.tsx`
- `admin/src/api/generated/models/ApplicationSourceResponse.ts`
- `admin/src/api/generated/models/SaveSourceRequest.ts`
- `admin/src/api/generated/models/index.ts`
- `api/openapi/openapi.json`
- `admin-app/lib/api/generated/lib/src/model/application_source_response.dart`
- `admin-app/lib/api/generated/lib/src/model/save_source_request.dart`
- `admin-app/lib/api/generated/lib/src/model/application_source_response.g.dart`
- `admin-app/lib/api/generated/lib/src/model/save_source_request.g.dart`
- `admin-app/lib/api/generated/lib/src/serializers.dart`
- `admin-app/lib/api/generated/lib/src/serializers.g.dart`
- OpenAPI 生成输入/输出对应文件

**测试场景：**

- 旧来源缺省显示 full；管理员切换 sparse、录入路径、保存后显示规范化摘要。
- sparse 空路径和非法路径显示服务端错误；保存失败不清除已有有效配置。
- 普通用户不能编辑策略，来源详情能看到 full/sparse 和路径摘要。
- 编辑取消、并发版本冲突、CSRF、未保存离开保护和现有 Git 分支刷新流程保持正常。
- 移动端/窄屏路径列表不造成页面横向溢出。

**开发步骤：**

1. 先更新 API/OpenAPI schema 和生成流程，再更新 React draft 类型；不要在 generated 文件与源 schema 不一致时手工补字段。
2. 在现有 Git 来源表单中加入模式控件和路径编辑器，复用当前 `Field`、`Select`、`TextInput`、错误通知和 dirty 逻辑。
3. 将多行文本转换为 paths 时保留原始输入直到服务端成功；成功后以 response canonical paths 回填，保证 UI 与服务端一致。
4. 为旧来源、sparse 保存成功/失败、普通用户只读、并发冲突和窄屏布局补测试。
5. 如果 admin-app 当前只消费而不编辑 Git 来源，仍需更新其生成模型并补反序列化 smoke；不要因为没有对应 Web 表单就遗漏 API model。

#### U5. 契约、schema、runbook 和验证资料

**目标：** 让实施、运维和后续接入方理解默认 full、显式 sparse 和失败边界。

**涉及路径：**

- `docs/standards/git-branch-deployment-contract.md`
- `docs/standards/agent-control-protocol.md`
- `docs/standards/application-deployment-json.md`
- `docs/runbooks/application-templates.md` 或实际 Git 来源接入 runbook
- `docs/runbooks/agent-recovery.md`

**测试/审查场景：**

- 文档示例与 API 字段、协议字段、默认值完全一致。
- runbook 能说明 `git_sparse_checkout_invalid`、`git_sparse_checkout_unavailable`、缺入口文件和回滚到 full 的处理边界；不得建议手工修改 deployment snapshot。
- 明确真实控制面部署需要单独授权，Deploy Go 控制面部署不等于业务节点 Agent 升级。

**开发步骤：**

1. 先更新 `git-branch-deployment-contract.md` 作为来源级契约权威；在 `application-deployment-json.md` 明确该字段不属于 parameter schema/verification config。
2. 根据最终实现的协议版本和错误码更新 agent-control protocol、Agent recovery 和 Git 来源接入 runbook。
3. 在文档复核时逐项比对 API schema、Rust serde 字段、前端模型、错误码和默认值，禁止文档使用尚未落地的临时命名。

## Dependencies and Sequence

1. U1 先落数据库和 API 规范，确定 canonical JSON、限制和默认 full。
2. U2 冻结 snapshot 与 payload 形状，完成协议字段与 capability/version 门禁设计。
3. U3 实现 Agent Git 物化和协议双向兼容测试；以本地 Git fixture 验证大文档未下载/未物化的可观测结果。
4. U4 接入管理端，基于 U1 的 API/OpenAPI 类型完成配置和只读展示。
5. U5 在实现完成后同步契约和 runbook，避免文档先于实际错误码或协议语义。
6. 执行 `ce-simplify-code`、高风险协议/API/数据库/可靠性/安全复核，再按项目约定提交和推送；真实控制面部署另行获得明确授权后执行。

## Risks and Mitigations

- **构建输入漏配：** sparse 工作区可能缺少隐式依赖。通过显式入口缺失失败、固定 allowlist、预览摘要和 fixture 覆盖降低风险；不自动 full 回退。
- **prepare 工作区责任缺口：** 当前 prepare 执行链没有像 release 一样在 runner 中显式调用 Git checkout；实现必须先补 Agent checkout preflight，确保业务脚本不会在旧目录或空目录上构建。
- **协议旧 Agent 拒绝未知字段：** full 缺省不发送 sparse-only 字段；sparse 任务走新协议/capability 门禁；保留历史 schema 和双向兼容测试。
- **Git 版本差异：** 在 Agent fixture 中验证 `sparse-checkout` 和 filter 能力；full 模式保持现有回退，sparse 模式给出明确不可用错误。
- **路径模式语义差异：** 首版限制 literal/`/**`，服务端规范化后 Agent 二次校验，不接受 arbitrary Git pattern。
- **跨节点行为不一致：** snapshot 同时生成 prepare/release 策略，跨节点测试覆盖；同节点 release 不重复物化。
- **workspace 被误套用：** source policy 分支在 API dispatcher、Agent executor 和测试中明确隔离，继续使用 workspace artifact。
- **大仓库身份等待：** 保留既有 `identity_wait_budget` 修复，不把 sparse 优化当作唯一可靠性修复；测试覆盖 clone 后 process identity 写入窗口。
- **敏感文件意外进入构建：** allowlist 默认收窄工作区；错误诊断只输出策略摘要/缺失路径，不输出 Git 凭证、Env 或完整内部路径。

## Verification Contract

- Rust：运行受影响 crate 的格式、单元测试、协议 schema 兼容测试和 API 集成测试；至少覆盖 `api/tests/application_sources_api.rs`、`api/tests/deployments_api.rs`、`api/tests/agent_end_to_end.rs`、`agent/tests/two_stage.rs`。
- 数据库：执行 `make setup-git-hooks`、`make verify-git-hooks`，运行 migration 测试和 SQLx 相关检查；不得修改或重排既有 migration。
- 管理端：运行 typecheck、lint、受影响的 Git 来源测试和现有前端测试/build。
- 静态检查：`git diff --check`，检查协议 unknown-field 行为、权限边界、错误码脱敏和 full 默认兼容。
- 行为验收：使用本地 Git fixture 构造包含大文档/备份和小源码的仓库，对比 full 与 sparse 的工作区文件、`.git` promisor 状态、checkout SHA、prepare/release 结果和失败诊断。
- 部署前门禁：完成代码复核、runbook 核对和回滚说明后，才可在获得明确授权时按正式控制面 runbook 发布；本计划本身不授权部署。

## Definition of Done

- [x] 未配置策略的现有 Git 来源和部署仍按 full 行为运行。
- [x] 管理员可以保存、修改和查看 full/sparse 策略，服务端拒绝非法路径和 sparse 空名单。
- [ ] 来源版本、preview、deployment snapshot、retry、prepare/release payload 的策略链路一致且可审计。
- [ ] Agent 能执行 partial clone + sparse checkout，固定 SHA/凭证/取消/超时/恢复和 dirty worktree 约束不回归。
- [ ] sparse 不可用或构建输入缺失时明确失败，不自动扩大为 full checkout。
- [x] workspace、image、普通 script 部署行为保持不变。
- [ ] 协议 schema、API/OpenAPI、管理端、标准文档、runbook 和测试均同步完成。
- [ ] 聚焦验证、代码复核、`git diff --check` 和 migration 门禁通过；真实控制面发布作为独立授权后的后续操作。

## Open Questions and Execution Notes

- 协议版本已按当前仓库状态确定为 v16；实施若发现发布流程无法同时发布 v16 Agent 和控制面，必须先停在协议/发布单元，不得通过向 v15 Agent 发送未知字段规避。
- 是否把策略摘要显示在部署预览 UI，属于实现阶段的可读性增强；snapshot 和审计必须包含，普通用户至少能在来源详情看到最终策略。
- 现有 generated API 文件的更新方式需遵循仓库当前 OpenAPI 生成流程，不能手工产生与 schema 不一致的模型。

## Sources

- `docs/standards/git-branch-deployment-contract.md`
- `docs/standards/application-deployment-contract.md`
- `docs/standards/agent-control-protocol.md`
- `docs/standards/application-deployment-json.md`
- `docs/plans/2026-08-06-004-git-branch-two-stage-deployment-plan.md`
- `docs/solutions/two-stage-prepare-identity-wait.md`
- `docs/solutions/workspace-reserved-module-admission.md`
- `docs/solutions/agent-reconnect-sequence-gap.md`
- `agent/src/git.rs`
- `agent/src/runner.rs`
- `agent/src/executor.rs`
- `api/src/application_sources/mod.rs`
- `api/src/deployments/mod.rs`
- `api/src/agents/dispatcher.rs`
- `admin/src/features/applications/ApplicationSourceSection.tsx`
