# 规格与任务入口

以 `FEATURE=specs/<feature>` 为稳定标识，通过项目 wrapper 定位正文。索引登记标识和存储位置；实施、验证、发布与人工签收以对应 tasks 的证据为准，不在索引复制进度。

## 任务入口

| 稳定标识 | 正文存储 | 阅读方式 |
| --- | --- | --- |
| specs/010-workflow-documentation | 仓库，本轮工作流与文档优化 | [需求](010-workflow-documentation/spec.md)、[方案](010-workflow-documentation/plan.md)、[任务与证据](010-workflow-documentation/tasks.md) |
| specs/001–009 | 项目 Obsidian，原 feature 标识保持不变 | 从[存储登记](../docs/standards/spec-kit-storage-register.md#逐项存储登记)选择具体 feature，再由 wrapper 定位实际正文 |

本轮入口为 010；续接其他任务时仍须显式指定相应 feature，不按目录顺序或 Git 分支猜测。旧计划尚未转入 Spec Kit 的，从[计划索引](../docs/plans/README.md)核对来源与真实状态后原地续接。新增或迁移 feature 时同步登记入口与存储，不建立全局活跃指针。

## 定位与续接

```bash
env -u QFY_OBSIDIAN_VAULT_ROOT -u QFY_SPEC_KIT_OBSIDIAN_ROOT \
  make spec-kit-check FEATURE=specs/010-workflow-documentation STAGE=implement
```

010 使用仓库模式，上述命令只在本次调用清除外部根；否则已配置的外部模式会将仓库同名目录判为冲突。读取返回的 `FEATURE_DIR`、`STORAGE` 和 `REQUIRED_READS`，再增量维护正文。结构检查通过不代表已实现、已验证或已签收；具体流程见[工作流手册](../docs/runbooks/spec-kit-workflow.md)。

001–007 的 29 份文件于 2026-10-28 原样迁入 Obsidian，008–009 同样使用项目 Obsidian，本仓库不保留这些 feature 的正文副本或符号链接。项目配置 [.specify/spec-kit.json](../.specify/spec-kit.json) 保存 `obsidian_project_path=QFY/项目/deploy-go`；外部输入为 `QFY_OBSIDIAN_VAULT_ROOT`（Vault 根）或本次显式的 `QFY_SPEC_KIT_OBSIDIAN_ROOT`（Obsidian 项目根）。

```bash
QFY_OBSIDIAN_VAULT_ROOT="<Vault绝对路径>" make spec-kit-check FEATURE=specs/006-privileged-release-artifact-handoff STAGE=implement
```

未配置外部根时可初始化全新仓库 feature，010 即使用此模式。已迁移 feature 必须配置外部根才能访问，不能为绕过缺失重新 init 同名副本。来源、迁移边界和恢复关系见[存储登记](../docs/standards/spec-kit-storage-register.md)与[迁移指纹](../.specify/spec-kit-storage-baseline.json)。外部正文不包含在仓库 push 中。
