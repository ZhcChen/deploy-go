# 构建代理可用性检测任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：规格与方案就绪，尚未实施。勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-001 在 api/src/config.rs 增加可选 `DEPLOY_GO_BUILD_PROXY_URL` 解析与校验（仅 http、回环主机、端口，无凭证/路径/查询，错误不回显原值），含单元测试。无前置依赖。
- [ ] T002 FR-002 FR-003 FR-004 FR-011 新增 api/src/build_proxy.rs 的探测器与状态机（固定目标、15 秒超时、不读全局代理、不跟随重定向、连续失败阈值、原因枚举、脱敏日志），用假探测器与本地假代理覆盖成功、超时、拒绝连接、非预期状态码、恢复与未配置。依赖 T001。
- [ ] T003 FR-005 FR-006 实现 `GET /api/v1/build-proxy/status` 与 `POST /api/v1/build-proxy/check`：管理员鉴权、CSRF、`no-store`、10 秒冷却和并发度 1，响应不含地址或节点信息；补接口测试。依赖 T002。
- [ ] T004 FR-002 FR-003 在 api/src/lib.rs、api/src/main.rs 接入 `with_build_proxy`、worker 启动（启动即探测、5 分钟周期）与优雅停止，登记 router 与 `ApiDoc`。依赖 T003。
- [ ] T005 FR-009 运行 `make api-openapi`、`make api-client-generate` 生成 OpenAPI 与 admin/admin-app 客户端；确认对外 `external.json` 不含新路径，不手改生成文件。依赖 T004。
- [ ] T006 FR-007 FR-008 新增 admin/src/features/build-proxy（api、BuildProxyCard、BuildProxyBanner），接入设置页、概览页和发起应用部署页；覆盖四种状态、手动检测、横幅出现与消失、非管理员不请求的组件测试。依赖 T005。
- [ ] T007 FR-010 更新 deploy/production/install.sh、deploy.sh 与 test-install-contract.sh（环境变量白名单、保留清单、透传条件、契约断言），并同步 docs/runbooks/systemd-deployment-production.md 与 specs/README.md、存储登记对 011 的登记。依赖 T001 的配置项名称。
- [ ] T008 FR-001 至 FR-011 验证：运行 `make api-check`、`make admin-check`、`make api-client-check`、`make client-sensitive-check`、`make deploy-production-check`、`make docs-check`、`make spec-kit-verify`、`make whitespace-check`，分别记录通过、失败与未验证项。依赖 T001–T007。
- [ ] T009 SC-004 自审最终 diff（尤其出站请求限制、响应与日志脱敏、权限与 CSRF），运行 converge；按小闭环提交，fetch/rebase/push main。依赖 T008。
- [ ] T010 SC-002 SC-005 上线后真实验证：需要用户明确授权部署正式环境；部署后核对面板状态为“可用”，停止再恢复 `mihomo-build-proxy` 服务观察状态变化与横幅（停止代理属于对正式环境的操作，需单独确认）。依赖 T009 与用户授权。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 未开始 | 未执行 | 不适用 | 待用户签收 | - |
| T002 | 未开始 | 未执行 | 不适用 | 待用户签收 | - |
| T003 | 未开始 | 未执行 | 不适用 | 待用户签收 | - |
| T004 | 未开始 | 未执行 | 不适用 | 待用户签收 | - |
| T005 | 未开始 | 未执行 | 不适用 | 待用户签收 | - |
| T006 | 未开始 | 未执行 | 不适用 | 待用户签收 | - |
| T007 | 未开始 | 未执行 | 不适用 | 待用户签收 | - |
| T008 | 未开始 | 未执行 | 不适用 | 待用户签收 | - |
| T009 | 未开始 | 未执行 | 不适用 | 待用户签收 | - |
| T010 | 未开始 | 未执行 | 未部署 | 待用户签收 | - |
