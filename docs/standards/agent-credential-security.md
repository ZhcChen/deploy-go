---
date: 2026-08-03
topic: agent-credential-security
status: accepted
version: 1
---

# Agent 凭证安全规范

## 凭证类型

- enrollment token：创建或重新绑定指定 Agent，一次性使用，默认 30 分钟过期。
- access token：默认 30 分钟有效，只用于 WSS 握手和当前连接续期。
- refresh token：每 Agent 独立，用于换取下一组 access/refresh token，不直接授权任务。

Token 使用带类型前缀的高熵随机值。服务端只保存带用途域分离的 SHA-256 摘要和必要元数据，比较使用常量时间实现；明文只在签发响应中出现一次。

## 注册

管理员创建 Agent 时，主控先持久化一对一节点和离线 Agent，再签发绑定该 Agent ID 的 enrollment token。生成安装命令时把 token 动态拼接进命令；命令包含 Agent ID 和短期一次性 enrollment token，名称不能作为注册身份来源。

Agent 无本地身份时才消费 enrollment token。已安装且 Agent ID 相同、凭证有效时，安装器保留凭证并只做校验或升级；凭证被撤销时必须使用新 token 重新绑定；Agent ID 不同则拒绝覆盖。

已消费 token 不能在第二台服务器或清空身份后再次注册。重生成安装命令会立即撤销此前未消费 token。

## 本地保存

refresh token 存放在固定数据目录的独立凭证文件中，由 `deploy-go-agent` 服务用户拥有，数据目录权限仅允许 `0700`，或安装器为 runner 路径遍历设置的 `0750 deploy-go-agent:deploy-go-runner`；共享组不得写入，凭证文件固定为 `0600 deploy-go-agent:deploy-go-agent`。安装器以 root 创建目录和文件后再交给服务用户，systemd unit、环境文件和启动参数不得内联长期 token。

更新凭证必须使用同目录临时文件、`fsync` 和原子 rename。发起刷新前先持久化 pending `rotation_id`；收到响应后在同一受保护文件中暂存旧 refresh token、pending `rotation_id` 和新 refresh token，不持久化 access token。Agent 只有在该状态落盘成功后才能发送 `auth_refresh` 确认，确认成功后再原子提升新 refresh token 并清除 pending 状态。任一阶段重启都使用旧 token 与同一 `rotation_id` 重放结果；写盘失败时继续使用仍有效的旧凭证并退避重试。

## 滚动轮换与重用

refresh endpoint 根据当前 credential family 和 generation 签发新 access/refresh token及唯一 `rotation_id`。提交前相同旧 token 与 rotation ID 的网络重试返回同一轮结果，不能产生多个有效后继。

同连接 `auth_refresh` 成功代表 Agent 已持久化新 refresh token。主控随后撤销旧 generation。确认后的旧 token 再次使用视为重用风险，撤销整个 credential family、关闭活动 WebSocket并写入脱敏审计。

管理员撤销 Agent 时同时撤销所有 enrollment、access 和 refresh 凭证，并关闭当前连接。恢复只能重新生成一次性安装命令，不存在全局共享 Agent token 或 SSH fallback。

## 禁止泄漏

以下位置不得出现 access/refresh token、token 摘要或完整安装命令（命令包含 enrollment token，等同敏感内容）：

- 普通应用日志、tracing 字段和错误正文
- 审计摘要
- OpenAPI/JSON Schema 示例
- Agent journal、任务 payload 和部署日志
- systemd unit、进程参数、崩溃上下文和客户端持久化存储

enrollment token 会出现在管理员主动生成的一键安装命令中，因此必须短期、一次性且仅通过 HTTPS 页面展示。安装器禁止 shell xtrace，并不得把 token 回显到 stdout、stderr 或 journald；命令复制出去后仍按凭证管理，过期、已消费或撤销时必须重新生成。
