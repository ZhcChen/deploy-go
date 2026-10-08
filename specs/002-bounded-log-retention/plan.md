# 容量主导日志保留实施方案

状态：方案就绪；唯一需求入口 spec.md。旧存储/采集计划已完成，本轮不重新实现 durable outbox。

## 技术上下文与约束

Rust workspace、SQLite、既有 tracing/SSE 和 Agent access 鉴权。新增轻量共用 runtime-log crate，仅提供受管 JSONL 分段、有限查询和脱敏层，不引入外部日志框架。复用已有依赖。节点日志不是业务 stdout，不增加读取 journal 的权限。以 AGENTS.md、constitution、日志/部署协议规范及相关运行手册为准。

## 实现设计

1. 控制面：RuntimeSettings 增加带 serde 默认值的 max_total_log_bytes（2 GiB），旧请求/配置兼容。新增更高版本 migration 维护 deployment_logs 的事务性字节计数，避免每条输出扫描历史正文。每分钟容量回收只作用于已结束部署日志，先失败前更老的成功日志仍按结束时间排序；不删任务结果和关键诊断。原 log_retention_days 为辅助，保留原值，不批量改生产配置。无可回收输出时记录安全诊断，容量是稳态目标，活动部署可产生受单部署预算约束的临时超额。
2. 运行日志：shared runtime-log 使用命名 JSONL 分段、单条 16 KiB 上限、有界扫描/内存；控制面 32 MiB×16 段；节点 agent/runner/executor/updater 各 16 MiB×2 段，合计 128 MiB。目录/文件拒绝符号链接，固定组件目录，不对活动文件 copytruncate。当前段损坏尾部只恢复自身日志，不触碰任务。磁盘失败计数，采集不阻塞任务；原 stdout 运维路径保留。
3. 集中：Agent 读取受管四组件目录，按组件持续序号批量 HTTP 上送，access 身份由控制面绑定节点。API 先写持久分段，再确认序号；响应丢失重复上传去重，恢复从保留分段重建来源水位。本地被覆盖部分显式报告 gap。独立于部署日志，不增加 WSS 协议版本或读取其他节点日志的权限。旧 API 404 时退避，旧 Agent 无新上传仍工作。
4. 节点权限：Broker 当前 root 服务，Executor/updater root；独立日志树 /var/log/deploy-go-agent 为 root:deploy-go-agent 0750，各 root 组件目录0750/文件0640，Agent自身0700/0600。Agent只读其他三个目录，不增加runner业务身份访问、附加组、journal或业务目录权限。安装器与自动 updater 均准备固定目录，Agent/updater unit只添加各自专属 ReadWritePaths；回滚保留日志，不删除新格式任务。四组件日志采集只输出内部事件位置和标识/数值白名单，原始 error/message/url 不上送。首版只采集 root Broker 调度诊断，不给业务 runner 增日志目录或专用FD；业务输出继续原通道。
5. 诊断：阶段交接成功/失败及恢复计数、日志 ACK/容量缺口、清理统计。使用现有事件和固定安全字段，不逐块或每 heartbeat 刷屏。控制面 RuntimeLogStore 保留条数限制并增加字节和队列限额，持久数据可按原 SSE 游标分页回查；查询增加 node_id/component，原字段兼容。设置 UI 增总容量字段、保留周期标注辅助；更新主 OpenAPI/client，外部部署接口不变。

### 复核后的恢复与回收合同

- 首轮自动升级仍由旧 updater 执行，旧 unit 对新日志树只读；Runner Broker unit 通过 ExecStartPre 同步执行 prepare-runtime-logs，在主进程启动前准备日志树，Broker/Executor 启动再检查，不能只依赖新版 updater 的 install_files。Agent/updater 的日志 ReadWritePaths 使用可选路径，目录缺失不阻止服务启动。后续 updater 对权限已正确的目录只校验、不重复 chown/chmod；日志目录准备失败不得回滚业务升级，采集游标暂不可读需退避重试。

- 去重不能只从可轮转日志推断。节点组件保存独立 epoch/序号元数据，重新初始化日志树生成新 epoch；API在新增 migration 中按 Agent/组件保存独立持久水位、epoch和末记录摘要，最多每个Agent四行，不随日志回收。新epoch须为更新的合法ULID，旧epoch重传拒绝，非法/倒退序号不接受。写入和去重更新的并发由同一持久存储锁串行；先日志持久化再更新水位确认，崩溃窗口通过保留记录恢复未提交水位。测试源记录全部轮转后重启/重传、epoch状态丢失以及写后水位未提交。
- API端安全回收判据：部署终态、无活动任务；每个关联任务须有终态结果，结果sequence等于last_sequence且1..last_sequence的receipt全部committed。这证明API完整接收，不证明ACK网络已到节点；receipt/source_digest和终态结果始终保留，节点仍按实际ACK决定本地回收。旧无receipt记录无法证明完整时保留并计入超预算，不按当前Agent版本推断旧任务合同。纯历史无任务部署输出可按既有终态处理。
- 容量和周期清理共用上述判据；容量清理同时回收 deployment_logs 与 agent_task_events(kind=output) 的正文副本，统计两份受管输出字节。保留 progress/result/diagnostic、部署状态与失败摘要、receipt及任务元数据。周期只清理安全终态的非诊断展示事件，diagnostic保留。SQLite页面/WAL不作为有效载荷预算；不在线执行全库VACUUM。

## 文件范围

runtime-log/、workspace manifests/Cargo.lock、发布 Dockerfile manifests；api/src/runtime_logs.rs、settings、deployments/runtime.rs、agents/log ingestion、migration 与测试；agent/src/runtime log sender、main/task_handler/log_delivery；executor/updater 初始化与安装器组件目录。admin 系统设置/运行日志筛选及 generated clients；docs/runbooks 的日志容量、权限、排障和回滚说明。

## 验证与回滚

容量 fixture 与重启/断尾/目录安全；API settings/dispatcher/deployment recovery/runtime logs；Agent HTTP 上传去重/缺口/权限隔离；本机 Docker Linux 身份回归；cargo fmt/clippy、OpenAPI/client、admin tests/check 与安装发布契约检查。独立安全/可靠性复核；converge 无遗漏后小闭环提交推送。migration 先安装并验证 Git hooks，绝不修改已有 migration。

按正式发布 runbook 发布新的完整成对版本 0.3.28，保留原 0.3.27。默认先验证远程构建，网络仍受阻时使用已验证本机构建兼容模式，不修改系统代理。控制面健康与自动升级可只读核对，不触发业务部署。回滚旧产物时新日志文件保留，新 migration 向前兼容；不还原/删除部署历史。
