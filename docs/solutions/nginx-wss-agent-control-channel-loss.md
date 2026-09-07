---
date: 2026-09-07
topic: nginx-wss-agent-control-channel-loss
---

# 反向代理迁移到 nginx 后 Agent 控制通道丢 WSS Upgrade

## 问题

公网入口从 Caddy 迁移到 nginx 后，Deploy Go 的 Agent 节点全部离线：

- API `/readyz` 正常，Agent 的 HTTPS 下载、`agent/refresh` 仍可达。
- `journalctl -u deploy-go-api` 中 `/api/v1/agent/control` 长期出现
  `status=400`，正常时应先鉴权得到 `401`，握手成功再得到 `101`。
- Agent 本地 `doctor` 全部 PASS，但 `CONTROL_CHANNEL_AUTH` 是 WARN；
  系统服务 active，日志只保留启动行，没有连接错误，说明 Agent 在静默退避重连。

## 根因

nginx 通用片段里使用了 `proxy_set_header Connection "";`，会把 WebSocket Upgrade
请求的 `Connection` 头清空，frp 链路因此不再把它识别为 Upgrade 请求，最终 API 收不到
合法 WebSocket 握手，返回 `400`。Agent 无法建立控制通道，心跳无法上报，
节点 `last_seen_at` 不再更新并最终显示 offline。

修复必须同时覆盖公网入口实际代理 API 的每个 server block：

- `deploy.quanxinfu.com`（Web/API 代理）
- `deploy-api.quanxinfu.com`（独立 API 入口）

## 修复

在对应 `location /` 中先建立 Upgrade 映射，再转发 Upgrade 相关头：

```nginx
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}

proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection $connection_upgrade;
```

修改后执行：

```bash
nginx -t
systemctl reload nginx
```

## 验证

```bash
curl -sS --http1.1 -D - -o /dev/null \
  -H 'Connection: keep-alive, Upgrade' \
  -H 'Upgrade: websocket' \
  -H 'Sec-WebSocket-Version: 13' \
  -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' \
  https://deploy.quanxinfu.com/api/v1/agent/control
```

预期不再出现 `400 Bad Request`；无有效 token 的探测会返回 `401 Unauthorized`。
API 日志中随后出现 `status=101`，节点页恢复 online，`agents.last_seen_at`
持续更新。

## 补充经验

- Caddy 的 `reverse_proxy` 默认支持 WebSocket，迁移到 nginx 时不能只验证 HTTP
  入口，必须用带 `Upgrade` 的请求覆盖 Agent 控制通道。
- nginx 通用片段若统一清空 `Connection`，必须确认所有可能承载 WSS/WebSocket 的
  server block 都单独覆盖该头，不能只修第一段配置。
- Agent offline 不能只按“服务 active + doctor PASS”判断；控制通道鉴权是否通过，
  应以 API 的 `/api/v1/agent/control` 日志和节点 `last_seen_at` 为准。
- 生产节点若本地 refresh token 已不在控制面数据库中，属于凭证丢失/串库状态，
  不能靠重试自愈；应按 `docs/runbooks/agent-recovery.md` 走撤销后 rebind，
  重绑不会删除节点、部署目标或历史部署记录。
