# 规格与任务入口

以 `FEATURE=specs/<feature>` 为稳定标识，通过项目 wrapper 定位正文。索引登记标识和存储位置；实施、验证、发布与人工签收以对应 tasks 的证据为准，不在索引复制进度。

## 任务入口

| 稳定标识 | 正文存储 | 阅读方式 |
| --- | --- | --- |
| specs/010-workflow-documentation | 仓库，本轮工作流与文档优化 | [需求](010-workflow-documentation/spec.md)、[方案](010-workflow-documentation/plan.md)、[任务与证据](010-workflow-documentation/tasks.md) |
| specs/011-build-proxy-health | 仓库，构建代理可用性检测 | [需求](011-build-proxy-health/spec.md)、[方案](011-build-proxy-health/plan.md)、[任务与证据](011-build-proxy-health/tasks.md) |
| specs/012-docs-fact-sync-and-gates | 仓库，文档事实修正与文档门禁增强 | [需求](012-docs-fact-sync-and-gates/spec.md)、[方案](012-docs-fact-sync-and-gates/plan.md)、[任务与证据](012-docs-fact-sync-and-gates/tasks.md) |
| specs/013-build-test-efficiency | 仓库，构建与检查效率优化（第一批） | [需求](013-build-test-efficiency/spec.md)、[方案](013-build-test-efficiency/plan.md)、[任务与证据](013-build-test-efficiency/tasks.md) |
| specs/014-merge-api-test-binaries | 仓库，合并 API 集成测试二进制 | [需求](014-merge-api-test-binaries/spec.md)、[方案](014-merge-api-test-binaries/plan.md)、[任务与证据](014-merge-api-test-binaries/tasks.md) |
| specs/015-stuck-task-unblock | 仓库，卡死任务不再堵塞节点（紧急修复） | [需求](015-stuck-task-unblock/spec.md)、[方案](015-stuck-task-unblock/plan.md)、[任务与证据](015-stuck-task-unblock/tasks.md) |
| specs/016-startup-recovery-and-build-cache | 仓库，API 启动恢复提速与部署构建缓存（紧急修复） | [需求](016-startup-recovery-and-build-cache/spec.md)、[方案](016-startup-recovery-and-build-cache/plan.md)、[任务与证据](016-startup-recovery-and-build-cache/tasks.md) |
| specs/017-node-concurrent-deployments | 仓库，同节点多应用并发部署（紧急） | [需求](017-node-concurrent-deployments/spec.md)、[方案](017-node-concurrent-deployments/plan.md)、[任务与证据](017-node-concurrent-deployments/tasks.md) |
| specs/018-remove-flutter-admin-app | 仓库，移除 Flutter 管理端（只保留 Web） | [需求](018-remove-flutter-admin-app/spec.md)、[方案](018-remove-flutter-admin-app/plan.md)、[任务与证据](018-remove-flutter-admin-app/tasks.md) |
| specs/019-lease-window-from-delivery | 仓库，任务排队后 Git 凭证租约不再提前过期（缺陷修复） | [需求](019-lease-window-from-delivery/spec.md)、[方案](019-lease-window-from-delivery/plan.md)、[任务与证据](019-lease-window-from-delivery/tasks.md) |
| specs/020-detail-page-live-refresh | 仓库，详情页自动刷新的缺口（Web 前端） | [需求](020-detail-page-live-refresh/spec.md)、[方案](020-detail-page-live-refresh/plan.md)、[任务与证据](020-detail-page-live-refresh/tasks.md) |
| specs/001–009 | 项目 Obsidian，原 feature 标识保持不变 | 从[存储登记](../docs/standards/spec-kit-storage-register.md#逐项存储登记)选择具体 feature，再由 wrapper 定位实际正文 |

当前进行中的入口为 011（构建代理可用性检测，使用仓库模式，命令同样需在调用时清除外部根）；010 为上一轮工作流与文档优化；续接其他任务时仍须显式指定相应 feature，不按目录顺序或 Git 分支猜测。旧计划尚未转入 Spec Kit 的，从[计划索引](../docs/plans/README.md)核对来源与真实状态后原地续接。新增或迁移 feature 时同步登记入口与存储，不建立全局活跃指针。

## 定位与续接

```bash
env -u QFY_OBSIDIAN_VAULT_ROOT -u QFY_SPEC_KIT_OBSIDIAN_ROOT \
  make spec-kit-check FEATURE=specs/010-workflow-documentation STAGE=implement
```

010 使用仓库模式，上述命令只在本次调用清除外部根；否则已配置的外部模式会将仓库同名目录判为冲突。读取返回的 `FEATURE_DIR`、`STORAGE` 和 `REQUIRED_READS`，再增量维护正文。结构检查通过不代表已实现、已验证或已签收；具体流程见[工作流手册](../docs/runbooks/spec-kit-workflow.md)。

001–007 的 29 份文件于 2026-10-28 原样迁入 Obsidian，008–009 同样使用项目 Obsidian，本仓库不保留这些 feature 的正文副本或符号链接。项目配置 [.specify/spec-kit.json](../.specify/spec-kit.json) 保存 Vault 相对的 `obsidian_project_path`（当前值以该文件为准，不在此复制）；外部输入为 `QFY_OBSIDIAN_VAULT_ROOT`（Vault 根）或本次显式的 `QFY_SPEC_KIT_OBSIDIAN_ROOT`（Obsidian 项目根）。

```bash
QFY_OBSIDIAN_VAULT_ROOT="<Vault绝对路径>" make spec-kit-check FEATURE=specs/006-privileged-release-artifact-handoff STAGE=implement
```

未配置外部根时可初始化全新仓库 feature，010 即使用此模式。已迁移 feature 必须配置外部根才能访问，不能为绕过缺失重新 init 同名副本。来源、迁移边界和恢复关系见[存储登记](../docs/standards/spec-kit-storage-register.md)与[迁移指纹](../.specify/spec-kit-storage-baseline.json)。外部正文不包含在仓库 push 中。
