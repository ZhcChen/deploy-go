---
name: deploy-go-deployer
description: 通过 Deploy Go 对外部署 API 创建非正式环境应用、列出可部署应用、查看应用与目标、编辑非正式环境应用、登记 Env、检查非生产 Env 指定键的存在性与长度、配置部署目标与固定工作区来源、发起部署、查询部署状态、失败诊断与部署日志，以及取消部署。用户要求对 Deploy Go 应用发起部署、创建应用、查看部署状态或日志、排查部署失败、取消部署、编辑非正式环境应用配置、登记 Env 或核实指定 Env 键元数据时使用；不用于读取 Env 明文、节点系统日志或执行其他管理面操作。
---

# Deploy Go 对外部署

通过 Skill 自带的 `deploy-go-deployer` CLI 调用 Deploy Go 对外部署 API，不手写 HTTP 请求。Env 检查只返回用户指定键的存在性和 UTF-8 字节长度，不返回明文。

## 定位 CLI

从当前已加载 `SKILL.md` 的所在目录确定 `<skill-dir>`，不要依赖当前工作目录或源码仓库：

- macOS/Linux：`<skill-dir>/scripts/deploy-go-deployer`
- Windows：`<skill-dir>\scripts\deploy-go-deployer.exe`

首次使用或排查安装时运行 `<cli> --version`。CLI 缺失或不可执行时，提示用户重新安装 Skill，不要改用手写 HTTP 请求。

## 前置条件

- `DEPLOY_GO_API_BASE_URL`：默认 `https://deploy.quanxinfu.com`。
- `DEPLOY_GO_API_KEY`：管理端创建的外部 API Key，格式 `dgx_...`。不得读取、回显、记录或写入 Key。

## 核心流程

1. 从用户输入中提取 `application_id`、`target_id`、`deployment_id`、动作和参数。
2. 需要新建应用时先确认名称、slug 与环境（只能是 `dev` / `test` / `staging`），再执行 `create-app`；此后沿用返回的应用 ID。
3. 应用或目标不明确时先执行 `list-apps`，需要目标、环境或版本时再执行 `show-app`；不要猜测标识。
4. 发起部署前确认应用、目标、发布版本和关键参数；编辑应用或登记 Env、配置目标前确认要修改的字段和目标应用。
5. 执行一次最小写操作，解析服务端返回的实际状态、版本或错误码。
6. 部署后立即执行 `status`，报告实际状态、阶段和错误；部署失败或状态异常时继续执行 `diagnose`。
7. `diagnose` 返回 `logs_available=true` 时执行 `logs`；若响应包含 `next_after`，使用它作为下一页 `--after`，直到 `next_after=null`。优先报告最后一个失败阶段附近的 stderr/stdout，不凭空推断未返回的日志。
8. 服务端返回 4xx/5xx 时停止，不猜测参数或重复写操作；只读的 `status`、`diagnose` 和 `logs` 可用于核实同一个部署 ID。

完整命令参数见 [references/commands.md](references/commands.md)。按任务选择性读取 [references/workflows.md](references/workflows.md)，错误处理见 [references/errors.md](references/errors.md)。

## 写入边界

- `create-app` 只能创建非正式环境（`dev` / `test` / `staging`）应用；`prod` 返回 403，正式环境应用只能由管理员在管理面手动创建。
- `create-app` 创建的应用会自动绑定到当前 API Key；slug 全局唯一，冲突返回 409。
- `deploy` 只能发起非正式环境部署；正式环境（`prod`）会被服务端拒绝。
- `update-app` 只能编辑非正式环境应用；正式环境应用或把环境改为 `prod` 都会被服务端拒绝。
- `register-env-file`、`update-env-file`、`delete-env-file`、`create-target`、`update-target`、`set-target-status`、`set-workspace-source` 同样只允许非正式环境应用。
- `inspect-env-file` 只允许非正式环境应用；只可检查用户明确指定的键名，返回存在性和 UTF-8 字节长度。
- 外部 API 不返回 Env 明文。不得读取、输出、记录或推断密钥值；内容写入仍必须来自用户提供的本地文件。
- `cancel` 可取消当前 Key 可见应用的部署，包括正式环境部署。
- `diagnose` 和 `logs` 只能读取当前 API Key 有权访问应用的结构化诊断与已持久化、已脱敏部署日志；不尝试访问节点 journal、任意文件或内部管理 API。
- 写操作前必须向用户确认应用、目标、版本和关键参数。
- 幂等键默认自动生成；需要可重放时由用户显式传入 `--idempotency-key`。
- `update-app`、`update-env-file`、`delete-env-file`、`set-workspace-source` 使用 `version` 做乐观锁；省略时 CLI 会先读取现状自动获取，服务端返回 409 时重新读取并让用户确认；`update-target`、`set-target-status` 必须显式传入 `--version`。
- 不执行任意 shell、Make target、部署脚本或容器命令。
- 不读取、回显或猜测 Env、密钥、SSH 凭证或应用参数以外的敏感数据。
- Env 检查仅报告指定键的存在性和长度；不得据长度猜测密钥内容。
- 不直接构造未包含在对外 OpenAPI 中的 HTTP 请求。
