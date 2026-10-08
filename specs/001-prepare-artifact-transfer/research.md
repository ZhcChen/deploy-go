# 已有证据与决策

## 运行证据（2026-10-08，只读核查）

部署：`deployment_01M4CW07DQ9BM2RXN0A2VM6756`，prepare 任务：`task_01M4CW07W71SV447Z17RKM4AMM`，构建 Agent：`agent_01KZAHSQKSYYWGHF5J156857MT`，固定提交反馈为 `8ad6708e`。

| 事实 | 证据 |
| --- | --- |
| prepare 失败、脚本退出 0 | 控制面 agent_tasks.result_json：error_code=artifact_transfer_failed，exit_code=0，summary=null |
| HTTP 传输错误 | Agent journal：2026-10-08T04:23:41Z，prepared artifact transfer failed，artifact HTTP 传输失败 |
| 初始化已到达控制面 | deployment_artifacts.upload_size=58516992，archive_digest 已登记；仅初始化 handler 写这组字段 |
| 上传未确认字节 | upload_offset=0，verified_at=null；后续记录为 expired，不把后续过期视为原始失败原因 |
| 业务构建结束 | 最后 deploy.finished 于 04:23:36.236Z，任务失败于 04:23:41.438Z，交接窗口约 5.2 秒 |
| prepare 用时 | started_at=04:23:05.263Z，finished_at=04:23:41.438Z，约 36.2 秒 |
| Git 来源解析 | discovery 任务创建=04:23:02.740Z，完成=04:23:03.299Z，约 0.56 秒 |
| 部署创建 API | POST 返回 201，elapsed_ms=650，request_id=req_01M4CW06TCGW9F29NJRF9H7QJS |

没有保留密钥、请求头、payload、完整 URL 或 Env。当前协议记录为 v18；agents.version 是数据版本，不能当成 Agent 软件版本，故当时运行二进制 commit/版本仍待证据核对。

## 结论与未确认项

业务构建已完成，当前失败收敛至后续发布物传输。初始化已到达控制面并登记数据，但不能证明 Agent 收到初始化成功响应；初始化响应丢失、分块或恢复查询失败均需保留为待验证场景。0 offset 不能单独证明服务器未收到任何字节，没有证据将本次归因为旧 finalize 提交故障。

未确认 DNS、TLS、代理、连接重置、超时、凭证准备或特定请求阶段。`send_authenticated` 会丢弃 reqwest 原因；chunk 失败后的状态恢复也可能掩盖原始错误。已有 journal 不足以进一步归因，当前连通性成功不能证明历史连接正常。

未确认调用方 2 分 33 秒的分布；本次 API 650ms 与 Git 0.56 秒不支持“Git 慢”结论。需要原调用方时间线，不能凭空设置平台性能指标。

## 决策

1. 先补齐结构化且脱敏的传输诊断，复用现有任务结果与查询接口；不开放任意日志/内部文件读取接口。
2. 根因修复必须经过故障注入或实际证据，保持取消、截止时间、进度校验和幂等；暂不决定修改代理、连接池、超时或 chunk 大小。
3. 优先复用 `agent/tests/artifact_transfer.rs` 与 `api/tests/artifacts_api.rs`。项目自身 fixture 约 60 MB，禁止复制业务发布物。
4. 数据存储优先现有 result_json/data，不默认新增 migration 或协议版本；若现有协议明确禁止新增字段，先记录证据再调整方案，不绕过历史门禁。

## 本地基线与 HTTP 合同

2026-10-08：cargo test -p deploy-go-agent --test artifact_transfer upload_baseline，2 项通过。测试通过表示证实当前代码在初始化响应 body 丢失、分块失败后状态查询失联时失败，未执行 finalize；不是修复验收。TCP fixture 使用本地 Axum 响应部分 JSON 后延迟断开，无业务制品/凭证。

只读核查 api/src/artifacts/http.rs：init 同大小/摘要重试返回当前 offset，chunk 已确认同字节范围幂等；status 要求 active+uploading，verified/consumed 后不支持 GET。finalize verified 快路径支持重复 POST，锁等待期间消费可能返回 artifact_lease_consumed；该码不能直接视为成功，须有限重复确认。安全字段与预算见 plan.md。

## 相关历史工作的区别

`docs/plans/2026-09-14-001-artifact-finalize-sqlite-lock-plan.md` 针对 offset 已满、finalize 文件/事务一致性问题；此次 offset 为 0，没有证据证明同一根因。本需求只对新故障和诊断闭环建立唯一入口，不迁移或重做旧计划任务。
