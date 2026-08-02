---
title: Web、Flutter 正式客户端与跨端 CI 实施计划
date: 2026-08-01
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# Web、Flutter 正式客户端与跨端 CI 实施计划

## Goal Capsule

在现有 Rust API、UI 预览和部署脚本契约基础上，交付可本地联调、可自动测试、可构建发布的 `admin/` Web 管理端与 `admin-app/` Flutter 管理端。两端通过同一份 OpenAPI 契约访问 API，完成认证、节点、SSH 凭证、应用、用户授权和部署主闭环，同时保持“只编排并执行应用自有脚本，不接管部署过程”的轻部署边界。

本计划只覆盖正式客户端、客户端所需的最小 API 补充和 CI。不会连接真实节点、执行真实部署、引入角色系统、邀请注册、多管理员、部署编排引擎或新的聚合后端。

## Product Contract

### Problem

当前仓库已有 Rust API、部署脚本规范和 `ui/` 交互预览，但尚无可交付的正式 Web/Flutter 客户端。API 契约、身份安全、SSE 日志续传、跨端生成客户端和发布产物也尚未形成端到端闭环，因此项目还不能作为日常部署管理工具上线使用。

### Actors

- **A1 管理员**：唯一管理员，负责用户、SSH 凭证、节点、应用、目标和部署管理。
- **A2 普通用户**：仅访问被授权应用及其部署信息，不能访问系统管理功能。
- **A3 CI/发布维护者**：验证契约和双端构建，生成不含敏感信息的发布产物。
- **A4 部署操作者**：管理员或获授权普通用户，通过预览、确认、日志和结果完成一次脚本部署。

### Requirements

- **R1**：创建独立的 `admin/` React + TypeScript + Vite 工程和 `admin-app/` Flutter 工程，均可独立开发、测试、构建。
- **R2**：`api/openapi/openapi.json` 是两端接口模型和请求方法的唯一来源；生成代码提交仓库并由 CI 检查漂移。
- **R3**：生成代码与业务适配层隔离，业务页面不得散落手写 endpoint、字段模型或错误解析。
- **R4**：完成正式客户端 API readiness audit，补齐 setup 状态、CSRF 恢复、用户授权读取、列表响应 schema、个人资料和通知偏好持久化，不新增无必要的概览聚合 API。
- **R5**：Web 使用 HttpOnly Cookie 会话，所有状态变更携带 `X-CSRF-Token`，生产默认 `Secure`、`SameSite=Lax`。
- **R6**：Flutter 使用 Cookie 会话，Cookie 与 CSRF Token 只写入平台安全存储，不写普通首选项、日志或崩溃上下文。
- **R7**：两端支持首次管理员 setup、登录、登出、恢复会话和会话失效统一处理；setup token 使用后不持久化。
- **R8**：管理员和普通用户权限由 API 强制；客户端提供可理解的隐藏、禁用、403 和深链回退，不以客户端判断替代授权。
- **R9**：Web 完成概览、应用、部署目标、部署、节点、SSH 凭证、用户、应用授权、设置、审计日志和“我的”页面。
- **R10**：Web 设置采用侧栏二级菜单，不用页面内部按钮模拟子页面导航；页面层级和 URL 可直接访问、刷新恢复。
- **R11**：Web 延续 GitHub 风格黑白灰色系，危险操作仅在语义必要时使用克制红色，不用装饰性红框。
- **R12**：Web 表格、筛选、cursor 分页、表单草稿、空态、错误态、加载态和权限态均可操作并可测试。
- **R13**：管理员可生成 SSH key、查看和复制公钥、重命名、删除未绑定凭证；私钥永不由 API 返回或进入客户端。
- **R14**：节点可绑定或解绑凭证；host key 扫描结果必须由用户独立确认后才能检查连接，不允许自动信任。
- **R15**：复杂 SSH 凭证和节点 onboarding 保持 Web 管理能力；Flutter 只提供节点、凭证绑定状态和健康状态的必要只读信息。
- **R16**：部署必须先 preview 再 confirm；confirm 携带幂等键和预览快照标识，配置变化导致旧预览被拒绝。
- **R17**：部署详情通过 SSE 实时展示日志，支持 `Last-Event-ID` 或 `after` 续传、去重、暂停跟随和恢复跟随；日志按不可信纯文本转义渲染，展示增强只能使用固定白名单。
- **R18**：两端支持取消、重试、失败和 interrupted 状态；动作必须有明确确认、进行中反馈及结果反馈。
- **R19**：应用、目标、部署和授权操作遵守 `docs/standards/deploy-script-contract.md`，平台只执行约定脚本并回馈结果。
- **R20**：Flutter 使用 Riverpod、`go_router`、Dio/CookieJar，底部主导航包含概览、资源、部署和“我的”。
- **R21**：Flutter “我的”采用固定顶部身份区，不使用多余顶部导航或权限说明；用户管理仅对管理员提供入口，不提供角色管理和邀请。
- **R22**：Flutter 保持圆润但有层级的移动端视觉，不设计 hover；触控目标最小 44 logical pixels，并覆盖窄屏和系统字体放大。
- **R23**：Flutter 进入后台再恢复时重新确认会话和部署状态，并从最后日志事件继续，不能假设长连接一直存活。
- **R24**：两端统一处理 API `code`、`message`、`request_id`、`details`，错误提示保留可排查的 request ID 且不泄露敏感信息。
- **R25**：列表统一遵守 `items` + `next_cursor`，筛选变化清空 cursor 链；刷新和返回不产生重复数据。
- **R26**：未提交表单提供离开保护或显式丢弃，提交期间防重复，幂等操作可安全重试。
- **R27**：日志、遥测、测试 fixture、截图和构建产物不得包含 SSH 私钥、主密钥、setup token、生产 Cookie、CSRF Token 或脚本 secret。
- **R28**：PR/main CI 覆盖 API、UI 预览、Web、Flutter、OpenAPI 生成漂移和聚焦 E2E。
- **R29**：tag release 保留 API amd64/arm64 产物，并新增 Web 静态包、Android debug APK、未签名 release AAB 构建输入和统一 checksum；Android 产物不宣称可用于生产分发。
- **R30**：iOS 首阶段执行 format/analyze/test 和 Simulator 安全会话 smoke；签名发布在凭证和发布策略明确前不默认启用，任何签名材料不进入 artifact。
- **R31**：Makefile 与 runbook 提供可重复的生成、检查、开发、联调和构建命令；本地默认使用 mock/fixture，真实远程执行需要用户对具体环境单独授权。
- **R32**：普通用户不自助注册；管理员创建账号时设置初始密码并通过系统外安全渠道交付，管理员重置密码后撤销该用户全部既有会话。

### Key Flows

- **F1 首次初始化与会话**：访问客户端 -> 探测 setup 状态 -> 输入一次性 setup token 创建唯一管理员 -> 登录 -> 恢复会话 -> 登出或会话过期。
- **F2 SSH 节点接入**：生成凭证 -> 查看并复制公钥 -> 将公钥放入目标节点 -> 创建节点 -> 绑定凭证 -> 扫描 host key -> 人工确认 -> 检查连接。
- **F3 应用与授权**：创建应用 -> 配置脚本规范和目标 -> 管理员创建普通用户并设置初始密码 -> 通过系统外安全渠道交付 -> 分配应用授权 -> 普通用户仅查看和操作已授权应用；忘记密码时由管理员重置并撤销旧会话。
- **F4 部署主闭环**：选择应用和目标 -> preview -> 核对 snapshot -> confirm -> 查看实时日志 -> 取消、重试或确认最终结果。
- **F5 移动端恢复**：打开部署详情 -> 接收日志 -> App 进入后台 -> 回到前台 -> 重新获取状态 -> 从最后 event ID 续传。
- **F6 契约与发布**：修改 OpenAPI -> 重新生成双端客户端 -> CI 验证无漂移 -> tag 构建 API/Web/Android -> 校验 checksum。

### Acceptance Examples

- **AE1**：全新实例通过 setup 创建管理员后可登录；刷新 Web 或重启 App 可恢复会话，客户端存储中不存在 setup token。
- **AE2**：Web 生成 SSH key 后只能读取公钥；复制公钥、绑定节点、scan、confirm、check 顺序成功，跳过 confirm 会被阻止。
- **AE3**：应用或目标配置在 preview 后变化时 snapshot hash 改变，使用旧 preview confirm 返回明确冲突，客户端要求重新预览。
- **AE4**：SSE 在日志事件 120 后断开，重连从 120 继续且页面中没有重复事件；暂停跟随不停止接收，恢复后滚到最新。
- **AE5**：Flutter 部署详情进入后台后连接释放，回到前台重新取状态并用最后 event ID 续传，不遗漏最终状态。
- **AE6**：普通用户通过深链进入用户或凭证管理时，API 返回 403，客户端显示权限页且不渲染受保护数据。
- **AE7**：个人资料和通知偏好修改后，在另一客户端重新登录仍可恢复；权限说明不作为“我的”页面内容。
- **AE8**：Cookie、CSRF、私钥和 secret 不出现在控制台、应用日志、普通存储、fixture、截图或 artifact。
- **AE9**：手工修改 OpenAPI 但未刷新任一生成客户端时，CI 的 drift check 失败并指出需运行的生成命令。
- **AE10**：发布 tag 后同时得到两种架构的 API、Web 静态包、Android debug APK、未签名 release AAB 构建输入和 checksum；文档明确 Android 非生产签名边界，iOS 无签名发布任务。

### Scope Boundaries

**In scope**

- 正式 Web 与 Flutter 客户端、最小 API delta、OpenAPI 生成、测试、Makefile、runbook 和 CI/release。
- 使用 mock、fixture、测试容器完成本地联调和自动验证。
- 唯一管理员与普通用户模型、应用级授权、SSH 凭证公钥展示和部署脚本执行反馈。

**Out of scope**

- 角色管理、邀请用户、公开注册、多管理员审批流。
- 平台接管代码拉取、构建、进程守护、流量切换或回滚实现；这些仍由应用脚本负责。
- 真实节点连接、真实远程脚本执行、生产迁移和生产发布。
- iOS 签名和商店发布、桌面客户端、离线部署、复杂通知渠道。

## Planning Contract

### Architecture

- `api/` 继续拥有服务端、migration 和 `api/openapi/openapi.json`。
- `admin/` 使用 React、TypeScript、Vite、React Router、TanStack Query、`lucide-react`；API 生成代码位于 `admin/src/api/generated/`，业务适配位于 `admin/src/api/` 其他文件。
- `admin-app/` 使用 Flutter、Riverpod、`go_router`、Dio/CookieJar 和平台安全存储；生成代码位于 `admin-app/lib/api/generated/`。
- Web SSE 使用支持 fetch、header 和重连控制的成熟库；Flutter 使用支持 header/cookie/事件 ID 的成熟 SSE 库，不自行拼接协议解析器。
- `ui/` 继续作为设计源和交互预览，不成为生产客户端运行依赖。
- 生成器版本、输入文件和生成命令固定；生成结果提交仓库，CI 重新生成后要求 `git diff --exit-code`。

### Key Technical Decisions

- **KTD1 双端独立工程**：采用 React Web 与 Flutter App，而不是共享 UI 运行时；共享边界是 OpenAPI 和产品规范。`session-settled: user-approved; rejected: 用单一跨端框架替代两端工程`
- **KTD2 OpenAPI 唯一契约**：客户端模型和请求代码由 `api/openapi/openapi.json` 生成，业务层只包裹认证、错误、分页和领域行为。`session-settled: user-approved; rejected: 两端各自手写 API model`
- **KTD3 Cookie 会话**：Web 使用 HttpOnly Cookie；Flutter 使用 CookieJar 并把 Cookie/CSRF 放入平台安全存储，不引入 bearer token 分支。`session-settled: user-approved; rejected: 为移动端新增长期 bearer token`
- **KTD4 权限模型保持简单**：唯一管理员加普通用户，系统管理只对管理员开放；不建设 role CRUD、邀请和注册。`session-settled: user-directed; rejected: 通用 RBAC 角色管理`
- **KTD5 脚本执行边界**：客户端配置和展示部署，API 执行规范脚本并回馈状态；平台不接管脚本内部部署过程。`session-settled: user-directed; rejected: 平台内置完整部署流水线`
- **KTD6 SSE 可恢复**：服务端现有 `Last-Event-ID`/`after` 契约保持权威，两端维护最后确认事件并做去重。`session-settled: user-approved; rejected: 只做不可恢复的实时日志流`
- **KTD7 移动端能力收敛**：App 完成高频查看和部署主闭环，复杂凭证和节点 onboarding 留在 Web；App 不复制所有管理表单。`session-settled: user-approved; rejected: App 首版完整复制 Web 系统管理`
- **KTD8 发布边界**：首轮发布 API、Web 和 Android；iOS 只检查测试，待签名策略明确后另行计划。`session-settled: user-approved; rejected: 在无签名方案时默认发布 iOS`

### Existing Patterns

- API 和安全契约：`docs/standards/api-contract.md`、`docs/standards/access-control.md`、`docs/standards/ssh-credential-security.md`。
- 部署边界：`docs/standards/deploy-script-contract.md`、`docs/runbooks/deployment-recovery.md`。
- 节点接入：`docs/runbooks/ssh-node-onboarding.md`。
- 本地运行：`docs/runbooks/local-development.md`、`Makefile`。
- 设计交接：`ui/docs/page-map.md`、`ui/docs/component-inventory.md`、`ui/docs/web-handoff.md`、`ui/docs/flutter-handoff.md`。
- CI/release：`.github/workflows/ci.yml`、`.github/workflows/release-artifacts.yml`、`docs/runbooks/github-actions-release.md`。

### Sequencing

```text
U1 -> U2 -> U3
U3 -> U4 -> U5 -> U6/U7/U8/U15
U3 -> U9 -> U10 -> U11
U6 + U8 + U11 + U15 -> U12
U4 + U9 -> U13
U12 + U13 -> U14
```

每个单元形成独立提交并聚焦验证。U6、U7、U8、U15 在 U5 后可顺序实施；为减少共享路由和领域状态冲突，默认不并行修改同一客户端。API migration 只新增更高版本文件，不修改任何已提交 migration。

### Risks And Controls

- **契约生成器不兼容**：U3 先用最小 endpoint 试生成并固定版本；业务开发前解决 nullable、日期和 SSE 边界。
- **Cookie 跨端差异**：U5/U9 分别做恢复、CSRF、过期测试；开发环境只通过显式配置放宽 Secure。
- **SSE 重复或漏日志**：以 event ID 为游标，重连、乱序、重复和最终状态单独测试；普通 HTTP client 不承担 SSE。
- **客户端范围过大**：按 U-ID 小闭环交付，App 系统管理明确只读边界，不补角色、邀请或聚合 API。
- **敏感数据泄漏**：fixture 使用虚构值；日志拦截器做 header/body 脱敏；CI 对高风险模式做聚焦扫描。
- **发布环境不完整**：Android 先产可侧载验证的 debug APK 和供后续签名的 release AAB 构建输入；两者均不作为生产发布物，iOS 不伪造可发布产物。

## Implementation Units

### U1 客户端契约与 UI 设计补齐

**Goal**：让 UI 设计源覆盖正式客户端实施所需但当前缺失的页面和交互。

**Requirements**：R7、R9、R13、R14、R15、R20、R21、R22。

**Files**：

- 修改 `ui/assets/app.js`
- 修改 `ui/assets/mock-data.js`
- 修改 `ui/docs/page-map.md`
- 修改 `ui/docs/component-inventory.md`
- 修改 `ui/docs/web-handoff.md`
- 修改 `ui/docs/flutter-handoff.md`
- 修改 `ui/tests/ui-preview.spec.js`

**Existing patterns**：沿用 `ui/` 的单页预览、GitHub 黑白 Web 主题、圆润 App 主题和现有交互测试。

**Approach**：补充首次 setup、SSH 凭证列表/生成/公钥/重命名/删除、节点绑定/解绑/scan/confirm、应用授权页面；明确 App 只读边界和管理员入口。保留“我的”固定身份区，移除权限说明、角色管理和邀请入口。

同时为新增页面维护状态矩阵，逐页定义 loading、empty、partial error、full error、submit pending、success、failure/retry 和 403 的可见行为。

**Test Scenarios**：setup 成功进入登录；凭证删除绑定阻断；scan 后必须 confirm；管理员可进入用户管理，普通用户无入口；App 无 hover 状态；UI preview 至少覆盖状态矩阵中的每类状态。

**Verification**：`make ui-check`、`make ui-test`，并检查 8050 预览各新增路由可点击且窄屏无溢出。

**Dependencies**：无。

### U2 客户端所需 API delta

**Goal**：补齐个人资料和通知偏好持久化，并把最终客户端契约写入 OpenAPI。

**Requirements**：R2、R4、R7、R24、R27。

**Files**：

- 新增 `api/migrations/<next_version>_user_preferences.sql`
- 修改 `api/src/` 下用户、认证、路由和存储相关模块
- 修改 `api/openapi/openapi.json`
- 修改 `api/tests/` 下对应集成测试
- 修改 `docs/standards/api-contract.md`

**Existing patterns**：沿用现有 auth/me、统一 error envelope、鉴权 middleware、migration 版本和 OpenAPI 手工维护/校验方式。

**Approach**：先逐项核对 U5-U12 所需 operation、请求、成功响应、错误、header 和分页 schema，至少补齐 setup 状态读取、用户授权读取和缺少 schema 的列表响应。增加当前用户 profile 更新与偏好读取/更新 endpoint；字段采用显式 allowlist，管理员身份和授权不可由 profile 接口修改。

增加认证后的 CSRF refresh endpoint：该 endpoint 不要求旧 CSRF Token，但必须通过会话、精确 Origin 和 Fetch Metadata 校验，返回并轮换新的 CSRF Token，使 Web 刷新和 App 重启后可恢复状态变更能力；并发标签页和轮换窗口必须有明确测试。Flutter 的认证请求从构建配置读取允许的 Origin 并显式发送，API 契约不得依赖浏览器自动补齐。新增 migration，不改历史 migration。

**Test Scenarios**：登录用户读写自身资料；未登录 401；越权字段被拒；偏好跨会话恢复；无效字段返回统一错误；setup 状态和授权集合可读取；所有客户端列表有生成模型；CSRF 刷新覆盖错误 Origin、并发标签页、重复轮换和登出；migration 从空库和当前版本均可应用。

**Verification**：`cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、API 聚焦测试、OpenAPI 校验和 migration 校验。

**Dependencies**：U1。

### U3 OpenAPI 双端生成基线

**Goal**：建立可复现的双端 API client 生成和漂移检查。

**Requirements**：R2、R3、R24、R25、R28、R31。

**Files**：

- 新增 `admin/src/api/generated/`
- 新增 `admin-app/lib/api/generated/`
- 新增或修改生成器配置文件
- 修改 `Makefile`
- 修改 `.github/workflows/ci.yml`
- 修改 `docs/runbooks/local-development.md`

**Existing patterns**：沿用根 Makefile 聚合命令和现有 CI job 命名、缓存与最小权限设置。

**Approach**：选择能稳定生成 TypeScript 和 Dart 的固定版本工具；封装 `make api-client-generate` 与 `make api-client-check`。对 Cookie、CSRF、error、cursor 和 SSE 建立生成代码之外的适配层接口，禁止手改 generated 目录。

**Test Scenarios**：干净生成无 diff；删除一个生成字段后 drift 失败；OpenAPI 非法时生成失败；生成结果包含 U5-U12 使用的全部 operation 和响应模型。

**Verification**：连续执行两次生成结果一致，`git diff --exit-code -- admin/src/api/generated admin-app/lib/api/generated` 无输出。TypeScript typecheck 和 Dart analyze 分别由 U4、U9 在工程配置建立后验证。

**Dependencies**：U2。

### U4 Web 工程与设计系统基线

**Goal**：创建可运行、可测试、可扩展的正式 Web 工程。

**Requirements**：R1、R3、R10、R11、R12。

**Files**：

- 新增 `admin/package.json`、`admin/vite.config.ts`、`admin/tsconfig*.json`
- 新增 `admin/src/main.tsx`、`admin/src/app/`、`admin/src/routes/`
- 新增 `admin/src/components/`、`admin/src/styles/`
- 新增 `admin/src/test/` 和测试配置

**Existing patterns**：以 `ui/docs/web-handoff.md` 和 UI preview 为设计权威，图标使用 `lucide-react`。

**Approach**：建立 app shell、侧栏、二级菜单、route metadata、QueryClient、error boundary、主题 token 和响应式布局；使用 Vitest、Testing Library、MSW，预留 Playwright smoke。

**Test Scenarios**：主/二级路由可深链刷新；窄屏菜单可操作；未知路由 404；loading/error/empty 不位移；危险色只用于危险语义；生成 TypeScript client 可通过 typecheck。

**Verification**：`npm run lint`、`npm run typecheck`、`npm test`、`npm run build`。

**Dependencies**：U3。

### U5 Web setup、认证与权限壳

**Goal**：完成 Web 安全会话和基于服务端权限的路由保护。

**Requirements**：R5、R7、R8、R24、R27。

**Files**：

- 新增 `admin/src/features/auth/`
- 新增 `admin/src/api/http-client.ts`
- 新增 `admin/src/routes/guards.tsx`
- 新增 `admin/src/features/errors/`
- 新增相关单元和 E2E 测试

**Existing patterns**：遵守 `docs/standards/api-contract.md` 和 `docs/standards/access-control.md`。

**Approach**：用 credentialed fetch/client 访问 HttpOnly Cookie；从登录响应或 CSRF refresh endpoint 提取 CSRF 并仅保存在内存；统一处理 401、403、request ID。setup token 只保留在表单生命周期，提交后清空。

**Test Scenarios**：AE1、AE6、AE8；另测登录重复提交、CSRF 缺失、会话过期返回登录并保留安全 return URL、普通用户导航隐藏；setup/login 对缺失 Origin、非 allowlist Origin 和端口不匹配 Origin 均拒绝，且不创建会话或回显敏感值。

**Verification**：auth 单元测试、MSW 集成测试和 Playwright setup/login/logout/403 smoke。

**Dependencies**：U4。

### U6 Web SSH 凭证与节点 onboarding

**Goal**：交付管理员可安全完成 SSH 凭证和节点接入的完整 Web 流程。

**Requirements**：R9、R13、R14、R15、R24、R27。

**Files**：

- 新增 `admin/src/features/credentials/`
- 新增 `admin/src/features/nodes/`
- 修改 `admin/src/routes/` 和二级菜单
- 新增对应测试和 MSW fixtures

**Existing patterns**：遵守 `docs/standards/ssh-credential-security.md` 和 `docs/runbooks/ssh-node-onboarding.md`。

**Approach**：将生成、查看公钥、复制、重命名和删除拆成明确动作；删除前读取绑定状态。节点 onboarding 使用状态机呈现 bind -> scan -> confirm -> check，扫描结果与确认动作分离。

**Test Scenarios**：AE2、AE8；凭证绑定时删除被阻止；扫描指纹变化需要重新确认；复制失败有回退；普通用户 403；loading、empty、partial/full error、submit pending、success 和 failure/retry 均有 MSW 覆盖。

**Verification**：特性单测、MSW 集成测试、Playwright 管理员 onboarding smoke，不发起真实 SSH。

**Dependencies**：U5。

### U7 Web 应用、目标与应用授权

**Goal**：完成部署前的应用、目标和应用授权配置。

**Requirements**：R9、R12、R19、R24、R25、R26。

**Files**：

- 新增 `admin/src/features/applications/`
- 新增 `admin/src/features/targets/`
- 新增 `admin/src/features/grants/`
- 修改路由、菜单和测试

**Existing patterns**：字段和脚本语义遵守部署脚本契约。

**Approach**：构建可复用但不过度抽象的 cursor 列表、受控表单和离开保护；应用授权采用用户-应用显式分配/撤销，不出现角色。概览由现有列表并行请求计算，不新增聚合 API。

**Test Scenarios**：cursor 翻页和筛选重置；草稿离开确认；普通用户只见授权应用；应用授权分配和撤销即时生效；列表和表单状态矩阵均有 MSW 覆盖。

**Verification**：领域单测、MSW 集成测试、Playwright 应用配置和用户授权 smoke。

**Dependencies**：U5。

### U15 Web 用户、设置、审计与个人资料

**Goal**：完成与部署前配置解耦的账号和系统管理页面，使其可独立验证与回滚。

**Requirements**：R9、R10、R12、R24、R25、R26、R32。

**Files**：

- 新增 `admin/src/features/users/`
- 新增 `admin/src/features/settings/`
- 新增 `admin/src/features/audit/`
- 新增 `admin/src/features/profile/`
- 修改路由、设置二级菜单和测试

**Existing patterns**：设置页面使用真实二级路由；用户权限遵守唯一管理员与普通用户模型。

**Approach**：管理员创建用户时设置初始密码，重置密码后撤销该用户既有会话；密码只能通过系统外安全渠道交付。设置、审计和个人资料使用各自二级路由，profile/preferences 通过 U2 契约持久化，不引入角色或邀请入口。

**Test Scenarios**：AE7；管理员创建用户而无邀请；管理员重置密码后旧会话全部失效且密码不进入日志；profile/preferences 跨会话恢复；普通用户深链系统设置收到 403；cursor、草稿和页面状态矩阵均有 MSW 覆盖。

**Verification**：领域单测、MSW 集成测试、Playwright 用户管理和设置二级路由 smoke。

**Dependencies**：U5。

### U8 Web 部署主闭环与 SSE

**Goal**：完成 Web preview、confirm、日志、取消、重试和最终结果闭环。

**Requirements**：R16、R17、R18、R19、R24、R26、R27。

**Files**：

- 新增 `admin/src/features/deployments/`
- 新增 `admin/src/api/sse-client.ts`
- 新增 SSE fixtures、单元测试和 E2E 测试

**Existing patterns**：遵守 OpenAPI 部署 endpoint、SSE `Last-Event-ID`/`after` 和 `docs/runbooks/deployment-recovery.md`。

**Approach**：preview 与 confirm 分屏/分步，confirm 使用稳定幂等键和 snapshot；日志 store 按 event ID 去重并限制内存展示窗口，保留暂停跟随。日志和未知事件正文默认按纯文本转义，禁止 HTML 注入；ANSI 或链接只允许固定白名单。重连采用有上限退避，最终状态后主动结束。

**Test Scenarios**：AE3、AE4；双击 confirm 只产生一个部署；cancel/retry 处理中禁用；interrupted 提供恢复说明；401/403/409/网络断开均有明确状态；HTML、脚本、控制字符和危险 URL 日志载荷不能改变页面结构或执行；普通用户使用未授权应用的 deployment ID 访问 detail、SSE、cancel、retry 均返回 403，且不泄露元数据或日志。

**Verification**：SSE 状态机单测、mock stream 集成测试、Playwright 完整部署 smoke，不执行真实脚本。

**Dependencies**：U5、U7。

### U9 Flutter 工程、主题、路由与安全会话基线

**Goal**：创建符合移动端交互和安全要求的 Flutter 工程基线。

**Requirements**：R1、R3、R6、R7、R20、R21、R22、R27。

**Files**：

- 新增 `admin-app/pubspec.yaml`、`admin-app/lib/main.dart`
- 新增 `admin-app/lib/app/`、`admin-app/lib/routing/`、`admin-app/lib/theme/`
- 新增 `admin-app/lib/api/`、`admin-app/lib/security/`
- 新增 `admin-app/test/` 和 `admin-app/integration_test/`

**Existing patterns**：以 `ui/docs/flutter-handoff.md` 为设计权威；状态使用 Riverpod，路由使用 `go_router`，普通请求使用 Dio/CookieJar。

**Approach**：建立“概览、资源、部署、我的”四项底部导航；资源根页使用应用/节点分段控件，详情页返回对应分段。建立圆润 token、无 hover 组件和 44px 触控约束；CookieJar/CSRF 通过平台安全存储适配，认证 client 从明确的构建配置读取并注入 API 允许的 Origin，日志拦截器默认脱敏。使用 `mocktail` 和 fake secure storage 测试。

**Test Scenarios**：安全存储读写和清除；连接本地测试 API 的 setup/login/session restore 携带匹配 Origin；底部激活态明确；字体放大和窄屏不溢出；组件不存在 hover-only 行为；生成 Dart client 通过 analyze；Android Emulator 和 iOS Simulator 覆盖进程重启后的 Cookie/CSRF 恢复、登出清除和会话过期。

**Verification**：`dart format --output=none --set-exit-if-changed .`、`flutter analyze`、`flutter test`。

**Dependencies**：U3。

### U10 Flutter 主要页面与权限体验

**Goal**：完成移动端高频查看、资源管理入口和“我的”页面。

**Requirements**：R8、R15、R20、R21、R22、R24、R25、R32。

**Files**：

- 新增 `admin-app/lib/features/overview/`
- 新增 `admin-app/lib/features/resources/`
- 新增 `admin-app/lib/features/profile/`
- 新增 `admin-app/lib/features/users/`
- 新增对应 widget/provider 测试

**Existing patterns**：沿用 UI preview 的四项底部导航和固定顶部身份区。

**Approach**：概览和资源通过现有列表聚合；节点/凭证只展示必要状态并引导复杂配置到 Web；管理员可进入用户管理，普通用户不可见且深链有权限页。我的页面保留 profile/preferences/logout，不展示权限说明或版本号。

**Test Scenarios**：AE6、AE7；管理员用户管理可点击；普通用户无系统管理；列表刷新和 cursor 去重；退出登录为克制危险样式并有确认；页面状态矩阵均有 widget/provider 覆盖。

**Verification**：widget golden/语义测试、provider 测试和关键路由 integration smoke。

**Dependencies**：U9。

### U11 Flutter 部署闭环、SSE 与生命周期恢复

**Goal**：让移动端可靠完成部署预览、确认、日志跟随和恢复。

**Requirements**：R16、R17、R18、R19、R23、R24、R26、R27。

**Files**：

- 新增 `admin-app/lib/features/deployments/`
- 新增 `admin-app/lib/api/sse_client.dart`
- 新增生命周期、SSE 和 integration 测试

**Existing patterns**：复用 Web 相同的服务端契约和 event ID 语义，但采用 Flutter 生命周期和 provider 状态管理。

**Approach**：普通 API 继续走 Dio，SSE 独立 client 注入 cookie/header；provider 保存最后 event ID、事件去重集合和最终状态。后台时释放连接，前台先刷新部署再决定续传。

**Test Scenarios**：AE3、AE5；重复事件去重；后台期间部署结束；confirm 防双击；cancel/retry 失败回滚 UI；安全存储和日志无敏感值；普通用户对未授权 deployment 的 detail、SSE、cancel 和 retry 均收到 403 且无缓存泄露。

**Verification**：provider/state-machine 单测、fake SSE 集成测试、Android 模拟器 lifecycle smoke。

**Dependencies**：U9、U10。

### U12 跨端一致性、错误与 E2E 收口

**Goal**：验证两端在权限、错误、分页、恢复和可访问性上的一致行为。

**Requirements**：R8、R12、R17、R23、R24、R25、R26、R27。

**Files**：

- 修改 `admin/src/` 和 `admin-app/lib/` 的共享行为适配
- 新增 `admin/e2e/` 场景
- 新增 `admin-app/integration_test/` 场景
- 新增跨端 fixture 契约说明

**Existing patterns**：使用同一组 OpenAPI 示例和语义 fixture，但不共享框架代码。

**Approach**：建立覆盖 401/403/409/422/500、cursor、SSE 重连、草稿、权限深链和敏感信息的契约矩阵；修正行为差异。Web 关键流程必须仅用键盘完成，Modal 覆盖首焦点、焦点循环、Escape 和关闭后焦点恢复，图标按钮与状态具有可访问名称并运行 axe smoke。Flutter 在系统字体 200% 下无裁切或操作遮挡，关键控件具有 Semantics label，widget 测试断言 44 logical pixels 触控目标。

**Test Scenarios**：AE4、AE5、AE6、AE8；request ID 可复制；筛选分页不重复；离开草稿行为一致；日志大批量输入不阻塞主操作；未授权 deployment 的 detail、SSE、cancel 和 retry 在两端均不泄露内容。

**Verification**：Web Playwright smoke、Flutter integration smoke、敏感模式扫描和完整聚焦测试矩阵。

**Dependencies**：U6、U8、U11、U15。

### U13 Makefile 与本地联调 runbook

**Goal**：提供开发者可重复执行的双端开发、生成、测试和 mock 联调入口。

**Requirements**：R28、R31。

**Files**：

- 修改 `Makefile`
- 修改 `docs/runbooks/local-development.md`
- 新增或修改本地 mock/fixture 配置
- 修改 `README.md`（仅补入口链接时）

**Existing patterns**：延续现有 `make ui-serve` 使用 Python 在 8050 启动预览，以及根 Makefile 聚合模块命令。

**Approach**：增加 Web dev/check/test/build、Flutter get/check/test/build、API client generate/check 和聚合 `check` 命令；端口、环境变量、Secure Cookie 本地开关、Flutter Origin 构建配置和 mock 模式写入 runbook。命令不隐藏真实远程操作。

**Test Scenarios**：全新依赖安装后命令可运行；8050 仍直接预览最新 UI；端口占用错误清晰；默认配置不会连接真实节点。

**Verification**：逐条执行新增 Make target 的非远程路径，并按 runbook 从干净依赖状态复演。

**Dependencies**：U4、U9。

### U14 GitHub Actions、发布产物与最终复核

**Goal**：将双端检查和发布纳入现有 GitHub Actions，形成可交付闭环。

**Requirements**：R2、R27、R28、R29、R30、R31。

**Files**：

- 修改 `.github/workflows/ci.yml`
- 修改 `.github/workflows/release-artifacts.yml`
- 修改 `docs/runbooks/github-actions-release.md`
- 新增 `docs/reviews/<date>-formal-clients-and-ci.md`

**Existing patterns**：保留现有 API amd64/arm64 matrix、artifact 命名、checksum 和最小 permissions；参考既有 Flutter stable 固定版本方式。

**Approach**：PR/main 增加 Web lint/typecheck/unit/build/E2E smoke、Flutter format/analyze/test、Android Emulator 和 iOS Simulator 安全会话 smoke、生成漂移；release 增加 Web archive、Android debug APK 和未签名 release AAB 构建输入。缓存键包含 lockfile；iOS 不执行签名发布且不读取签名凭证。最终按本计划逐项复核。

**Test Scenarios**：AE9、AE10；任一生成目录漂移失败；Web/Android artifact 可解包；checksum 匹配；artifact 和日志敏感扫描无命中；API 原有产物不回归。

**Verification**：本地 action lint（若仓库已有工具）、workflow YAML 解析、与 CI 等价的本地命令、tag dry-run 构建；记录不能本地模拟的 GitHub 托管环境风险。

**Dependencies**：U12、U13。

## Verification Contract

### Verification Layers

- **V1 契约层**：OpenAPI 校验、双端生成幂等和 drift check，覆盖 R2-R4、R24-R25。
- **V2 API 层**：Rust format、clippy、单元/集成测试和 migration 验证，覆盖 R4-R8、R24、R27；包含 setup/login 的 Origin 精确匹配负向测试。
- **V3 Web 层**：lint、typecheck、Vitest、Testing Library/MSW、build、axe 和 Playwright 键盘 smoke，覆盖 R5、R7-R14、R16-R18、R24-R27。
- **V4 Flutter 层**：format、analyze、unit/widget/provider/integration 测试、Android/iOS 安全会话 smoke、200% 字体和 44px 触控断言，覆盖 R6-R8、R15-R18、R20-R27、R30、R32。
- **V5 UI 设计层**：UI preview 检查和交互测试，覆盖 R9-R15、R20-R22。
- **V6 CI/发布层**：workflow 解析、同构本地命令、artifact 解包和 checksum，覆盖 R28-R31。
- **V7 安全层**：权限负向测试、敏感值扫描、存储检查和日志审查，覆盖 R5-R8、R13-R14、R27、R30；包含 Origin 校验和未授权 deployment detail/SSE/cancel/retry 的对象级授权测试。

### Traceability Matrix

| 范围 | 实施单元 | 验证层 | 主要验收 |
| --- | --- | --- | --- |
| R1-R4 | U2-U4、U9 | V1、V2、V3、V4 | AE9 |
| R5-R8、R32 | U5、U9、U10、U12、U15 | V2、V3、V4、V7 | AE1、AE6、AE8 |
| R9-R12 | U1、U4、U6-U8、U15 | V3、V5 | AE2、AE7 |
| R13-R15 | U1、U6、U10 | V3、V4、V5、V7 | AE2、AE8 |
| R16-R19 | U8、U11、U12 | V3、V4、V7 | AE3-AE5 |
| R20-R23 | U1、U9-U12 | V4、V5 | AE5-AE7 |
| R24-R27 | U2-U12、U15 | V1-V4、V7 | AE4-AE8 |
| R28-R31 | U3、U13、U14 | V1、V6、V7 | AE9、AE10 |

### Required Commands

最终命令名以 U13 落地为准，至少提供并通过以下等价入口：

```bash
make api-check
make ui-check
make ui-test
make api-client-check
make admin-check
make admin-test
make admin-build
make admin-app-check
make admin-app-test
make check
git diff --check
```

任何涉及 SSH、远程脚本、重启、迁移共享环境或真实部署的命令均不属于本计划验证；只能使用 fixture、mock 或明确隔离的测试容器。

### Exit Criteria Per Unit

每个 U-ID 完成前必须满足：相关测试通过；`git diff --check` 通过；生成文件无非确定性变化；runbook/standard 与实际命令同步；只提交本单元相关文件；提交后 fetch、rebase `origin/main` 并推送。若实现范围超出对应 Requirements，先更新本计划并确认，不在代码中静默扩张。

## Definition of Done

- U1-U15 全部完成并可从提交历史独立解释、验证和回滚。
- `admin/` 和 `admin-app/` 可按 runbook 从干净环境安装依赖、运行、测试和构建。
- AE1-AE10 全部由自动测试或有记录的聚焦验证覆盖，未自动化项说明原因和复现步骤。
- OpenAPI 是双端唯一 API 来源，重新生成无 diff，手工制造漂移会使 CI 失败。
- 管理员可在 Web 完成 SSH 节点接入、应用/用户授权和部署主闭环；普通用户只能访问授权范围。
- Flutter 可完成登录、概览、资源查看、部署和“我的”主流程，并可靠处理后台恢复。
- Web 和 Flutter 的 401、403、409、分页、草稿、SSE 和敏感数据行为满足统一契约。
- tag release 保留 API 双架构产物，并产出 Web、Android 和 checksum；未错误宣称 iOS 可签名发布。
- CI、artifact、日志、fixture 和普通客户端存储中无受保护凭证或 secret。
- `docs/runbooks/`、`docs/standards/`、UI handoff、OpenAPI 与实际实现一致。
- 最终复核记录写入 `docs/reviews/`，不存在未处理的 P0/P1 问题或未说明的范围漂移。

## Appendix

### Deferred Follow-ups

- iOS 签名、TestFlight/App Store 发布在签名凭证、Bundle ID 和发布责任明确后单独规划。
- 多管理员、可配置角色、邀请、公开注册不在当前产品模型中；只有产品需求变化时重新 brainstorm。
- 推送通知、桌面客户端、离线部署和服务端概览聚合 API 仅在有可量化需求后评估。
- 真实节点试运行必须由用户在届时对具体节点和操作明确授权，并单独按 runbook 执行。
