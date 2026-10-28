# Spec Kit 稳定入口

官方兼容的 `specs/<feature>` 标识与默认仓库模式保留。001–007的29份文件已于2026-10-28原样迁入Obsidian，008亦在同一项目目录；本目录不再保存这些feature正文，不创建符号链接或重复副本。

项目级配置：`.specify/spec-kit.json` 中 `obsidian_project_path=QFY/项目/deploy-go`。相对Vault的规格目录为 `QFY/项目/deploy-go/10-项目文档/03-计划与任务/Spec Kit/<feature>`。`QFY_OBSIDIAN_VAULT_ROOT` 只提供Vault根；单次 `QFY_SPEC_KIT_OBSIDIAN_ROOT` 指向Obsidian项目根（与参考项目 qfy-trade-in-mall 一致）。

```bash
QFY_OBSIDIAN_VAULT_ROOT="<Vault绝对路径>" make spec-kit-check FEATURE=specs/006-privileged-release-artifact-handoff STAGE=implement
```

读取返回的FEATURE_DIR/STORAGE/REQUIRED_READS后再续写。不配置外部根时仍可初始化全新仓库feature；已迁移feature需配置后访问，不能为绕过缺失新建同名副本。来源、生命周期、指纹及恢复关系见 `docs/standards/spec-kit-storage-register.md` 与 `.specify/spec-kit-storage-baseline.json`，运行规则见 `docs/runbooks/spec-kit-workflow.md`。
