# 节点遥测实施复核

## 结论

节点遥测计划 U1-U5 已完成本地实现与复核。当前控制协议 latest 为 v12、最低兼容 v11；v11 节点保留部署能力但不发送遥测，v12 节点连接旧控制面时可降级。此次复核未连接真实节点，未执行生产 migration、控制面部署、Agent 升级或业务应用发布。

## 契约核对

| 项目 | 实现结论 |
| --- | --- |
| 协议 | 初始 Hello 使用 v11 envelope 并声明 11-12；仅协商 v12 后发送 `node_telemetry`。 |
| 采样与失效 | 采样间隔 30 秒；服务端接收时间超过 90 秒标记 stale。 |
| 数据生命周期 | current 持久保存；history 保留 24 小时，查询最多 720 个两分钟聚合点。 |
| 工作盘 | Agent 使用 `data_dir/apps`，默认 `/var/lib/deploy-go-agent/apps`；任务目录不作为工作盘。 |
| GPU | 最多 8 张，只保存有限名称和聚合数值；缺硬件、后端不可用、权限、超时、解析和数据源失败使用稳定原因码。 |
| 状态 | capability 为 `supported` / `unsupported` / `unavailable`；freshness 为 `fresh` / `stale` / `empty`，与节点连接状态独立。 |
| 授权 | telemetry 只读接口复用节点可见性；无权节点按 404 隐藏，不进入 external API。 |
| 发布物 | manifest schema v3 包含 Agent/executor 的 x86_64 与 aarch64、三个 systemd unit，并声明协议范围 11-12。 |
| Migration | `0026` 新增 current/history，`0027` 新增原因约束；已提交 migration 未被修改。 |

## 风险复核

- Agent 采集和发送使用独立有界队列，超时或背压只丢样本，不阻塞 heartbeat、token rotation 或任务流。
- API 校验 Agent、连接代次、sequence、时钟偏差和 payload 上限；单连接及全局预算限制写入压力。
- current/history 在同一事务写入，重复和乱序不覆盖 current；历史达到节点或全局上限时仍允许 current 收敛。
- 24 小时清理分批执行，失败不阻塞控制流。生产执行 migration 前仍需一致性备份和单独授权。
- 管理端轮询仅在节点详情可见时运行；失败保留最后成功数据，节点列表不请求 telemetry。
- GPU 命令使用固定路径、参数、超时和输出预算；不保存 UUID、原始输出、IP/MAC、进程或完整挂载路径。

## 发布与回滚边界

后续生产发布应先升级控制面，再逐节点安装 v12 配对 Agent。控制面升级不能把 v11 Agent 变为不可调度；控制面回滚到 v11 时，v12 Agent 应降级到 heartbeat/部署模式并停止遥测。migration 已执行后不得假设旧 API 二进制可直接回滚，必须按 `docs/runbooks/api-migrations.md` 的备份和前进规则处理。

本记录不授权执行上述生产动作，也不把 telemetry 状态作为业务部署门禁。

## 验证记录

实施阶段已通过协议、Agent、API、migration、OpenAPI/client、管理端单测、构建、Playwright 响应式检查和容量隔离测试。U5 收尾继续以计划 Verification Contract 的完整命令集为权威；最终结果以本次会话的测试输出和提交记录为准。

## 独立复核修正

独立复核后补充修正：首次未连接 Agent 不再误报 `supported`；多盘 busy 取设备最大差分；retention 单轮分批追赶；落库增加全局并发和超时预算；趋势按时间缺口断线；Agent 拒绝协商版本不一致的后续 envelope。

仍保留以下非阻断风险，后续应按独立计划处理：不可取消的阻塞系统调用在超时后仍可能占用 blocking thread；非法超大 telemetry 在完成外层 JSON 解析前无法应用 16 KiB payload 限制；GPU OpenAPI 仍是弱类型 JSON；history 行数上限当前通过事务内 `COUNT(*)` 检查，接近上限时存在额外 SQLite 成本。以上风险不授权生产部署，发布前应结合真实容量和监控另行 Go/No-Go。
