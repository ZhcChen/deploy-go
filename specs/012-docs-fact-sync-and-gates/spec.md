# 文档事实修正与文档门禁增强

状态：规格就绪
来源：2026-11-05 会话的文档与工作流深度分析（三路只读核查加人工抽查），用户确认先做 A（事实修正）与 B（门禁增强）批次；C（历史文档治理）、D（工作流与 AGENTS 简化）不在本任务内。

## 目标与范围

修正当前有效文档（README、AGENTS 之外的 runbooks、standards、solutions 及入口）中已核实的事实错误和断链式引用，并让 `make docs-check` 能自动发现同类问题，避免再次累积。

明确不做：不改历史 plans、reviews、handoffs 的正文；不批量补状态头；不合并 solutions；不改 AGENTS.md 与 constitution 的规则；不改 `spec-kit.cjs` 行为；不改任何运行代码与部署行为。

## 用户场景与验收

### US1：按文档操作不再被失效内容误导

- 优先级：P1
- Given：读者按 runbook 本地开发、部署或排障
- When：遇到已删除的 UI 目标、两层部署变量、过期的版本示例、不一致的主密钥权限、失效的代理提示
- Then：文档给出与代码一致的说法，且不再引用不存在的 `make ui`、`make ui-test`
- 边界：历史记录里的旧事实保留原样，只在当前有效文档中修正。

### US2：新增的构建代理和 Spec Kit 存放位置可被找到

- 优先级：P1
- Given：读者从 README、docs/README、runbooks 索引或排障入口进入
- When：想了解构建代理、部署卡在拉镜像或下载依赖、或查找 specs/001–009 的位置
- Then：能从导航直达构建代理说明和按症状的排障条目；文档里出现的 `specs/00N-*` 路径旁有统一的“Obsidian 存储”提示

### US3：门禁能发现同类失效

- 优先级：P1
- Given：有人在当前有效文档里写了不存在的 make 目标、失效的仓库路径或坏锚点，或新建了没有状态头的计划
- When：运行 `make docs-check`
- Then：检查失败并指出文件和具体内容；合理的例外（业务应用自己的目标、占位符、Obsidian 存储的 spec、构建产物）不误报

## 功能要求

- FR-001：从 local-development.md、ssh-node-onboarding.md、standards/ssh-credential-security.md 中移除对 `make ui`、`make ui-test` 和“UI 静态检查/UI 预览”的引用；ui-preview-no-cache.md 在顶部标明已过时（`ui/` 已移除）。
- FR-002：systemd-deployment-production.md 增加“变量分层”说明：本机设 `DEPLOY_*`，`deploy.sh` 写入 `install.env` 并转为 `DEPLOY_GO_*`，不要手设 `DEPLOY_GO_API_*`；补充 `DEPLOY_HEALTH_WAIT_SECONDS`。
- FR-003：版本示例改占位或指向来源：agent-onboarding.md 的 `0_3_4` 下载地址、local-development.md 的 fixture 版本说明、development-guide.md 的 fixture 版本数字。
- FR-004：主密钥文件权限统一为“不得带组或其他位（正式环境为 `0400`）”，覆盖 credential-master-key-rotation.md、local-development.md 与生产手册。
- FR-005：生产手册的 Docker Hub `i/o timeout` 排障条目改为指向构建代理，并补充“下载很慢但不报错”的判断与处理；更新过时的 `10808` 提示。
- FR-006：docs/README.md、docs/runbooks/README.md 与 README.md（如适用）加入构建代理入口；remote-development.md 补 `make dev-remote-*` 入口。
- FR-007：runbooks 与 solutions 中的 `specs/001–009` 路径统一标注“Obsidian 存储，见存储登记”；`spec-kit-workflow.md` 的命令示例使用明显占位名。
- FR-008：`scripts/test/docs-check.py` 增加四项检查，范围限于当前有效文档（README.md、AGENTS.md、docs/README.md、docs/runbooks、docs/standards、docs/solutions）：
  1. 反引号 `make <目标>` 必须在 Makefile 中存在（允许清单：业务应用自有目标 `deploy-go-prepare`、`deploy-go-release`；忽略以 `-` 开头的参数）；
  2. 反引号里的仓库相对路径必须存在（忽略含占位符 `<>`、`NNNN`、通配符的路径；允许 Obsidian 存储的 `specs/001`–`009`、`.data/`、`admin-app/build/` 等构建或运行产物前缀）；
  3. Markdown 链接的 `#锚点` 必须对应目标文件里的标题；
  4. 日期不早于 2026-11-05 的 `docs/plans/*.md` 必须有 `status:`（front matter 或首部“状态：”行）。
- FR-009：docs-check 自带可运行的自检（构造正反例），并纳入 `make docs-check`，避免规则本身退化。
- FR-010：`whitespace-check` 覆盖 AGENTS.md、specs 与 .specify 下的 Markdown。

## 成功标准

- SC-001：`make docs-check` 在修正后通过，并对故意构造的四类坏例子（不存在的 make 目标、失效路径、坏锚点、缺状态头的新计划）分别失败。
- SC-002：误报为 0：当前仓库全部有效文档通过检查，无需大量白名单。
- SC-003：`make spec-kit-verify`、`make whitespace-check`、`make docs-check` 全部通过；没有运行代码、API、前端或部署行为变化。

## 风险与验证要求

- 只改文档、文档门禁脚本与 Makefile 的检查范围，不涉及权限、协议、migration 或部署行为；不需要 Rust、前端或 Linux 专项验证，也不需要部署授权。
- 门禁规则的主要风险是误报和性能：以当前仓库全量通过作为误报基线，自检用例覆盖正反例；检查只扫描当前有效文档集合。
- 实现者审阅最终 diff，核对没有改动历史正文和任何非文档运行文件。

## 未决事项

无。批次 C、D 另行决定。
