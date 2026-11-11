# 卡死任务不再堵塞节点实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

事故证据（只读核对）：节点上 job 的 `state.json` 为 `failed`、`last_sequence=149`；`output.jsonl` 第 53 至 56 行长 14KB 至 27KB，从第 50 帧起连续 6 帧即超过 64KB；executor 日志每 5 秒一条 `frame exceeds configured limit`；Agent 以 `failure_stage="output_read"` 无限重试。主控启动恢复 `recover()` 只中断“没有任何任务记录”的部署，重启无法解堵；节点自动升级要求该 Agent 没有活动任务，因此卡死任务同时挡住升级。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md)：不手改线上状态；部署需用户明确授权；版本联动同步生成产物；Linux 相关路径用 Docker 专项验证。实现先于规格落笔的原因是用户明确要求紧急处理，已在规格中如实记录。

## 实现方案与文件范围

1. `agent-executor/src/release_job.rs`：`MAX_OUTPUT_PAGE_BYTES`，按落盘行长度累计预算；`agent-executor/tests/release_lifecycle.rs` 增加 400KB 无换行输出的分页回归测试。
2. `api/src/agents/dispatcher.rs`：`interrupt_task_with_code` 与 `interrupt_overdue_deployment_tasks`（`julianday` 比较）；`api/src/deployments/runtime.rs` 在 `process_one` 调用，失败只告警；`api/src/external/mod.rs` 错误码白名单；`api/tests/agents/agent_dispatcher.rs` 增加测试。
3. 五个 crate 版本 0.3.34，`make api-openapi api-external-openapi api-client-generate`。
4. `docs/runbooks/deployment-recovery.md`、索引与存储登记。

## 验证计划

修复前后各跑一次 executor 回归测试；主控新测试；`make api-check`、`make agent-check`、`make agent-executor-cgroup-check`、`make api-client-check`、`make check`、`make docs-check`、`make spec-kit-verify`、`make whitespace-check`。上线验证在获得部署授权后进行。

## 风险与回退

回退为回滚该提交并重新部署；超期兜底只影响已超期的任务。上线后首轮 worker 轮询会立即中断当前所有超期任务（已知只有事故中的一个），需要在部署前再核对一次线上活动任务清单。
