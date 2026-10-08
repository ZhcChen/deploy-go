# prepare 发布物传输可靠性与失败诊断实施方案

规格：spec.md
状态：本地实施、验证、独立审查及收敛检查已完成，真实发布验收未完成

### 已核实的恢复合同（2026-10-08）

本地 TCP 基线证实初始化响应 body 丢失、分块失败后 GET body 丢失均会提前退出。init 同参数可幂等重试；PUT 已确认范围同字节重放可接受；GET 只查询 uploading，不能确认 verified。finalize 已提交后同 lease POST 可幂等返回 verified，并发消费窗口的 artifact_lease_consumed 可有限重试。

实现采用 init/GET/finalize 每轮最多 3 次（含 body 读取）；PUT 每轮 1 次，401 凭证刷新仍受 3 次上限约束。上传最多 3 次恢复查询轮次，不因单块成功无限重置总预算；GET 自身最多 3 次，临时失联可再查询，总计最多 9 次恢复 GET。退避单次最高 200ms，全部服从既有任务总 deadline 和取消。永久 403/404/422、内容/会话冲突不重试；409 仅白名单 offset_conflict/lease_consumed 按对应阶段恢复。finalize 读取 verified 与完整长度后才成功，不增加 API 路由或协议顶层字段。

对外复用 summary 显示白名单结构化诊断，不新增响应字段；data.artifact_transfer 只在受控内部结果存储，API 根据白名单重建摘要，旧数据缺失时使用固定提示。无法取得的请求次数为 null，不填入 0。

实现补充：上传请求内部共享 task 剩余 deadline，响应头已明确永久拒绝时不等待正文；外层 timeout 多留 100ms 仅作 watchdog，HTTP 请求/退避不会延长授权 deadline。成功响应保留本轮 HTTP 状态、尝试次数和耗时，校验失败复用这些事实。控制连接重连与终态重放用现有 two_stage fixture 扩展，不只测试私有函数。

验证文件范围补充：api/tests/two_stage_deployment.rs 验证真实 dispatcher 持久化 failed+exit0 且不创建 release；api/Cargo.toml/Cargo.lock 仅增加已有 workspace 包的测试依赖，复用 Agent 上传实现经 TCP 验证真实 ArtifactStore，无新增外部依赖版本。

发布范围补充：会话已有正式控制面自动发布授权。修复提交后以独立发布闭环将 API、Agent、executor、updater、deployer 五个 package 同步升级至 0.3.27，并更新 Cargo.lock。新版本确保 Agent 自动升级能取得本次上传修复，不覆盖既有 0.3.26 发布物；不自动重发业务部署。按正式 runbook 默认在 qfy-test2 构建和部署，不更改系统代理。

## Technical Context

Rust Agent 使用 reqwest 上传；现有初始化、1 MiB 分块、进度查询与 finalize 均通过 send_authenticated。上传后续失败保留脚本退出码 0；TaskResult 有可选 summary/data 且禁止额外顶层字段。API 的 External diagnostics 从 result_json 投影有限字段，未知错误码转为 agent_error。

已有 Axum/TCP 本地 fixture、内存 SQLite、临时 ArtifactStore 与 CLI 测试，不引入新依赖或远程测试平台。研究证据见 research.md；此次上传 0 offset 与旧 finalize SQLite 问题分开处理。

## Constitution Check

遵守 AGENTS、constitution、`docs/standards/api-contract.md`、`docs/standards/deploy-script-contract.md`、`docs/runbooks/deployment-recovery.md`、`docs/runbooks/external-deploy-api.md`、`docs/runbooks/local-development.md` 与正式部署 runbook。只修改 deploy-go，main 开发，不修改历史 migration，不关闭系统代理。

规划批准不新增运行态权限；此前针对该部署的只读查询授权按会话保留。完整实施已授权时自主推进，不机械逐阶段审批；真实发布、更新、重启与业务验证需核对具体操作授权。

## 实现方案与文件范围

### U1：锁定基线并复现

复用 `agent/tests/artifact_transfer.rs`，用本地 TCP 故障注入覆盖初始化已提交但响应丢失、首块连接断开、分块失败后恢复查询连续失联再恢复、分块已提交但响应丢失、永久故障、响应截断与 finalize 结果未知。先记录修改前的失败与 offset 时间线，再实施最小修复。

现场网络原因不是需求澄清阻塞，但不能用本地复现冒充历史同因；当时运行二进制版本也不能凭 agents.version 推断。没有证据不改代理、连接池、全局超时或 chunk 大小。

### U2：诊断与任务结果

在 `agent/src/artifact_transfer.rs` 保留有界错误描述：stage 为 upload_init/upload_chunk/upload_status/upload_finalize/access_prepare，category 为可确认的 connect/timeout/body/decode/http_rejected/unknown，附可用 HTTP status、attempts、elapsed_ms 和已确认 offset。DNS/TLS 不从 connect 类别推断。

保留上传原始失败与恢复查询失败的因果关系，不让最后一次 GET 覆盖 PUT 错误。统一安全格式化，不打印 reqwest 原始 Display/source、完整 URL、请求头、响应正文或内部路径。失败日志和成功交接耗时按阶段记录，不为每字节写日志。

`agent/src/task_handler.rs` 将白名单字段持久化到现有 result_data 的 artifact_transfer 对象，并从该对象生成受控 summary；其他任务保持原行为，不新增 TaskResult 顶层字段或业务事件、不伪造业务 step。

### U3：External 与 CLI 投影

`api/src/external/mod.rs` 放行稳定 artifact 错误码，未知错误保持通用化。summary 复用现有字段；若公开 artifact_transfer 详情，使用新增可空字段与严格白名单，缺失状态码/耗时为 null，不直通 data/result_json。

按实际响应变化更新 `api/tests/external_api.rs`、`api/tests/external_openapi_contract.rs`、`api/openapi/`、`deploy-go-deployer/src/main.rs`、`deploy-go-deployer/tests/` 和 `skills/deploy-go-deployer/references/`。旧 CLI 忽略新增可选字段，旧 Agent 无详情仍可查询；不扩大 skill 的节点日志/内部 API 权限。

### U4：有证据的恢复修复

保持同一 archive、lease 和任务。请求层最多三次尝试与上传层恢复计数不能叠成无界循环；transport/有限 5xx 与 401/403/404、过期 lease、冲突和非法进度区分处理。

针对 U1 证实的恢复缺口，在有限上传恢复预算内允许进度查询失联后再次查询；未确认服务端进度前不盲发未知范围。offset 不得倒退、超过归档总长或跳过已发送范围，保留大小与摘要校验。finalize 结果未知时核对同一会话状态或已验证状态后再幂等重试，不新建任务。

遵守 task_handler 的总 deadline 与取消；退避不得越过预算。prepare 已传输并校验成功后才创建 release，失败终态不因 exit0 复活。若证据指向其他网络实现，则先更新研究和文件范围，不以状态机补丁宣称现场全部解决。

### 文件范围

核心为 `agent/src/artifact_transfer.rs`、`agent/src/task_handler.rs`、`agent/tests/artifact_transfer.rs`、`agent/tests/two_stage.rs`、`api/src/external/mod.rs`、`api/tests/external_api.rs`、`api/tests/artifacts_api.rs`；生命周期 fixture 不适用时复用 task_handler 内部测试并记录理由。

默认不改 API 上传服务、http_client 全局策略、协议顶层、migration、Web/App 或版本号；新增范围需复现证据。本方案集中维护字段合同，不另建重复数据模型或接口文档。

## 阶段与依赖

U1 基线与字段合同 → U2/U3 诊断闭环 → U4 恢复修复 → U5 大归档与生命周期/安全回归 → U6 独立审查和文档交付 → U7 授权真实验收。

读任务可并行，代码由主 agent 串行集成，只有隔离 worktree 且文件无交集时才并行写。每个闭环可单独验证回滚，稳定 tasks 编号保留。诊断完成不代表 U7 完成。

## 验证与验收

聚焦命令如下，规划阶段未执行业务测试，不将其列为已通过：

```bash
make spec-kit-check FEATURE=specs/001-prepare-artifact-transfer STAGE=implement
cargo test -p deploy-go-agent --test artifact_transfer
cargo test -p deploy-go-agent --test two_stage
cargo test -p deploy-go-agent --lib
cargo test -p deploy-go-api --test artifacts_api
cargo test -p deploy-go-api --test external_api
cargo test -p deploy-go-api --test external_openapi_contract
cargo test -p deploy-go-deployer
cargo fmt --all --check
git diff --check
```

契约变化时执行 `make api-external-openapi`、`make api-external-openapi-check` 和 `make deployer-check`。既有 lint 告警单独报告。可用本机隔离 Docker，不连接共享基础设施。

- 同一故障修复前失败、修复后恢复；分块提交后响应丢失不重复有效交接、不跳过未知字节。
- 控制连接断线重连后的同任务恢复与终态重放不重复构建、交接或发布；若采用 task_handler 内部 fixture，必须执行上列 --lib 测试并记录场景结果。
- 合成约 60 MB fixture 经真实 HTTP 上传至 ArtifactStore，长度/摘要/finalize 验证通过；prepare 后只创建执行一次 release。目标脚本为本项目合成 fixture，不复制业务发布物。
- 403/过期 lease/内容冲突不被重试掩盖；永久 transport、取消、deadline 均有界收敛；failed+exit0 对外保留具体 code 与摘要。
- 假 token、URL/query、路径、响应正文敏感标记不出现在新增日志/结果/CLI；不同应用不可查询对方诊断。
- 旧 Agent/CLI、无详情历史数据、未知错误及混合版本仍可查询，现有协议/日志游标不回归。

本地通过只能证明被测场景。真实故障关闭需要授权链路通过；缺授权或证据时保留 U7，不重复创建部署或操作节点。

## 运行与恢复

同步 `docs/runbooks/deployment-recovery.md` 与 `docs/runbooks/external-deploy-api.md`，说明脚本退出码、交接结果、失败环节、时间线及同任务恢复。

发布遵循 `docs/runbooks/systemd-deployment-production.md`，保留 Agent 成对串行自动更新合同。真实验收部署核对具体应用、目标、版本和幂等键；不复活失败任务、不手工删状态。

代码用独立提交 revert 回滚；运行版本用正式部署 runbook 的备份恢复，不删除记录/制品状态/节点目录。新增字段可空，回滚后历史数据仍可读；默认无 migration。
