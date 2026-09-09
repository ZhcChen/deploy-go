# CLI 命令参考

以下命令中的 `<cli>` 表示当前 Skill 目录内的 `scripts/deploy-go-deployer`，Windows 使用 `scripts\deploy-go-deployer.exe`。

所有命令都支持 `--api-base`、`--api-key` 与 `--json`。业务成功结果默认输出易读文本；加 `--json` 输出原始 JSON。

## 环境与自检

```text
DEPLOY_GO_API_BASE_URL=https://deploy.quanxinfu.com   # 可选，默认正式控制面
DEPLOY_GO_API_KEY=dgx_...                             # 必填，管理端创建的外部 API Key

<cli> --version
<cli> openapi [--output <PATH>]
```

不得把 API Key 放进命令行参数、普通文件、日志或错误文本。`openapi` 只输出内置契约，不访问网络。

## 应用查询

```text
<cli> list-apps
<cli> show-app <APPLICATION_ID>
```

`list-apps` 只返回当前 Key 绑定且处于启用状态的应用。`show-app` 返回应用元数据、当前 `version`、参数 Schema、部署后验证配置和可用目标；发起写操作前先读取。

## 编辑应用

```text
<cli> update-app <APPLICATION_ID> [--version <N>] \
  [--name <NAME>] [--slug <SLUG>] [--description <TEXT>] \
  [--environment dev|test|staging] \
  [--app-type <TYPE>] [--type-version <VERSION>] \
  [--tag <TAG>]... [--clear-tags] \
  [--parameter-schema <JSON> | --parameter-schema-file <PATH>] \
  [--verification-config <JSON> | --verification-config-file <PATH>]
```

- 只能编辑非正式环境应用；服务端会拒绝正式环境应用和把环境改为 `prod` 的请求。
- `--version` 省略时 CLI 先执行 `show-app` 自动获取；并发修改返回 409 时重新读取并确认。
- `--tag` 可重复传入；`--clear-tags` 清空全部标签，两者不能同时使用。
- JSON 参数必须是 object；文件内容同样按 JSON 解析。
- 至少提供一个要修改的字段，否则 CLI 直接报错。

## 部署

```text
<cli> deploy <APPLICATION_ID> \
  [--target-id <TARGET_ID>] \
  [--release-version <VERSION>] \
  [--release-strategy automatic|manual] \
  [--parameter KEY=VALUE]... \
  [--idempotency-key <KEY>]
```

- 省略 `--target-id` 时部署应用全部启用目标。
- `--parameter` 可重复传入；两阶段部署通常需要 `release-version` 和 `modules` 参数。
- 正式环境部署会被服务端拒绝，错误码为 `external_production_deployment_forbidden`。
- 幂等键省略时自动生成；需要可重放时显式传入。

## 状态与取消

```text
<cli> status <DEPLOYMENT_ID>
<cli> cancel <DEPLOYMENT_ID>
```

`status` 返回部署状态、阶段和各目标运行状态。`cancel` 请求取消部署，不会删除部署记录。
