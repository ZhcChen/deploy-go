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
