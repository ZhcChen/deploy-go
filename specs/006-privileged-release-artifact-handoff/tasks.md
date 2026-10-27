# 特权发布制品交接修复任务

唯一入口：本目录 spec.md、plan.md。状态：规划就绪，实施未开始；后续保留编号并增量记录证据。

## 阶段一：复现与设计确认

- [ ] T001 在 api/tests/two_stage_deployment.rs 和 Agent fixture 复现旧单目标 Git 缺字段任务与拒单，记录无 ReleaseStart 及入口盘点证据。对应 FR-001/002/009、SC-001。
- [ ] T002 核对上传/finalize/下载身份绑定和配置行为，补读相关规范，在 plan.md 固定门禁位置、稳定错误码及 queued/delivered/accepted/running/canceling 处理矩阵。依赖 T001；对应 FR-003/004/005/006。

## 阶段二：交接闭环

- [ ] T003 修复 dispatcher 的旧单目标 Git prepare 上传授权与 release verified 制品/摘要/下载 lease 绑定，保持事务和 payload 不可变。依赖 T002；范围 api/src/agents/dispatcher.rs 及对应测试；对应 FR-001/002/005。
- [ ] T004 增加入队、待投递任务与配置禁用门禁，必要时补创建/预览守卫；按 T002 安全收敛历史任务，不误伤已接单/运行任务。依赖 T003；对应 FR-003/004/006。
- [ ] T005 通过真实 dispatcher、本地 HTTP、Agent 传输/特权准入完成旧单目标 Git 合成交接；覆盖同/不同节点，校验摘要、唯一 release 并保留修复前后证据。依赖 T003/T004；对应 SC-001、FR-001/002/008。

## 阶段三：诊断与回归

- [ ] T006 精确放行拒单码及必要新门禁码，补 external API/CLI、旧记录、未知码、脱敏和权限隔离测试；记录公开结构是否变化及生成要求。依赖 T002/T004；对应 FR-007、SC-004。
- [ ] T007 验证配置禁用、无制品/未 verified/过期、摘要/Agent/target_run/lease 不符和历史缺字段均无 ReleaseStart，状态一致收敛。依赖 T004/T005；对应 FR-003/004/005、SC-002。
- [ ] T008 回归 workspace/多目标/镜像、自动/手动、取消、API 重启/重投、终态重放、已接单任务不重复执行及既有 monitor 恢复。依赖 T005；对应 FR-006/008、SC-003。

## 阶段四：门禁与交付

- [ ] T009 更新恢复/外部 API runbook 及必要契约文档；运行 plan.md 的聚焦测试、make api-check、make privileged-release-check、make client-sensitive-check 和按实际改动追加的门禁。依赖 T006/T007/T008；对应 FR-004/007/008、SC-002/003/004。
- [ ] T010 安排独立只读复核授权绑定、lease、历史恢复和脱敏，修正并复验；只读收敛后提交推送，交接 HEAD、范围、通过/未运行项。依赖 T009；对应 SC-005。

## 现场验收（不属于本轮授权）

- [ ] T011 取得具体授权后按正式 runbook 发布必要组件，核对现场快照/版本/配置；另经授权执行具体业务验证部署，记录交接、唯一 release 与终态。未有证据前保持现场未关闭。依赖 T010 及运行授权；对应 FR-009、SC-005。
