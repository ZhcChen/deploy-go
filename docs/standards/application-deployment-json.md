---
date: 2026-08-28
topic: application-deployment-json
status: accepted
schema_version: 1
---

# 应用部署契约 JSON 使用说明

## 目标

本文说明管理端应用详情中的两个 JSON 配置：

- 参数 JSON Schema：定义一次部署允许填写哪些参数。
- 部署后验证配置：定义部署完成后的验证方式。

两个 JSON 都是应用级配置，同一应用的所有部署目标读取同一份生效值，
不按目标重复维护。它们会进入部署预览和不可变 snapshot，保存后必须重新
生成预览，旧预览不能继续确认。

JSON 编辑入口在「应用详情 → 部署契约」，也可以在「从模板创建应用」向导中
按模板预填后修改。JSON 不支持注释和尾逗号。

## 1. 参数 JSON Schema

### 1.1 用途

参数 JSON Schema 控制部署页面的参数表单，也决定业务脚本收到的参数。
平台按参数名排序，转换为命令行参数：

- 字符串、整数、数字：`--name value`
- 布尔 `true`：`--name`
- 布尔 `false`：不传递
- `null`、数组、对象：拒绝

两阶段部署会隐藏 `release-version` 和 `modules`：

- `release-version` 由平台自动生成并复用预览中的值。
- `modules` 使用多选框，调用时以逗号分隔字符串传给业务脚本。

### 1.2 完整示例

```json
{
  "type": "object",
  "properties": {
    "release-version": {
      "type": "string",
      "maxLength": 32
    },
    "modules": {
      "type": "string",
      "maxLength": 512,
      "x-options": ["worker", "api", "admin"],
      "x-default-selected": ["worker", "api"]
    },
    "region": {
      "type": "string",
      "enum": ["cn", "sg"]
    },
    "dry-run": {
      "type": "boolean"
    }
  },
  "required": ["release-version", "modules"],
  "additionalProperties": false
}
```

### 1.3 顶层约束

| 字段 | 要求 |
| --- | --- |
| `type` | 必须为 `object` |
| `properties` | 必须存在，最多 50 个字段 |
| `required` | 必填字段列表 |
| `additionalProperties` | 必须为 `false` |

顶层只允许以上四个字段，未知字段会被拒绝。

### 1.4 字段名约束

- 长度 1 到 64。
- 只能使用小写字母、数字和 `-`。
- 不能以 `-` 开头或结尾。
- 实际命令行参数名称会自动使用 `--name`，编写 Schema 时不要写前缀。

### 1.5 字段 Schema 约束

| 字段 | 适用类型 | 说明 |
| --- | --- | --- |
| `type` | 全部 | 仅允许 `string`、`integer`、`number`、`boolean` |
| `enum` | 单值字段 | 前端显示为单选 |
| `minimum` | 数字 | JSON Schema 最小值 |
| `maximum` | 数字 | JSON Schema 最大值 |
| `minLength` | 字符串 | JSON Schema 最小长度 |
| `maxLength` | 字符串 | JSON Schema 最大长度 |
| `x-options` | 逗号分隔多值字段 | 声明允许值，1 到 32 个非空字符串；管理端目前仅对 `modules` 渲染多选框 |
| `x-default-selected` | 仅 `modules` | 设置默认选中模块 |

字段 Schema 只允许上述键；`title`、`description` 等扩展键目前不保存。

### 1.6 模块默认选择

`modules` 可以只声明 `x-options`；需要设置默认选中项时必须同时声明
`x-options` 和 `x-default-selected`：

```json
{
  "type": "string",
  "maxLength": 512,
  "x-options": ["worker", "api", "admin"],
  "x-default-selected": ["worker", "api"]
}
```

行为：

- 不配置 `x-default-selected`：默认全选。
- 配置模块子集：默认选中配置的模块，显示顺序仍按 `x-options`。
- 配置空数组 `[]`：默认不选，用户必须手动选择后才能生成预览。
- 配置项必须是 `x-options` 中的字符串，且不能重复。

## 2. 部署后验证配置

### 2.1 允许的类型

验证配置只允许 `http`、`tcp`、`command` 三种类型。字段必须完整，不能包含
未知字段，也不能把任意 shell 文本作为命令。

### 2.2 HTTP 验证

```json
{
  "type": "http",
  "path": "/healthz",
  "expected_status": 200,
  "timeout_ms": 5000
}
```

| 字段 | 要求 |
| --- | --- |
| `type` | 固定 `http` |
| `path` | 必须为 `/` 开头的绝对 URL 路径，不能包含控制字符 |
| `expected_status` | 100 到 599 |
| `timeout_ms` | 100 到 60000 |

### 2.3 TCP 验证

```json
{
  "type": "tcp",
  "port": 6379,
  "timeout_ms": 5000
}
```

| 字段 | 要求 |
| --- | --- |
| `type` | 固定 `tcp` |
| `port` | 1 到 65535 |
| `timeout_ms` | 100 到 60000 |

### 2.4 命令验证

```json
{
  "type": "command",
  "path": "/srv/apps/check",
  "args": ["--ready"],
  "timeout_ms": 5000
}
```

| 字段 | 要求 |
| --- | --- |
| `type` | 固定 `command` |
| `path` | 必须为绝对路径，并位于目标节点工作根目录内 |
| `args` | 字符串数组，最多 32 项；每项不超过 1024 字节，不能包含 NUL、换行或回车 |
| `timeout_ms` | 100 到 60000 |

`path` 是固定可执行文件路径，`args` 是直接传递的参数数组。平台不会拼接
shell 命令，也不接受 `bash -c "..."` 字符串。

## 3. 执行边界

当前控制面对部署后验证配置执行形状校验，并将其纳入应用契约和 target
snapshot。业务 release 脚本仍然负责实际健康检查，并通过
`deploy.verification.started`、`deploy.verification.succeeded` 或
`deploy.verification.failed` 事件驱动部署时间线的验证阶段。

如果业务脚本已经有健康检查，部署后验证配置应选择与检查结果一致的类型；
不要把它看成平台会无条件自动执行的一组命令。

## 4. 修改与验证

1. 在应用详情点击编辑，确认当前应用 `version` 和现有 JSON。
2. 只修改本次需要的字段，不应改动目标节点配置。
3. 保存前检查 JSON 格式、未知字段、模块选项和验证字段范围。
4. 保存成功后再发起新部署；已有预览会因契约变化而失效。
5. 多目标应用优先使用 `http` 或 `tcp`，避免用依赖单一节点工作路径的
   `command` 配置。
6. 不要在 JSON 中放密码、token、连接串或完整 Env 内容，敏感值继续放在
   应用配置和 Env 文件中。

## 5. 给 AI/开发人员的快速检查

- 修改参数前先读取应用当前的 `parameterSchema` 和 `verificationConfig`。
- 只能修改应用级字段，不能把配置写到 `deploymentTarget`。
- 两阶段应用必须包含 `release-version` 和 `modules`。
- `modules.x-options` 必须非空且不重复。
- 需要默认少选模块时使用 `x-default-selected`。
- 不使用未在白名单中的 JSON Schema 字段。
- 不使用未知验证类型、缺失字段、越界端口/状态码/超时。
- 不把任意 shell、密码或 Env 内容写入部署契约。

相关接入步骤见 `docs/runbooks/application-onboarding.md` 和
`docs/runbooks/application-templates.md`；阶段与事件约定见
`docs/standards/deploy-script-contract.md`。
