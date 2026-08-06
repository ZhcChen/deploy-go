---
title: 运行日志查看功能计划
status: completed
date: 2026-08-06
---

# 运行日志查看功能计划

## 目标

在不改变 API stdout 主日志输出的前提下，为管理员提供进程运行日志的实时查看能力。所有 HTTP 请求完成时记录 `req_ + ULID` 请求 ID、方法、路径、状态码和耗时，并允许页面按级别、Request ID 和 target 筛选。

## 范围边界

- 沿用 `tracing` 与 `tracing-subscriber`，不引入另一套日志 facade。
- stdout 始终独立输出；查看功能故障不得影响 stdout 或请求处理。
- 自定义 `Layer` 只采集结构化事件，通过 bounded Tokio channel 的 `try_send` 投递给后台任务。
- 后台任务维护固定容量内存环形缓冲并广播新增事件；进程重启后日志清空。
- 只向管理员开放查询与 SSE 接口。
- 不采集请求体、响应体、Cookie、Authorization、CSRF、密码、密钥、token 或部署 secret。
- 首版不落 SQLite，不接入 Loki、ELK、OpenTelemetry Collector 等外部平台。

## 实施单元

### U1：异步采集链路

新增运行日志模块，实现结构化字段 visitor、有界发送通道、单调 sequence、内存环形缓冲和广播。stdout formatter 与采集 layer 组合注册。

验证：采集不阻塞；缓冲按容量淘汰；保留 level、target、message、request_id 和普通结构化字段。

### U2：请求日志与管理接口

请求中间件记录所有完成请求的状态和耗时。新增管理员运行日志 SSE 接口，支持游标、level、request_id、target 筛选，并持续验证会话有效性。

验证：普通用户被拒绝；管理员可读取历史窗口并继续接收新事件；无效游标和筛选参数返回明确错误。

### U3：运行日志页面

在系统设置导航增加“运行日志”，提供终端式日志视图、实时连接状态、自动跟随、清空当前视图和筛选控件。

验证：页面可实时追加；筛选变化会重连并清空旧结果；日志数量受浏览器端上限约束。

### U4：测试与发布验证

补齐 Rust 单元/接口测试和 React 页面测试，运行格式化、静态检查、单元测试与构建。部署动作仅在用户明确授权后执行。

## 风险控制

- channel 满时丢弃 UI 采集副本并累计 dropped 数，不阻塞业务线程。
- SSE 慢消费者落后时通过广播 lag 事件提示重新连接。
- 日志字段由代码显式生成；页面以文本渲染，禁止 HTML 注入。
- 内存容量固定，避免日志流量导致无界增长。
