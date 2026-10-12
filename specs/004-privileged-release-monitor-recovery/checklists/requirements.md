# 特权 Release 终态可靠收敛需求质量检查

规格阶段复核；不作为实现验收。需求入口：`specs/004-privileged-release-monitor-recovery/spec.md`。

- [x] 明确节点 durable job 终态与 Deploy Go TaskResult 是两个独立事实。
- [x] 覆盖输出序号缺口、分页未完、暂时性失败、Agent 重启和重复投递边界。
- [x] 明确同一 job 恢复约束，不因未知结果重发 release 或伪报成功。
- [x] 明确 target run 下载态转执行态及终态投影要求。
- [x] 明确相同 TaskResult 重放、冲突结果和迟到状态的幂等边界。
- [x] 规定诊断脱敏、业务应用不改、真实节点不操作等范围边界。
- [x] 每项要求可通过 Agent/executor/API fixture 验证；未决业务歧义为无。
