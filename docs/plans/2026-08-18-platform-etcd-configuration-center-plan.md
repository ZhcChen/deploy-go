---
title: 平台单节点 etcd 配置中心实施计划
date: 2026-08-18
type: feature
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# 平台单节点 etcd 配置中心实施计划

## Goal Capsule

- **目标：** 让管理员通过 Deploy Go 部署并绑定一个单节点 etcd，在平台内按“应用 + 环境”维护配置，并让业务应用在 release 时获得最小权限的 etcd 连接信息后直接读取配置。
- **手段：** 控制面直接管理 etcd KV、用户与 Role；Agent 只提供通用、用途绑定的 Secret environment lease 注入能力，不内置 etcd 客户端或模板专用逻辑（KTD4、KTD7）。
- **权威边界：** 当前用户指令、`AGENTS.md`、`docs/standards/api-contract.md`、`docs/standards/deploy-script-contract.md`、`docs/standards/agent-control-protocol.md` 与相关 runbook 优先于本计划。
- **停止条件：** 本计划只授权本地规划和后续代码实现，不授权连接真实节点、执行共享环境 migration、部署 etcd、升级 Agent、重启控制面或修改业务应用运行态。
- **执行轮廓：** 先建立数据与 Secret 契约，再升级模板初始化和平台绑定，随后实现业务 RBAC/KV、部署注入与管理端，最后补生命周期保护、切换流程及运行文档。每个单元独立验证、提交和推送。

## Product Contract

### Summary

Deploy Go 管理唯一一个平台单节点 etcd，并提供轻量 KV 管理能力。管理员仍通过现有模板和部署链路部署 etcd；首次部署的管理员密码由控制面生成、加密持久化并只展示一次。普通应用可选择平台 etcd、自有 etcd 或继续只使用 Env。使用平台 etcd 时，控制面为每个“应用 + 环境”创建只读账号和独占前缀，部署脚本只接收连接所需环境变量，业务进程直接读取 etcd。

### Problem Frame

现有应用配置主要依赖逐目标同步的 Env 文件。节点增加后，配置维护和一致性成本随目标数增长。仓库虽已有单节点 etcd 模板，但它只绑定回环地址、不启用 Auth/RBAC，并明确不适合生产配置中心；控制面也没有配置中心模型、KV API、业务账号生命周期或部署注入协议。因此不能只修改 Compose 文件，必须补齐从安全初始化、平台绑定、应用隔离到 release 快照的完整链路。

### Requirements

- **R1. 单一平台实例：** 一个 Deploy Go 控制面同一时刻只能绑定一个生效的平台 etcd。首期只支持单节点 etcd，不建设成员管理、Raft 扩缩容或自动故障转移；测试、预发布和正式应用可以共用该实例。
- **R2. 既有部署链路：** etcd 必须作为现有 `etcd 3.6` 应用模板，经应用、目标、preview/confirm 和 Agent release 链路部署，不新增绕过部署审计的安装入口。
- **R3. 安全首次初始化：** 创建平台管理的 etcd 应用时，由控制面生成高强度管理员密码并加密保存，通过不进入命令行、持久文件、stdout、stderr、部署事件或审计摘要的 Secret 通道交给初始化脚本。初始化必须幂等；重试和升级沿用既有密码、数据卷与 Auth 状态。
- **R4. 一次性展示：** 部署成功后，管理员可在受保护页面查看一次平台生成的 etcd 管理员用户名和密码。明文读取必须原子消费，刷新、第二个标签页或再次请求不得重新显示；控制面保留加密凭据供后续管理，但普通读取 API 永不回显密码。凭据遗失时，管理员通过近期重新认证的轮换流程让控制面使用内部旧凭据设置新随机密码、更新密文版本并产生新的一次性 reveal，不得重新展示旧密码或重建数据。
- **R5. 平台绑定与检测：** 管理员可从部署成功页面“立即设为平台 etcd”，或在系统设置中填写 Endpoint、用户名及只写密码。保存和绑定前必须检测连接、凭据、etcd API 兼容性、Auth 已启用，以及账号管理用户、Role 和配置前缀的权限；设置页支持修改、清除和主动检测。平台与自有 Endpoint 都必须受服务端配置的 etcd 出站 CIDR allowlist 约束，并防止 DNS 重绑定、云 metadata 和非 etcd 重定向。控制面检测只证明管理路径；每个依赖目标还必须在 release 前完成独立、受限的 Endpoint 网络可达性探测，结果绑定 target run。
- **R6. 应用模式：** 每个应用可选择 `none`、`platform_etcd` 或 `custom_etcd`。`none` 保持既有 Env 行为；`platform_etcd` 使用全局实例；`custom_etcd` 保存独立 Endpoint、用户名、只写密码和前缀，仅负责检测与部署注入，不纳入平台实例、RBAC、KV 管理、迁移或生命周期保护。
- **R7. 项目与环境隔离：** 平台 etcd 使用 `/deploy-go/apps/{application_id}/{environment}/` 作为规范前缀。每个“应用 + 环境”拥有独立用户名和 Role，业务账号默认只能读取该前缀，不得读取其他应用/环境或写入配置。应用环境变化必须以迁移或重新绑定处理，不能静默扩大旧 Role。
- **R8. KV 管理：** Deploy Go 管理端直接提供平台 etcd 前缀内 KV 的浏览、新增、编辑、删除、搜索、前缀筛选、批量导入/导出和差异确认。Value 允许明文保存业务 Secret；列表和详情按管理员权限展示，不建设 Value 加密、遮罩或 reveal grant 体系。Deploy Go 不承诺业务进程热更新：KV 写入后记录配置 revision 变化，业务应用自行 watch/刷新；不具备动态刷新能力的应用必须重新部署，UI 明确显示这一责任。
- **R9. 并发与审计：** KV 写入必须基于 etcd revision 做 compare-and-swap，冲突稳定返回 `409` 并展示差异。API 返回 create revision、mod revision 和 version；Deploy Go 发起的变更另存操作人、时间和请求 ID，外部直接改写的 Key 明确显示操作人/修改时间未知。审计只记录 Key、revision、动作，以及使用审计专用派生密钥计算的 Value HMAC，不记录可离线猜解的普通 hash、Value 正文或连接密码。reveal 消费、凭据轮换、平台绑定/清除、切换确认和故障解绑也必须记录操作者、时间、request ID 和脱敏结果。
- **R10. 批量安全：** 导入支持预览、差异确认和显式覆盖策略；实际写入使用受限 etcd transaction 分批执行，任何一批比较失败均停止并返回未应用差异。导出只包含当前应用环境前缀下的 Key/Value 和必要元数据，不包含 etcd 管理凭据或业务账号密码。
- **R11. 部署快照与注入：** 配置中心选择、类型、Endpoint、规范前缀和凭据版本必须固化进部署 snapshot。release 前通过用途绑定、短期、单任务、可重试但不可跨任务复用的 Secret environment lease 注入 `DEPLOY_CONFIG_CENTER_TYPE`、`DEPLOY_CONFIG_CENTER_ENDPOINTS`、`DEPLOY_CONFIG_CENTER_PREFIX`、`DEPLOY_CONFIG_CENTER_USERNAME`、`DEPLOY_CONFIG_CENTER_PASSWORD`。调用方未显式传 type 时按 `etcd` 解析，但 snapshot 仍保存 `etcd`。
- **R12. 失败隔离：** 配置了配置中心的应用在 preview/confirm、派发和 release 前必须检查绑定、凭据与能力；缺失或不可用时阻止新部署并返回稳定错误。未启用配置中心的应用、既有 Env 同步、普通部署和节点在线状态不得受影响。
- **R13. 生命周期保护：** 当前平台 etcd 对应应用、启用目标、管理 Env 和数据卷受到强保护：禁止归档应用、停用或删除目标、删除初始化所需 Env，以及发起清卷部署；允许不清数据的普通重试、重新部署与升级，并返回明确冲突原因。受保护 Env 由成功部署的 etcd template manifest 登记为稳定文件 ID/名称集合并保存在平台绑定中，不能通过普通文件重命名或目标引用变化绕过；解除绑定后才解除该集合的保护。具有 active `platform_etcd` 绑定的业务应用也必须先完成解绑和远端身份撤销，才能归档；远端不可用时保持 `cleanup_pending` 并禁止归档。
- **R14. 切换平台实例：** 正常切换必须先部署并检测新实例，复制配置，比较 Key 数量和确定性摘要，重建每个业务账号/Role，等待管理员确认后原子切换全局绑定。旧实例解除保护但不自动删除；已运行的依赖应用标记为“待重新部署”，重新部署后才获得新 Endpoint/凭据。
- **R15. 故障解绑：** 原实例不可用时，管理员可输入 etcd 应用名称二次确认并只解除绑定。操作不得删除数据、容器、应用或目标；依赖应用显示配置中心不可用并阻止新部署，未启用配置中心的应用保持正常。
- **R16. 安全边界：** 首期允许受内网和 IP 白名单保护的 HTTP Endpoint，TLS 延后；所有 etcd 密码仍必须使用控制面主密钥加密、零化临时明文、禁止日志输出并支持主密钥轮换。UI 和 API 可返回不含认证信息的规范化 Endpoint，但不返回加密字段、nonce、密钥版本或任何包含凭据的连接串。
- **R17. 兼容演进：** Env 能力继续保留，可与配置中心同时使用，但二者属于不同命名空间，Deploy Go 不合并 Key、推断重复项或定义业务代码读取优先级；应用必须在自身部署契约中声明哪个来源权威，preview/UI 对未声明的共存给出冲突提示。新增 Secret environment lease 必须作为通用 Agent 能力设计；旧 Agent 仍可部署未启用配置中心的应用，启用配置中心的目标必须在保存和部署前得到明确的升级提示。

### Actors

- **平台管理员：** 部署/绑定平台 etcd，管理连接、KV、应用配置中心选择、切换与故障解绑。
- **业务应用：** 使用按应用环境隔离的只读账号，在运行时直接读取规范前缀。
- **控制面 API：** 加密保存凭据，调用 etcd 管理 API，生成 RBAC，固化部署快照并执行生命周期保护。
- **节点 Agent 与 executor：** 获取任务绑定的 Secret environment lease，并只向目标初始化或 release 进程注入允许的环境变量。

### Key Flows

1. 管理员从 etcd 模板创建应用和目标。控制面生成并保存加密管理员密码；部署 snapshot 引用初始化凭据版本。Agent 在首次 release 中获取 Secret environment lease，脚本启动 etcd、等待健康、幂等创建 root/启用 Auth；重试识别既有数据并验证而非重置。
2. etcd 部署成功后，管理员消费一次性凭据展示并点击“立即设为平台 etcd”。控制面用内部凭据检测权限，通过后建立唯一平台绑定和生命周期保护。
3. 管理员为业务应用选择平台 etcd。控制面创建规范前缀、独立用户和只读 Role，将业务密码加密保存；应用配置页开始提供该前缀的 KV 管理。
4. 管理员编辑或导入 KV。控制面携带读取到的 revision 执行 compare-and-swap，并把不含 Value 的操作事实写入审计和 KV mutation metadata。
5. 应用部署 preview 固化配置中心描述与凭据版本；release 任务通过 Secret environment lease 得到五个固定变量，executor 清理环境后注入它们，脚本和日志均不能打印密码。
6. 管理员部署新 etcd 并发起切换。控制面复制和核验 KV、重建账号后等待确认，原子改变全局绑定并标记依赖应用待重新部署。
7. 管理员修改已绑定应用的 environment 或 mode 时，先预览旧/新前缀、身份和运行影响；平台复制并核验 Key、重建身份，确认后切换绑定并标记待重新部署。取消或失败保留原绑定，解绑不会自动删除旧前缀数据。

### Key Decisions

- **KD1. 全局共用单节点 etcd：** 接受跨环境共享故障域和单点风险，以降低当前运维复杂度；不扩展为集群。Governs R1、R14。（session-settled: user-directed — chosen over per-environment or three-node clusters）
- **KD2. Deploy Go 内建轻量 KV UI：** 避免引入独立 etcd UI，平台只提供本需求所列的受限管理面。Governs R8-R10。（session-settled: user-directed — chosen over a separate UI project）
- **KD3. Value 明文存储：** 接受 etcd 管理员可读业务 Secret，以认证、RBAC、内网和 IP 白名单控制风险，不建设 Value 加密层。Governs R8、R16。（session-settled: user-directed — chosen over application-layer value encryption）
- **KD4. 业务运行时直读：** Deploy Go 只注入连接参数，不将 etcd 配置拉取并落成本地 Env。Governs R7、R11。（session-settled: user-directed — chosen over deployment-time materialization）
- **KD5. 平台与自有 etcd 并存：** 平台实例承担完整管理，自有实例只做连接检测和注入。Governs R5-R8。（session-settled: user-directed — chosen over forcing all applications onto the platform instance）
- **KD6. TLS 延后：** 首期允许受控内网 HTTP，后续独立增加 TLS；计划不能把此边界描述为等同于 TLS 安全性。Governs R16。（session-settled: user-directed — chosen over first-release TLS provisioning）

### Acceptance Examples

- **AE1.** 全新 etcd 模板部署成功后 Auth 已启用，管理员密码未出现在部署日志；第一次查看可见用户名和密码，刷新后密码不再返回。
- **AE2.** 同一部署失败后重试不会生成新密码、重建数据卷或关闭 Auth；已初始化实例升级后原有 KV 和账号仍存在。
- **AE3.** 平台绑定检测能区分 Endpoint 不通、认证失败、Auth 未启用、版本不兼容和权限不足，并且失败时不改变当前平台绑定。
- **AE4.** 应用 A/prod 与应用 B/prod 的业务账号只能读取各自前缀，均不能写入；A/test 与 A/prod 也不能互读。
- **AE5.** 两名管理员基于同一 revision 编辑同一 Key，只有首个 compare-and-swap 成功，后一个收到当前 revision 和可确认差异，不会静默覆盖。
- **AE6.** 配置了平台 etcd 的部署 snapshot 固化 type、Endpoint、prefix 和凭据版本；release 环境包含五个约定变量，任务 payload、日志和数据库 snapshot 不含密码明文。
- **AE7.** 当前仍受支持的 v11/v12 Agent 继续部署 `none` 模式应用；依赖配置中心的目标被阻止并提示升级到支持 Secret environment lease 的 v13 Agent。
- **AE8.** 正常切换在复制摘要不一致时不能确认；成功切换后旧实例未删除，依赖应用显示待重新部署。
- **AE9.** 故障解绑只解除全局绑定并阻止依赖应用新部署，不改变未启用配置中心的应用，也不删除 etcd 资源。
- **AE10.** 控制面检测可用但某业务目标无法建立到 Endpoint 的 TCP 连接时，该 target run 在脚本执行前以稳定原因失败；其他目标保留各自状态。
- **AE11.** Env 与 etcd 同时启用但应用未声明权威来源时，preview 明确提示冲突；KV 更新只声明 revision 已变化，不误导为业务已热更新。
- **AE12.** 隔离黑盒 consumer 只使用五个 `DEPLOY_CONFIG_CENTER_*` 变量即可用业务只读账号读取自身 prefix；etcd 不可达时按应用部署契约明确失败，不静默退回 Env。

### Success Criteria

- 隔离测试中的真实单节点 etcd 3.6 可完成 loopback 首次初始化、平台绑定、RBAC 隔离、KV CRUD、CAS 冲突、批量导入、应用 environment/mode 迁移和平台切换复制。
- 除管理员主动访问且带 `no-store` 的 KV Value 管理响应外，任一 API、Agent、executor、部署事件、审计或浏览器普通读取响应中都找不到 etcd 管理员密码、业务账号密码或用户保存的业务 Secret 明文。
- 未启用配置中心的既有应用和 v11/v12 Agent 行为保持兼容；Env 同步与 release gate 回归测试通过。
- migration、OpenAPI、生成 Web/Flutter client、Admin 构建、Rust fmt/clippy/test 和模板校验全部通过。
- 一个不依赖 Deploy Go 内部模块的黑盒测试 consumer 使用注入变量完成真实 etcd 读取，证明契约到业务消费端闭环；该 fixture 不代表改造任何正式业务应用。

### Scope Boundaries

**In scope**

- 单节点 etcd 3.6 模板的 Auth/RBAC 幂等初始化、一次性凭据展示与平台绑定。
- 平台/自有/不使用三种应用模式，平台前缀/RBAC、KV UI、审计、部署注入和生命周期保护。
- 平台实例正常切换、复制核验和不可用时故障解绑。

**Deferred to follow-up work**

- TLS/mTLS 自动签发和轮换、etcd 集群成员管理、高可用、自动故障转移、备份调度与恢复 UI。
- Watch 推送、动态配置 SDK、配置 Schema、灰度配置、审批流、历史 Value 版本恢复和长周期审计报表。
- 自动触发依赖应用滚动重启；首期记录 revision 变化并由应用自行刷新，不能动态刷新的应用由管理员重新部署。

**Outside this feature**

- 分布式锁 SDK 或业务锁语义、Redis 迁移、业务应用代码改造、正式环境 etcd 部署或任何真实节点操作。
- 对自有 etcd 执行用户/Role/KV 管理、数据复制或生命周期保护。

### Dependencies And Assumptions

- 使用兼容 etcd v3.6 gRPC API 的 Rust client；实现阶段锁定依赖版本并验证连接超时、keepalive、Auth token 刷新及 TLS-disabled 行为。
- Endpoint 由管理员明确填写或由模板部署结果生成，必须是规范化的 `http://host:port` 列表；控制面不能从容器日志猜测地址。
- etcd 出站 CIDR allowlist 未配置或为空时 fail closed；本地测试只有显式配置后才能允许 loopback。
- 单节点和跨环境共用的可用性风险已经接受，但 runbook 必须提供数据卷、快照备份和故障解绑边界。
- Product Contract preservation：本计划由当前会话已确认决策直接 bootstrap，无独立 brainstorm artifact；已确认范围未改变。

### Sources

- `examples/templates/etcd/`、`container-template/src/lib.rs`、`admin/src/features/targets/imageTemplates.ts`：现有 etcd 模板和类型校验。
- `api/src/application_envs/mod.rs`、`api/src/crypto/mod.rs`：加密 Secret、一次性 lease 与逐目标 Env 的既有模式。
- `api/src/deployments/mod.rs`、`api/src/agents/dispatcher.rs`、`agent-protocol/src/lib.rs`、`agent/src/secret_lease.rs`、`agent-executor/src/release.rs`：snapshot、release 派发、Secret lease 和清洁进程环境。
- `api/src/applications/mod.rs`、`api/src/deployment_targets/mod.rs`：应用归档、目标停用和应用环境约束。
- `admin/src/features/settings/SettingsPage.tsx`、`admin/src/features/applications/ApplicationDetailPage.tsx`、`admin/src/features/application-envs/`：设置页、应用详情和配置编辑 UI 模式。
- `docs/standards/api-contract.md`、`docs/standards/deploy-script-contract.md`、`docs/runbooks/api-migrations.md`：API、敏感参数和 migration 约束。

## Planning Contract

### Key Technical Decisions

- **KTD1. 独立领域表而非扩展通用 settings JSON：** 新增平台连接、应用绑定、加密凭据、一次性 reveal、业务身份、KV mutation metadata 和切换作业表。唯一生效平台绑定使用数据库唯一约束和事务保证；密码使用独立 AAD context 加密，便于主密钥轮换和防止跨资源替换。满足 R1、R4-R7、R9、R14。
- **KTD2. 控制面直接使用 etcd v3 API：** API 进程维护有界 client 缓存和明确 connect/request timeout，执行 health/status/auth/user/role/kv/txn 操作。检测不写业务 Key，只用受控临时 Role/User 或维护前缀验证管理权限并立即清理；不得仅以 health 成功判定可用。满足 R5、R7-R10。
- **KTD3. 模板初始化采用 loopback bootstrap 和幂等状态探测：** 空数据目录首次只在 loopback listener 启动未认证 etcd，创建 root、启用并验证 Auth 后才重启/切换到经校验的内网 bind/advertise URL；受控主机防火墙在切换完成前不得放行内网端口。后续运行先以持久凭据验证已有 Auth。出现“数据存在但凭据不匹配”“Auth 状态不完整”时 fail closed，不执行重置或数据清理。满足 R2-R4。
- **KTD4. 扩展通用 Secret lease，不把 etcd 写进 Agent：** 在现有 WSS Secret lease 机制上新增 `environment-v1` payload，变量类别分别固定为业务连接和模板初始化，并绑定 task/deployment/credential version、template ID/version/digest、release stage、executor audience、目标进程和 payload digest。Agent 在内存中获取并通过升级后的本地 executor IPC `ReleaseStartRequest` 转交；executor 验证 release authorization claims 后只向该子进程注入，完成后零化。普通业务 release 永远不能请求模板管理员变量。etcd 管理逻辑仅存在于控制面和模板脚本。满足 R3、R11、R17。
- **KTD5. 协议能力升级为 v13：** v13 增加可选 `secret_environment` release descriptor 和 `secret_environment_v1` capability，最低兼容版本保持当前 v11。控制面只为依赖配置中心或 etcd 初始化的任务要求 v13；其他任务继续按既有能力派发给 v11/v12。控制面先发布，随后按需升级 Agent。满足 R12、R17。
- **KTD6. 一次性展示使用消费记录而非删除持久凭据：** 管理凭据始终只以密文保存；独立 reveal 状态允许部署成功后一次原子消费，响应携带 `Cache-Control: no-store`，要求管理员会话、CSRF 和近期重新认证。消费失败不改变凭据，响应成功即不可重放；遗失后只能走使用内部旧凭据完成远端更新、密文替换和新 reveal 的轮换 API。满足 R4、R16。
- **KTD7. 应用配置中心是版本化资源并进入 snapshot：** 应用绑定保存 mode、endpoint、prefix、credential/identity version、检测状态和版本。preview 解析默认 type 并固化实际 `etcd`；confirm 比较 snapshot hash，dispatcher 不回读“最新密码”改变已确认部署。凭据轮换后旧 snapshot 不可新派发，要求重新 preview。满足 R6、R11、R12。
- **KTD8. 平台业务身份由控制面派生稳定名称并随机生成密码：** 用户名/Role 名由不可逆短 ID 构成，不依赖可变 slug；密码加密保存。创建顺序为 Role、权限、User、授权，并在失败时可幂等重试。解绑应用先阻止部署，再撤销远端用户/Role；远端不可用时保留待清理状态。满足 R7、R12、R15。
- **KTD9. KV API 以 revision 为并发边界：** 单项 PUT/DELETE 要求 `expected_mod_revision`；新建要求 version 为 0。批量导入先读取固定 header revision 生成 preview token，再按有界批次 compare-and-swap。etcd 原生 revision 是数据真相；SQLite metadata 只补充 Deploy Go 操作者/时间，不能覆盖外部修改事实。满足 R8-R10。
- **KTD10. 切换是可恢复作业而非长 HTTP 事务：** 控制面持久化 source/destination、阶段、游标、Key 数量/摘要和错误；复制使用目标前缀、分页读取和有界 transaction，确认前重新核验 source revision。最终绑定切换与依赖应用 `pending_redeploy` 标记在 SQLite 单事务完成。满足 R14。
- **KTD11. 生命周期保护落在服务端写入口：** 应用归档、目标状态、Env 删除和清卷参数的 API 必须查询当前平台绑定并返回稳定 `platform_etcd_protected`；UI 只解释原因，不能作为唯一防线。存在平台身份/依赖时禁止通过普通设置直接 rebind，必须走切换作业。故障解绑使用单独高风险 endpoint，要求管理员会话、CSRF、近期重新认证和应用名称确认。满足 R13、R15。
- **KTD12. etcd Endpoint 使用专用出站策略：** 启动配置提供明确的 etcd CIDR allowlist；每次连接均解析全部地址并验证 scheme、端口和目标 IP，拒绝 public、link-local、metadata 或 allowlist 外地址，连接层禁止 HTTP redirect，并在 DNS 变化后重新校验。测试环境可显式允许 loopback，生产默认不允许。满足 R5、R16。
- **KTD13. 目标可达性使用通用有界网络 probe：** v13 Agent 增加不携带凭据、不执行任意请求的 endpoint TCP-connect probe descriptor，只允许 snapshot 中已校验的 host/port、短超时和有限地址数。控制面管理权限检测与目标网络 probe 分开记状态；只有配置中心 target 要求该 capability。真实鉴权/前缀读取仍由控制面验证，业务启动后的读取语义由应用负责。满足 R5、R11、R17。

### Canonical State And API Contract

- 平台连接状态：`unconfigured`、`unchecked`、`available`、`unavailable`；检测错误使用 `endpoint_unreachable`、`authentication_failed`、`auth_disabled`、`incompatible_version`、`management_permission_missing`、`timeout`。
- 应用 mode：`none`、`platform_etcd`、`custom_etcd`；配置状态：`ready`、`unavailable`、`pending_redeploy`、`provisioning`、`cleanup_pending`。
- Endpoint 保存规范化、有序、去重后的 URL 数组；首期只接受 `http`，拒绝 userinfo、query、fragment 和空 host。响应可回显 Endpoint/用户名/prefix，不回显密码或密文字段。
- `DEPLOY_CONFIG_CENTER_ENDPOINTS` 使用规范化 Endpoint 的 JSON string array；其余四个注入变量为单个 UTF-8 string，完整编码契约写入部署脚本标准。
- 平台前缀由服务端从 application ID 和 application environment 计算，平台模式不可由客户端覆盖。自有模式要求显式 prefix，并按绝对 etcd key prefix 校验。
- KV Key API 使用相对于规范前缀的 UTF-8 key；拒绝空 key、路径逃逸、NUL 和超限 Key/Value。响应包含相对 key、Value、create/mod revision、version，以及可空的 Deploy Go actor/time。
- 一次性 reveal 只用于平台模板生成的管理员凭据，不用于业务只读账号；业务账号密码只通过 deployment Secret lease 使用。
- `etcd-init` purpose 只允许 `ETCD_INIT_ROOT_USERNAME`、`ETCD_INIT_ROOT_PASSWORD`；业务连接 purpose 只允许五个 `DEPLOY_CONFIG_CENTER_*` 变量，两个集合不可混用。
- KV 批量格式使用版本化 UTF-8 JSON（键为相对 key、值为 string）作为规范导入/导出格式；另允许将现有 `dotenv-v1` 解析为同级 Key 的迁移输入，但不会自动删除或改写原 Env 文件。
- 管理 API 仅管理员可写和查看 Value。应用授权普通用户首期不能读取 KV Value、连接用户名或 Endpoint；后续若开放需独立权限设计。
- external deployment API 不新增 KV 管理能力；部署 preview/confirm 继续按应用授权工作，但不能返回配置中心密码。

### Security And Failure Rules

- etcd client、Secret lease、executor 和模板脚本的错误必须使用稳定脱敏原因；任何 `Debug`、tracing field、审计 JSON、task payload、snapshot 和部署 event 都不得包含明文密码或完整认证 URL。
- Endpoint 检测和业务调用共用同一出站策略；不得只在保存时校验字符串后让 client 再次解析到不同地址，也不得跟随 redirect 或通过 URL userinfo 传递认证。
- Secret environment lease 只允许服务端定义的五个业务变量和模板初始化变量；拒绝 `PATH`、loader、runtime、executor 配置及任意客户端自定义变量名，避免环境注入改变执行边界。
- lease 必须绑定 task ID、Agent ID、connection generation、payload digest、purpose 和凭据版本；过期、跨任务、跨 Agent、已撤销或 snapshot 版本不符时拒绝。传输失败可在任务 lease 内重发同一密文对应的值，不重新生成密码。
- lease 状态固定为 `issued`、`granted`、`consumed`、`revoked`、`expired`：同一运行中任务在传输失败或 Agent 重连时，可在固定 TTL 内重新取得同一凭据版本；不因续传生成新密码。executor 已接收授权并启动后若重启，沿用现有 fail-closed 语义将当前 task 标记为 `executor_restarted`，由 deployment retry 创建新 task/lease 并引用同一加密凭据版本，不复活已消费的一次性授权。任务终态统一撤销未终结 lease，超出任务 deadline 不续租。
- etcd 调用使用短超时和并发上限。etcd 不可用不能阻塞 API heartbeat/dispatcher worker；KV/检测请求独立失败，依赖部署在创建任务前 fail closed。
- 控制面主密钥缺失时拒绝启动已有配置中心凭据的管理能力；credential re-encrypt 模式必须覆盖新增凭据表并可在当前/previous key 间迁移。
- 模板不得将密码写入 Compose env 文件或宿主机永久脚本参数。若 etcd 自身需要持久认证，只保存其数据目录中的 Auth 状态；Deploy Go 的管理员密码密文留在控制面。
- 所有包含 KV Value 的列表、详情、导入预览和导出响应使用 `Cache-Control: no-store`；前端不得把 Value 写入 URL、browser storage、持久 query cache 或可被代理复用的下载缓存。

### System-Wide Impact

- **数据库：** 新增前进式 migration `0030` 及后续修正版本；历史 migration 不修改。涉及唯一平台绑定、应用绑定、凭据、reveal、业务身份、KV metadata、切换作业和 Secret environment lease。
- **协议与发布：** Agent protocol 提升 v13，控制面保持当前 v11/v12 兼容；需要同步 schema、Agent/executor、多架构 release 和安装手册。模板不是 Agent 内置能力，未来普通 Compose/脚本模板不要求升级 Agent。
- **API 与客户端：** internal OpenAPI 是 Admin 和 Flutter client 的共同契约。新增 platform etcd、应用配置中心、KV、reveal、切换 API；external OpenAPI 仅反映部署 preview/详情中的非敏感状态，不开放管理操作。
- **部署状态机：** preview/snapshot/confirm、dispatcher 和 executor 增加可选配置中心 gate；`none` 路径保持字节级兼容的既有任务 shape，避免无关 Agent 被迫升级。
- **运维：** runbook 增加模板初始化、平台绑定、备份、切换、故障解绑、凭据轮换和控制面/Agent 分阶段升级；任何真实环境步骤仍需当前对话单独授权。

### Risks And Dependencies

| 风险 | 缓解 |
| --- | --- |
| 单节点故障影响多个环境 | 明确单点边界，提供可用性检测、快照备份、正常切换和故障解绑；不宣称高可用。 |
| bootstrap 中途失败导致 Auth 半初始化 | 使用状态探测和幂等步骤；不匹配时 fail closed，保留数据供人工恢复，不自动重置。 |
| 密码经任务、日志或浏览器泄露 | 密文持久化、一次性 reveal、no-store、用途绑定 lease、变量白名单、日志回归测试和零化。 |
| 外部直接写 etcd 使操作人/时间失真 | etcd revision 为权威；SQLite metadata 仅在 revision 精确匹配时关联，否则显示未知。 |
| 控制面直连 etcd 拖慢核心 API或形成 SSRF | 独立 client/timeout/并发预算和 CIDR allowlist，固定解析结果且禁止 redirect；不在 heartbeat 热路径调用，部署只做本地状态 gate。 |
| 切换期间源继续被修改 | 记录 source header revision，复制结束重新扫描数量/摘要并在确认前校验；变化则要求重新复制。 |
| v13 升级扩大范围 | v11/v12 保持普通部署能力，只有配置中心任务要求新 capability；协议、executor 和 release 分独立单元验证。 |
| HTTP 传输被内网旁路窃听 | 文档明确风险并要求网络白名单；TLS/mTLS 作为后续优先项，不在 UI 中标记为“安全连接”。 |

### Sequencing

1. U1 固化 migration、加密凭据、领域状态和 API 骨架。
2. U2 在稳定 Secret 模型上实现 v13 通用 Secret environment lease。
3. U3 升级 etcd 模板和首次初始化，并接入一次性 reveal。
4. U4 实现平台设置、检测、绑定和生命周期保护基础。
5. U5 实现应用模式、平台 RBAC 和自有连接检测。
6. U6 实现 KV API、CAS、导入导出和审计 metadata。
7. U7 将配置中心固化到部署 snapshot 并完成 release 注入/gate。
8. U8 完成 Admin 页面及部署成功快捷绑定。
9. U9 完成平台切换、故障解绑、runbook、OpenAPI/client 和整体复核。

## Implementation Units

### U1. 数据模型、加密凭据与领域 API 骨架

- **Goal:** 建立唯一平台绑定、应用配置中心、凭据、reveal、业务身份、KV metadata、切换作业和 lease 的持久化基础。
- **Requirements:** R1、R4-R7、R9、R14-R16。
- **Files:** 新增 `api/migrations/0030_configuration_centers.sql`，按实现发现新增更高版本修正 migration；`api/src/crypto/mod.rs`、新增 `api/src/configuration_centers/mod.rs`、`api/src/lib.rs`、`api/src/main.rs`、`api/tests/migrations.rs`、`api/tests/database_constraints.rs`、新增 `api/tests/configuration_centers_api.rs`。
- **Approach:** 使用前进式表和约束表达唯一 active 平台绑定、版本化应用绑定及凭据/reveal 生命周期；为管理员和业务凭据使用不同 algorithm/AAD context；扩展 credential re-encrypt。先提供不依赖真实 etcd 的 CRUD/状态 API 和稳定错误，密码字段只写不回显。
- **Test scenarios:** migration 从空库和既有 0029 库前进成功；并发创建两个 active 平台绑定只有一个成功；密文不能跨资源/用途解密；previous key 可读并可重加密；普通 GET/OpenAPI/审计不含密文、nonce、key version 或密码；乐观版本冲突稳定返回 409。
- **Verification:** `make setup-git-hooks && make verify-git-hooks`，`cargo test -p deploy-go-api --test migrations --test database_constraints --test configuration_centers_api`，`make migration-git-guard-self-test`。
- **Dependencies:** 无。

### U2. v13 通用 Secret environment lease

- **Goal:** 让控制面以通用协议向单个 release 进程安全注入用途受限的敏感环境变量，而非增加 etcd 专用 Agent 代码。
- **Requirements:** R3、R11、R12、R16、R17。
- **Files:** `agent-protocol/src/lib.rs`、`agent-protocol/schema/agent-control.schema.json`、新增 `agent-protocol/schema/agent-control-v12.schema.json`、`agent-protocol/tests/schema_compatibility.rs`、`api/src/agents/dispatcher.rs`、`api/src/agents/websocket.rs`、`agent/src/secret_lease.rs`、`agent/src/executor.rs`、`agent-executor/src/protocol.rs`、`agent-executor/src/authorization.rs`、`agent-executor/src/release.rs`、`agent-executor/tests/release_lifecycle.rs`、相关 `agent/tests/` 与协议 fixture、`docs/standards/agent-control-protocol.md`、`docs/standards/privileged-agent-executor.md`。
- **Approach:** 保存当前 canonical v12 Schema 为不可变 `agent-control-v12.schema.json`，将 `agent-control.schema.json` 更新为 canonical v13，并继续保留既有 v11 fixture；定义 Hello/HelloAck、Envelope 严格解码、v12 fallback 和 capability gate。同步提升 Agent-executor 本地 IPC 版本，`ReleaseStartRequest` 增加只存在于内存帧的受众绑定 Secret 字段和 digest，release authorization claims 覆盖其类别、模板/阶段/目标进程与摘要；job snapshot 和 status/output 响应不保存/返回值。服务端 allowlist 固定 `etcd-init` 的两个 `ETCD_INIT_ROOT_*` 变量和业务连接的五个 `DEPLOY_CONFIG_CENTER_*` 变量。lease 按既定状态机允许授权前同任务重取，executor 接收后仅持有至子进程启动/退出并零化；携带 Secret 的 release 原始 stdout/stderr 不持久化。executor 重启保持既有 fail-closed 语义，由 deployment retry 创建新 task/lease。
- **Test scenarios:** v11/v12 控制面与 Agent fixture 继续完成普通部署；v12 严格 decoder 不接收 v13 字段且通过协商得到旧任务 shape；v13 配置中心任务获取并注入允许变量；模板管理员变量在普通业务、跨模板、跨阶段、跨 executor audience 时被拒绝；未知变量、`PATH`/loader 变量、错误 digest、过期/重放/跨任务/跨 Agent lease 被拒绝；传输失败和 Agent 重连在有效任务内重取相同值，executor 重启失败化后可由 deployment retry 继续，任务终态撤销；stdout/stderr、journal、authorization Debug、IPC fixture 和 task JSON 不出现明文。
- **Verification:** `cargo test -p deploy-go-agent-protocol`、`cargo test -p deploy-go-agent`、`cargo test -p deploy-go-agent-executor`、`cargo test -p deploy-go-api --test agent_websocket --test agent_dispatcher`，对应 clippy。
- **Dependencies:** U1。

### U3. etcd 模板认证初始化与一次性凭据展示

- **Goal:** 将现有开发模板升级为可由平台安全、幂等初始化的单节点 etcd 模板。
- **Requirements:** R2-R4、R13、R16。
- **Files:** `examples/templates/etcd/compose.yaml`、`examples/templates/etcd/etcd.env.example`、`examples/templates/etcd/parameter-schema.json`、`examples/templates/etcd/deploy-go.yaml`、`examples/templates/etcd/README.md`、模板 release 脚本与 `test-contract.sh`、`container-template/src/lib.rs` 内现有契约测试、`api/src/deployments/mod.rs`、`api/src/configuration_centers/mod.rs`、`api/tests/env_sync_dispatcher.rs`、新增模板集成测试 fixture。
- **Approach:** 扩展 `ImageDeploySpec`、parameter schema、snapshot 和 Compose 渲染，显式区分 client bind address、advertise URL 与 host port，并受 CIDR allowlist 校验。空卷 bootstrap 只监听 loopback，初始化脚本从 Secret environment lease 读取凭据，探测空/已初始化/Auth 半完成状态，绝不使用虚构的 `ETCD_ROOT_PASSWORD` 自动初始化能力；Auth 验证后才暴露内网 listener。Compose healthcheck 只承担无 Secret 的进程/本机存活，release 脚本在内存环境中执行带认证的 readiness，密码不进入 Compose 配置。控制面把 reveal 资格绑定到成功的目标运行；消费 endpoint 使用近期重新认证、CSRF、no-store 和原子状态更新。
- **Test scenarios:** 空数据卷在未认证阶段无法从内网并发读写，完成 root/Auth 后才开放内网；重复 release 和升级保留数据/密码；有数据但凭据不匹配或 Auth 状态异常时失败且不清卷；Auth 后 Compose liveness 与认证 readiness 均正确，错误凭据不会无限等待；部署失败不可 reveal；成功后只可 reveal 一次；模板日志和 Compose 配置不含密码；bind/advertise/Endpoint snapshot 一致，拒绝公网误配置、空 host、userinfo、query、fragment 和非法 URL。
- **Verification:** `cargo test -p deploy-go-container-template`、模板 shell 静态检查、隔离 Docker Compose 集成测试、`cargo test -p deploy-go-api --test env_sync_dispatcher --test configuration_centers_api`。
- **Dependencies:** U1、U2。

### U4. 平台 etcd 设置、检测、绑定与基础保护

- **Goal:** 提供唯一平台 etcd 的配置、权限检测、模板部署快捷绑定及服务端生命周期保护。
- **Requirements:** R1、R5、R13、R16。
- **Files:** `api/Cargo.toml`、`Cargo.lock`、`api/src/configuration_centers/mod.rs`、`api/src/applications/mod.rs`、`api/src/deployment_targets/mod.rs`、`api/src/deployments/mod.rs`、`api/src/lib.rs` 中 internal OpenAPI 定义、`api/tests/configuration_centers_api.rs`、`api/tests/applications_api.rs`、`api/tests/deployment_targets_api.rs`。
- **Approach:** 引入锁定版本的 `etcd-client` Rust crate 和可注入测试 adapter，并在集成前验证 connect timeout、keepalive、Auth token 刷新和 TLS-disabled 行为；实现配置、检测、绑定、清除 API。检测分别验证 endpoint/status/auth/管理权限，不修改当前绑定；绑定仅接受成功检测的同版本配置。平台绑定同时登记 template manifest 给出的受保护 Env 文件 ID/名称、目标和数据卷事实；应用/目标/Env 重命名或删除/清卷写入口据此保护并返回稳定冲突码，解绑后在同一事务解除保护。
- **Test scenarios:** 各检测错误码、超时和凭据更新；失败检测不覆盖现有绑定；并发绑定保持唯一；模板部署结果与 Endpoint 不匹配被拒绝；allowlist 外地址、DNS 重绑定、metadata、userinfo 和 redirect 被拒绝；平台 etcd 应用/目标/管理 Env/数据卷及 active platform-etcd 业务应用归档被阻止，业务应用解绑并撤销身份后可归档；普通升级/重试可继续；API 不在 heartbeat 或列表请求中同步探测 etcd。
- **Verification:** `cargo test -p deploy-go-api --test configuration_centers_api --test applications_api --test deployment_targets_api --test deployments_api`，使用隔离 etcd 的集成测试。
- **Dependencies:** U1、U3。

### U5. 应用配置中心模式、平台 RBAC 与自有连接

- **Goal:** 支持应用选择 none/platform/custom，并为平台模式建立真正隔离的业务只读身份。
- **Requirements:** R6、R7、R12、R16、R17。
- **Files:** `api/src/configuration_centers/mod.rs`、`api/src/applications/mod.rs`、`api/src/crypto/mod.rs`、`api/src/lib.rs` 中 internal OpenAPI 定义、`api/tests/configuration_centers_api.rs`、`api/tests/applications_api.rs`、隔离 etcd 集成测试。
- **Approach:** 应用绑定作为独立版本化资源；platform prefix 服务端派生且不可编辑，创建独立 Role/User 并加密业务密码；custom 保存独立密文，只做连接/read-prefix 检测。已有绑定的 environment/mode 变化走持久化迁移作业：预览源/目标 prefix 和身份，复制并核验 Key 摘要、重建身份，管理员确认后切换并标记 `pending_redeploy`；取消或失败保留原绑定，显式解绑不删除旧数据。
- **Test scenarios:** 三种 mode 的首次选择和已有绑定迁移；保存 platform/custom 时逐目标检查 v13 `secret_environment_v1`/network-probe capability，旧 Agent 返回可展示的升级提示但不影响 none；platform 身份只能读自身 prefix、不能写或跨应用/环境读；环境迁移复制/摘要冲突/失败重试/重复确认/解绑；重复 provision 幂等；远端创建中断后可恢复；custom 凭据只写不回显且不触发 RBAC/KV API；平台不可用时状态明确并阻止依赖部署。
- **Verification:** `cargo test -p deploy-go-api --test configuration_centers_api --test applications_api`，隔离 etcd RBAC 黑盒测试。
- **Dependencies:** U4。

### U6. KV 管理、CAS、批量导入导出与审计

- **Goal:** 在平台前缀内提供有界、可并发保护、可审计的轻量 KV 管理 API。
- **Requirements:** R8-R10、R16。
- **Files:** 新增 `api/src/configuration_centers/kv.rs` 或等价领域模块、`api/src/audit/mod.rs`、`api/src/lib.rs` 中 internal OpenAPI 定义、新增 `api/tests/configuration_center_kv_api.rs`、集成 etcd fixture。
- **Approach:** 所有 key 都以相对形式入参并由服务端拼接规范 prefix；首期默认 Key 不超过 1 KiB、Value 不超过 256 KiB、列表每页不超过 200 项、导入每批不超过 64 项且编码后不超过 1 MiB、单次导入/导出不超过 32 MiB，且任何预算不得超过实测 etcd server request/transaction 上限；preview 和 apply 使用同一预算并在执行前拒绝超限。PUT/DELETE 使用 mod revision CAS。导入先生成带 source revision、摘要和过期时间的 preview，再按有界 transaction 应用；导出流式并使用 `no-store`。mutation metadata 仅在 etcd revision 匹配时关联 actor/time，审计摘要使用专用 HMAC。
- **Test scenarios:** CRUD、空前缀、Unicode key、非法/超限 key/value；CAS 创建/编辑/删除冲突；前缀逃逸和跨应用访问；外部修改后 actor/time 变 unknown；导入新增/修改/删除预览、覆盖确认、批次中冲突停止；导出不含凭据；审计和错误不含 Value。
- **Verification:** `cargo test -p deploy-go-api --test configuration_center_kv_api`，隔离 etcd 集成测试和 OpenAPI schema 检查。
- **Dependencies:** U5。

### U7. 部署 snapshot、能力 gate 与 release 注入

- **Goal:** 将配置中心事实稳定固化到部署，并在 release 时安全注入业务运行参数。
- **Requirements:** R11、R12、R16、R17。
- **Files:** `api/src/deployments/mod.rs`、`api/src/agents/dispatcher.rs`、`agent-protocol/src/lib.rs`、`agent/src/executor.rs`、`agent-executor/src/release.rs`、`api/tests/deployments_api.rs`、`api/tests/env_sync_dispatcher.rs`、`agent/tests/executor.rs`、`agent-executor/tests/release_lifecycle.rs`、`docs/standards/application-deployment-contract.md`、`docs/standards/deploy-script-contract.md`。
- **Approach:** preview 解析应用绑定并写入非敏感 descriptor/credential version，检查 Env/config-center 权威来源声明；confirm 校验 snapshot hash。dispatcher 创建用途绑定 lease，要求 v13 capability 并在派发前验证绑定状态/版本；目标 Agent 先执行 snapshot 约束的有界 TCP-connect probe并把结果绑定 target run。executor 经升级后的 IPC/authorization 注入固定五变量，与现有 `DEPLOY_ENV_DIR` 分属不同命名空间；密码绝不进入 snapshot、payload JSON 或持久 job state。应用部署契约明确变量语义、只读 prefix、进程重启刷新和 etcd 不可达时不得静默回退，并用独立黑盒 consumer fixture 验证真实读取。
- **Test scenarios:** type 缺省固化为 etcd；none snapshot/task shape 保持兼容；platform/custom 注入正确值；黑盒 consumer 用五变量读取自身 prefix 且不能跨 prefix/写入；Env 与 etcd 共存未声明权威来源时提示冲突；控制面可用但目标 TCP 不可达时只阻止对应 run；凭据轮换、绑定切换、unavailable/pending_redeploy、旧 Agent 和 lease 失效分别阻止部署并给出稳定提示；prepare 成功后配置失效不丢部署状态且 release fail closed；日志和事件无密码。
- **Verification:** `cargo test -p deploy-go-api --test deployments_api --test env_sync_dispatcher`、`cargo test -p deploy-go-agent-protocol -p deploy-go-agent -p deploy-go-agent-executor`。
- **Dependencies:** U2、U5。

### U8. Admin 设置、应用 KV 与部署成功快捷绑定界面

- **Goal:** 提供管理员可完成平台连接、应用选择、KV 管理和一次性凭据处理的完整 Web 工作流。
- **Requirements:** R4-R10、R12-R15。
- **Files:** `admin/src/features/settings/SettingsPage.tsx`、`admin/src/features/settings/api.ts`、`admin/src/features/applications/ApplicationDetailPage.tsx`、新增 `admin/src/features/configuration-centers/`、`admin/src/features/deployments/DeploymentDetailPage.tsx`、`admin/src/routes/AppRoutes.tsx`、`admin/src/routes/routeMetadata.tsx`、`admin/src/styles.css`、对应 `admin/src/test/*.test.tsx`、生成的 `admin/src/api/generated/`。
- **Approach:** 设置页增加独立“平台 etcd”区段，显示当前绑定、依赖应用、检测和 Endpoint 状态；提供正常切换向导（预检、复制、摘要核验、确认、失败重试）及输入应用名称的近期重新认证故障解绑流程。应用详情增加配置中心 mode/RBAC 状态、权威来源、environment/mode 迁移或解绑向导和 KV workspace；取消或失败保留原绑定，成功后显示 provisioning/pending-redeploy。部署成功仅对 etcd 模板显示一次性凭据与快捷绑定；reveal 前明确不可恢复并二次确认，响应成功即视为消费，提供复制结果反馈和不回显旧值的凭据轮换/恢复入口，轮换生成新密码并产生新 reveal。密码和 KV Value 只在组件内存存在。KV 使用列表/详情双栏、搜索/前缀筛选、revision 冲突 diff、`dotenv-v1`/JSON 导入预览和当前前缀 JSON 导出；定义 loading、空前缀、无搜索结果、超时、权限拒绝、CAS 冲突、preview 过期、批次部分失败状态，冲突/失败保留本地编辑与未应用差异。
- **Test scenarios:** 默认 none、首次选择及已有 platform/custom 的迁移/解绑、取消/失败/成功与未保存保护；保存时展示各目标旧 Agent 升级提示；密码留空表示保持、显式更新/清除语义；检测各种状态；正常切换和故障解绑的确认、重试及影响展示；一次性 reveal 预警、复制反馈、消费后刷新不可见和轮换入口；KV CRUD/CAS 冲突/diff/导入预览/过期/部分失败/导出/超限/重试；包含 Value 的响应不进入持久缓存；只读用户看不到管理入口；受保护操作的错误说明；窄屏将双栏变为可返回的列表/详情导航，键盘焦点、可见焦点、标签、异步状态公告和触控尺寸可用，Value 长文本不撑破布局。
- **Verification:** `npm --prefix admin test`、`npm --prefix admin run typecheck`、`npm --prefix admin run build`，使用隔离 API/etcd 的浏览器关键流程与桌面/移动截图验证。
- **Dependencies:** U4、U5、U6、U7。

### U9. 平台切换、故障解绑、文档与发布准备

- **Goal:** 完成平台实例生命周期尾部，并使 API、客户端、runbook 和发布顺序可执行。
- **Requirements:** R13-R17。
- **Files:** `api/src/configuration_centers/mod.rs`、新增 `api/src/configuration_centers/switch.rs`、`api/src/applications/mod.rs`、`api/src/deployments/mod.rs`、`api/tests/configuration_center_switch.rs`、`docs/runbooks/application-templates.md`、`docs/runbooks/application-onboarding.md`、新增 `docs/runbooks/platform-etcd.md`、`docs/runbooks/api-migrations.md`、`docs/standards/api-contract.md`、`docs/standards/application-manifest.md`、internal/external OpenAPI 与生成 Web/Flutter client、`docs/reviews/` 下复核记录。
- **Approach:** 用持久化阶段作业执行 copy/verify/rebuild identities/confirm/switch；提供可恢复重试和状态查询。故障解绑是独立强确认操作，只改变绑定及依赖状态。runbook 固化 etcd snapshot 备份的独立密钥加密、最小文件权限、受限存储、保留/销毁期限、审计和不产生明文临时副本的恢复流程，并说明现有 Env 通过 `dotenv-v1` preview/import、核验、选择权威来源和重新部署迁移到平台 etcd，以及管理员凭据轮换、控制面先行/v13 Agent 按需升级和回滚；发布步骤只写 runbook，不在实施中自动操作真实环境。
- **Test scenarios:** 大于单 transaction 的分页复制、断点恢复、源 revision 变化、目标冲突、摘要不一致、身份重建失败、重复确认；成功切换原子更新并标记依赖应用；故障解绑名称不匹配拒绝、成功后不删资源；旧实例解除保护；备份文件未持有独立密钥时不可读取，恢复无明文临时副本并遵守保留销毁；OpenAPI 仅在管理员 KV Value/reveal 响应含受控明文且带 no-store，其他响应无 Secret 字段；生成客户端保持构建。
- **Verification:** `cargo test -p deploy-go-api --test configuration_center_switch --test configuration_centers_api --test deployments_api`、OpenAPI/client generation 检查、Admin/Flutter client 构建、runbook 命令静态复核。
- **Dependencies:** U4-U8。

## Verification Contract

每个单元先运行其聚焦测试并完成 `git diff --check`；涉及 migration 的单元在暂存前执行 Git hook 安装/校验和 migration 门禁。最终至少执行：

```bash
make setup-git-hooks
make verify-git-hooks
cargo fmt --all --check
cargo clippy -p deploy-go-api --all-targets -- -D warnings
cargo clippy -p deploy-go-agent-protocol --all-targets -- -D warnings
cargo clippy -p deploy-go-agent --all-targets -- -D warnings
cargo clippy -p deploy-go-agent-executor --all-targets -- -D warnings
cargo test -p deploy-go-container-template
cargo test -p deploy-go-agent-protocol -p deploy-go-agent -p deploy-go-agent-executor
cargo test -p deploy-go-api --test migrations --test database_constraints
cargo test -p deploy-go-api --test configuration_centers_api --test configuration_center_kv_api --test configuration_center_switch
cargo test -p deploy-go-api --test deployments_api --test env_sync_dispatcher --test applications_api --test deployment_targets_api
npm --prefix admin test
npm --prefix admin run typecheck
npm --prefix admin run build
git diff --check
git diff --cached --check
```

- 使用固定版本隔离 etcd 容器完成 Auth/RBAC/KV/CAS/切换黑盒测试；测试数据、端口和卷与真实环境隔离，结束后清理。
- 扫描 API 响应、task/snapshot JSON、部署日志、审计、Agent journal、executor job state 和前端缓存，确认不存在测试密码明文。
- 对 protocol v11/v12 fixture、none 模式应用、现有 Env gate、普通两阶段/image 部署执行回归，证明 v13 不会强制所有节点立即升级。
- 运行 internal OpenAPI 生成和 Web/Flutter client 漂移检查；external OpenAPI 不新增平台管理能力，也不暴露配置中心凭据。
- U8 完成后用一种浏览器工具链验证设置、应用 KV、一次性 reveal、CAS 冲突和窄屏布局；任务结束关闭隔离会话。
- 重要改动进入 `$ce-code-review`，重点检查 Secret 泄露、RBAC 逃逸、migration 约束、切换原子性、Agent 兼容和测试缺口；问题返回对应 U 单元修正后重跑验证。

## Definition of Done

- R1-R17 和 AE1-AE12 均有实现与自动化测试证据，没有 launch-blocking open question。
- 单节点 etcd 模板从空卷可幂等启用 Auth，重试/升级不改变密码、不清数据，一次性凭据展示不可重放。
- 平台绑定唯一且检测语义明确；应用/环境前缀和只读 RBAC 经真实 etcd 黑盒测试证明隔离。
- KV CRUD、搜索、CAS、导入导出、revision 和脱敏审计完整；外部直接修改不会被错误归因给 Deploy Go 操作者。
- 配置中心信息进入部署 snapshot，Secret 仅经用途/模板/阶段/executor audience 绑定 lease 和版本化内存 IPC 到 release 进程；目标网络 probe、none 模式和 v11/v12 普通部署兼容。
- 生命周期保护、正常切换、故障解绑和 `pending_redeploy` 状态均由服务端强制，不依赖 UI。
- migration、协议 schema、internal/external OpenAPI、生成客户端、Rust/Admin 全部门禁通过，runbook 与实现一致。
- 代码按 U1-U9 小闭环提交；只暂存本轮文件，提交后 fetch/rebase/push `origin main`，不强推。
- 所有实验代码、临时测试密码、隔离容器/卷、调试日志和未使用依赖已清理；工作区不混入无关改动。
- 未执行任何真实环境部署、migration、Agent 升级、重启、切流或业务应用操作；这些动作必须在未来对话中按具体环境重新授权。
