# 计划与执行记录

本目录保存历史实施计划和仍在维护的 legacy 计划。它不是当前产品规范的默认入口；当前规范以 `AGENTS.md`、`docs/standards/`、`docs/runbooks/` 和经 wrapper 定位的 Spec Kit 产物为准。

## 当前任务与旧任务续接

新任务和本轮工作从[规格与任务索引](../../specs/README.md)进入，按实际 spec/plan/tasks 续写；本目录保留旧计划的决定、验证和签收，不为同一任务再开一份执行记录。

续接旧计划时先核对代码、最近提交、已执行的验证和未验收边界，再说明本次承接的剩余范围。继续原计划时显式记录它是本次执行入口；转入 Spec Kit 时在原计划标明唯一新入口，只迁移剩余任务。

## 状态约定

新计划在 YAML front matter 中记录：

- `status: active`：仍是当前任务的执行入口，必须链接对应需求、规范、验证和剩余任务。
- `status: completed`：已完成的历史记录，只用于背景和决策追溯。
- `status: superseded`：已被新的 spec、plan 或方案替代，开头注明首选入口。
- `status: paused`：暂缓执行，说明暂停原因和恢复条件。

历史文件中缺少 `status` 的，默认按背景资料处理；只有核对代码、测试和提交证据后，才能把它恢复为 active。不要通过修改历史计划的完成勾选来伪造当前进度。

旧状态按下表解释，不批量替换历史字段：

| 历史写法 | 读取方式 |
| --- | --- |
| `complete`、`completed` | 当时声明结束；具体完成范围、验证和人工签收以原文证据为准 |
| `implemented` | 当时声明已实现，仍需核对验证、发布和签收 |
| `accepted` | 当时的接受记录，按原文判断接受的是方案还是实现，不自动推广到后续版本 |
| `in_progress` | 当时声明进行中，不能据此判定当前仍活跃；续接前重新核对事实 |
| 无统一字段 | 背景资料，先核对代码与证据，再决定是否续接 |

任务 `[ ]` 只代表尚未由用户勾选，不单独表达实施进度。当前任务按编号分别记录实施、验证、发布与人工签收；本地测试通过或组件发布不自动等于业务现场验收。

## 历史资料的当前首选入口

| 历史主题 | 当前入口 |
| --- | --- |
| 早期产品、bootstrap 与 UI 计划 | [项目现状](../../README.md)、[开发工程指南](../runbooks/development-guide.md)、[客户端契约](../standards/client-behavior-contract.md) |
| CE/Spec Kit 工具迁移记录 | [当前工作流](../runbooks/spec-kit-workflow.md) |
| 构建、部署和恢复实施记录 | [运行手册目录](../runbooks/README.md)；特定结论再追溯原计划和复核 |

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
