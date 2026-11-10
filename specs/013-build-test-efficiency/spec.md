# 构建与检查效率优化（第一批）

状态：规格就绪
来源：用户需求（2026-11-10 会话）“分析构建与测试优化”及其后续“同意先做第 1、2、3 项”。分析数据来自本机实测：`make check` 串行约 376 秒，其中 `make api-client-check` 单步约 214 秒；全量 nextest 墙钟约 45 秒；部署日志不留存，无法分析部署各阶段耗时。

## 目标与范围

缩短本地全量检查（`make check`）的墙钟时间，并让部署的各阶段耗时可被事后分析；不改变任何检查的判定语义，不放松任何门禁。

范围（第一批，三项）：
1. `api-client-check` 在输入未变时不重复生成与比对（记忆化），并减少无谓的联网。
2. `make check` 去除重复执行的检查，并让互不依赖的检查组并行运行，输出按组聚合。
3. 部署脚本记录分阶段耗时，并把完整日志留存到本机忽略目录，保留最近若干份。

明确不做：测试二进制合并、长尾用例缩减、Dockerfile `COPY --link`、同机部署跳过产物往返、Rust 与前端并行构建（这些留给后续批次，需先量化原型）；不改产品代码、API、Agent 协议、migration；不运行真实部署，不连接服务器。

## 用户场景与验收

### US1：未改 API 契约时，全量检查不再花 3 分半在客户端生成上

- 优先级：P1
- Given：`api/openapi/openapi.json`、生成脚本、生成器配置、Flutter `pubspec.lock`、已提交的两个生成目录、Dart 与 Node 版本都与上一次通过检查时一致
- When：运行 `make api-client-check`
- Then：数秒内通过并说明“输入未变，已跳过”；任何一项输入变化（包括手改已提交的生成文件）都会重新完整生成并比对
- 边界：从未通过、记忆文件缺失或损坏、`--force`，都执行完整检查；检查失败不得写入记忆文件。

### US2：`make check` 更快且结论不变

- 优先级：P1
- Given：干净工作区
- When：运行 `make check`
- Then：互不依赖的检查组并行运行，同一组内的 Cargo 步骤保持串行（共用 target 锁）；每组的输出完整保留并在组完成时整体输出；末尾给出每组耗时和通过/失败汇总；任一组失败则整体非零退出，并列出所有失败组
- 边界：仍可用串行方式运行（`make check-serial`），行为与改造前一致；不再重复执行已被 `api-check` 覆盖的 `cargo test` 与 `api-openapi-check`，单独运行 `make deployer-check` 等目标时仍保持完整。

### US3：部署耗时可分析

- 优先级：P2
- Given：运行 `make deploy-production`（或构建类部署）
- When：部署结束（成功、失败或被中断）
- Then：本机 `.data/deploy-logs/` 下有该次的完整日志，日志中有每个阶段的耗时行和总耗时；仅保留最近 20 份
- 边界：日志与耗时行不得包含密钥、`install.env` 内容或代理节点信息；日志目录创建失败时部署仍继续并给出提示。

## 功能要求

- FR-001：`scripts/generate-api-clients.mjs --check` 计算输入指纹（`openapi.json`、生成脚本自身、`admin/openapitools.json`、Flutter 已提交的 `pubspec.lock`、两个已提交生成目录的全部文件内容、`dart --version` 与 Node 版本），与 `target/.api-client-check.stamp` 一致时直接通过；仅在完整检查无漂移且 `verifyInvalidSpec` 通过后写入新指纹；支持 `--force` 跳过记忆。
- FR-002：Flutter 客户端生成的 `dart pub get --enforce-lockfile` 先尝试 `--offline`，失败后再联网重试；不改变锁文件语义。
- FR-003：新增 `scripts/test/run-checks.sh`，定义检查组并行运行；每组输出缓冲到临时文件，完成后整体打印；打印各组耗时、总墙钟与失败清单；失败时退出码非零；`--serial` 可顺序运行。
- FR-004：Makefile 的 `check` 调用该脚本；新增 `check-serial` 保持原有串行语义；`deployer-check`、`app-template-check`、`external-deploy-check` 拆出不含 `cargo test` 与 `api-openapi-check` 的静态部分，供 `check` 使用，原目标保持完整行为。
- FR-005：`deploy/production/deploy.sh` 记录阶段耗时（前置检查、源码上传、远程构建、产物回传、上传安装）并在结束时（含失败与中断）输出汇总；完整输出同时写入 `.data/deploy-logs/deploy-<时间>-<提交短哈希>.log`，保留最近 20 份；`remote-build.sh` 输出 Rust 构建与前端构建各自耗时。
- FR-006：契约测试覆盖：阶段耗时与日志留存的关键字符串、日志目录不可写时的降级、日志不含 `install.env` 内容。
- FR-007：`docs/runbooks/development-guide.md`、`local-development.md`、`systemd-deployment-production.md` 同步新命令、记忆化与强制重跑方式、日志位置；`make help` 同步。

## 成功标准

- SC-001：未改 API 契约时，`make api-client-check` 第二次运行耗时不超过 15 秒；改动 `openapi.json` 或手改一个生成文件后会完整重跑，并能发现漂移。
- SC-002：`make check` 墙钟明显低于改造前串行基线（约 376 秒）；目标是在本机预热后不超过约 120 秒，实测值如实记录，达不到则记录原因而不是放宽检查。
- SC-003：`make check` 在被故意破坏的输入下（例如格式错误、文档坏链接、客户端漂移）能失败并指出组名；全部通过时退出码为零。
- SC-004：部署契约测试通过；`deploy.sh` 的日志与耗时行经检查不含密钥。

## 风险与验证要求

- 记忆化的正确性是关键风险：指纹必须覆盖会影响结果的全部输入，漏项会掩盖漂移。以故意修改每类输入各一次验证会触发重跑；失败不写记忆。
- 并行风险：Cargo 步骤保持同组串行，避免 target 锁争用和资源竞争；非 Cargo 组互不共享可写目录；输出按组聚合，避免交错。
- 仅涉及脚本、Makefile、文档与部署脚本的日志行为，不涉及 Rust 产品代码、Agent、协议、migration 或运行时行为。验证：`make docs-check`、`make spec-kit-verify`、`make whitespace-check`、`make deploy-production-check`、`make api-client-check`（首轮与记忆命中）、改造后的 `make check` 与 `make check-serial` 的结果对照。
- 不运行真实部署；部署脚本改动只用现有 mock 契约测试验证，真实效果在下次授权部署后核对。

## 未决事项

无。后续批次（测试二进制合并、长尾用例、Dockerfile 分层、同机产物往返）在量化原型后另行决定。
