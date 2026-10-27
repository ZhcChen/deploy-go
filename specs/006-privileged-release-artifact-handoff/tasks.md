# 特权发布制品交接修复任务

唯一入口：本目录 spec.md、plan.md。状态：本地实施、验证与只读复核完成，按本轮提交交付；现场未验收。保留编号并增量记录证据。

## 阶段一：复现与设计确认

- [x] T001 在 api/tests/two_stage_deployment.rs 和 Agent fixture 复现旧单目标 Git 缺字段任务与拒单，记录无 ReleaseStart 及入口盘点证据。对应 FR-001/002/009、SC-001。
- [x] T002 核对上传/finalize/下载身份绑定和配置行为，补读相关规范，在 plan.md 固定门禁位置、稳定错误码及 queued/delivered/accepted/running/canceling 处理矩阵。依赖 T001；对应 FR-003/004/005/006。

## 阶段二：交接闭环

- [x] T003 修复 dispatcher 的旧单目标 Git prepare 上传授权与 release verified 制品/摘要/下载 lease 绑定，保持事务和 payload 不可变。依赖 T002；范围 api/src/agents/dispatcher.rs 及对应测试；对应 FR-001/002/005。
- [x] T004 增加入队、待投递任务与配置禁用门禁，必要时补创建/预览守卫；按 T002 安全收敛历史任务，不误伤已接单/运行任务。依赖 T003；对应 FR-003/004/006。
- [x] T005 通过真实 dispatcher、本地 HTTP、Agent 传输/特权准入完成旧单目标 Git 合成交接；覆盖同/不同节点，校验摘要、唯一 release 并保留修复前后证据。依赖 T003/T004；对应 SC-001、FR-001/002/008。

## 阶段三：诊断与回归

- [x] T006 精确放行拒单码及必要新门禁码，补 external API/CLI、旧记录、未知码、脱敏和权限隔离测试；记录公开结构是否变化及生成要求。依赖 T002/T004；对应 FR-007、SC-004。
- [x] T007 验证配置禁用、无制品/未 verified/过期、摘要/Agent/target_run/lease 不符和历史缺字段均无 ReleaseStart，状态一致收敛。依赖 T004/T005；对应 FR-003/004/005、SC-002。
- [x] T008 回归 workspace/多目标/镜像、自动/手动、取消、API 重启/重投、终态重放、已接单任务不重复执行及既有 monitor 恢复。依赖 T005；对应 FR-006/008、SC-003。

## 阶段四：门禁与交付

- [x] T009 更新恢复/外部 API runbook 及必要契约文档；运行 plan.md 的聚焦测试、make api-check、make privileged-release-check、make client-sensitive-check 和按实际改动追加的门禁。依赖 T006/T007/T008；对应 FR-004/007/008、SC-002/003/004。
- [x] T010 安排独立只读复核授权绑定、lease、历史恢复和脱敏，修正并复验；只读收敛后提交推送，交接 HEAD、范围、通过/未运行项。依赖 T009；对应 SC-005。

## 现场验收（不属于本轮授权）

- [ ] T011 取得具体授权后按正式 runbook 发布必要组件，核对现场快照/版本/配置；另经授权执行具体业务验证部署，记录交接、唯一 release 与终态。未有证据前保持现场未关闭。依赖 T010 及运行授权；对应 FR-009、SC-005。

## 本地实施证据（2026-10-27）

- T001：修复前 `single_git_prepare_requires_artifact_transfer_even_on_same_node` 复现缺上传授权并失败；Agent 既有 `env_and_artifact_gate_failures_never_connect_executor` 证明缺下载时无 executor 连接。
- T002/T003/T004：plan.md 已固定状态矩阵；旧 Git 同节点强制交接、事务内校验绑定，配置禁用在预览/确认/调度兜底，保留已投递 prepare 与运行任务；新增恢复 sweep。
- T005：`cargo test -p deploy-go-api --lib artifact_handoff_tests` 2/2 通过，真实 HTTP 上传/download 与 Agent 准入，合成 executor 只收到一次 ReleaseStart；覆盖同/不同节点及多目标运行保护。
- T006/T007/T008：`cargo test -p deploy-go-api --test two_stage_deployment --test agent_end_to_end` 共 36/36 通过；external/CLI、权限隔离与既有恢复测试纳入完整 workspace 门禁。没有公开 DTO/schema/版本变化，生成产物无需改动。
- 测试迭代中的失败均已修正：fixture 必须显式启用传输并插入完整 verified 事实；坏摘要通过历史 payload 或 lease 模拟，不修改 verified 不可变事实或关闭数据库约束。
- T011：没有真实环境授权，未读取节点、发布组件、重试业务部署或修改业务应用；历史现场版本/配置仍未归因。

- T009/T010：`make api-check` 最终通过，957/957 测试通过、2 项既有跳过，doc-tests 与 OpenAPI 校验通过；`make privileged-release-check`、`make client-sensitive-check`、Spec Kit converge/verify 通过。另一个只读复核者四项发现已修复并回归，最终 findings=[]；完整记录见 `docs/reviews/2026-10-27-privileged-release-artifact-handoff-review.md`。
- 完整门禁第一次出现既有 runtime_log_ingest 写锁时序失败；无相关代码改动，单独复查和完整重跑通过。最终 nextest 有 1 项既有端到端测试 leaky 提示，门禁退出 0，不隐瞒为零告警。

- T011 部分完成（用户后续明确授权“部署正式环境”）：源码 7751015 已发布，API/Web、正式首页与内外 OpenAPI 健康。业务验证部署未获授权、未执行，历史现场未关闭；发布证据见上述 review 记录。
