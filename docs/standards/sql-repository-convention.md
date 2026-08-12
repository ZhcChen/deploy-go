# SQL / Repository 统一规范

本规范约束 `api/` 模块的 SQLite 数据访问与 migration 规则，避免 SQL 散落、migration 不可控和破坏性 schema 变更进入共享环境。

## 目录边界

```text
api/migrations/*.sql    SQLx migration，唯一 schema 变更入口
api/src/db/             migration 执行与过滤逻辑
api/tests/migrations.rs SQLx migration 链测试
```

- 表、字段、索引、约束、默认值、字典骨架变化，必须通过新增 migration 落地。
- 已提交的 `api/migrations/*.sql` 默认不可修改、删除、重命名或重排；修复结构问题只能新增更高版本 migration。
- 历史 migration 例外只允许在用户明确说明当前环境可清库重建并授权迁移链路整理时使用；没有这类明确授权时，一律按 forward-only 新增 migration。

## Migration 命名与内容

SQLx migration 文件名必须是：

```text
NNNN_snake_case.sql
```

- `NNNN` 是四位零填充版本号，例如 `0021_application_environment.sql`。
- 版本必须严格大于 `HEAD` 中 `api/migrations/` 的最大版本，同批次不得重复。
- 文件必须是目录根下的普通文件，Git index mode 必须为 `100644`。
- SQLx 将整个文件视为 `Up`，不存在独立 `Down` 段。

已不使用的表或字段只停止应用读写，并在代码和相关文档中标记 deprecated；不得通过新增 migration 执行 `DROP TABLE` 或 `DROP COLUMN`。物理删除属于默认禁止的特殊操作，只能按 `docs/runbooks/api-migrations.md` 的当前对话显式授权边界处理。

## 本地 Git migration 门禁

版本化的 `.githooks/pre-commit` 是本地开发门禁。首次 clone 或切换 worktree 后执行：

```bash
make setup-git-hooks
make verify-git-hooks
```

门禁只在 Git index 命中 `api/migrations/` 时触发，执行以下检查：

- 只允许新增直接位于目录根的 `.sql` 文件，拒绝修改、删除、重命名、复制和模式变更。
- 文件名、版本单调性和批次内版本唯一性按上文规则校验。
- 暂存内容优先，工作树内容不得绕过。
- 新增 migration 拒绝 `DROP TABLE` 和 `DROP COLUMN`；注释和 SQL 字符串中的文字不误报。

开发时可执行 `make migration-git-guard`、`make migration-git-guard-staged` 和 `make migration-git-guard-self-test`。该本地门禁不能替代 SQLx migration 测试、环境 `status`、备份和发布验收。

## 硬规则

1. 数据库结构变化只能新增 migration，不能修改历史 migration。
2. migration 命名、版本和目录边界必须符合本规范。
3. 新增 migration 不得物理删除表或列。
4. 不得使用 `git commit --no-verify`、`git push --no-verify`、临时改写 `core.hooksPath` 或其他方式跳过本地 migration 门禁。
5. migration 修改后必须同步更新 `api/tests/migrations.rs` 的迁移链测试，并运行 `cargo test -p deploy-go-api --test migrations`。
