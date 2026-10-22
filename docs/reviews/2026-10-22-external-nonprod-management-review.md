# 复核记录：对外 API 与 Skill 最大化管理非生产应用（specs/005）

- 范围：`git diff 3ea203f..HEAD`（不含无关的测试夹具修复 `5a52e35`），对照 `specs/005-external-nonprod-management/` 的 FR-001～FR-018。
- 方式：独立 agent 只读复核（权限边界、敏感信息泄露面、内部行为回归、节点白名单语义、migration、测试质量），未给出实现方的结论。复核方**没有运行任何测试**，结论来自读代码；实现方对每条发现重新对照代码验证，并做变异测试。
- 结论：未发现写操作绕过 `prod` 守卫的路径；内部管理面 handler（`application_sources` / `applications` / `application_envs` / `deployments` / `runtime_probe`）的鉴权顺序、响应、状态码与审计摘要与重构前一致；新增响应结构均为显式字段白名单，未发现主机、端口、用户名、工作根目录、lease、Env 或配置内容泄露；migration 0043 只有 `CREATE TABLE` / `CREATE INDEX`。有 4 条确认缺陷、1 条怀疑项被证实，均已修复。

## 发现与处置

| # | 严重度 | 发现 | 处置 |
| --- | --- | --- | --- |
| 1 | 中 | 新增的 `GET git-source`、`GET git-source/refreshes/{id}`、`GET runtime-status` 对 `prod` 应用可读，违反 FR-013“不新增任何对 prod 的读取”。既有 `sources.git` 摘要不含仓库地址与凭证名，所以来源详情是**新增暴露**。原测试把它们归入“保持现状”。 | 已修复：三个 handler 加非生产守卫（`prod` 返回 403 `external_production_application_forbidden`），测试迁入 `PROD_READ_FORBIDDEN_CASES`；原计划“GET 沿用 Key 绑定即可读”与 FR-013 矛盾，已修订 plan。 |
| 2 | 中 | Key 能重新启用管理员归档的非生产应用（状态接口放宽为只看绑定），推翻管理员的停用。 | 已修复：已归档应用只能由归档它的同一个 Key 重新启用，依据该应用最近一次 `application.status.update` 审计记录，查不到或不一致一律拒绝，返回 403 `external_application_reactivation_forbidden`（无需新 migration）。 |
| 3 | 中 | Key 的节点集合含已归档节点时，管理面无法保存（选择器过滤归档节点但回传仍含它，服务端 404），摘要也继续显示已归档节点。 | 已修复：摘要的 `node_ids` 只含未归档节点；整体替换会一并清掉旧的授权行。 |
| 4 | 低 | 被停用的目标可经状态接口重新启用而绕过节点白名单。 | 已修复：`status=active` 时校验目标所在节点授权；停用仍随时允许。 |
| 5 | 怀疑→证实 | `probe-runtime` 与 `retry-env-sync` 对“非生产应用 + 正式环境目标”（现有夹具即可构造）会触达生产节点，缺少 `require_no_production_targets`。 | 已修复并补测试。 |

## 未处理项（已接受或留作后续）

- Git 来源 URL 校验只在 userinfo 含 `:` 时拒绝，`https://TOKEN@host/repo` 可通过并明文入库，且 Agent 侧对仓库地址没有任何协议或主机限制（内网 IP 可被构建节点访问）。**后续已处理（对外入口）**：新增仓库地址策略，仅作用于对外保存这一步，见 `docs/runbooks/external-deploy-api.md`；内部管理面的校验未改动。
- `require_key_agent_node` 对不存在的 Agent 返回 404、对存在但未授权的返回 403，可枚举 Agent ID。风险很低，接受。
- Agent 侧确认没有协议/主机限制；对外入口已用字符串级策略拦截 IP 字面量，但**不防域名被解析到内网地址**，彻底解决需要 Agent 侧限制协议（如 `GIT_ALLOW_PROTOCOL=ssh:https`）与出口，涉及 Agent 发版，另议。

## 验证

- 新增测试覆盖上述 5 条，另有 7 个针对修复点的变异测试（撤销任一修复，对应用例失败）：git-source 读取守卫、runtime-status 守卫、探测的生产目标检查、Env 同步重试的生产目标检查、重新启用限制、目标重新启用的节点授权、归档节点过滤。
- 二次验证中发现一处无关的测试问题：`discovery_and_branch_flow...` 用生产应用验证“别的应用读不到发现记录”，加守卫后变成被生产守卫拦下而不再测到归属检查，已改用同 Key 绑定的另一个非生产应用。
- `make api-check` 949/949、`make external-deploy-check`、`make api-openapi-check api-client-check`、`make admin-check`、`make client-sensitive-check` 通过。

## 复核方未覆盖

`deploy-go-deployer/src/main.rs` 与 `skills/` 内容、`external.json` / 双端 client 漂移、`docs/` 改动、各新增测试全文；其中 OpenAPI 漂移与 CLI 契约已由 `make api-openapi-check api-client-check` 与 `cargo nextest run -p deploy-go-deployer` 覆盖，文档一致性由实现方在 T030 核对。

## 追加修订：节点限制改为“未配置即不限制”（2026-10-22，用户要求）

用户明确要求：本功能不能影响任何环境下的原有应用，初衷只是让 Skill 对非生产环境更自由，不要复杂。
回头核对发现，**节点白名单“默认拒绝”本身就违反这条**：上线后所有现有 Key 在管理员逐个配置节点之前，原本可用的非生产写操作
（创建/修改部署目标、保存工作区来源）都会返回 403。当时为让两个原有用例通过，还在 `external_api.rs` 里给它们补了授权——这本身就是行为已改变的信号，却被当成了普通的测试适配。独立复核没有指出这一点，因为复核对照的是规格，而规格里写的正是“默认拒绝”。

处置：
- `require_key_node`：Key 没有任何节点配置时直接放行；配置后才限制。统计所有授权行（含已归档节点），避免节点归档后限制悄悄失效。
- 构建 Agent 目录：未配置时列出全部可用 Agent，配置后只列集合内的。
- 撤销对 `external_api.rs` 原有用例的授权改动：该文件相对规格提交现在只有路径清单新增，24 个原有用例未经任何修改通过。变异验证：把逻辑改回“默认拒绝”，这些原有用例会失败。
- 管理面文案、Skill、runbook、标准文档、规格/计划同步修订；migration `0043` 文件头注释保留（已提交，按规则不改），runbook 中说明其被取代。

顺带发现并修复一处我自己写的测试时序隐患（`discovery_and_branch_flow...` 里模拟 Agent 先于“重复发起”应答，之前几次侥幸通过）。
