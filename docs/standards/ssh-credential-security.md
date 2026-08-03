---
date: 2026-08-03
topic: ssh-credential-security
status: legacy
version: 3
---

# SSH 凭证安全规范（Legacy）

## 边界

SSH 凭证只用于迁移前数据的兼容读取、删除、主密钥轮换和历史审计。新节点接入、节点检查和部署执行不得使用 SSH、host key 或 SSH executor；当前执行边界以 `docs/standards/agent-control-protocol.md`、`docs/standards/agent-credential-security.md` 和 `docs/standards/deploy-script-contract.md` 为准。

## 允许的操作

- 唯一管理员可查询历史凭证列表和详情；响应只包含 ID、名称、算法、公钥、指纹和时间，不包含 ciphertext、nonce 或私钥。
- 唯一管理员可携带有效 CSRF token 删除历史凭证。API 必须在同一事务中清空历史节点引用、删除凭证并审计 `detached_nodes`，不得连接节点。
- 离线 `credential-reencrypt` 可为尚未清理的数据轮换 AEAD 主密钥；数据库只保存 ciphertext、随机 nonce 和 key version。

## 禁止的操作

- 不得生成、重命名、绑定或解绑 SSH 凭证。
- 不得扫描、确认或重新信任 host key。
- 不得以 SSH 作为 Agent 离线时的 fallback，也不得通过 SSH 自动安装 Agent。
- 正式 Web、Flutter 和 UI 预览不得把 legacy 凭证展示为日常管理入口。

## 敏感数据

历史私钥、主密钥和解密错误不得进入 API 响应、Debug、tracing、审计正文、客户端、fixture 或部署日志。主密钥仍从权限受控文件或环境加载，轮换与恢复遵循 `docs/runbooks/credential-master-key-rotation.md`。

历史节点接管与清理步骤见 `docs/runbooks/ssh-node-onboarding.md`。
