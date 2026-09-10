---
name: deploy-go-deployer
description: 通过 Deploy Go 对外部署 API 列出可部署应用、查看应用与目标、编辑非正式环境应用、登记 Env、配置部署目标与固定工作区来源、发起部署、查询部署状态和取消部署。用户要求对 Deploy Go 应用发起部署、查看部署状态、取消部署、编辑非正式环境应用配置或登记 Env 时使用；不用于读取 Env 明文或执行其他管理面操作。
---

# Deploy Go 对外部署

通过 Skill 自带的 `deploy-go-deployer` CLI 调用 Deploy Go 对外部署 API，不手写 HTTP 请求。

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
2. 应用或目标不明确时先执行 `list-apps`，需要目标、环境或版本时再执行 `show-app`；不要猜测标识。
3. 发起部署前确认应用、目标、发布版本和关键参数；编辑应用或登记 Env、配置目标前确认要修改的字段和目标应用。
4. 执行一次最小写操作，解析服务端返回的实际状态、版本或错误码。
5. 部署后立即执行 `status`，报告实际状态、阶段和错误；服务端返回 4xx/5xx 时停止，不猜测参数重试写操作。

完整命令参数见 [references/commands.md](references/commands.md)。按任务选择性读取 [references/workflows.md](references/workflows.md)，错误处理见 [references/errors.md](references/errors.md)。

## 写入边界

- `deploy` 只能发起非正式环境部署；正式环境（`prod`）会被服务端拒绝。
- `update-app` 只能编辑非正式环境应用；正式环境应用或把环境改为 `prod` 都会被服务端拒绝。
- `register-env-file`、`update-env-file`、`delete-env-file`、`create-target`、`update-target`、`set-target-status`、`set-workspace-source` 同样只允许非正式环境应用。
- Env 只能写入、不能读取：对外接口不返回 Env 明文，也不提供查看命令；内容必须来自用户提供的本地文件。
- `cancel` 可取消当前 Key 可见应用的部署，包括正式环境部署。
- 写操作前必须向用户确认应用、目标、版本和关键参数。
- 幂等键默认自动生成；需要可重放时由用户显式传入 `--idempotency-key`。
- `update-app`、`update-env-file`、`delete-env-file`、`set-workspace-source` 使用 `version` 做乐观锁；省略时 CLI 会先读取现状自动获取，服务端返回 409 时重新读取并让用户确认；`update-target`、`set-target-status` 必须显式传入 `--version`。
- 不执行任意 shell、Make target、部署脚本或容器命令。
- 不读取、回显或猜测 Env、密钥、SSH 凭证或应用参数以外的敏感数据。
- 不直接构造未包含在对外 OpenAPI 中的 HTTP 请求。
