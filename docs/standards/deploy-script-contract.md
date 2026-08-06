---
date: 2026-07-31
topic: deploy-script-contract
status: accepted
schema_version: 1
---

# 部署脚本接入契约

## 目标

Deploy Go 只托管脚本的执行，不接管应用内部发布逻辑。接入脚本必须可审计、可取消、可验证，并同时服务终端排障和平台状态机。

## 脚本基线

```bash
#!/usr/bin/env bash
set -euo pipefail
```

脚本必须：

- 使用严格模式并引用所有变量。
- 不使用 `eval`，不接受任意 shell 片段。
- 对外部命令执行可用性检查。
- 对环境、模块、版本和目标等输入执行白名单或安全字符校验。
- 在顶层脚本统一处理参数、模块顺序、失败兜底和最终退出码。
- 部署后执行最小必要验证，不能把“命令执行完成”直接视为部署成功。

## 标准输入

平台通过参数表达用户选择，通过环境变量传递运行上下文和敏感值。建议参数：

- `--environment <name>`
- `--modules <comma-separated>`
- `--release-version <version>`
- `--target <target>`
- `--no-build`

部署目标的 `parameter_schema` 属性名使用不带前导 `--` 的 kebab-case 长选项名。平台按属性名稳定排序并按以下规则构造参数数组，所有 token 仍须经过统一 POSIX 编码器：

- 字符串、整数和数字：`name=value` 转为 `--name value`。
- 布尔 `true`：转为 `--name`；布尔 `false`：不传递该选项。
- `null`、数组、对象、未知字段和 schema 外字段：拒绝创建部署。

平台保留环境变量：

- `DEPLOY_ID`
- `DEPLOY_APP_ID`
- `DEPLOY_ENVIRONMENT`
- `DEPLOY_RELEASE_VERSION`
- `DEPLOY_TARGET`
- `DEPLOY_CANCEL_FILE`

敏感值只能通过进程环境或权限受控文件传入，不拼接到命令行，不写入事件、日志和错误正文。

首版平台只管理节点本地敏感文件引用。平台传递受控文件路径，不读取或经 SSH 传送敏感文件内容。

## 双轨输出

### 人类日志

文本日志使用 UTF-8，可使用 `[信息]`、`[阶段]`、`[完成]`、`[警告]` 和 `[失败]` 标记。平台完整保留文本，但不依赖自然语言正文推进状态。

### 结构化事件

格式固定为每行一个事件：

```text
DEPLOY_EVENT {"schema_version":1,"event":"deploy.started",...}
```

要求：

- 前缀固定为 `DEPLOY_EVENT `。
- 后接单行 JSON，不允许 pretty print 或跨行。
- 必填字段为 `schema_version`、`event`、`timestamp` 和 `status`。
- JSON 解析失败时平台保留原始行并标记 `malformed_event`，不能导致日志页面崩溃。
- 未知字段必须被忽略，以便协议向后兼容。
- 未知事件名按普通日志保存并记录 `unknown_event`，不得推进部署状态。

标准事件的机器可读 Schema 位于 `docs/standards/deploy-event.schema.json`。Schema 允许未知字段以保持向后兼容；未知事件名仍按上述规则降级处理。

事件的 `status` 只允许 `running`、`succeeded`、`failed` 和 `canceled`。`queued`、`canceling` 和 `interrupted` 是平台状态，不由应用脚本输出。

## 标准事件

- `deploy.started`
- `deploy.preflight.started`
- `deploy.preflight.succeeded`
- `deploy.preflight.failed`
- `deploy.module.started`
- `deploy.module.succeeded`
- `deploy.module.failed`
- `deploy.step.started`
- `deploy.step.succeeded`
- `deploy.step.failed`
- `deploy.verification.started`
- `deploy.verification.succeeded`
- `deploy.verification.failed`
- `deploy.finished`

任务常用字段：

- `deploy_id`、`environment`、`modules`、`release_version`、`target`
- `duration_ms`、`exit_code`、`message`
- `candidate_release`、`current_release`、`current_switched`
- `failure_stage`、`recovery_hint`

步骤常用字段：

- `module`、`module_name`、`step_id`、`step`、`status`、`duration_ms`

## 进度层级与生命周期

结构化进度分为部署、模块和步骤三级：

```text
deploy
  module
    step
```

- `deploy.started` 和 `deploy.finished` 定义整个部署的唯一外层边界。
- `.started`、`.succeeded` 和 `.failed` 事件的 `status` 必须分别为 `running`、`succeeded` 和 `failed`；`deploy.finished` 还允许 `canceled`。
- 每个实际执行的模块必须输出一对 `deploy.module.started` 与模块终态事件。
- 模块内部需要在界面单独展示、排障或计时的关键动作使用步骤事件；普通命令输出不需要事件化。
- 同一模块内的 `step_id` 必须稳定且唯一，建议使用 `<module>.<action>`，例如 `api.migrate`。
- `module` 和 `step_id` 是稳定的机器标识；`module_name` 和 `step` 是可展示文本，修改展示文本不得改变标识。
- `deploy.started.modules` 按计划执行顺序列出模块。未在该列表中的模块事件属于协议异常，不推进进度。
- 模块和步骤开始后必须恰好输出一个对应终态事件：`succeeded` 或 `failed`。取消时允许不补齐当前模块和步骤的终态，但必须输出 `deploy.finished` 且状态为 `canceled`。
- 步骤必须位于所属模块的开始与终态之间；模块不得在仍有未结束步骤时输出 `succeeded`。
- 任一步骤失败后，所属模块不得输出 `succeeded`；任一模块失败后，部署不得输出 `succeeded`。
- 串行部署一次只能有一个运行中模块；需要并行时允许多个模块同时运行，但每个步骤仍必须通过 `module` 明确归属。
- `duration_ms` 只出现在终态事件中，表示对应开始事件到终态事件的耗时。

平台根据已声明模块、模块终态和步骤事件展示确定性进度，不要求脚本估算百分比。缺失、重复、越级或顺序冲突的事件必须保留为诊断信息，不能覆盖进程退出码和有效失败事件。

### 输出边界

- 每个 `DEPLOY_EVENT` 必须独占一行并一次性写入 stdout，避免并发日志将 JSON 拆分或交错。
- stderr 保留给人类可读的警告和错误，不输出结构化事件。
- Agent 只负责识别前缀、保序并转发；主控 API 负责 Schema 校验、持久化、协议诊断和进度计算。
- 应用可以封装 Bash helper 统一生成事件，但 helper 不是协议组成部分，也不是接入前置条件。

## 退出码与最终状态

- `deploy.finished.status=succeeded` 必须对应退出码 `0`。
- `failed` 和 `canceled` 必须对应非 `0`。
- 中间步骤失败后不得继续输出成功结论或用 `exit 0` 掩盖失败。
- 进程异常退出且没有 `deploy.finished` 时，平台按退出码结束任务并标记协议不完整。
- 退出码 `0` 且缺少 `deploy.finished` 时可以形成 `succeeded`，但 `protocol_complete=false`。
- 退出码非 `0` 且缺少 `deploy.finished` 时形成 `failed`，但平台取消流程已确认终止的任务形成 `canceled`。
- `deploy.finished` 与退出码冲突时以失败侧为准，并记录 `protocol_conflict`。

## 预检与验证

脚本应按需检查：

- 目标连接、工作目录与磁盘空间。
- 依赖命令和运行时版本。
- 端口冲突、配置格式与发布包完整性。
- 部署后的 HTTP、TCP、容器、systemd 或静态文件状态。

阻断项必须失败退出；警告项可以继续，但需要结构化事件和可读说明。

## 取消

脚本必须处理 `SIGTERM`，并可轮询 `DEPLOY_CANCEL_FILE`。收到取消后应停止启动新步骤、执行必要清理、输出 `deploy.finished` 的 `canceled` 状态并以非零退出。

取消不等同于回滚。脚本已经产生的变更是否恢复，必须由应用脚本明确实现并在 `recovery_hint` 中说明。

Agent 包装器在部署专属运行目录中记录任务摘要、进程身份、stdout、stderr、转发偏移、精确退出码、取消文件和原子完成标记。取消请求先创建取消文件并发送 `SIGTERM`，默认宽限期 30 秒；超时后发送 `SIGKILL`。Agent 重启后必须校验进程 start-time 等身份信息再继续跟踪；无法证明进程归属或最终状态时，平台标记 `interrupted`，不得假定取消或部署成功。

主控只通过版本化 Agent 控制协议下发结构化任务，不下发任意 shell 字符串。包装器版本固定在主控与 Agent 的兼容契约中，任务只能引用双方支持的版本，不能由用户参数或自定义下载地址替换。

## 日志限制

- stdout 和 stderr 分别标记来源，并共享部署内单调递增序号。
- 输入按字节读取；非法 UTF-8 使用替换字符保存，并记录一次诊断事件。
- 单行默认上限为 64 KiB，超出部分丢弃并标记 `line_truncated`。
- 单任务默认持久化上限为 50 MiB。达到上限后平台继续消费输出，但停止保存正文并记录 `log_budget_exceeded`。
- 限额可以由系统设置在服务端安全范围内调整，不能关闭硬上限。
- SSE 只发送已经持久化的日志，使用日志序号作为续传游标。

## 安全

- 禁止输出密码、token、私钥、完整 `.env`、连接串原文和第三方密钥。
- 禁止平台隐式使用 `sudo`；需要的节点权限应由部署账号预先配置。
- Agent 和部署脚本统一以 `deploy-go-agent` 用户运行；接入检查必须确认工作目录、脚本及敏感文件引用对该用户可访问。
- 工作目录解析后的真实路径必须位于该应用允许的根目录中。
- 脚本变更需要独立权限、diff 审查和发布确认。
- 平台展示层继续脱敏，但不能把展示脱敏当成脚本泄密的补救措施。

## 最小示例

```text
DEPLOY_EVENT {"schema_version":1,"event":"deploy.started","deploy_id":"dep-1042","timestamp":"2026-07-31T00:00:00Z","environment":"production","modules":["api"],"release_version":"v2.8.4","target":"sh-prod-01","status":"running"}
DEPLOY_EVENT {"schema_version":1,"event":"deploy.module.started","deploy_id":"dep-1042","timestamp":"2026-07-31T00:00:05Z","module":"api","module_name":"API 服务","status":"running"}
DEPLOY_EVENT {"schema_version":1,"event":"deploy.step.started","deploy_id":"dep-1042","timestamp":"2026-07-31T00:00:10Z","module":"api","step_id":"api.release","step":"切换 current release","status":"running"}
DEPLOY_EVENT {"schema_version":1,"event":"deploy.step.succeeded","deploy_id":"dep-1042","timestamp":"2026-07-31T00:01:10Z","module":"api","step_id":"api.release","step":"切换 current release","status":"succeeded","duration_ms":60000}
DEPLOY_EVENT {"schema_version":1,"event":"deploy.module.succeeded","deploy_id":"dep-1042","timestamp":"2026-07-31T00:01:15Z","module":"api","module_name":"API 服务","status":"succeeded","duration_ms":70000}
DEPLOY_EVENT {"schema_version":1,"event":"deploy.finished","deploy_id":"dep-1042","timestamp":"2026-07-31T00:02:18Z","environment":"production","modules":["api"],"release_version":"v2.8.4","target":"sh-prod-01","status":"succeeded","duration_ms":138000,"exit_code":0}
```
