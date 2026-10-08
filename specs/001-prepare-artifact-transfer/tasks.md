# prepare 发布物传输修复任务

唯一入口：本目录 spec.md、plan.md、research.md。状态：全部任务完成；业务方真实测试环境验收通过。

## 阶段一：证据与规格

- [x] T001 核对现场失败、上传进度及时间边界，记录到 specs/001-prepare-artifact-transfer/research.md；原始 artifact_transfer_failed、exit0、offset0，底层网络原因未确认。
- [x] T002 定义需求及本地/真实验收边界，完成 specs/001-prepare-artifact-transfer/spec.md 与 plan.md；对应 FR-001 至 FR-009。

## 阶段二：公共基础

- [x] T003 在 agent/tests/artifact_transfer.rs 建立真实 TCP 故障基线：初始化提交后响应丢失、首块断开、PUT 响应丢失、进度查询连续失联、响应截断、永久失败与 finalize 结果未知；记录修复前行为到 specs/001-prepare-artifact-transfer/research.md。对应 FR-004、FR-005。
- [x] T004 在 specs/001-prepare-artifact-transfer/plan.md 明确安全字段、请求/恢复预算及 finalize 可查询状态合同；先核对 api/src/artifacts/ 实际路由与 lease 消费行为，不假设已验证查询存在。记录每请求最多三次、恢复轮次上限、总 deadline 与退避上限，未确定前不得实施恢复。对应 FR-002、FR-004、FR-007。

## 阶段三：US1 准确诊断（P1）

独立验收：五类失败环节可区分，脚本 exit0 与交接失败并存，对外无敏感数据，旧记录仍可查询。

- [x] T005 [US1] 在 agent/tests/artifact_transfer.rs 与 api/tests/external_api.rs 增加失败环节、旧记录、未知错误码、脱敏和跨应用权限测试；使用假凭证与敏感标记验证结果不泄露。对应 FR-001、FR-002、FR-006、FR-007、SC-001、SC-004。
- [x] T006 [US1] 在 agent/src/artifact_transfer.rs 保留白名单错误类别、阶段、状态码、尝试次数、耗时与确认 offset；保留 PUT 与恢复 GET 失败因果，不输出原始 reqwest 错误/URL/响应正文。依赖 T003、T004、T005；对应 FR-002、FR-006。
- [x] T007 [US1] 在 agent/src/task_handler.rs 复用 summary/data.artifact_transfer 持久化安全诊断，保持 failed+exit0，不新增 TaskResult 顶层字段。依赖 T006；对应 FR-002、FR-003、FR-007。
- [x] T008 [US1] 在 api/src/external/mod.rs 保留稳定 artifact 错误码与安全摘要；公开详情只做可空白名单投影，禁止直通 result_json。依赖 T007；对应 FR-001、FR-006、FR-007。
- [x] T009 [US1] 按实际契约变更同步 api/openapi/、api/tests/external_openapi_contract.rs、deploy-go-deployer/src/main.rs、deploy-go-deployer/tests/ 与 skills/deploy-go-deployer/references/；验证旧 CLI 与无详情记录，未变接口记录无需调整的理由。依赖 T008；对应 FR-007、SC-004。

## 阶段四：US2 可靠恢复（P1）

独立验收：约 60 MB 合成发布物正常/断线恢复成功，摘要一致且仅一次 release；永久失败、取消与 deadline 有界结束。

- [x] T010 [US2] 在 agent/src/artifact_transfer.rs 修复 T003 证实的上传恢复缺口：进度查询暂时失联在预算内恢复，未知 offset 不盲传；校验倒退/超长/跳跃进度，区别永久拒绝与瞬时故障。依赖 T004、T006；对应 FR-004、FR-005、SC-003。
- [x] T011 [US2] 在 agent/tests/artifact_transfer.rs 验证 403、lease 过期、内容冲突、非法 offset、永久网络失败、取消和 deadline，以及 finalize 响应丢失的实际合同；不无限重试、不创建新任务。依赖 T010；对应 FR-004、SC-003。
- [x] T012 [US2] 在 agent/tests/artifact_transfer.rs 与 api/tests/artifacts_api.rs 用约 60 MB 合成归档进行真实 HTTP 正常/中断恢复、长度/摘要/finalize 验证，禁止使用业务发布物。依赖 T010、T011；对应 FR-005、SC-002。
- [x] T013 [US2] 在 agent/tests/two_stage.rs 或 agent/src/task_handler.rs 内部 fixture 验证上传失败不创建 release、恢复成功只交接/执行一次，以及控制连接断线重连后的同任务恢复和终态重放不重复构建/发布；内部 fixture 须执行 cargo test -p deploy-go-agent --lib，不适用现有 fixture 时在 research.md 记录原因。依赖 T007、T012；对应 FR-003、FR-004、SC-002、SC-003。

## 阶段五：US3 耗时边界（P2）

独立验收：创建处理与交接耗时可核对；没有调用方时间线时不归因 2 分 33 秒。

- [x] T014 [US3] 在 agent/src/artifact_transfer.rs、agent/src/task_handler.rs 记录有界阶段耗时，复用 API 既有请求 elapsed_ms；在 agent/tests/artifact_transfer.rs 验证单位与缺失字段，不引入逐块高频日志。依赖 T006、T007；对应 FR-008、SC-006。
- [x] T015 [US3] 在 specs/001-prepare-artifact-transfer/research.md 保留来源解析约 0.56 秒、创建请求 650ms、prepare 约 36.2 秒及交接失败约 5.2 秒的证据边界；调用方时间线缺失标为未确认，不追加无证据性能修复。依赖 T014；对应 FR-008、SC-006。

## 阶段六：复核与交付

- [x] T016 独立复核部署可靠性、权限隔离、兼容与错误传播，将发现及修正验证记入 docs/reviews/2026-10-08-prepare-artifact-transfer-implementation-review.md。依赖 T009、T013、T015；对应 SC-004。
- [x] T017 同步 docs/runbooks/deployment-recovery.md 与 docs/runbooks/external-deploy-api.md 的诊断、脚本退出码、交接结果、同任务恢复及回滚命令；不扩大节点操作权限。依赖 T016；对应 FR-009。
- [x] T018 按 plan.md 执行聚焦 cargo 测试、契约检查、fmt 与 diff 检查，记录真实结果到上述 implementation-review.md；运行 speckit-converge，只追加必要遗漏，完成本地小闭环提交推送。依赖 T017；对应 SC-001 至 SC-004、SC-006。
- [x] T019 根据会话具体授权完成控制面/Agent 发布及目标链路验收，把版本、时间、部署 ID、prepare/release/健康检查证据记录到上述 implementation-review.md；无授权不执行，未取得真实成功证据不关闭现场故障。依赖 T018；对应 FR-009、SC-005。

## 依赖与执行策略

### 本地实施证据（2026-10-08）

T003/T004：TCP 基线与实际 ArtifactStore 合同见 research.md、plan.md；GET 不支持 verified，finalize 仅按同 lease POST 幂等恢复。T005–T015：Agent 130、API 70、CLI 21 个聚焦测试通过，共 221 个；其中约 60 MiB 制品正常与提交 PUT 后响应丢失两种路径均经过真实 API ArtifactStore，校验长度和 SHA-256。控制连接重连、取消、deadline、终态重放及 failed+exit0 不创建 release 均有 fixture 验证。API 复用原 summary，OpenAPI 和 CLI 命令未变，历史无详情记录与旧协议 fixture 保持兼容；未运行历史 CLI/Agent 二进制，不将契约测试等同于旧二进制运行验证。

T016/T017：独立审查指出的永久拒绝正文停滞、deadline 丢失诊断、校验元数据失真及退避越界均已修正并回归。操作说明与 External/skill 诊断同步完成，详细结果见 docs/reviews/2026-10-08-prepare-artifact-transfer-implementation-review.md。T018 收敛无新增本地遗漏，修复提交 abe8f25 已推送 main；T019 不以本地测试代替真实业务链路验收。

T019 发布证据：2026-10-08 14:15:49（Asia/Shanghai）正式控制面运行 0.3.27，发布源码提交 2952966；三个在线正常节点在 14:16:08–14:16:39 串行自动升级成功，当前 heartbeat 均为 0.3.27/v18。正式 API healthz/readyz 和运行产物 SHA-256 验证通过。

T019 业务验收证据：用户转述同一业务项目通过 deploy-go-deployer Skill 发起并核验的测试部署 deployment_01M4D3CHZ0EE5WR2GEFMQ5VD5T，提交 a6c79b30，五个模块全部发布成功；API 蓝绿切换、API/Worker 健康检查、前端资源检查通过，agent_error 未复现。前置检查约 25 秒（Git 晋级约 2 秒）、构建约 42 秒、发布约 54 秒，总计约 2 分 4 秒。本轮记录业务方报告，未再次远程查询或重发部署；具体执行时间与原始测量时间线未提供，不从这次成功反推历史底层网络原因。

T001/T002 已有现场证据及文档，不代表代码修复。T003 → T004 为实现前门槛；US1 先交付安全诊断，US2 才验证恢复，US3 依赖共享计时描述。最终 T016 → T017 → T018 → T019。

读密集测试分析、契约核查和独立复核可并行委派；当前代码/测试文件有交集，未标记 [P]，主 agent 串行集成。若并行写入，必须隔离 worktree 并明确无交集文件范围。

最小可交付单元为 US1 的诊断闭环；它不满足实际故障关闭标准。US2 完成本地可靠性验证后才进入授权真实验收。所有未勾选任务须以实际证据更新，不用结构检查代替测试。
