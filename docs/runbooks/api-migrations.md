# API SQLite Migration

## 适用范围

本手册用于本地或经明确授权的环境执行 API SQLite migration。开发 migration 不等于获得操作远程或共享数据库的授权。

## 不可变规则

- 已提交的 `api/migrations/*.sql` 默认不可修改、删除、重命名或重排。
- 即使 migration 尚未提交，只要已经应用到任何共享环境，其内容同样不可再修改；
  本地提交前调整只能在从未被共享环境应用过时进行。
- 修正 schema 必须新增更高版本 migration。
- 只有用户明确说明相关环境可清库重建并授权整理迁移链时，才允许改变历史 migration。

## 本地 Git 暂存门禁

版本化的 `.githooks/pre-commit` 是本地开发门禁。首次 clone、切换 worktree，或本地 Git 工具目录被清理后，先显式安装并校验：

```bash
make setup-git-hooks
make verify-git-hooks
```

没有 GNU Make 的环境可直接执行等价入口：

```bash
bash scripts/test/migration-git-guard.sh --setup
bash scripts/test/migration-git-guard.sh --verify
```

`setup-git-hooks` 只为当前仓库写入 `core.hooksPath=.githooks`，并把已提交的 guard 安装到 Git 元数据目录下的私有工具路径。它们不进入工作树或 Git。pre-commit 只直接执行该已安装 guard，不读取工作树的 `Makefile` 或 guard；缺少本地策略副本时会 fail closed 并提示重新运行安装命令。hook 本身不会下载模块、启动 Docker、连接数据库或执行 migration。

门禁只在 Git index 命中 `api/migrations/` 时执行，并只允许在目录根新增 Git index mode 为 `100644`、命名为 `NNNN_snake_case.sql` 的 SQL。版本必须严格大于 `HEAD` 中该目录最大版本，同批次不得重复。已存在 SQL 的修改、删除、重命名、复制、类型变更和未知状态均会被拒绝。

新增 migration 的暂存内容视为权威内容，工作树中未暂存的修复不能绕过门禁。门禁同时拒绝新增 migration 中的 `DROP TABLE` 和 `DROP COLUMN`，注释和 SQL 字符串中的文字不会误报；已不使用的表或字段应标记 deprecated 并保留兼容性，物理删除只能按当前对话显式授权边界处理。

开发时可提前执行：

```bash
make migration-git-guard
make migration-git-guard-staged
make migration-git-guard-self-test
```

该本地控制可以被 `--no-verify`、篡改 hook 或改写 `core.hooksPath` 技术性绕过，不能替代 CI 或服务端策略；本项目协作规则禁止这些绕过方式。它也不能替代 SQLx migration 测试、环境 `status`、备份和发布验收。

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

`0003_node_agents.sql` 会重建 `nodes`，以允许 Agent 节点不保存旧 SSH 连接字段；`0005_agent_node_online_status.sql` 再次重建该表，使无 SSH 配置的 Agent 节点可以进入 `online`；`0008`、`0012` 重建 `agent_tasks`；`0020` 重建 `deployment_targets`。SQLx 的 SQLite migrator 无法在其事务内切换 `foreign_keys`，因此 API migration runner 会逐版本执行 migration，并对 0003、0005、0008、0012、0020 在同一专用连接上执行以下受控流程；运行方仍必须按前置条件停止其他写入进程。

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
make migration-git-guard-self-test
make api-check
```

运行环境完成后检查：

- `make api-migrate` 退出码为 0。
- `_sqlx_migrations` 中全部记录 `success = 1`。
- API `/readyz` 返回 `200`。
- 服务日志没有 migration checksum 或约束错误。

### 特权终端会话 migration

`0017_privileged_terminal_sessions.sql` 为现有节点增加过渡期的
`privileged_execution` 列，并创建只保存会话生命周期和字节计数元数据的
`terminal_sessions` 表。该表不保存终端输入、输出、命令或 transcript 正文。
该历史列不得修改或删除；当前 v11 控制面不读取、不返回也不提供配置入口。

升级后额外确认：

```sql
SELECT node_id, COUNT(*)
FROM terminal_sessions
WHERE status IN ('opening', 'active', 'closing')
GROUP BY node_id HAVING COUNT(*) > 1;
PRAGMA foreign_key_check;
```

两项结果都必须为空。升级后的终端与 release 可用性由在线、身份有效且同时声明
`pty_terminal`、`privileged_release` 的 v11 Agent 决定；旧 Agent 必须重新安装。

### 应用类型与 target_code migration

`0022_application_manifest_and_runtime_status.sql` 做以下非破坏性变更：

- `applications` 增加 `app_type`（`binary` / `redis` / `postgres`）与
  `type_version`，历史应用回填 `binary` / `1`。
- `deployment_targets` 增加 `target_code`，历史值回填为环境值，并新增唯一
  索引 `(application_id, node_id, target_code)`；历史
  `(application_id, environment, node_id)` 唯一约束继续保留。新增目标必须
  在应用内使用稳定、不冲突的 target_code，且仍不能在相同应用、节点和环境
  上重复创建目标。
- `0022` 曾新增 `application_runtime_statuses` 与 `agent_tasks.runtime_status_id`，
  用于运行时状态只读任务；该功能已从平台代码移除，历史表/列按迁移门禁保留但不再使用。

升级前确认没有「同应用同节点、同环境」的历史重复目标（0020 之后不应存在）。
升级后核对：

```sql
SELECT application_id, node_id, target_code, COUNT(*)
FROM deployment_targets
GROUP BY application_id, node_id, target_code
HAVING COUNT(*) > 1;
PRAGMA foreign_key_check;
```

两项结果都必须为空。升级不创建任何部署、不绑定真实节点，也不会自动改变
现有容器。


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

### 已应用 migration 被修改后的 checksum 事故

SQLx 会校验 `_sqlx_migrations.checksum` 与二进制内 migration 源文件的
SHA-384 是否一致。已应用 migration 内容被修改后，新版本 API 启动时报
`migration N was previously applied but has been modified` 并拒绝监听端口；
systemd 会反复重启失败，部署安装器在健康检查超时后回滚到旧二进制。

2026-08-13 在 `qfy-test` 发生过该事故：`0024_application_deploy_contract.sql`
已按旧方案应用到共享环境，随后本地为满足 migration guard 改成了保留旧列的
方案并推送，导致线上 checksum 不匹配。恢复前数据库没有 0023 状态的可用备份，
而线上 schema 实际已满足新迁移的目标状态，因此采用一次性 checksum 对齐修复。

该修复绕过 SQLx 的 migration 完整性校验，只能作为紧急运维手段，并必须同时满足：

1. 当前对话已明确授权执行该数据库修复。
2. 线上 schema 已人工核对，与目标 migration 的最终语义等价。
3. 先停止 API 及其他写入进程，并用 SQLite backup API 生成一致性备份。
4. 只更新 `_sqlx_migrations.checksum` 为当前迁移源文件的 SHA-384，不直接改 schema。
5. 更新后执行 `PRAGMA integrity_check`、`PRAGMA foreign_key_check`，并核对
   `hex(checksum)` 与本地 `shasum -a 384 api/migrations/NNNN_*.sql` 一致。
6. 保留失败库、修复前备份和修复后验证输出，记录到 `docs/reviews/` 或会话交接。

```bash
# 停服务后备份
ssh qfy-test 'systemctl stop deploy-go-api'
ssh qfy-test 'sqlite3 /var/lib/deploy-go/deploy-go.db \
  ".backup /var/lib/deploy-go/backups/pre-checksum-fix-$(date +%Y%m%d%H%M%S).db"'

# 用当前 migration 文件计算 SHA-384，并更新对应版本的 checksum
shasum -a 384 api/migrations/NNNN_name.sql
ssh qfy-test "sqlite3 /var/lib/deploy-go/deploy-go.db \
  \"UPDATE _sqlx_migrations SET checksum = X'<sha384-hex>' WHERE version = <N>;\""
```

常规修复路径仍以恢复一致性备份或新增修正 migration 为准，不鼓励重复使用
checksum 对齐绕过校验。
