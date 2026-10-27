---
date: 2026-10-27
topic: privileged-release-artifact-handoff
related_plan: specs/006-privileged-release-artifact-handoff/plan.md
---

# 特权发布制品契约必须覆盖旧单目标路径

本地根因：dispatcher 对所有 release 设置 `privileged=true`，旧单目标 Git 却只有 `_cross_node=true` 才生成上传/下载字段。同节点旧快照未归一化，构建成功后形成缺 `artifact_download` 的特权任务，Agent 在脚本前以 `privileged_release_artifact_required` 拒单。外部白名单未包含该码，进一步掩盖成 `agent_error`。

修复复用现有 verified 制品、下载 lease 与特权授权，不增加无制品执行旁路。拓扑不能决定是否交接；传输开关禁用应在预览、确认和调度明确拒绝，不能静默等待。新门禁检查从未投递的任务，已投递 prepare 可能执行完但 ACK 丢失，已投递 release 可能下载完后 lease 到期，均不能依据当前动态授权状态推翻执行事实。收敛跨多个数据库步骤时必须能恢复 claim 后的 API 重启；多目标失败必须限定尚未调度的 run。

回归不能只断言 JSON 有字段：`api/src/agents/artifact_handoff_tests.rs` 使用真实 dispatcher、本地 HTTP、Agent 上传/下载与特权准入，接合成 executor 验证同/异节点且只启动一次。坏摘要测试模拟初始坏任务或 lease 绑定，不修改 verified 制品不可变事实，不绕过约束。既有模拟 prepare 成功的 fixture 需补齐 verified 记录。

上述证明本地缺陷和修复行为；用户转述的历史现场尚缺版本/配置快照，不能宣称已定位其唯一原因或现场已修复。操作流程见 `docs/runbooks/deployment-recovery.md`，本轮未连接真实节点、执行发布或重试业务部署。
