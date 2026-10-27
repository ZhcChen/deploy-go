# AGENTS.md

面向在本仓库工作的所有 AI Agent（Codex、Claude Code 等）。权威顺序：用户当前指令 > 本文件 > `CLAUDE.md` > `docs/standards/` > `docs/runbooks/` > `docs/solutions/` > `specs/` 与 `docs/plans/`。本文件只写通用协作规则，命令、架构与工程细节以 `CLAUDE.md` 为准；冲突时以代码为准并回头修正文档。本文件是合并 Claude 阶段经验后的新版，不还原历史旧版结构。

## 1. 分工、交接与提交权

- Codex 承担常规开发：实现、跑验证、并在本轮小闭环内提交。Claude Code 保留独立复核角色：只报告有证据的发现，不把自审称为独立复核，也不替实现者改代码（用户明确要求时除外）。
- 同一工作区同一时刻只允许一个写入者。交接必须写清：改动文件与范围、已运行/未运行/跳过的验证、当前 HEAD 与未提交状态、遗留问题。
- 提交权归当前写入者：由写入者按下方规则提交并推送；复核者与观察者不提交他人改动，也不代为强推、改写历史或调整他人暂存区。
- 接手方先执行 `git status --short` 与 `git log --oneline -5` 核对真实状态，不凭交接文字假设进展。
- 遇到文件范围冲突、他人未提交改动或方向不明时立即停下报告；不回滚、不清理、不混入他人改动。
- 需要并行写入时使用隔离 worktree 且文件范围不交叉；否则串行，由当前写入者集成。

## 2. 语言与路径

- 文档、注释、提交信息、复核结论默认简体中文；函数名、类型名、API、配置键、命令、路径、协议字段等领域标识保持英文。
- 文档内的路径统一使用仓库相对路径。

## 3. 目录与流程

- `specs/<feature>/`：新需求的 spec / plan / tasks；`docs/plans/`：既有计划与进度；`docs/brainstorms/`、`docs/reviews/`、`docs/runbooks/`、`docs/solutions/`、`docs/standards/` 按既有用途写作。
- 中型及以上任务先确认或新建 spec/plan/tasks 再动手；目标明确的小任务可直接实施并做聚焦验证；优先续写已有产物，不新开平行文档。
- 涉及运行、部署、迁移、排障或联调的改动，实施前查对应 `docs/runbooks/`，完成前同步更新该 runbook。

## 3.1 开发环境隔离

- 本机用于编辑、静态检查、单测和轻量调试；API/Web 完整装配与浏览器验收默认遵循 `docs/runbooks/remote-development.md` 的独立远端 Compose project。
- qfy-test2 同时是正式环境。开发栈不发布宿主端口，不挂载正式目录/密钥，不接入真实业务 Agent；不接管 `deploy-go-test` 等旧环境。所有 Docker 命令显式 context，Compose 显式 project/文件/基础设施配置来源；创建前登记并核对现场归属、镜像和预算。
- 该约定不扩大第 8 节授权；不得把开发调整视为正式部署、业务重试或清理其他 project 的授权。

## 4. 验证与提交（仓库无 CI，全部靠本地门禁）

- 提交前必须运行与改动范围匹配的 make 目标：`make api-check`、`make agent-check`、`make admin-check`、`make admin-app-check`，改动面广时 `make check`。
- 不在 `make check` 内、需显式运行：`make admin-test-e2e`、`make admin-app-test-integration DEVICE_ID=<id>`，以及 Docker 内的 Linux 验证 `make agent-executor-cgroup-check`、`make agent-runner-isolation-check`、`make privileged-terminal-check`、`make privileged-release-check`。
- 改动 `cfg(target_os = "linux")` 代码（cgroup、PTY、身份切换等）必须用上述 Docker 目标验证，不能以 macOS 通过为准；macOS 连 clippy 都不检查这些路径。
- 生成产物禁止手改：`api/openapi/openapi.json`、`api/openapi/external.json`、`admin/src/api/generated/`、`admin-app/lib/api/generated/` 由命令生成并入库；新增 handler 还要把 path 与 schema 登记进 `api/src/lib.rs` 的 `ApiDoc`（对外 API 在 `api/src/external/mod.rs`），再跑 `make api-openapi`。
- 版本号联动：api / agent / agent-executor / agent-updater / deploy-go-deployer 的 `version` 保持一致，升版必须同时 `make api-openapi` 与 `make api-client-generate`；普通改动不顺手改版本。
- 提交纪律：只暂存本轮相关文件，不用 `git add .`；提交前跑 `git diff --check`、`git diff --cached --check`、`git diff --cached`；提交信息用简体中文并带 `feat:` / `fix:` / `docs:` / `test:` / `chore:` / `refactor:` 前缀。
- 禁止 `--no-verify`、禁止强推、禁止绕过或临时改写 hooks；hook 坏了就修 hook，不是绕开。涉及 migration 时先 `make setup-git-hooks` 与 `make verify-git-hooks`。

## 5. Migration 与不可变资产

- `api/migrations/` 中已提交、可能被共享环境应用的 migration 默认不可变：不得修改、删除、重命名或重排，修正只能新增更高版本。
- 新增 migration 不得含 `DROP TABLE` / `DROP COLUMN`（门禁拒绝），用"新增列 + 回填 + 旧列标 deprecated"。
- 改 schema 要同步 `api/tests/migrations.rs`，并跑 `cargo test -p deploy-go-api --test migrations --test database_constraints`。
- `agent-protocol/schema/agent-control-v*.schema.json` 是不可变历史 schema；受管上游资产保留原文与原始 hashes。

## 6. 修改范围边界

- 只允许修改 deploy-go 自身的代码、测试、文档、协议、Agent、控制面与部署工具。严禁修改被部署的业务应用：其代码、配置、脚本、仓库与发布物一律不动，业务侧问题只能通过本项目的控制面、Agent、协议与诊断解决。
- 线协议类型只定义在 `agent-protocol/src/lib.rs`，改动同步 `agent-protocol/schema/agent-control.schema.json` 与 `agent-protocol/tests/schema_compatibility.rs`；`agent` 与 `agent-executor` 的本机协议要求精确匹配，二者必须成对发布。
- 任务 payload 一律严格枚举 + `deny_unknown_fields`：不新增任意 shell / command / Make target / 环境变量 map / 带凭证 URL 字段；能力或版本未知时 fail closed。

## 7. 隐私与 Env

- Secret、Env 明文、凭证、终端正文、lease 值不得进入日志、tracing、journal、OpenAPI 示例或浏览器持久化存储；凭证与配置类数据用 `crypto::MasterKeyRing` 加密落库，涉及 Env/配置的响应带 `Cache-Control: no-store`。
- 不为调试读取、导出或回显 Env 明文与密钥，只用契约允许的键存在性与长度检查；Env 内容由运行期注入，不落盘、不进报告。
- 提交前可运行 `make client-sensitive-check`，扫描客户端产物中的私钥、token 与 lease。

## 8. 远程执行与正式环境授权

- 开发、测试或验证部署功能不等于获得连接真实节点或执行真实部署的授权。
- 只有用户在当前对话中明确要求对具体环境或节点执行具体操作（部署、远程脚本、重启、迁移、切流、清理）才构成授权；"继续""验证""提交""开始开发"都不构成授权。
- 无授权时只能做本地实现、测试、构建、文档整理与模拟验证；本地 fixture、mock 服务与明确隔离的测试容器不算真实节点，但仍遵守最小影响与可恢复原则。
- 例外：用户在当前对话说出"部署正式环境"，即视为对 Deploy Go 唯一正式环境授权执行 `make deploy-production`（并连带让符合条件的节点自动串行升级）；部署前仍须按 runbook 核对服务器身份，只有真实异常才停下。该授权不覆盖业务应用部署、重发或取消业务部署、手改线上状态。
- 其他有副作用或写仓库之外的命令同样按此判定：`make deploy-production`、`make deployer-skill-install`、`make credential-reencrypt`。
- 子 Agent 继承与父级相同或更严格的授权边界，不得自行扩大或以"换条路"绕过；超出时停下并如实报告。
