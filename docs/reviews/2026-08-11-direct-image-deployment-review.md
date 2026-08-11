# 镜像直连部署 U7 高风险复核

## 结论

**U1-U6 已完成并提交；U7 复核发现并修复两个高风险问题，聚焦验证全部通过。**
U8 全量门禁已执行并通过，执行记录见文末。

## 复核范围

- 镜像直连部署的 `image_spec` 契约：模板必选 Env 文件、镜像安全字符、
  端口范围、Env 文件名白名单与唯一性。
- 镜像目标引用 Env 文件的删除/同步边界：删除 Env 后不得让 release 在执行
  阶段才发现缺少 `compose.env` / `redis.env` / `postgres.env`。
- dispatcher Env 门禁：Env 缺失或已删除时不得创建 release 任务，executor
  必须零调用。
- Web/模板向导与共享模板目录同步，以及文档和测试夹具的一致性。

## 发现与修复

### 1. 模板必选 Env 文件未在创建目标时强制

原 `validate_image_spec` 只要求 1-16 个 Env 文件，但 release 脚本固定要求
`compose.env` + `redis.env` / `postgres.env`；漏选会在执行阶段才失败。

已修复：

- `container-template` 新增 `required_env_files`，`validate_image_spec`
  强制模板必选文件存在；新增漏选失败、extra 文件成功测试。
- Agent/API/Admin/Web 测试夹具全部同步为 `compose.env` +
  `redis.env` / `postgres.env`。
- Admin 目标编辑器与模板向导新增“模板必选”标注和提交前校验，E2E mock
  同步为两个 Env 文件。

### 2. 镜像目标引用的 Env 文件可被删除

原删除 Env API 未检查镜像目标引用，可把 `compose.env` 等文件 tombstone；
dispatcher 随后可能把 release 任务发给 Agent，release 脚本在执行阶段
才发现缺少文件。

已修复：

- `delete_env` 在事务内检查该应用全部 `execution_mode='image' AND
  status='active'` 目标，`image_spec_json` 引用该文件时返回 HTTP 409
  `env_file_referenced_by_image_target`，并附 `target_count` 与
  `target_ids`；目标停用或移除引用后删除成功。
- dispatcher `load_release_env_gate` 对镜像模式逐一核对 `env_files`：
  文件不存在或已删除时返回 Env 门禁未就绪，不创建 release 任务。
- 新增 API 测试：引用必选文件删除被拒、解除引用后删除成功、停用目标后
  删除成功；新增 dispatcher 测试：缺少必选文件不创建 release，补充文件后
  创建，文件删除后再次等待。

## 已验证

- `cargo test -p deploy-go-api --test application_envs_api --test
  deployment_targets_api --test two_stage_deployment --test agent_dispatcher`
  ：40 项通过。
- `cargo test -p deploy-go-container-template -p deploy-go-agent --test
  image_release`：模板单测与 Agent 镜像 release 集成测试通过。
- `make app-template-check`：模板契约与 container-template 单测通过。
- `npm run check --workspace deploy-go-admin`：123 项 Vitest、lint、
  typecheck 与 build 通过。
- `npm run test:e2e --workspace deploy-go-admin --
  application-configuration.spec.ts`：3 项通过。
- `cargo fmt --all --check` 与 `cargo clippy -p deploy-go-api -p
  deploy-go-container-template -p deploy-go-agent --all-targets -- -D
  warnings` 通过。

## 复核无问题项

- executor 仍只执行固定 `make --no-print-directory deploy-go-release`，
  不接受 `command`、`executable`、`args`、Make target 或 env map。
- `image_spec` 镜像引用安全字符、端口 1-65535、Env 文件唯一白名单与
  模板必选文件均有服务端校验，Web 仅做同规则预检。
- 镜像模式无业务 Git prepare，主控平台制品与 Agent 固定 checkout 摘要
  校验沿用既有 release authorization 链路。
- 旧 `script` / `two_stage` 与 launcher 兼容路径未改动，不自动降级。
- Env 明文不进入列表、日志、错误与审计；删除门禁返回的是目标 ID 而非
  Env 内容。

## U8 全量门禁执行记录

- `make privileged-release-check` 完整通过：Linux cgroup v2 4 项、runner 身份
  2 项，以及 Agent/API/OpenAPI/client/DeploymentFlow 聚焦门禁。
- `make check` 完整通过：全仓 Rust 单测/集成测试、OpenAPI 契约、Agent/installer/
  launcher/模板契约、deployer 契约、Admin lint/typecheck/123 项 Vitest/build、
  Flutter format/analyze/51 项 test、客户端敏感扫描 159 文件、`git diff --check`。
- 复核过程中发现并修复的 deployer 契约外部环境干扰与 CSRF 敏感扫描误报，
  已分别提交并随本轮复核推送。
- 本复核未连接、未修改任何真实节点或 WSL 测试节点。
