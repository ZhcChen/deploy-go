# 同节点多应用并发部署实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

现状（只读核对）：
- 控制面 `api/src/agents/dispatcher.rs`：`try_dispatch` 的投递语句对部署类任务带 `NOT EXISTS (同一 Agent 上其他活动的部署类任务)`；`has_queued_deployment_waiting_for_agent` 用同一规则判断是否有任务在等；全平台并发由 `deployments/runtime.rs` 的系统设置控制。
- Agent 侧：`agent/src/runner_service.rs` 的 broker 用 `Mutex<Option<String>>` 单槽位；启动、取消、清理、恢复都围绕它；任务目录权限在 `INACTIVE_TASK_MODE (0o3700)` 与 `ACTIVE_TASK_MODE (0o3770)` 之间切换，同一时刻只有一个任务目录对 runner 组开放。executor 的 release job 本来就按任务独立（独立进程组与 cgroup，`controls` 按 job 编号）；Agent 的 `admission_lock` 只覆盖启动 runner 并等待进程身份的短窗口，不覆盖任务运行期间；`storage_cleanup` 按同一部署判断活动任务，并发不会误删别的部署。
- broker 与 Agent 共用 `/etc/deploy-go-agent/config`（`EnvironmentFile`），升级不覆盖，适合放本机开关。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md)：新 migration 只新增更高版本、不含 DROP；线协议改动同步 schema 与兼容性测试，不新增任意命令字段；Linux 代码用 Docker 目标验证；不修改业务应用；节点本机配置与重启属于运行态操作，需另行授权。

## 设计

1. **两层控制**：本机开关（能力）× 节点设置（数量）。有效并发 = 能力在 ? 节点设置 : 1。本机开关决定 broker 上限（关 1、开 16）；控制面数量范围 1 至 16。两者天然一致，无需上报数值。
2. **协议**：只新增能力枚举值；broker 本机协议新增 `Capacity` 动作（不改已有的 `VersionResponse`，避免旧 Agent 解析失败）。
3. **隔离**：沿用共享运行身份，已知限制，默认关闭。
4. **控制面**：公共的 SQL 片段计算有效上限，投递与等待判断共用，避免两处分歧。

## 实现方案与文件范围

1. `agent-protocol/src/lib.rs`、`schema/agent-control.schema.json`、`tests/schema_compatibility.rs`：新增能力与常量。
2. `agent/src/runner_service.rs`：活动集合与上限；`Capacity` 动作与客户端方法；恢复多个活动 runner；单元测试（跨平台部分）；`agent/tests/runner_identity_linux.rs` 或新增 Linux 测试覆盖多槽位。
3. `agent/src/main.rs`：启动时查询并声明能力。
4. `api/migrations/0044_node_concurrent_deployments.sql`、`api/tests/migrations.rs`：列、种子与设置调整。
5. `api/src/nodes/mod.rs`：响应字段与新接口，登记 `ApiDoc`；`api/src/agents/dispatcher.rs`：有效上限与两处判断；`api/tests/agents/agent_dispatcher.rs`、`api/tests/platform/…`：测试。
6. 管理端：节点详情页与测试；生成客户端。
7. 文档：两份标准、手册、开启步骤与回退；登记 017。
8. 版本升至 0.3.35，生成 OpenAPI 与双端客户端。

## 阶段与依赖

1. 协议与 broker（含 Linux 验证），无前置。
2. Agent 声明能力，依赖 1。
3. 控制面（迁移、接口、调度），依赖 1 的能力名。
4. 管理端，依赖 3 的接口。
5. 文档、版本、全量验证、提交；上线与节点开启另行授权。

## 验证计划

broker 单元与 Linux 容器测试；控制面调度测试（N=1、N=3、能力缺失、释放名额）；迁移测试；节点接口测试；前端测试；`make api-check`、`make agent-check`、`make admin-check`、`make admin-app-check`、`make api-client-check`、`make agent-runner-isolation-check`、`make agent-executor-cgroup-check`、`make check`、`make docs-check`、`make spec-kit-verify`、`make whitespace-check`。

## 风险与回退

回退：把节点并发数设回 1，或在节点本机关闭开关并重启 runner 与 Agent，行为即回到串行；代码回退为回滚提交并重新部署。迁移新增列与数据调整不可逆删除，但默认值对未开启节点没有影响。
