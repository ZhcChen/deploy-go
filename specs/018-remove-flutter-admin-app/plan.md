# 移除 Flutter 管理端实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

现状证据：`admin-app/` 受版本控制 434 个文件，本机另有约 72MB 被忽略的构建与 IDE 产物（`build/`、`.dart_tool/`、`ios/Pods/`、`.idea/` 等，已核对没有签名密钥、证书、环境文件）。后端没有 app 专属代码：移动端用到的接口（含 `/api/v1/auth/preferences`）Web 同样在用。

引用盘点（`git grep -il "admin-app\|flutter\|dart"`，剔除 `Artifact` 等子串误报）：
- 工具链：`Makefile`（目标、帮助、`.PHONY`、`check-serial`、`DEVICE_ID`、`DEPLOY_GO_API_BASE_URL`）、`scripts/generate-api-clients.mjs`（Flutter 目标与 Dart 步骤、`dart` 版本指纹）、`scripts/test/run-checks.sh`（`flutter` 组）、`scripts/check-client-sensitive-data.mjs`、`scripts/test/docs-check.py`、`.dockerignore`。
- 当前有效文档：`README.md`、`AGENTS.md`（`CLAUDE.md` 是它的符号链接）、`docs/runbooks/{development-guide,local-development,ssh-node-onboarding}.md`、`docs/standards/{README,access-control,agent-control-protocol,api-contract,client-behavior-contract,ssh-credential-security}.md`。
- 历史记录（保持原样）：`docs/plans/`、`docs/reviews/`、`docs/brainstorms/`、`specs/010`–`013`。
- 文档检查只对“当前有效文档”校验 make 目标、仓库路径和锚点，对全部 Markdown 校验相对链接；全仓只有 `README.md` 一处相对链接指向 `admin-app/README.md`。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md)：只改本仓库自身；不手改生成产物（Web 客户端若需更新只用 `make api-client-generate`）；不触碰 migration、线协议与版本号；不涉及远程执行、部署或节点；提交只暂存本轮相关文件，信息用简体中文；`CLAUDE.md` 是符号链接，只改 `AGENTS.md`。

## 实现方案与文件范围

1. 删除：`git rm -rf admin-app`，再 `rm -rf admin-app` 清掉被忽略的本机产物。
2. `Makefile`：删除 `admin-app*` 目标与帮助行，`.PHONY`、`check-serial` 同步，删除 `DEVICE_ID` 与 `DEPLOY_GO_API_BASE_URL`，更新 `api-client-*` 帮助文字。
3. `scripts/generate-api-clients.mjs`：`targets` 只留 Web；删除 `buildDart` 分支、`alignDartSdkConstraint`、`removeUnsafeDartCookieAuth`、`dart --version` 指纹、`.dart` 源文件收集；更新指纹说明注释。`scripts/test/run-checks.sh` 删除 `flutter` 组。
4. `scripts/check-client-sensitive-data.mjs`、`scripts/test/docs-check.py`、`.dockerignore`：删除 `admin-app` 项。
5. 当前有效文档逐份修订：删除 Flutter 专属段落与命令，把“Web 与 Flutter”“双端”改为只有 Web，更新验证映射与耗时表；`client-behavior-contract.md` 去掉 Flutter 的可访问性规则，保留与 Web 相关的约束。
6. `docs/plans/README.md` 加说明；登记 018。

## 验证计划

- 生成脚本：`node --check`；`make api-client-generate` 前后 `git status` 对 `admin/src/api/generated/` 无差异；`node scripts/generate-api-clients.mjs --check --force` 与不带 `--force` 的记忆化路径各跑一次。
- 门禁：`make docs-check`、`make client-sensitive-check`、`make whitespace-check`、`make spec-kit-verify`、`make admin-check`、`make api-client-check`，最后 `make check` 确认检查组不再含 Flutter。
- 清扫：忽略大小写检索 `admin-app`、`flutter`、`dart-define`、`pubspec`，范围见 SC-002，历史记录之外应无命中。
- 在没有 `dart`/`flutter` 的环境下运行 `make api-client-check`（本机确认 PATH 中去掉二者后仍通过）。

## 风险与回退

- 回退：从 Git 历史恢复 `admin-app/`（`git checkout 1e003e0 -- admin-app`）；本机被忽略的产物可由 `flutter pub get` 等再生。
- 与 017 并存：017 的 Flutter 生成物改动随删除消失；先提交本规格，再提交 017；共享的 `docs/standards/agent-control-protocol.md` 先只暂存本规格的单行改动。
- 误删风险：删除前已列出被忽略文件并检索密钥类文件，结果为空。
