# 特权发布制品交接实施方案

规格：spec.md
状态：规划就绪，尚未实施

## 已核实基线

HEAD 161f179，规划开始时工作区干净。api/src/agents/dispatcher.rs 中 create_stage_task 将所有 release 设置 privileged=true，却仅在 image_mode 或 _cross_node=true 时填充 artifact_download。ensure_deployment_task 的旧单目标 Git 分支直接使用原快照，不像 workspace/多目标分支补齐传输上下文和制品绑定。

agent/src/task_handler.rs 对上述组合拒单并返回 privileged_release_artifact_required，尚未启动脚本；api/src/external/mod.rs 白名单漏掉该码，因而显示 agent_error。

DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED 默认关闭，控制部分调度与上传授权；只开启它不会自动补齐旧单目标 Git 的 _cross_node。这证明当前代码有不一致，不证明历史现场运行的版本和触发分支。

## 约束与文件范围

遵循 AGENTS.md、CLAUDE.md、.specify/memory/constitution.md、docs/runbooks/spec-kit-workflow.md、docs/runbooks/deployment-recovery.md。实施前补读 docs/runbooks/external-deploy-api.md、docs/runbooks/local-development.md、docs/standards/application-deployment-contract.md 和现有特权/制品授权规范。

当前写入者串行集成；独立复核者只读。默认不改 migration、线协议、DTO、版本号或全局传输重试策略。本轮仅规划。

## 实施方案

### A：复现与设计确认

在 api/tests/two_stage_deployment.rs 复现旧单目标 Git 真实调度产生 privileged=true/artifact_download=None；用 agent/tests/two_stage.rs 或 task_handler 内部 fixture 验证拒单且没有 ReleaseStart。盘点管理面/外部入口和历史快照的实际可达性。

核对上传/finalize/下载 lease 的身份绑定、校验和撤销，确定共用门禁位置及稳定错误码。_cross_node 不能作为同节点免交接的理由。

### B：修复完整交接链路

在 api/src/agents/dispatcher.rs 为旧单目标 Git prepare 生成现有上传授权；release 复用 verified 制品查询和下载 lease，绑定摘要、target_run 与 Agent。制品绑定、target run、lease 和任务创建保持事务一致，失败不能留下可投递半成品。

不能只扩大 artifact_download 的条件：上游必须实际上传并形成可信记录。派生调度上下文不覆写 immutable snapshot 或已有任务 payload/digest。

保留现有传输开关，在最早可确定处对禁用/能力不满足明确拒绝或失败，调度器兜底处理配置变化与历史入口；必要时仅在 api/src/deployments/ 创建/预览逻辑补门禁。已经运行的 prepare/上传任务按实际状态机收敛，不全部一刀切失败，不永久等待。

### C：入队与历史投递保护

新任务入队和 queued/delivered 重投前验证下载请求及数据库绑定；无可信制品不签发授权，不从节点 staging 补造制品，不自动重跑 prepare。历史缺字段任务使用既有失败收敛机制同步 task/target run/deployment；已失败任务不复活。

accepted/running/canceling 不批量改判失败，不修改 payload，不重发 ReleaseStart，沿用原任务对账、取消和 monitor 恢复。不确定状态保留 interrupted/人工核查语义。T002 在编码前固定分状态动作矩阵及对应测试。

Agent 的拒单保护保留，预期无需修改 Agent 执行逻辑；集成验证若发现新缺口，先记录证据并更新范围。

### D：诊断与文档

api/src/external/mod.rs 精确放行原拒单码与必要新门禁码，不放开任意前缀。api/tests/external_api.rs、external_openapi_contract.rs 与 deploy-go-deployer/tests/ 验证真实响应/CLI、旧记录、未知码、脱敏和权限隔离。

更新 docs/runbooks/deployment-recovery.md、docs/runbooks/external-deploy-api.md，必要时同步 docs/standards/application-deployment-contract.md，说明开关前置条件、历史任务处理和禁止盲重试。结构不变无需生成客户端；确有 schema 变化才运行生成器，不手改生成产物。

## 验证与验收

矩阵：旧单目标 Git 同/不同节点 × 自动/手动；workspace、多目标、镜像正常交接；传输禁用、制品缺失/未 verified/过期、摘要/Agent/target_run/lease 不符；重复调度、API 重启、取消和终态重放；历史 queued/delivered 缺字段，accepted/running 不误伤。

至少一个旧单目标 Git 用例复用 api/tests/artifacts_api.rs 的本地 HTTP 和 Agent 客户端，完成上传/finalize/下载，再接既有 Agent/executor 隔离准入 fixture，证明只执行一次 release。使用 deploy-go 合成 fixture，不使用业务代码/制品。

实施阶段命令：

```bash
cargo test -p deploy-go-api --test two_stage_deployment --test artifacts_api
cargo test -p deploy-go-api --test external_api --test external_openapi_contract
cargo test -p deploy-go-agent --test two_stage --test artifact_transfer --test privileged_release_recovery
cargo test -p deploy-go-deployer
make api-check
make privileged-release-check
make client-sensitive-check
git diff --check
```

改 Agent/协议/安装器另跑 make agent-check；改 Linux 路径必须执行匹配 Docker 门禁，不能以 macOS 代替。改公开 schema 则执行 make api-openapi、make api-client-generate、make api-client-check，并按实际客户端改动运行对应门禁。

规划仅运行 Spec Kit plan/tasks/analyze 检查、make spec-kit-verify、make whitespace-check 和 diff 检查，不将业务测试列为已通过。

## 发布与恢复

本地通过后安排独立只读复核身份绑定、lease 生命周期、配置变化、历史活动任务与脱敏，修复发现后复验并做收敛检查。

预期控制面修复可复用现有 Agent 协议，不预先安排全平台升版；若证据要求 Agent 改动，按成对发布和版本联动规则另列清单。控制面发布、节点更新、真实查询或业务验证部署均须具体授权，本轮不执行。

代码用独立提交 revert，运行版本按正式 runbook 备份恢复。不删除制品、lease、journal，不手改数据库，不复活 failed 部署。历史无可信制品部署保留失败事实；修复后经具体授权才能新建业务部署。真实验收核对应用、目标、版本和幂等键，未完成前只报告本地结果。
