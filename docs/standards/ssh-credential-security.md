---
date: 2026-07-31
topic: ssh-credential-security
status: legacy
version: 2
---

# SSH 凭证安全规范

> 本规范只约束迁移前已保存 SSH 凭证的兼容读取、删除和历史审计。新节点接入与新部署不得使用 SSH 凭证或 SSH executor；当前执行边界以 `docs/standards/agent-control-protocol.md` 和 `docs/standards/deploy-script-contract.md` 为准。

## 术语与边界

- 产品界面使用“SSH 密钥”，领域模型和 API 使用 `ssh_credentials`。
- “SSH 证书”专指 OpenSSH CA 签发证书，首版不实现。
- 平台生成并保管用于节点登录的私钥，只展示公钥、算法和 SHA256 指纹。
- 平台不自动修改远端 `authorized_keys`，管理员负责安装公钥。

## 密钥生成

- 首版只生成 Ed25519 密钥，并使用操作系统密码学安全随机源。
- 公钥保存为 OpenSSH authorized_keys 格式，指纹使用 OpenSSH SHA256 格式。
- 私钥不得在生成响应、详情、日志、错误、审计或测试快照中回显。
- 生成操作只审计名称、算法、指纹和 actor。

## 私钥加密

- 私钥使用经审查的 AEAD 实现加密后写入 SQLite。
- 每条凭证使用独立随机 nonce，并把凭证 ID、算法和密钥版本作为 associated data。
- 数据库保存 ciphertext、nonce、key version、公钥和指纹，不保存主密钥。
- 主密钥从环境变量或权限受控文件加载，生产环境缺失或不合法时服务拒绝启动。
- 主密钥配置、派生结果和解密错误不得进入日志正文。

## 主密钥轮换

- 配置允许一个 current key 和一个 previous key，并指定不同版本号。
- 新写入使用 current key；读取按记录版本选择密钥，不尝试无界密钥列表。
- 离线重加密命令逐批解密、重新加密和校验，并支持中断后继续。
- previous key 只能在全部记录迁移并统计校验后移除。
- 轮换前备份数据库和旧主密钥，恢复步骤写入 `docs/runbooks/credential-master-key-rotation.md`。

## 节点绑定

- 一个凭证可以绑定多个节点，一个节点只能绑定一个有效凭证。
- 被节点引用的凭证禁止删除，API 返回引用节点的最小摘要。
- 解绑后节点进入 `missing_credential`，禁止检查和新部署。
- 绑定、更换和解绑写入审计；更换后节点必须重新检查。

## Host Key 信任

- 首次 `ssh-keyscan` 只生成待确认指纹，不能自动加入 known_hosts。
- 管理员提交待确认记录的 snapshot hash，确认后才写入平台独立 known_hosts。
- 平台固定启用严格 host key 检查，不使用 `StrictHostKeyChecking=no`。
- 已信任 key 变化时阻断连接，保留旧、新指纹供管理员核对。
- 重新信任是独立管理员操作并写审计，节点随后重新执行完整检查。
- `ssh-keyscan` 不能证明身份；生产环境应通过其他可信渠道核对指纹。

## SSH 进程与参数

以下规则只用于回归验证和解释历史实现，不授权新运行链继续发起 SSH：

- 本地 OpenSSH 参数由固定模板构造，host、port、user 和路径分别校验。
- 远端命令经过登录 shell，每个 token 必须经过唯一且受测的 POSIX shell 编码器。
- 禁止 `eval`、二次 shell 和把用户输入直接拼入命令正文。
- 平台固定包装器通过 SSH stdin 发送，并在执行前校验内置 checksum。
- 包装器只负责运行目录、固定上下文、脚本启动、输出转发、PID、取消和退出码。

## 节点本地敏感文件

- 首版只支持节点本地普通文件，不支持平台托管 secret 或外部 secret manager。
- 节点配置 `secrets_root`，引用真实路径必须位于该目录内，并拒绝符号链接逃逸。
- 平台只传递受控文件路径，不读取、上传或记录文件内容。

## 验证要求

- 测试搜索 API、Debug、tracing、错误、审计和日志，证明没有私钥或主密钥。
- 测试覆盖随机 nonce、错误主密钥、版本选择、轮换中断、绑定删除冲突和 host key 变化。
- SSH 测试只使用 mock server 或隔离 fixture，不连接真实节点。
