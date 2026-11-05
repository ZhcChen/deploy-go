# 文档事实修正与文档门禁增强任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：T001–T004 已实施并验证，T005 自审与提交见下表；全部任务待用户签收。勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-008 FR-009 在 scripts/test/docs-check.py 实现四项检查与 `--self-test`，并在现状上运行，得到真实失败清单。无前置依赖。
- [ ] T002 FR-001 FR-002 FR-003 FR-004 修正已删除 UI 目标引用、部署变量分层、版本示例与主密钥权限说明。依赖 T001 的清单。
- [ ] T003 FR-005 FR-006 FR-007 更新构建代理入口与排障、`make dev-remote-*` 入口，统一 specs/001–009 的 Obsidian 存储标注。依赖 T001。
- [ ] T004 FR-010 扩展 whitespace-check 范围并接入 docs-check 自检到 Makefile。依赖 T001。
- [ ] T005 SC-001 SC-002 SC-003 验证：运行 make docs-check、spec-kit-verify、whitespace-check，构造四类坏例子确认失败；自审最终 diff 与范围，执行 converge；提交并推送。依赖 T002–T004。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已实现 | 通过 | 不适用 | 待用户签收 | docs-check.py 新增 make 目标、仓库路径、锚点、新计划状态头四项检查与 `--self-test`；现状首跑发现 4 项真实问题（`make ui`、`make ui-test`、`agent/refresh`），后收紧占位符与业务示例规则，误报为 0 |
| T002 | 已实现 | 通过 | 不适用 | 待用户签收 | 移除已删除 UI 目标引用并为 ui-preview 标 obsolete；补部署变量分层与 DEPLOY_HEALTH_WAIT_SECONDS；版本示例改占位或指向来源；主密钥权限统一按代码 `mode & 0o077` 表述；每条均回到代码核对 |
| T003 | 已实现 | 通过 | 不适用 | 待用户签收 | 构建代理入口接入 docs/README 与 runbooks 索引；生产手册排障条目更新（代理失效、下载慢无报错、过时的 10808 提示）；补 `make dev-remote-*` 说明；runbooks 与 solutions 中的 specs/00N 路径加 Obsidian 存储标注，spec-kit-workflow 示例改占位名 |
| T004 | 已实现 | 通过 | 不适用 | 待用户签收 | whitespace-check 范围增加 AGENTS.md、specs、.specify；docs-check 目标先运行规则自检；help 文案同步 |
| T005 | 已完成 | docs-check、spec-kit-verify（21 项）、whitespace-check 通过；故意注入 3 类坏引用均被发现并已还原 | 不适用 | 待用户签收 | 自审 diff：19 个文件，仅文档、检查脚本与 Makefile；未触及历史 plans/reviews/handoffs、运行代码与部署脚本；无新增任务，C、D 批次不在本任务内 |
