---
artifact_contract: "ce-handoff/v1"
created_at: "2026-08-06T13:56:34Z"
title: "qfy-test 正式环境部署恢复：生产库迁移 checksum 修复"
summary: "生产库 1-7 CRLF checksum 已在一致性备份后校正，迁移 8-11 与正式部署均已完成，API/Web/Agent 健康"
keywords: ["deploy", "qfy-test", "systemd", "sqlx", "migration", "checksum", "handoff"]
cwd: "/Users/chen/code/deploy-go"
resume_focus: "问题已解决；后续部署应保持 migration LF 不可变，并确保生产 Agent manifest 覆盖当前协议版本"
repository: "ZhcChen/deploy-go"
branch: "main"
head: "669ebc8"
worktree_path: "/Users/chen/code/deploy-go"
---

# 交接：qfy-test 正式环境部署恢复

## 1. 目标与当前状态

- 目标：把当前 `main`（v0.1.0）正式部署到 `qfy-test`，systemd 管理 API/Web，Agent 由部署机本机构建上传。
- 当前状态：**问题已解决并完成正式部署**。生产库 migration 1-11 全部成功，API/Web active，Agent release 0.1.0 已安装。
- 修复前已获得用户对 `qfy-test` 迁移台账修复、重新部署和失败恢复的明确授权。
- 一致性备份：`/var/lib/deploy-go/backups/deploy-go.db.pre-migration-checksum-repair.20260806T140242Z`，SHA-256 为 `b2ba03c3de00cf71c1e74d11ee0e51b1f9f3ccbc196c296fc13795619a7d0d19`。

## 2. 现场环境

- SSH alias：`qfy-test`（root 可登录），服务器架构 `x86_64`。
- systemd 服务：
  - `deploy-go-api`：`127.0.0.1:30100`
  - `deploy-go-web`：`127.0.0.1:30101`
- 生产数据：
  - SQLite：`/var/lib/deploy-go/deploy-go.db`（WAL）
  - Agent release：`/var/lib/deploy-go/agent-releases/0.1.0`
  - 配置：`/etc/deploy-go/api.env`、`/etc/deploy-go/master.key`
  - 安装目录：`/opt/deploy-go/`
- 正式域名：`https://deploy.quanxinfu.com`（HTTPS/WSS 由服务器现有反向代理终止）。
- 当前 qfy-test 状态（已确认）：
  - `deploy-go-api` / `deploy-go-web` 均 `active`
  - `curl http://127.0.0.1:30100/readyz` 返回 `{"status":"ready"}`
  - Web 首页返回 200
  - 无残留 `/opt/deploy-go/.rollback.*`

## 3. 已完成的部署脚本修复（已提交并推送 main）

| 提交 | 内容 |
| --- | --- |
| `9abd917` | Agent Docker 构建缺少 `docs/standards/deploy-artifact-manifest.schema.json`，修复 `.dockerignore` 与 `agent/docker/release/Dockerfile`，并加部署契约断言 |
| `a61013e` | 远端 `install.sh` 依赖 `jq` 但 qfy-test 没有，改为 python3 校验 manifest，移除 jq 依赖并加契约断言 |
| `0ab4bb4` | 修正 `install.sh` 中 manifest 校验 heredoc 的缩进问题（heredoc 内容必须顶格） |
| `669ebc8` | 将正式部署生成和校验的 Agent manifest 协议范围从 `1..1` 同步为 `1..2`，并增加部署契约断言 |

同步更新：
- `deploy/production/test-install-contract.sh`：增加 Dockerfile 上下文、`.dockerignore`、禁止 jq 依赖的回归断言。
- `docs/runbooks/systemd-deployment-production.md`：前置条件说明 Agent release 校验用 Python 3，不需要安装 `jq`。

部署流程本身已验证到远端安装阶段：API/Web 本机构建成功、Agent x86_64/aarch64 构建成功、远端 staging 上传、Agent release 安装成功。

## 4. 问题根因与证据

### 4.1 新 API 启动失败

```text
Error: 执行数据库 migration 失败
Caused by:
    migration 1 was previously applied but has been modified
```

安装器触发回滚并恢复旧版本，旧 API 随后正常启动。

### 4.2 生产库迁移台账

- 生产库 `_sqlx_migrations` 只有 1-7，尚未应用 8-11。
- 生产库 1-7 的 checksum 与当前二进制期望值**全部不同**。
- 生产库 `sqlite_master.sql` 中的 SQL 带 `\r\n`；当前仓库迁移文件是 `\n`。
- 去掉 `\r` 后，生产库与当前 1-7 迁移的表结构一致（差异仅来自尚未应用的 8-11）。
- 结论：生产库由 Windows 环境下的 CRLF migration 初始化；当前仓库在 macOS 下已转为 LF。CRLF 文件的 SHA-384 与生产台账逐项精确匹配，已排除 SQLx 算法变化。

### 4.3 checksum 对照

| version | 生产库当前 checksum | 当前二进制期望 checksum |
| --- | --- | --- |
| 1 | `fca4adf421af408ba312575e3ec16e7c21f862582c6fb8117601faa5af0f51e467f2462773d16bd0f6487b99ddc2c2f9` | `4ac7fafd832bd679cee4f625d531b98180e80d684f7e2f4a444be437011c82b43665d46354f40b232513e9f8b674146c` |
| 2 | `0c809b7a6c18172b14494fdb208c384099523414032750df7db028f9257a73d1d126304e1c5870e56eb4ef81d8a6ad45` | `1b192b741e5a33283c330d4f5e288e6238da2a2cd8f3d28dc71d21b882fbd1a1b14a19a868c0358094e6a79bbf62632c` |
| 3 | `c091f3cee6bb618c2835c6764eda5b361c2c4b3299393ba4ce9e5ac32a60c6656c00a5e575bfe6bc017737fe3e6eb680` | `e2cc53397cfe191816f653cb1079526a8b5e6c427478bd6fe572b64cbfb571b92fb8fee38497312246cff8fa740363d1` |
| 4 | `e553df8cc482c711895aec5a759b04f1c64ad11a706ec919f2e6baa98393dcb2048006fbba8c6a4215969cf0404fc1cb` | `4ed4e23b5122ed3af79d0cafe8cc77272dd1ffb536b5bc531c9e251411ade41e9b1dc7d03c1a6a18746b710c1a770fab` |
| 5 | `e1bedcda0b72369048d423ac0598a5d7b210b298634a4487e53e05a2de1576e7fb621279267fe6d31b10a4188c744043` | `20fa5a8f1bb504c1b475696b8392832ce6a68ffd4f97fd308e66447789899e2b0cee4cc3cabe04077a7d83a47ea7aec1` |
| 6 | `0c5615a4dc923ab98b4fe55d05e3792e4d46450eebff889487ce79e04d67db6373f2241ba3df1a316f1139638afa3984` | `f6cc286597636a1b1a45c0d77a9f509d1fbf16cb4d45f89b46d4509ea0e5d2cd31be4adf142c3a05a4fb8096045ad822` |
| 7 | `f62e3ad22878dfccc061ed917cda4d9b6a7667a1ae2df894f6470d4eec5b8f91a5ace0f6141953c5d86dc6c79feac297` | `3ad09fd3adc01739cbccb831f7d9f7ffaa1afa006760c0883093c399c447d16109e0026988317e8ccb39102807d6adca` |

期望 checksum 生成方式（可复现）：

```bash
tmpdb=$(mktemp /tmp/deploy-go-migrate.XXXXXX.db)
DEPLOY_GO_DATABASE_URL="sqlite://$tmpdb" cargo run -q -p deploy-go-api -- migrate
python3 -c "import sqlite3; db=sqlite3.connect('$tmpdb'); [print(r[0], r[1].hex()) for r in db.execute('select version, checksum from _sqlx_migrations order by version')]"
```

## 5. 修复后状态

- 分支：`main`，部署修复提交 `669ebc8` 已推送。
- 工作区干净，已推送。
- `_sqlx_migrations` 1-7 已校正为 LF 文件 checksum，8-11 已由新 API 正常应用。
- `deploy-go-api`、`deploy-go-web` 均为 `active`；服务器内 `/healthz`、`/readyz`、Web 和 OpenAPI 均通过。
- 正式域名首页与 OpenAPI 返回 HTTP 200；Agent manifest 为 `protocol.minimum=1`、`protocol.maximum=2`。
- 仓库通过 `.gitattributes` 固定 `api/migrations/*.sql` 使用 LF，防止跨平台再次漂移。

## 6. 已执行修复步骤（历史记录）

> 以下操作已经在获得用户明确授权后执行完成，仅作为审计和恢复依据，不是待执行指令。

### 6.1 用户授权

明确说明：只更新 `_sqlx_migrations.checksum` 1-7 为当前二进制期望值；不改迁移文件、不清库、不删数据；执行前做一致性备份。

### 6.2 备份生产库（远端，SQLite backup API）

```bash
ssh qfy-test 'python3 - <<"PY"
import sqlite3
source = sqlite3.connect("/var/lib/deploy-go/deploy-go.db")
backup_path = "/var/lib/deploy-go/deploy-go.db.backup.pre-migration-repair"
target = sqlite3.connect(backup_path)
source.backup(target)
target.close()
source.close()
print("backup", backup_path)
PY'
```

### 6.3 更新迁移台账

```bash
ssh qfy-test 'python3 - <<"PY"
import sqlite3
expected = {
    1: "4ac7fafd832bd679cee4f625d531b98180e80d684f7e2f4a444be437011c82b43665d46354f40b232513e9f8b674146c",
    2: "1b192b741e5a33283c330d4f5e288e6238da2a2cd8f3d28dc71d21b882fbd1a1b14a19a868c0358094e6a79bbf62632c",
    3: "e2cc53397cfe191816f653cb1079526a8b5e6c427478bd6fe572b64cbfb571b92fb8fee38497312246cff8fa740363d1",
    4: "4ed4e23b5122ed3af79d0cafe8cc77272dd1ffb536b5bc531c9e251411ade41e9b1dc7d03c1a6a18746b710c1a770fab",
    5: "20fa5a8f1bb504c1b475696b8392832ce6a68ffd4f97fd308e66447789899e2b0cee4cc3cabe04077a7d83a47ea7aec1",
    6: "f6cc286597636a1b1a45c0d77a9f509d1fbf16cb4d45f89b46d4509ea0e5d2cd31be4adf142c3a05a4fb8096045ad822",
    7: "3ad09fd3adc01739cbccb831f7d9f7ffaa1afa006760c0883093c399c447d16109e0026988317e8ccb39102807d6adca",
}
db = sqlite3.connect("/var/lib/deploy-go/deploy-go.db")
for version, checksum in expected.items():
    db.execute("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?", (bytes.fromhex(checksum), version))
db.commit()
print("updated", db.total_changes)
db.close()
PY'
```

只允许更新 1-7；`_sqlx_migrations` 中不存在 8-11 时不要插入假记录，交给部署二进制正常应用。

### 6.4 重新部署（已完成）

```bash
make deploy-production
```

预期：API 应用 8-11 迁移后启动，Web 重启，Agent 0.1.0 已由远端安装器安装。

## 7. 部署后验证清单

```bash
ssh qfy-test 'systemctl is-active deploy-go-api deploy-go-web'
ssh qfy-test 'curl --fail http://127.0.0.1:30100/readyz'
ssh qfy-test 'curl --fail http://127.0.0.1:30101/'
ssh qfy-test 'curl --fail http://127.0.0.1:30101/api/v1/openapi.json'
ssh qfy-test 'ls /var/lib/deploy-go/agent-releases/0.1.0'
ssh qfy-test 'journalctl -u deploy-go-api --since "10 minutes ago" --no-pager | tail -50'
ssh qfy-test 'python3 - <<"PY"
import sqlite3
db=sqlite3.connect("/var/lib/deploy-go/deploy-go.db")
print(db.execute("select version, description, success from _sqlx_migrations order by version").fetchall())
PY'
```

实际结果：1-11 全部成功，`deploy-go-api`/`deploy-go-web` active，服务器内健康接口、正式域名首页和 OpenAPI 均验证通过。

## 8. 风险与约束

- **必须先备份**：更新台账前用 SQLite backup API 生成一致备份，禁止只复制主 `.db` 文件。
- **不改迁移文件**：`api/migrations/0001-0011` 保持原样，修正只能新增更高版本 migration；本次只是修正台账 checksum，不是修改 SQL。
- **不清库**：不能清库重建；生产库有正式用户/应用/部署数据。
- **不绕过安装锁**：安装器已有 `/run/lock/deploy-go-install.lock`，不要删除锁绕过。
- **回滚策略**：`install.sh` 只自动回滚产物、unit 和 env，不自动回滚数据库。migration 已前进时不能假定旧二进制可启动，必须先核对备份与日志，再按 `docs/runbooks/systemd-deployment-production.md` 和 `docs/runbooks/api-migrations.md` 恢复。
- **远程授权**：本交接不授予任何执行权；下一个 AI 必须按 `AGENTS.md` 与当前用户确认后再动远端。

## 9. 权威参考

- 运行手册：`docs/runbooks/systemd-deployment-production.md`
- 部署脚本：`deploy/production/deploy.sh`、`deploy/production/install.sh`
- 部署契约测试：`deploy/production/test-install-contract.sh`
- 迁移文件：`api/migrations/0001_initial_schema.sql` 至 `0011_deployment_log_stage.sql`
- 计划与执行状态：`docs/plans/2026-08-06-004-git-branch-two-stage-deployment-plan.md`
- 恢复手册：`docs/runbooks/deployment-recovery.md`
- 项目规则：`AGENTS.md`
