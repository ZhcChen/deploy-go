# 卡死任务不再堵塞节点任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：T001–T006 已完成，0.3.34 已于 2026-11-11 上线（见 016 的上线证据）。全部任务待用户签收。勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-001 executor 输出按字节预算分页，并补回归测试（修复前失败、修复后通过）。
- [ ] T002 FR-002 FR-003 主控超期兜底与错误码白名单，并补测试。
- [ ] T003 FR-004 升版 0.3.34，同步 OpenAPI 与双端客户端。
- [ ] T004 FR-005 更新 deployment-recovery 手册，登记 015。
- [ ] T005 SC-001 至 SC-003 本地验证、自审 diff、提交推送。
- [ ] T006 SC-004 上线验证：需用户授权“部署正式环境”；部署前核对线上活动任务，部署后核对卡死任务被中断、排队部署继续、节点升级到 0.3.34。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已实现 | 通过 | 未发布 | 待用户签收 | `ReleaseJobManager::output` 增加 `MAX_OUTPUT_PAGE_BYTES`（协议帧上限减 4KB），至少返回一帧。新增 400KB 无换行输出的回归测试：修复前失败（一页 1,602,192 字节，上限 65,536），修复后通过；Linux 容器内 `make agent-executor-cgroup-check` 通过（含新测试，10 个通过） |
| T002 | 已实现 | 通过 | 未发布 | 待用户签收 | 新增 `interrupt_overdue_deployment_tasks`（期限加 5 分钟，`julianday` 比较，错误码 `task_deadline_exceeded`），worker 每轮调用且失败只告警；对外诊断白名单同步。新测试覆盖宽限期内不处理、超期中断并释放 Agent、`Z` 与 `+00:00` 两种时间写法、不重复处理、排队任务不受影响 |
| T003 | 已实现 | 通过 | 未发布 | 待用户签收 | api、agent、agent-executor、agent-updater、deploy-go-deployer 升至 0.3.34；`make api-openapi api-external-openapi api-client-generate` 生成，`api-client-check` 完整重跑通过（248 秒） |
| T004 | 已实现 | 通过 | 不适用 | 待用户签收 | deployment-recovery 手册新增“超期兜底”与“长输出导致 executor 分页超限”两节；specs/README 与存储登记已登记 015 |
| T005 | 已完成 | make check 全部通过（并行 89 秒；nextest 959 通过、2 跳过）；agent-check、agent-executor-cgroup-check、docs-check、spec-kit-verify、whitespace-check 通过 | 不适用 | 待用户签收 | 验证中发现上一批（013）部署日志功能的缺陷：日志文件名只到秒，同一秒两次运行写入同一文件，契约测试 6 次失败 1 次；已给文件名加进程号并修正测试对“成功部署日志”的判定，连续 10 次通过 |
| T006 | 已完成（部分验证） | 通过 | 已部署 | 待用户签收 | 0.3.34 于 19:29–19:37 部署成功，三个节点全部自动升级到 0.3.34 并在线，executor 分页修复随节点升级到位。**未能验证的部分**：第一次部署失败后线上回滚，卡住的任务在 18:46 被 Agent 重连对账收尾为失败（`deadline_expired`），所以上线时没有超期任务，015 的超期兜底**尚未在线上真实触发过**，只有本地测试覆盖 |
