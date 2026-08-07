# API SQLite Migration

## 适用范围

本手册用于本地或经明确授权的环境执行 API SQLite migration。开发 migration 不等于获得操作远程或共享数据库的授权。

## 不可变规则

- 已提交的 `api/migrations/*.sql` 默认不可修改、删除、重命名或重排。
- 修正 schema 必须新增更高版本 migration。
- 只有用户明确说明相关环境可清库重建并授权整理迁移链时，才允许改变历史 migration。

## 本地执行

默认数据库：

```bash
make api-migrate
```

指定数据库：

```bash
DEPLOY_GO_DATABASE_URL=sqlite:///absolute/path/deploy-go.db make api-migrate
```

API 正常启动时也会先执行 migration。migration 失败时服务拒绝监听端口。

## 执行前检查

1. 确认 `DEPLOY_GO_DATABASE_URL` 指向目标文件。
2. 确认当前对话已经授权操作该环境。
3. 停止写入进程或确认应用版本支持在线 migration。
4. 使用 SQLite backup API 或一致性文件副本生成备份。
5. 记录当前提交、数据库路径、文件大小和 `_sqlx_migrations` 内容。

不得在 WAL 模式写入期间只复制主 `.db` 文件，否则备份可能缺少 WAL 中的数据。

### 节点表重建 migration

`0003_node_agents.sql` 会重建 `nodes`，以允许 Agent 节点不保存旧 SSH 连接字段；`0005_agent_node_online_status.sql` 再次重建该表，使无 SSH 配置的 Agent 节点可以进入 `online`。SQLx 的 SQLite migrator 无法在其事务内切换 `foreign_keys`，因此 API migration runner 会逐版本执行 migration，并对 0003、0005 在同一专用连接上执行以下受控流程；运行方仍必须按前置条件停止其他写入进程。

1. 正常执行当前重建版本之前的 migration。
2. 在事务外临时执行 `PRAGMA foreign_keys = OFF`。
3. 由 SQLx 在单个事务中执行当前重建、外键检查和 migration 记录写入。
4. 无论成功或失败，都在同一连接恢复 `PRAGMA foreign_keys = ON`。
5. 继续执行后续 migration；到达下一个隔离重建版本时重复上述步骤。

执行前必须停止 API 及其他数据库写入方，并使用 SQLite backup API 创建一致性备份。例如本机安装了 `sqlite3` 时：

```bash
sqlite3 /absolute/path/deploy-go.db ".timeout 5000" ".backup '/absolute/path/backups/deploy-go-before-node-rebuild.db'"
sqlite3 /absolute/path/backups/deploy-go-before-node-rebuild.db "PRAGMA integrity_check; PRAGMA foreign_key_check;"
```

两项检查必须分别返回 `ok` 和空结果。备份文件不得与运行数据库使用同一路径。

## 验证

```bash
cargo test -p deploy-go-api --test migrations --test database_constraints
make api-check
```

运行环境完成后检查：

- `make api-migrate` 退出码为 0。
- `_sqlx_migrations` 中全部记录 `success = 1`。
- API `/readyz` 返回 `200`。
- 服务日志没有 migration checksum 或约束错误。

### 特权终端会话 migration

`0017_privileged_terminal_sessions.sql` 为现有节点增加默认关闭的
`privileged_execution` 开关，并创建只保存会话生命周期和字节计数元数据的
`terminal_sessions` 表。该表不保存终端输入、输出、命令或 transcript 正文。

升级后额外确认：

```sql
SELECT COUNT(*) FROM nodes WHERE privileged_execution NOT IN (0, 1);
SELECT node_id, COUNT(*)
FROM terminal_sessions
WHERE status IN ('opening', 'active', 'closing')
GROUP BY node_id HAVING COUNT(*) > 1;
PRAGMA foreign_key_check;
```

三项结果都必须为空或计数为 `0`。升级不会自动开启任何节点的特权执行能力；
管理员必须在节点能力满足协议 v5 与 `pty_terminal` 后显式开启。

## 失败恢复

- migration 命令失败后不要反复修改原 migration 重试。
- 保留失败数据库和日志用于定位。
- 如果 migration 在事务内失败，确认 schema 已回滚后新增修正 migration。
- 如果必须恢复备份，先停止 API，保留当前失败文件，再恢复完整一致性备份。
- 恢复后重新核对 `_sqlx_migrations` 和应用提交，不得跳过 migration checksum 校验。

节点表重建失败后的恢复步骤：

1. 保持所有 API 和写入进程停止。
2. 将失败数据库及其 `-wal`、`-shm` 文件整体移到独立诊断目录，不覆盖备份。
3. 使用 SQLite backup API 将已验证的备份恢复到新的运行数据库文件。
4. 对恢复文件执行 `PRAGMA integrity_check`、`PRAGMA foreign_key_check`，并确认 `_sqlx_migrations` 的最高版本仍为恢复前版本。
5. 仅在修复版本和新增 migration 准备完成后重新启动 API；不得编辑已经发布的 0003 或 0005 后原地重试。
