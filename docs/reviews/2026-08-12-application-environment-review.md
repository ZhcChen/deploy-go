# 应用环境标识与部署同步复核

## 结论

**U1-U5 已完成并推送，`make check` 全量门禁通过。** 应用环境
`dev/test/staging/prod` 已作为 `DEPLOY_ENVIRONMENT` 的唯一权威来源，
部署目标环境只读继承并跟随应用环境；历史数据通过新增 migration 0021
安全回填。本轮未连接、未修改任何真实节点或部署目标。

## 复核范围

- migration 0021：应用环境字段、按节点 Agent 环境回填、目标环境跟随应用
  环境，以及历史多环境冲突时不阻塞迁移。
- API：应用 create/update/list/show 的环境必填与枚举校验；应用环境变更时
  在同一事务同步全部部署目标并递增版本；目标 create/update 不再接受覆盖
  环境。
- External API：应用列表/详情仅新增 `environment`，不暴露 Env 内容。
- Admin/Flutter/OpenAPI/双端 client：应用环境展示、编辑下拉、模板向导默认
  `prod`，契约产物同步。
- 部署链路：dispatcher 按目标环境映射 `Environment::Test` 等，业务脚本收到
  正确的 `DEPLOY_ENVIRONMENT`。

## 实现要点

- `applications.environment` 使用与 Agent 相同的四枚举 CHECK；回填优先取
  目标节点未吊销、未归档且最近活跃的 Agent 环境。
- 目标环境回填仅对“同应用同节点唯一目标”或已与环境一致的目标执行，历史
  冲突组合保持原值，避免 `UNIQUE(application_id, environment, node_id)`
  失败。
- 应用环境变更通过 `UPDATE ... WHERE application_id=?` 同步目标，唯一键
  冲突返回明确 409；审计记录 `deployment_target.environment.sync`。
- 目标 API 的 `environment` 保留为只读响应字段；删除
  `TARGET_ENVIRONMENT_COMPAT_VALUE` 后，目标创建/更新从应用读取环境。
- Agent 控制协议、executor 固定命令、授权协议均未改动；旧 launcher 与
  低权限 release 兼容路径不受影响。

## 已验证

- `cargo test --workspace`：全仓通过；其中 migrations 11 项、applications
  2 项、deployment_targets 8 项、agent_dispatcher 14 项、two_stage_deployment
  14 项、external_api 6 项、external_openapi_contract 4 项全部通过。
- `make api-openapi-check` / `make api-client-check`：OpenAPI 与管理端
  client 无漂移。
- `make privileged-release-check`：此前完整通过（含 Linux cgroup v2 容器
  测试、privileged bridge、recovery 等），本轮应用环境改动未触碰特权
  executor 链路。
- `make check`：完整通过，覆盖 Rust fmt/clippy/test、Agent/installer/
  launcher/模板契约、deployer 与 production 部署安全契约、Admin 123 项
  Vitest、Flutter format/analyze/51 项 test、客户端敏感扫描 159 文件。
- `git diff --check`：无空白错误。

## 契约与安全结论

- 对外 OpenAPI 只增加应用环境字段，仍仅暴露应用列表/详情与部署操作，
  不读取、不返回 Env。
- 目标环境不允许按目标覆盖，部署任务使用的环境来自应用，避免测试应用误用
  `prod` profile。
- 历史 snapshot 保持冻结，应用环境变更只影响后续部署；重试仍复用原
  commit SHA 与 snapshot。
- migration 已提交且不可修改；后续如需调整环境同步规则必须新增更高版本
  migration。

## 上线前待用户操作

- `qfy-voucher-hub-testing` 当前目标环境仍为兼容值 `prod`，正式环境部署后
  需在管理端将该应用环境改为「测试环境（test）」。该操作属于正式控制面
  变更，需用户明确授权后执行，本复核未代为操作。

## 执行记录

- `make check`：2026-08-12 完整执行，退出码 0。
- 本轮新增提交：`09b955d`、`11acb1f`、`7467d69`，以及本文档与计划、
  runbook、标准文档同步提交。
