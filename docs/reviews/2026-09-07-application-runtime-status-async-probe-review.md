# 应用列表运行状态异步探测复核

## 结论

U1-U5 已完成本地实现与复核：应用列表「运行状态」在首屏展示最近部署验证
结果后，管理端会对当前页活动应用异步发起平台真实运行探测并回填。本次复核
未连接真实节点，未执行生产 migration、控制面部署、Agent 升级或业务发布。

## 契约核对

| 项目 | 实现结论 |
| --- | --- |
| 探测端口 | `verification_config.http.port` 可选；缺失时回退 `image_spec.host_port`，仍缺失则跳过平台探测并展示原因。 |
| 多模块边界 | 多 active target 应用不做单端口探测，继续沿用最近部署验证结果。 |
| 安全边界 | runtime probe 只访问 `127.0.0.1`；不开放 command、任意 URL、shell 或 env。 |
| 协议 | latest v15，新增 `runtime_probe_v1` capability 与 `runtime_probe` 普通任务；v14 schema 快照不可变。 |
| API | 批量 `POST /api/v1/applications/runtime-probes`，上限 20、管理员 + CSRF；离线/旧 Agent 收敛为带原因的 failed。 |
| 列表回填 | pending/running 显示检测中；成功覆盖最近部署状态；服务可达类失败显示异常；Agent 不可达/旧版不把运行中误标为异常。 |
| UI | 管理端首屏后自动发起探测，1.5 秒轮询到终态后停止；Tooltip 区分部署验证与平台探测时间。 |

## 复核发现与修正

- 协议 v15 升级时，`workspace_release_compatibility` 一直使用当前
  `PROTOCOL_VERSION` 作为 workspace 最低门槛，导致 v15 发布后 v14
  workspace 任务被误判不兼容；`deployment_targets` 与
  `application_workspace_sources` 同样被顶到 v15。本次在 `agents` 模块统一
  声明 `WORKSPACE_MIN_PROTOCOL_VERSION = 14`，dispatcher 兼容检查、sweep
  SQL、workspace 来源校验与目标创建同步使用该常量；
  `agent_dispatcher` 全量 31 个测试恢复通过。
- `sweep_incompatible_agent_tasks` SQL 新增 runtime probe 条件时丢失外层
  右括号，产生 `near "ORDER": syntax error`；已修复并补充 bind 常量。
- 应用列表与重复探测入口只认「有活跃 Agent 任务的 pending/running」或
  终态记录，避免历史遗留的 pending 行永久卡住状态。
- Agent enroll OpenAPI 的协议上限同步到 v15，修复协议 v15 提交遗漏。
- 管理端探测状态改为 `useMutation` + 定时收敛，避免 React Hooks lint 的
  同步 setState-in-effect 问题；MSW 增加 runtime-probes 默认 handler。

## 验证记录

```text
cargo test -p deploy-go-agent-protocol          # 23 passed
cargo test -p deploy-go-agent --lib runtime_probe # 3 passed
cargo test -p deploy-go-api --test runtime_probe_api \
  --test agent_dispatcher --test applications_api # 42 passed
cargo clippy -p deploy-go-api --all-targets -- -D warnings
make api-openapi-check
make api-client-check
npm run check --workspace deploy-go-admin        # lint/typecheck/154 tests/build
git diff --check
```

## 回滚与发布边界

本变更不修改历史 migration，复用 `0022` 已保留的
`application_runtime_statuses` 与 `agent_tasks.runtime_status_id`；不需要
新增数据库 migration。生产发布应先升级控制面，再按节点升级到 v15 Agent；
v11-v14 Agent 的部署、PTY、Env 与 workspace 任务不被 runtime probe
capability 门禁阻塞。真实环境执行仍必须由当前对话单独授权。

本记录不授权执行上述生产动作。
