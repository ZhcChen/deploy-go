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

## 失败恢复

- migration 命令失败后不要反复修改原 migration 重试。
- 保留失败数据库和日志用于定位。
- 如果 migration 在事务内失败，确认 schema 已回滚后新增修正 migration。
- 如果必须恢复备份，先停止 API，保留当前失败文件，再恢复完整一致性备份。
- 恢复后重新核对 `_sqlx_migrations` 和应用提交，不得跳过 migration checksum 校验。
