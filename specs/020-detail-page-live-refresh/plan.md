# 详情页自动刷新缺口实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

证据与根因见规格“背景与证据”。现状要点：
- `DeploymentDetailPage`：`refetchInterval` 对非终态返回 2000；全局 `QueryClient` 默认 `refetchOnWindowFocus: false`。
- `DeploymentFlowPanel`：事件查询 `refetchInterval: isTerminalDeployment(deployment.status) ? false : 2000`；`buildDeploymentFlow` 中事件优先于阶段任务状态。
- `streamSse`：`attempts` 在循环外，成功后不清零；`maxRetries` 默认 5（运行日志页传 8）。
- `DeploymentLogPanel`：流放弃后进入 `disconnected`，只有手动按钮增加 `generation` 重连。
- `NodeDetailPage`：节点、`["agents","node-links"]`、终端能力三个查询均无轮询；`useNodeStatusSocket` 只做失效通知。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md)：只改 `admin/` 与文档；不手改生成产物；不改全局默认值；日志与存储不含 Secret；提交只暂存本轮文件；部署需要用户当前对话的“部署正式环境”授权。

## 实现方案与文件范围

1. `admin/src/features/deployments/DeploymentFlowPanel.tsx`：用 ref 记录上一次是否终态；由非终态变为终态时调用一次 `events.refetch()`；事件查询加 `refetchOnWindowFocus`，终态时关闭。
2. `admin/src/features/deployments/DeploymentDetailPage.tsx`：详情查询加 `refetchOnWindowFocus`，终态时关闭。
3. `admin/src/api/sse-client.ts`：新增可注入的 `now`；每次连接记录建立时间与是否收到 `log` 事件；失败时若“收到过 `log` 事件或保持连接不少于 10 秒”，先把 `attempts` 清零再计算退避。
4. `admin/src/features/shared/useReconnectOnResume.ts`（新增）：`enabled` 为真时监听 `online` 与 `visibilitychange`（可见才触发），调用 `reconnect`；卸载或 `enabled` 变假时清理。`DeploymentLogPanel` 在 `connection === "disconnected"` 时启用，回调与手动按钮一致。
5. `admin/src/features/nodes/NodeDetailPage.tsx`：三个查询加 `refetchInterval: 5000`、`refetchIntervalInBackground: false`、`refetchOnWindowFocus: true`。
6. 文档：`docs/standards/client-behavior-contract.md`。
7. （追加）`admin/src/features/nodes/useNodeStatusSocket.ts`：重连状态机——失败计数、`reconnectDelay`（3 秒起倍增、上限 60 秒）、收到首条消息才清零、连续失败 10 次后停止；监听 `online` 与 `visibilitychange`，在没有连接时重置计数并立即重试；`NodesPage.tsx`、`NodeDetailPage.tsx`：只对管理员传令牌。

## 验证计划

- 单元与组件（Vitest + MSW，`admin/src/test/`）：
  1. 流程面板：部署由 running 变 succeeded 后事件接口再请求一次、流程图变为最终状态；首次加载终态部署只请求一次。
  2. 详情页与事件：窗口回到前台（`focusManager`）重新请求，终态不请求。
  3. `streamSse`：每次连接收到日志后断开，8 次仍继续并收到 `terminal`；立即断开、只收 `stream-error` 仍按上限停止；无事件但保持 ≥ 10 秒（注入 `now`）视为健康。
  4. `useReconnectOnResume`：启用时 `online` 与可见性变化触发一次；隐藏时不触发；未启用不触发；卸载后清理。
  5. 节点详情页：只伪造 `setInterval`，推进 5 秒后节点状态变化可见。
  6. （追加）`useNodeStatusSocket`：握手被拒时按 3、6、12、24、48、60 秒递增并在 10 次后停止；成功连接后断开从 3 秒重来；联网与回到前台立即重试、隐藏时不触发；已有连接不重复建立；放弃后回到前台再试；卸载清理；没有令牌不连接；`snapshot_invalidated` 使节点与 Agent 查询失效。节点页：非管理员不建立 WebSocket、管理员照常建立。
- 变异验证：去掉终态补刷、去掉聚焦刷新、把失败计数改回累计、去掉 `online`/可见性监听、去掉节点页轮询，对应用例必须失败；（追加）WebSocket 重连另做 12 项变异——去掉退避、去掉失败上限、成功连接后不清零、把握手被拒当成成功、不监听联网、不监听回到前台、`connect` 不检查已有连接、卸载时不清定时器或不摘监听、列表页或详情页不分身份连接、身份判断写反，各自对应的用例必须失败；还原后全部通过。
- `make admin-check`、`make docs-check`、`make whitespace-check`、`make spec-kit-verify`、`make check`。

## 风险与回退

- 风险与控制见规格。回退即回滚 Web 构建（`git revert`）；无后端或数据变化。
