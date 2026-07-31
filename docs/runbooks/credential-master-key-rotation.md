# SSH 凭证主密钥轮换

## 适用范围

本手册用于离线轮换 SQLite 中 SSH 私钥密文的主密钥。操作不连接节点，也不执行远程脚本。

## 前置条件

- 停止 API 写入流量，确保轮换期间没有创建或修改 SSH 凭证。
- 备份 SQLite 数据库及其 `-wal`、`-shm` 文件，或在停止服务后使用 SQLite backup API 生成一致备份。
- 在独立的凭证系统中保留 current 与 previous 主密钥，确认两者版本号不同且均为正整数。
- 主密钥为 Base64 编码的 32 字节随机值。文件方式要求普通文件且权限为 `0600`。

## 配置

将新密钥配置为 current，将旧密钥配置为 previous：

```bash
export DEPLOY_GO_MASTER_KEY_VERSION=2
export DEPLOY_GO_MASTER_KEY_FILE=/secure/deploy-go-master-v2
export DEPLOY_GO_PREVIOUS_MASTER_KEY_VERSION=1
export DEPLOY_GO_PREVIOUS_MASTER_KEY_FILE=/secure/deploy-go-master-v1
export DEPLOY_GO_DATABASE_URL=sqlite://deploy-go.db
```

环境变量直接值与 `_FILE` 变量只能选择一种。命令错误和日志不会打印主密钥正文。

## 执行与验证

```bash
make credential-reencrypt
```

命令每批读取最多 100 条非 current 版本记录，逐条解密、用随机 nonce 重新加密、回读校验后更新。中断后可重复执行；已经迁移的记录会跳过。

完成后检查数据库中只剩 current 版本：

```bash
sqlite3 deploy-go.db 'SELECT key_version, COUNT(*) FROM ssh_credentials GROUP BY key_version;'
```

重新执行 `make credential-reencrypt` 应报告迁移数量为 0。随后以 current + previous 配置启动 API，检查 SSH 密钥列表与详情，再按节点接入手册使用隔离 fixture 验证解密路径。

## 移除 previous

只有同时满足以下条件才可移除 previous：

- 数据库查询只返回 current 版本。
- 重加密命令再次执行迁移数量为 0。
- 已完成数据库备份和隔离恢复演练。

移除三个 `DEPLOY_GO_PREVIOUS_MASTER_KEY*` 变量后重启 API。不要删除旧主密钥备份，直到组织定义的恢复窗口结束。

## 失败恢复

若命令因错误 previous key、损坏密文或数据库错误中止：

1. 保留 current 与 previous，不要继续启动只含 current 的服务。
2. 修正配置后重复执行；已迁移记录不会重复迁移。
3. 若数据库完整性无法确认，停止服务并从轮换前一致备份恢复数据库。
4. 恢复后继续使用旧版本作为 current，确认服务可读取全部 SSH 凭证，再重新安排轮换。

任何恢复步骤都不得把主密钥、私钥明文或密文解密结果写入日志、工单或仓库。
