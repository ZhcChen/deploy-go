# 工作流

## 发起部署

1. 用户给出应用名称或 ID 时，先执行 `list-apps`；多个候选时让用户确认，不猜测。
2. 执行 `show-app <APPLICATION_ID>`，确认环境、可用目标和 `execution_mode`。
3. 需要指定目标时使用 `--target-id`；否则确认是否要部署全部启用目标。
4. 与用户确认发布版本和关键参数后执行 `deploy`。
5. 记录返回的 `id`，立即执行 `status <DEPLOYMENT_ID>`，报告实际状态和阶段。

示例：

```text
<cli> list-apps
<cli> show-app app_01KZ...
<cli> deploy app_01KZ... --target-id target_01KZ... --release-version 1.2.0 --parameter modules=api,admin
<cli> status dep_01KZ...
```

## 创建应用并接入部署

适用于外部系统需要新建一个非正式环境应用并完成首次部署的场景。

1. 与用户确认应用名称、slug 和环境（只能是 `dev` / `test` / `staging`），以及应用类型与版本。
2. 执行 `create-app`，记录返回的应用 ID；正式环境应用一律停止，让用户改走管理面。
3. 用返回的应用 ID 继续接入：`register-env-file` 登记 Env、`create-target` 配置部署目标、按需 `set-workspace-source`。
4. 配置完成后执行 `show-app` 确认环境、目标与 `version`。
5. 发起部署并立即执行 `status` 报告实际状态。

示例：

```text
<cli> create-app --name "Clickhouse 测试" --slug clickhouse-test --environment test --tag clickhouse
<cli> list-targets app_01KZ...
<cli> create-target app_01KZ... --node-id node_01KZ... --script-path /srv/apps/deploy.sh --timeout-seconds 600
<cli> deploy app_01KZ... --release-version 1.0.0
<cli> status dep_01KZ...
```

边界：

- 只创建非正式环境应用；`prod` 返回 403，正式环境只能由管理员在管理面手动创建。
- slug 全局唯一，冲突返回 409 `application_slug_exists`，与用户确认后更换 slug，不要反复重试同一请求。
- 不支持从模板创建：需要模板预置配置的应用由管理面创建后再接手接入。

## 编辑应用

1. 先执行 `show-app <APPLICATION_ID>`，确认应用环境和当前 `version`。
2. 只能编辑非正式环境应用；应用环境为 `prod` 时停止，不尝试通过其他接口规避；提示用户改走管理面。
3. 与用户确认要修改的字段；只传需要变更的字段，不要构造完整覆盖请求。
4. 执行 `update-app`。省略 `--version` 时 CLI 自动读取当前版本。
5. 服务端返回 409 `resource_version_conflict` 时重新执行 `show-app`，向用户说明并发修改并等待确认，不自动重试。

示例：

```text
<cli> show-app app_01KZ...
<cli> update-app app_01KZ... --description "测试环境卡券系统" --tag voucher --tag test
<cli> update-app app_01KZ... --parameter-schema-file ./parameter-schema.json
```

## 配置应用接入（Env、部署目标、工作区来源）

适用于给已有非正式环境应用补齐接入配置，或接入不依赖 Git 的两阶段部署应用。

1. 执行 `show-app <APPLICATION_ID>` 确认应用环境不是 `prod`，并记录当前 `version`。
2. 按需执行 `list-env-files` / `list-targets` / `show-workspace-source`，先了解现状再写入。
3. 登记 Env：把内容写入本地文件后执行 `register-env-file`；已有文件用 `update-env-file`。
4. 配置部署来源：Git 来源需在管理面维护；固定工作区来源用 `set-workspace-source`。
5. 配置部署目标：用 `create-target` 新增、`update-target` 修改、`set-target-status` 启停。
6. 每次写入后重新执行对应的只读命令确认结果，并报告实际返回的版本与状态。
7. 配置变更后提示用户：部署预览可能已失效，需要重新生成预览再部署。

示例：

```text
<cli> show-app app_01KZ...
<cli> list-targets app_01KZ...
<cli> register-env-file app_01KZ... --file-name api.env --module api --content-file ./api.env
<cli> set-workspace-source app_01KZ... --build-agent-id agent_01KZ... --workspace-path /srv/workspace
<cli> create-target app_01KZ... --node-id node_01KZ... --script-path /srv/apps/deploy.sh --timeout-seconds 600
```

边界：

- Env 明文只能由用户提供，不允许回读、猜测或把明文写入日志。
- 正式环境应用的全部配置写操作都返回 403，停止并提示改走管理面。
- 删除 Env 文件前必须与用户确认文件名；被镜像目标引用时服务端会拒绝。

## 取消部署

1. 先执行 `status <DEPLOYMENT_ID>`，确认部署仍在可取消状态。
2. 与用户确认部署 ID 和取消原因。
3. 执行 `cancel <DEPLOYMENT_ID>`，再执行 `status` 报告最终状态。

## 只读分析

- 分析应用与目标时只使用 `list-apps`、`show-app`、`status`。
- 不读取或推断 Env、密钥、SSH 凭证、节点连接信息或应用脚本内容。
- 需要的信息不在 CLI 输出中时，说明能力边界，不构造额外 HTTP 请求。

## 失败处理

- 401：API Key 缺失、无效或已吊销；停止并让用户检查配置。
- 403：正式环境限制或权限不足；停止，不通过其他目标重试。
- 404：应用、目标或部署不存在，或当前 Key 无权访问；先重新查询，不猜测。
- 409：版本冲突、slug 冲突或部署状态冲突；重新读取后让用户确认。
- 422：参数 Schema、验证配置或字段格式不合法；根据服务端 `message` 修正，不盲目重试。
