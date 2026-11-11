# 移除 Flutter 管理端

状态：规格就绪
来源：用户需求（2026-11-11 会话）：“我只要保留 web 客户端就行, app 端废弃，并移除清理干净”。

## 目标与范围

管理客户端只保留 Web（`admin/`）。Flutter 管理端 `admin-app/` 废弃并整体移除，连同只为它存在的构建、生成、检查、文档和配置一并清理，使仓库里不再有会误导后续工作的 app 入口。

适用对象：Deploy Go 的开发者与协作的 AI Agent。

范围：
1. 删除 `admin-app/` 整个目录（含本机构建产物与 IDE 文件）。
2. Makefile：删除全部 `admin-app*` 目标、帮助文本、仅 Flutter 使用的变量，以及 `check` / `check-serial` 中的 Flutter 检查。
3. API client 生成与漂移检查只面向 Web（TypeScript），不再依赖 Dart / Flutter 工具链。
4. 其他脚本与配置中的 `admin-app` 引用：客户端敏感模式扫描、文档检查、`.dockerignore`。
5. 当前有效文档（README、AGENTS.md、`docs/runbooks/`、`docs/standards/`）中把 Flutter 当作现行客户端的描述、命令、工具链要求、验证映射和 Flutter 专属规则。

明确不做：
- 不改后端接口、数据库和 migration：`/api/v1/auth/preferences` 等接口 Web 同样在用，已提交的 migration 不可变。
- 不改 Web 的功能、样式和测试逻辑，不改 Agent、线协议与版本号。
- 不改写历史记录：`docs/plans/`、`docs/reviews/`、`docs/brainstorms/`、已完成的 010–013 规格与 Git 历史保持原样，只在计划索引补一句说明。
- 不触碰仓库外的内容：Obsidian 笔记、本机已安装的 Flutter / Dart 工具链。

## 用户场景与验收

### US1：现行入口里只剩 Web 客户端

- 优先级：P1
- Given：app 端已移除
- When：开发者执行 `make help`，阅读 README、AGENTS.md、工程指南和本地开发手册
- Then：看不到任何 Flutter / `admin-app` 的目标、命令或现行说明，客户端只有 `admin/`；`make check` 不再要求安装 Flutter / Dart
- 关键失败或边界场景：历史计划、复核和已完成规格里仍有 app 记载，这是历史记录，不算遗漏。

### US2：API client 生成只面向 Web

- 优先级：P1
- Given：没有 `dart` / `flutter` 命令的机器
- When：运行 `make api-client-generate` 和 `make api-client-check`
- Then：只生成并比对 `admin/src/api/generated/`，结果与移除前逐字节一致，检查通过
- 关键失败或边界场景：生成物漂移时仍然报错；记忆化指纹随目标减少保持正确，任何输入变化都会触发完整检查。

### US3：门禁继续有效

- 优先级：P1
- Given：清理完成
- When：运行 `make docs-check`、`make client-sensitive-check`、`make whitespace-check`、`make spec-kit-verify`、`make api-client-check`、`make admin-check`、`make check`
- Then：全部通过，且 `make check` 的并行检查组中不再有 Flutter 组
- 关键失败或边界场景：文档检查对当前有效文档里的失效 make 目标和失效路径仍然报错，不因删除而放宽。

## 功能要求

- FR-001：Git 不再追踪任何 `admin-app/` 路径，工作区不再存在该目录。
- FR-002：Makefile 中没有 `admin-app*` 目标、`DEVICE_ID`、仅 Flutter 使用的 `DEPLOY_GO_API_BASE_URL`，`.PHONY`、`make help`、`check-serial` 同步；`DEPLOY_GO_ALLOWED_ORIGIN(S)` 仍由 `api-run` 使用，保留。
- FR-003：`scripts/generate-api-clients.mjs` 只含 Web 目标，删除 Dart 专属步骤与 `dart` 版本指纹；`scripts/test/run-checks.sh` 删除 `flutter` 检查组。
- FR-004：`scripts/check-client-sensitive-data.mjs` 默认只扫描 `admin/src`、`admin/e2e` 和 `test-fixtures`，不再排除 `.dart_tool`。
- FR-005：`scripts/test/docs-check.py` 不再把 `admin-app` 当作仓库路径根，也不再允许 `admin-app/build/` 缺失；`.dockerignore` 删除 `admin-app` 两项。
- FR-006：README、AGENTS.md、`docs/runbooks/`、`docs/standards/` 不再把 Flutter / `admin-app` 描述为现行客户端：删除 Flutter 工具链版本、命令、验证映射、耗时行与 Flutter 专属规则，把“Web 与 Flutter”“双端”等措辞改为只有 Web。
- FR-007：`docs/plans/README.md` 补一句说明：app 端已废弃并移除，历史计划与复核中的相关记载保持原样。
- FR-008：不修改后端接口、migration、线协议、Agent 与版本号；`admin/` 下除措辞外不改动。
- FR-009：登记 018（`specs/README.md`、存储登记）。

## 成功标准

- SC-001：`git ls-files admin-app` 为空且目录不存在。
- SC-002：在 README、AGENTS.md、`docs/runbooks/`、`docs/standards/`、Makefile、`scripts/`、`.dockerignore`、`admin/`（不含 `node_modules`）、`api/`、`agent*/` 中检索 `admin-app`、`flutter`、`dart-define`、`pubspec`（忽略大小写），除 018 自身的 feature 标识与登记外无命中。
- SC-003：`make docs-check`、`make client-sensitive-check`、`make whitespace-check`、`make spec-kit-verify`、`make api-client-check`、`make admin-check`、`make check` 通过。
- SC-004：`make api-client-generate` 前后 `admin/src/api/generated/` 无差异。

## 风险与验证要求

- 删除不可逆：已跟踪的 434 个文件可从 Git 历史恢复（最后包含 app 的主干提交为 `1e003e0`）；工作区被忽略的构建与 IDE 产物（约 72MB）无法恢复但可再生，已确认其中没有签名密钥、证书、环境文件或其他本机独有配置。
- 与同期工作交叉：017 重新生成过 `admin-app/lib/api/generated/`，随目录删除一并消失；两项工作分开提交，共享的文档文件按提交范围分别暂存。
- 生成脚本改动：用 `--force` 完整检查证明 Web 生成物不变；记忆化指纹逻辑随目标减少后仍覆盖全部输入。
- 文档遗漏：用忽略大小写的全仓检索清扫，历史记录之外不得残留；`make docs-check` 校验 make 目标、仓库路径和锚点。
- 不涉及 `cfg(target_os = "linux")` 代码、migration、Agent 线协议或 schema，无需 Linux 专项验证；不涉及远程执行与部署。

## 未决事项

无。以下为默认选择，用户可调整：历史记录保持原样；仓库外的 Obsidian 笔记不动；后端 API 不因 app 废弃而裁剪。
