# 构建代理可用性检测实施方案

规格：[spec.md](spec.md)。状态：方案就绪；执行证据见 [tasks.md](tasks.md)。

## Technical Context

控制面 API（Rust/axum，SQLite）以 systemd 服务直接运行在构建机 qfy-test2 上，可直接访问构建代理 `127.0.0.1:17890`；`reqwest`（rustls）已是 `api` 的依赖，无需新增 crate。后台任务沿用现有 `tokio::spawn` + `watch` shutdown 的 worker 模式（见 `api/src/main.rs` 的 `run_session_purge_worker`）。设置接口沿用 `api/src/settings/mod.rs` 的管理员鉴权、CSRF 与 `Cache-Control: no-store` 约定；前端沿用 `admin/src/features/settings` 的 React Query 与 OpenAPI 生成客户端。

部署脚本预检（`deploy/production/deploy.sh`）已经经代理访问 `https://registry-1.docker.io/v2/` 并以 401/200 判定可用；面板使用同一目标与判定。

状态只保存在内存：探测结果是瞬时运行状态，进程重启后立即重测即可，不新增表和 migration，避免触发 migration 门禁。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md) 与[项目原则](../../.specify/memory/constitution.md)：只修改 deploy-go 自身；不触碰被部署业务应用；不接入 Agent 线协议；响应与日志不含代理配置、节点或密钥；真实代理与线上展示的验证需要新的部署授权，实现阶段只做本地与假代理验证。

## 实现方案与文件范围

1. `api/src/config.rs`：读取可选 `DEPLOY_GO_BUILD_PROXY_URL`，校验只含 `http://`、回环主机和端口，不允许用户名密码、路径、查询；非法值使启动失败，错误信息不回显原值。
2. `api/src/build_proxy.rs`（新增模块）：
   - 探测器：用 `reqwest::Client::builder().no_proxy().proxy(Proxy::all(<配置地址>))` 构建客户端（不读全局代理环境变量），请求固定目标 `https://registry-1.docker.io/v2/`，超时 15 秒，不跟随重定向，响应 401/200 为成功，失败映射为有限的原因枚举。
   - 状态机：`Arc<Mutex<…>>` 保存 `status`、`last_checked_at`、`last_success_at`、`last_latency_ms`、`consecutive_failures`、`last_failure_reason`；连续失败 ≥2 为 `failing`，成功即 `ok`；首次探测前为 `checking`；未配置为 `unconfigured`。状态机与探测器分离，便于用假探测器做单元测试。
   - `run_worker`：启动即探测一次，之后每 5 分钟一次，随 shutdown 信号退出；手动重测与 worker 共用一把 `tokio::sync::Mutex` 保证并发度 1，并带 10 秒冷却。
   - 路由：`GET /api/v1/build-proxy/status` 与 `POST /api/v1/build-proxy/check`，均 `require_administrator`，POST 另校验 CSRF，响应加 `Cache-Control: no-store`；响应体只含状态、时间、耗时、连续失败次数和原因枚举。
3. `api/src/lib.rs`、`api/src/main.rs`：`AppState` 增加可选的构建代理句柄（`with_build_proxy`），在 `main` 中按配置创建并 `tokio::spawn` worker，优雅停止时一并等待；路径与 schema 登记进 `ApiDoc`，router 注册。
4. OpenAPI 与客户端：`make api-openapi`、`make api-client-generate` 生成 `api/openapi/openapi.json`、`admin/src/api/generated/`、`admin-app/lib/api/generated/`，不手改；不进入对外 API（`external.json` 不含此路径）。
5. 前端 `admin/src/features/build-proxy/`（新增）：`api.ts`、`BuildProxyCard.tsx`（设置页卡片）、`BuildProxyBanner.tsx`（失效横幅，仅管理员请求接口，`failing` 才渲染，轮询间隔 60 秒）；`SettingsPage.tsx` 引入卡片；`OverviewPage.tsx` 与 `NewDeploymentPage.tsx` 引入横幅；横幅文案指向部署手册的“构建代理”一节。状态用文字呈现，沿用现有 `status-badge` 样式。
6. 部署与文档：`deploy/production/install.sh` 将 `DEPLOY_GO_BUILD_PROXY_URL` 加入白名单、保留清单和环境文件生成，仅在显式提供，或构建机存在 `mihomo-build-proxy.service` 时写入默认 `http://127.0.0.1:17890`；`deploy.sh` 把已有的 `DEPLOY_BUILD_PROXY_URL` 在构建机与部署机相同时透传给安装器；`test-install-contract.sh` 增加对应契约；`docs/runbooks/systemd-deployment-production.md` 同步配置项、面板核对和更换节点后的验证步骤；`specs/README.md` 与存储登记登记 011。

## 阶段与依赖

1. 配置与探测核心（`config.rs`、`build_proxy.rs` 的探测器和状态机）及其单元测试，无前置依赖。
2. 路由、AppState、worker 接线和接口测试，依赖阶段 1。
3. OpenAPI 与客户端生成，依赖阶段 2。
4. 前端卡片、横幅与测试，依赖阶段 3 的生成客户端。
5. 安装器、部署脚本契约与手册同步，依赖阶段 1 的配置项名称，可与阶段 2–4 并行，但写入者串行集成。
6. 全量验证、自审最终 diff、converge，提交并推送；真实部署另需授权。

## 验证计划

- Rust：`cargo test -p deploy-go-api build_proxy` 与接口集成测试（假代理用本地 TCP 服务模拟：成功 401、超时、拒绝连接、非预期状态码、非回环地址配置被拒绝）；`make api-check`（含 fmt、clippy、全 workspace）。
- 接口：非管理员 403、未登录 401、POST 缺 CSRF 被拒、冷却内重复调用不重复探测、响应不含地址或节点信息。
- 前端：`make admin-check`；组件测试覆盖 `ok`、`failing`、`checking`、`unconfigured`、手动检测、横幅出现与消失、非管理员不请求。
- 生成产物：`make api-openapi`、`make api-client-generate` 后 `make api-client-check`、`make client-sensitive-check`。
- 部署契约与文档：`make deploy-production-check`、`make docs-check`、`make spec-kit-verify`、`make whitespace-check`。
- 不涉及 Linux 专项代码和 migration，不运行相关 Docker 专项；真实代理与线上展示在获得部署授权后另行验证并追加证据。

## 风险与回退

- 出站请求只允许回环代理和固定目标，避免 SSRF；探测失败只影响面板展示，不影响部署流程。
- 探测任务以独立任务运行并带超时，崩溃或失败不影响其他 worker；功能为可选配置，缺省不启用，回退只需移除环境变量或回滚提交。
- 发布涉及 API 与前端，升版与版本联动遵守 AGENTS 约定：本功能不顺手改版本号，是否升版由发布时决定。
