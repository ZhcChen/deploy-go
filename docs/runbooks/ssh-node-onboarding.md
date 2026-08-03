# SSH 节点接入（已退役）

## 当前结论

SSH 不再是新节点接入、节点检查或部署执行的运行依赖。不得通过 Deploy Go 创建 SSH 凭证、扫描/确认 host key、手工创建 SSH 节点或使用 SSH fallback。新节点接入以 `docs/runbooks/agent-onboarding.md` 为准。

## 历史数据

- 已存在的 `ssh_credentials`、节点 SSH 字段和 host key 字段由 migration 保留，便于审计与迁移核对。
- 管理 API 只保留管理员查询和删除 legacy SSH 凭证；不提供生成、重命名、绑定或连接操作。
- 删除 legacy 凭证时 API 在同一事务中清空历史节点引用并记录 `detached_nodes`，不会连接节点。
- 正式 Web、Flutter 和 UI 预览不提供 SSH 日常入口。

## 清理前检查

1. 确认目标历史节点已有 Agent 关联，并且节点、部署目标和历史记录保持原 ID。
2. 确认 Agent 在线，`SystemInspect` 已验证 `deploy-go-agent` 对工作目录、脚本和 secret 引用具有所需权限。
3. 确认新部署已通过 Agent task 完成，且没有任何运行链引用 SSH executor。
4. 备份 SQLite 后再通过 legacy DELETE API 清理凭证；不要直接修改数据库外键。

对真实节点安装 Agent、修改目录权限或执行验证脚本仍需当前对话中的明确授权。
