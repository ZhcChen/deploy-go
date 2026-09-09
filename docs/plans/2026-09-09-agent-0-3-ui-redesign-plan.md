---
title: Agent 0.3.0 与控制面暗色 UI 重构计划
status: completed
date: 2026-09-09
schema_version: 1
---

# Agent 0.3.0 与控制面暗色 UI 重构计划

## 背景

节点侧 Agent 已完成运行态优化，但没有控制协议 wire 变更，可以发布新的应用版本。
本轮把 API、Agent、executor 与 deployer 同步升级到 `0.3.0`，保留 Agent 控制协议 `v15`、
最低兼容协议 `v11`、executor 本机协议 `v3`。

同时按参考截图重构正式控制面：以深色为主视觉，节点列表改为可点击、可承载运行遥测的
卡片网格，其余页面通过统一 CSS tokens 继承同一套暗色主题。

## 范围

- 四端版本同步、发布目录 fixture、OpenAPI 和双端生成客户端。
- 控制面全局暗色 tokens、AppShell、节点卡片与节点遥测。
- 不修改 Agent 控制协议、执行器协议、安全边界和数据库 migration。
- 不连接真实节点，不部署真实环境。

## 执行单元

### U1：版本升级

同步四个 `Cargo.toml`、`Cargo.lock`、`admin-app` 支持版本、当前版本 fixture、测试与文档，
运行 OpenAPI/客户端生成后提交。

### U2：节点列表暗色卡片化

节点列表按可见节点并发读取遥测，失败不阻塞渲染；卡片显示状态、环境、Agent、协议、
CPU/内存/工作盘指标，并在窄屏保持可用。

### U3：全局暗色主题

统一背景、表面、边框、按钮、表单、状态徽标和页面容器；保留日志、代码编辑器和终端
已有的深色工作区。

### U4：回归与发布物验证

执行 `make admin-check`、聚焦 API/Agent 发布检查、OpenAPI/客户端漂移检查和
`make privileged-release-check`；完成后提交推送。

## 验证命令

```bash
make admin-check
make admin-test-e2e
make api-openapi-check
make api-client-check
make privileged-release-check
make deploy-production-agent-build
```

`deploy-production-agent-build` 仅在本机构建，不上传、不连接真实节点。

## 验证记录

全部通过：

- `make admin-check`：154 个 Web 组件测试、lint、typecheck、production build。
- `make admin-test-e2e`：23 个 Playwright 用例，含桌面/窄屏节点卡片回归与截图。
- `make admin-app-check`：Flutter analyze 与 51 个测试。
- `make api-openapi-check`、`make api-client-check`：生成物无漂移。
- `make privileged-release-check`：安装、权限、隔离、协议、发布授权及 API 聚焦测试。
- `make deploy-production-agent-build`：本机成功构建 Agent/executor Linux x86_64 与 aarch64 发布物。
