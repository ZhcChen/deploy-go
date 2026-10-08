---
date: 2026-10-08
topic: agent-refresh-wal-download-failure
---

# 下载前刷新失败被误报为制品传输失败

## 可复用排查路径

准备成功但发布只留下“正在下载发布物”时，先读任务终态与耗时，
不要从最后一行推断网络超时。若 release 在百毫秒内失败，
同时查 refresh 请求；下载前的访问凭证准备失败可能根本未发送制品 GET。
使用 task ID、lease ID、时间和 request ID 关联数据库结果与控制面日志，
不读取或输出凭证正文。

生产样本 `deployment_01M4D8BZ14NS8EV03EPT18AGS9` 在该边界失败，
同瞬间 refresh 返回 500；历史底层数据库错误码未保存，因此不能将本地
复现的 SQLite 错误码当作该请求的确证。

## 已验证的代码缺陷

SQLite WAL 的普通事务先 SELECT 后 UPDATE，需要升级读事务。
另一连接持有写锁时，refresh 可立即返回 500；另一连接在 SELECT 后
提交写入时，旧快照写入精确返回 `SQLITE_BUSY_SNAPSHOT`（517）。
busy timeout 不能让过期快照变为可写。

刷新是必然写入的轮换操作，应在读取前 `BEGIN IMMEDIATE`，
然后完成读取、轮换/重放/撤销和 commit。不得只重试事务内部 INSERT，
不得把锁竞争视为 token 失效，也不得放松 token 重用撤销规则。

## 诊断与验证

0.3.30 保留刷新 HTTP 状态，将凭证准备失败独立为
`artifact_download_access_failed`；外部诊断白名单必须同步，否则会再次
降级为 `agent_error`。输出仅允许固定阶段、类别和数值状态码，
不打印 SQL、绑定值、响应正文、URL 或私有路径。

失败诊断先可靠落盘，网络发送最多等待 1 秒，随后保存终态。
验证不能只用正常消费队列：暂停消费并填满队列，检查任务仍能进入 Failed。
真实文件数据库多连接测试还须验证同 ID 重放只产生一个后继，
不会误撤销凭证族或写入重用审计。

操作与恢复入口见 `docs/runbooks/agent-recovery.md`；
本轮证据和验证见 `docs/reviews/2026-10-08-refresh-download-failure.md`。
