# 计划与执行记录

本目录保存历史实施计划和仍在维护的 legacy 计划。它不是当前产品规范的默认入口；当前规范以 `AGENTS.md`、`docs/standards/`、`docs/runbooks/` 和经 wrapper 定位的 Spec Kit 产物为准。

## 状态约定

新计划在 YAML front matter 中记录：

- `status: active`：仍是当前任务的执行入口，必须链接对应需求、规范、验证和剩余任务。
- `status: completed`：已完成的历史记录，只用于背景和决策追溯。
- `status: superseded`：已被新的 spec、plan 或方案替代，开头注明首选入口。
- `status: paused`：暂缓执行，说明暂停原因和恢复条件。

历史文件中缺少 `status` 的，默认按背景资料处理；只有核对代码、测试和提交证据后，才能把它恢复为 active。不要通过修改历史计划的完成勾选来伪造当前进度。

## 何时新增计划

- 新增功能、协议、权限、数据库 schema、部署阶段或跨模块行为，默认在 `specs/<feature>` 走 `specify -> plan -> tasks -> analyze -> implement -> converge`；计划正文由项目 wrapper 解析到仓库或 Obsidian。
- 只改一条文档、入口链接或已有 runbook 的同步内容，不新建计划。
- 需要保留历史背景时，可以在本目录新增带日期的计划，但必须在开头标明状态、来源和当前首选入口。
- 计划中的命令、文件路径和验收结论变成当前运行规则后，必须同步写入 `docs/standards/` 或 `docs/runbooks/`，不能只留在计划里。

## 与其他文档的关系

```text
brainstorm / 需求来源
        ↓
Spec Kit spec → plan → tasks → implement / converge
        ↓
当前 standard / runbook ← review / solution / handoff（证据与背景）
```

迁移旧计划时保留原始结论和提交证据，补充当前首选入口；不要为了统一格式批量重写历史正文。
