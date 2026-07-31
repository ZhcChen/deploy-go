# SSH 节点接入与检查

## 适用范围

本手册说明如何把节点接入 Deploy Go 并完成 SSH host key 与能力检查。只有用户在当前对话明确授权具体节点时，才允许对真实节点执行这些操作；常规开发和测试必须使用 `FakeProbe` 或隔离 fixture。

## 前置条件

- API 已配置 SSH 凭证主密钥并完成唯一管理员初始化。
- 管理员已在平台生成 Ed25519 SSH 密钥，并通过 SSH 密钥详情复制公钥。
- 节点部署账号、工作根目录和 secrets root 已由节点管理员预先创建；平台不会安装组件或修改权限。
- 节点管理员已把公钥安装到部署账号的 `authorized_keys`，并从可信渠道取得节点 Ed25519 host key 指纹。

## 接入顺序

1. 创建节点，填写 host、port、部署账号、工作根目录、secrets root，并绑定 SSH 密钥。
2. 请求 host key 扫描。扫描只展示待确认指纹，不建立信任。
3. 通过云控制台、机房控制台或节点管理员提供的独立渠道核对 Ed25519 SHA256 指纹。
4. 提交扫描记录的 `check_id`、`snapshot_hash` 和节点 `version` 完成显式确认。
5. 发起节点检查。检查通过后节点进入 `online`，才可用于部署目标。

`ssh-keyscan` 结果本身不能证明节点身份。不得因为网络可达就直接确认指纹。

## 检查内容

平台使用系统 OpenSSH 客户端并固定启用：

- `BatchMode=yes`
- `IdentitiesOnly=yes`
- `StrictHostKeyChecking=yes`
- 独立临时 `UserKnownHostsFile`
- 固定连接超时和总进程超时

平台通过 SSH stdin 发送固定能力检查脚本，只读取 OS、架构、工作目录存在性和可用磁盘。平台不会隐式使用 `sudo`、安装软件或执行应用部署脚本。

## 失败处理

| failure code | 含义 | 处理 |
| --- | --- | --- |
| `authentication_failed` | 公钥未安装、账号错误或密钥不匹配 | 核对部署账号和 `authorized_keys` |
| `dns_failed` | host 无法解析 | 核对 DNS 或改用明确地址 |
| `timeout` | TCP/SSH 建连超时 | 核对防火墙、端口和网络边界 |
| `host_key_changed` | 已信任 host key 与当前节点不一致 | 立即阻断；通过独立渠道核实后重新扫描和确认 |
| `connection_failed` | 其他 SSH 连接错误 | 查看节点与网络状态，不要关闭严格 host key 校验 |
| `invalid_output` | 能力输出不完整 | 核对远端 shell、基础命令和工作目录 |

更换 SSH 密钥、修改连接地址或工作根目录后节点必须重新检查。解绑 SSH 密钥后节点进入 `missing_credential`，禁止检查和新部署。
