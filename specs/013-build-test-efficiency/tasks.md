# 构建与检查效率优化（第一批）任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：T001–T005 已实施并验证；真实部署的日志与耗时效果待下次获授权部署后核对。全部任务待用户签收。勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-001 FR-002 在 generate-api-clients.mjs 实现输入指纹记忆化、`--force` 与离线优先的 `pub get`；逐类输入变更验证重跑，失败不写记忆。无前置依赖。
- [ ] T002 FR-003 FR-004 新增 scripts/test/run-checks.sh，拆分 deployer/app-template/external-deploy 的静态子目标，`check` 改为并行运行器并保留 `check-serial`；与串行结果对照并注入失败验证。依赖 T001。
- [ ] T003 FR-005 FR-006 为 deploy.sh、remote-build.sh 增加分阶段耗时与日志留存，并补契约测试（含降级与不含 install.env 内容）。无前置依赖。
- [ ] T004 FR-007 同步 development-guide、local-development、systemd-deployment-production 与 Makefile help，登记 013。依赖 T001–T003。
- [ ] T005 SC-001 SC-002 SC-003 SC-004 验证：api-client-check 首轮与命中、`make check` 与 `make check-serial` 对照、注入失败、deploy-production-check、docs-check、spec-kit-verify、whitespace-check；自审最终 diff 与范围，执行 converge；提交并推送。依赖 T004。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已实现 | 通过 | 不适用 | 待用户签收 | `--check` 增加输入指纹记忆化与 `--force`。完整检查实测 230 至 320 秒（随网络浮动），记忆命中 0.1 秒；逐类变更 openapi.json、openapitools.json、pubspec.lock、生成脚本、手改已提交生成文件，均不命中并重新检查（手改文件报告“内容不同”）；失败不写指纹。实测发现 `dart pub get --offline` 在缓存缺失时会空等约 2 分钟再失败，反而更慢，已放弃该做法，仅保留记忆化；耗时分解见 `API_CLIENT_TIMING=1` |
| T002 | 已实现 | 通过 | 不适用 | 待用户签收 | 新增 run-checks.sh（6 个组并行、输出按组聚合、汇总耗时与失败组）；deployer/app-template/external-deploy 拆出无 cargo 的静态子目标，原目标行为不变。预热后并行 `make check` 总墙钟 97 至 100 秒，串行 `make check-serial` 210 秒，两者都通过；注入一个坏文档链接后并行检查非零退出并指出 docs 组 |
| T003 | 已实现 | 通过（mock 契约） | 未部署 | 待用户签收 | deploy.sh 增加阶段耗时与日志留存（`.data/deploy-logs/`，保留 20 份），stdout 与 stderr 分别 tee 以保持流语义；remote-build.sh 输出 Rust 与前端各自耗时；契约测试覆盖阶段齐全、不含 install.env 内容或密钥、目录不可写降级、保留数量，并使用独立目录避免污染真实日志。失败路径（代理预检失败）仍输出耗时与日志位置。真实部署效果未验证 |
| T004 | 已实现 | 通过 | 不适用 | 待用户签收 | development-guide、local-development、systemd-deployment-production 与 Makefile help 已同步；specs/README 与存储登记已登记 013 |
| T005 | 已完成 | docs-check、spec-kit-verify（21 项）、whitespace-check、deploy-production-check、并行与串行 make check 通过 | 不适用 | 待用户签收 | 自审 diff：10 个修改文件加 run-checks.sh 与 013 产物，未触及产品代码、协议、migration、OpenAPI；SC-002 目标 120 秒内达成（并行 97 至 100 秒） |
