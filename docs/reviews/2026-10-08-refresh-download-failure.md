# 凭证刷新与发布物下载失败修复

## 范围与证据

本轮为有明确边界的 Bug 修复，沿用项目直接复现、实施、验证流程，
不建立与 `specs/001-prepare-artifact-transfer/` 平行的制品传输规格。
仅修改 Deploy Go；业务应用代码、配置、脚本和发布物未修改，不重发业务部署。

生产部署 `deployment_01M4D8BZ14NS8EV03EPT18AGS9` 的 prepare 成功，
制品 finalize 返回 200。release 从 07:59:50.463 UTC 投递到
07:59:50.586 UTC 失败，错误为 `artifact_download_failed`。
07:59:50.547 UTC 的 refresh 请求 `req_01M4D8D5RJEX8C1YQYNWS18KKM`
返回 500，未见该 download lease 的 GET。旧实现凭证获取失败归为 Transport，
任务层再吞为通用下载失败。历史日志未保留底层 SQL 错误，原请求的数据库错误码未确认。

## 修改与验证设计

- API refresh 在读取凭证前取得 SQLite 写锁，保留轮换、同 ID 重放和重用撤销语义。
- 刷新数据库失败记录 request ID、固定 stage/category 和数据库错误码，不记录 SQL/绑定值/错误原文。
- Agent 保留刷新 HTTP 状态；下载凭证准备失败返回独立错误码，并在任务终态前输出脱敏诊断。
- 对外诊断白名单保留新错误码与下载超时码，未知错误继续降级为 `agent_error`。
- 文件数据库 WAL、多连接回归：未修真实 refresh 在 writer 持锁期间返回 500；修后返回 200。
- 精确快照复现：读事务完成 SELECT 后另一连接提交实际修改，读事务写入返回 517。
- 验证轮换仅生成 generation 1/2、两个 access session、族不撤销、无重用审计；HTTP 500 不输出响应正文。
- 验证凭证准备失败不发送制品 GET，网络失败保持传输分类，失败正文先于 TaskResult。

## 执行状态

本地验证通过：API enrollment 16、external API 24、WSS 7；
Agent artifact transfer 29、token refresh 5、task handler 7（包含满队列终态落盘）。
Agent lib/bins 严格 clippy 通过；API 严格 clippy 仍有既有 5 类告警，
显式允许这些基线类别后 lib/bins 通过，无本轮新增告警。
正式部署脚本/安全契约与 Web server 5 项检查通过。
独立 correctness 复核发现新增诊断发送可能受背压无界阻塞；
改为先持久化、网络发送最多等待 1 秒，补回归后复核无阻塞发现。
OpenAPI 两份产物仅版本变为 0.3.30，双端生成已执行。
成对发布版本为 0.3.30，正式上线验收后补充运行结果。

## 恢复

本轮无 migration。可按 `docs/runbooks/systemd-deployment-production.md` 恢复上一套成对产物；
保留数据库、凭证族、任务日志与恢复元数据，不重绑 Agent 或重跑历史业务任务。
