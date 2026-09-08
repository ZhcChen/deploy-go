---
date: 2026-09-08
topic: artifact-download-idle-timeout
---

# 公网 FRP 链路发布物下载卡死与续传

## 问题

跨节点部署的发布物由目标节点 Agent 主动通过 HTTPS 下载。控制面日志、Agent 进程和
HTTP 连接都正常，但部署详情一直停留在“验证候选服务”或等待第二阶段；人工观察时
下载字节在一段时间内没有前进，页面不会进入失败态。

## 根因

Agent 原先只依赖 reqwest 的整体超时。公网入口经过 nginx 与 FRP 时，如果 TCP/HTTP
连接仍然存活但服务端没有继续发送 body，Agent 会一直等待新 chunk；控制面无法从
“连接未断开”和“传输状态”区分出真实无进展，部署状态因此长时间不可确定。

## 修复

在 `agent/src/artifact_transfer.rs` 中增加
`DEFAULT_DOWNLOAD_READ_IDLE_TIMEOUT = 120s`，对每个 HTTP body chunk 的读取设置
无进展窗口：

- 每次收到新 chunk 都重置计时，因此这不是整体下载总超时。
- 连续 120 秒无新字节时，Agent 中止当前响应，按本地
  `.part` 文件已写长度重新发送 `Range` 请求。
- 单次下载最多允许 4 次中断续传，之后返回明确的传输错误；最终仍校验归档摘要。
- 关联的 `agent/src/task_handler.rs` 同时输出“正在下载发布物”和“发布物下载完成”
  阶段日志，便于部署详情定位卡点。

公网入口 nginx 只为 Deploy Go 控制面的两个域名关闭 buffering 并放宽代理超时，
保留 WebSocket Upgrade 转发；修改只涉及
`/etc/nginx/conf.d/qfy-443-frp-web.conf` 的对应 server block，没有改动全局
snippet 或业务站点。修改前已备份：

```text
/etc/nginx/conf.d/qfy-443-frp-web.conf.bak.20260908150932
```

## 验证

- `make deploy-production-agent-build` 通过，生成 Agent 0.2.0 x86_64/aarch64 release。
- `make deploy-production` 通过，目标为 `qfy-test2`（正式控制面）。
- `systemctl is-active deploy-go-api deploy-go-web` 均为 `active`。
- `http://127.0.0.1:30100/readyz` 返回 `{"status":"ready"}`。
- `https://deploy.quanxinfu.com/` 与对应 OpenAPI 入口返回正常。
- 部署详情恢复“正在下载发布物”阶段日志；任务成功、失败或取消后不再残留
  active download lease。

## 补充经验

- 公网链路的“无进展”必须由下载方主动检测，不能只依赖 HTTP 整体超时。
- 阈值应明显小于整体超时，同时给公网抖动留出空间；不要选择个位秒。
- 修改 Agent 下载行为后要同步更新
  `docs/standards/application-deployment-contract.md` 和
  `docs/runbooks/systemd-deployment-production.md`。
