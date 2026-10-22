# 工作流

## 发起部署

1. 用户给出应用名称或 ID 时，先执行 `list-apps`；多个候选时让用户确认，不猜测。
2. 执行 `show-app <APPLICATION_ID>`，确认环境、可用目标和 `execution_mode`；`two_stage` 核对 `sources.git.deployment_branch` 与 `status=verified`，`two_stage_script` 核对 `sources.workspace`。来源缺失、未验证或配置分支与用户预期不同，停止写操作并报告，不用发布版本推断分支。
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
3. 若用户需要核实指定 Env 键，执行 inspect-env-file <APPLICATION_ID> <ENV_FILE_ID> --key <KEY>；只报告存在性和字节长度，不尝试获取或推断值。
4. 登记 Env：把内容写入本地文件后执行 `register-env-file`；已有文件用 `update-env-file`。
5. 配置部署来源：Git 来源用 `set-git-source`（见下节“配置 Git 来源并设置部署分支”）；固定工作区来源用 `set-workspace-source`。
6. 配置部署目标：用 `create-target` 新增、`update-target` 修改、`set-target-status` 启停。
7. 每次写入后重新执行对应的只读命令确认结果，并报告实际返回的版本与状态。
8. 配置变更后提示用户：部署预览可能已失效，需要重新生成预览再部署。

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
- inspect-env-file 仅适用于非生产环境和用户明确指定的键；只汇报存在性及 UTF-8 字节长度，不推断密钥值。
- 正式环境应用的全部配置写操作都返回 403，停止并提示改走管理面。
- 删除 Env 文件前必须与用户确认文件名；被镜像目标引用时服务端会拒绝。

## 配置 Git 来源并设置部署分支

适用于非正式环境应用接入 Git 两阶段部署，或变更仓库、构建节点、部署分支。

1. 执行 `show-app` 确认应用环境不是 `prod`；`prod` 一律停止，提示改走管理面。
2. 执行 `show-git-source` 了解现状（未配置会返回 404，属于正常）。
3. 执行 `list-build-agents`，选择 `ready=true` 的 Agent；列表为空或没有可用 Agent 时停止，让用户联系管理员授权节点或恢复节点，不要尝试其他节点。
4. 需要认证时执行 `list-git-credentials`，让用户确认使用哪条凭证；不要询问或接收密钥内容。
5. 与用户确认仓库地址、构建 Agent、凭证和检出方式（full 或 sparse 路径）后执行 `set-git-source`。
6. 执行 `refresh-git-branches --wait`，把返回的分支列表展示给用户，由用户选择部署分支，不自行挑选。
7. 执行 `set-git-branch --branch <NAME>`，再执行 `show-git-source` 确认 `status=verified` 与 `deployment_branch`。
8. 之后才可以 `deploy`；部署响应的 `deployment_branch` 与 `resolved_commit_sha` 才是该次部署实际使用的分支和提交。

示例：

```text
<cli> list-build-agents
<cli> list-git-credentials
<cli> set-git-source app_01KZ... --repository-url git@git.example.com:team/app.git --build-agent-id agent_01KZ... --git-credential-id gitcred_01KZ...
<cli> refresh-git-branches app_01KZ... --wait
<cli> set-git-branch app_01KZ... --branch develop
<cli> show-git-source app_01KZ...
```

边界：

- 变更仓库地址、凭证或构建 Agent 会清空已验证的部署分支，需要重新发现并设置分支。
- 发现失败时报告原因码（例如 `git_authentication_failed`、`git_repository_unreachable`），不要读取节点日志或猜测凭证问题；让用户检查凭证和仓库授权后再重试。
- 403 `external_node_not_allowed` 表示该 Key 未被授权使用此节点，停止并让用户联系管理员，不要更换节点绕过。
- 不要重复提交同一个写请求；409 先重新读取现状，由用户确认后再继续。

## 取消部署

1. 先执行 `status <DEPLOYMENT_ID>`，确认部署仍在可取消状态。
2. 与用户确认部署 ID 和取消原因。
3. 执行 `cancel <DEPLOYMENT_ID>`，再执行 `status` 报告最终状态。

## 只读分析

- 分析应用与目标时只使用 `list-apps`、`show-app`、`status`。
- 核实非生产 Env 配置时，只使用 inspect-env-file 检查用户指定的键。
- 不读取或推断 Env、密钥、SSH 凭证、节点连接信息或应用脚本内容。
- 需要的信息不在 CLI 输出中时，说明能力边界，不构造额外 HTTP 请求。

## 失败处理

- 401：API Key 缺失、无效或已吊销；停止并让用户检查配置。
- 403：正式环境限制或权限不足；停止，不通过其他目标重试。
- 404：应用、目标或部署不存在，或当前 Key 无权访问；先重新查询，不猜测。
- 409：版本冲突、slug 冲突或部署状态冲突；重新读取后让用户确认。
- 422：参数 Schema、验证配置或字段格式不合法；根据服务端 `message` 修正，不盲目重试。
