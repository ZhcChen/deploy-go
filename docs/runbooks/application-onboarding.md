# 业务应用接入部署手册

本手册说明业务应用接入 Deploy Go 两阶段部署与镜像直连部署时的准备、launcher 安装与本地验证步骤。手册不授权连接真实 Agent、执行真实部署、migration、重启、切流或清理；真实节点操作必须由当前对话明确授权。

## 1. 应用仓库要求

业务仓库根目录必须提供：

```makefile
.PHONY: deploy-go-prepare deploy-go-release

deploy-go-prepare:
	# 只生成发布物到 DEPLOY_OUTPUT_DIR

deploy-go-release:
	# 只消费 DEPLOY_ARTIFACT_DIR 中的已校验发布物
```

接口与事件要求见 `docs/standards/application-deployment-contract.md` 和 `docs/standards/deploy-script-contract.md`。最小参考实现位于 `examples/branch-deployment/`。

## 1A. 镜像直连模式（免业务仓库）

Redis / PostgreSQL 等平台模板应用可以选择 `image` 执行模式，不要求业务 Git
仓库：

- 应用在管理端「从模板创建应用」向导选择「镜像直连（无需仓库）」，无需
  Git 来源、固定分支或构建节点。
- 首版 Env 在应用详情 → 应用配置登记/导入；部署前 Env 门禁校验目标节点
  版本与摘要。
- 部署目标只接受模板、镜像引用、宿主端口与已登记 Env 文件白名单；
  release 固定使用 Agent 原生特权，不再提供 `privileged_release` 开关，
  不接收任意 Compose、命令、参数或 env map。
- 主控用 `container-template` 生成固定发布物，Agent 下载复验后生成固定
  checkout；root executor 固定执行 `make --no-print-directory
  deploy-go-release`。
- 镜像直连不需要 launcher、sudoers 或系统目录安装脚本。业务仓库仍可使用
  Git 两阶段模式并提供固定 `make deploy-go-release` 与业务发布脚本。

## 1B. 应用环境标识

- 应用环境在应用详情编辑，取值 `dev` / `test` / `staging` / `prod`，与
  Agent 环境枚举一致。
- 部署目标环境只读继承应用环境，不能按目标单独配置；应用环境变更会在
  同一事务内同步全部目标并递增目标版本。
- `DEPLOY_ENVIRONMENT` 由应用环境注入业务脚本，测试应用应选择 `test`，
  生产应用选择 `prod`。业务脚本据此选择 testing / production profile，
  不要依赖历史兼容值 `prod` 判断环境。
- 测试应用示例：`qfy-voucher-hub-testing` 应编辑为「测试环境」，使其
  部署脚本收到 `DEPLOY_ENVIRONMENT=test`。

## 1C. 应用清单与 target_code 绑定

- 应用根目录应提供 `deploy-go.yaml`，声明 `type` / `type_version` /
  `modules` / `env_files`，规范见 `docs/standards/application-manifest.md`。
  应用详情保存的 `app_type` / `type_version` 是控制面权威值；模板应用与
  平台注册表不一致时拒绝部署。
- 每个部署目标维护稳定 `target_code`：业务 release 用它注入 `DEPLOY_TARGET`。
  新建目标留空时默认按环境标识生成，应用内保持稳定、不冲突。

## 2. 特权发布 launcher

业务脚本需要 Docker、root 或系统级操作时，应用仓库必须提供：

- launcher 源脚本，遵守 `docs/standards/privileged-release-launcher.md`。
- root 所有、固定绝对路径的发布入口或内部固定动作。
- sudoers 配置样例和本地契约测试。

本地验证：

```bash
make privileged-launcher-check
```

## 3. 安装与校验

由节点管理员在目标节点执行：

1. 复制 launcher 到固定路径，例如 `/usr/local/sbin/qfy-voucher-hub-release-launcher`。
2. 复制发布入口及依赖到固定 root-owned 目录，例如 `/usr/local/lib/qfy-voucher-hub/`。
3. 设置 `root:root` 和 `0755`，确认目录链不被 `deploy-go-agent` 写。
4. 创建 `/etc/sudoers.d/` 精确规则，权限 `0440`。
5. 执行 `visudo -c` 和 `sudo -l -U deploy-go-agent` 核对。

安装前必须确认：

- launcher 不接受 shell、Docker 参数、URL、环境文件内容或任意命令路径。
- 输入 JSON 只包含规范字段，launcher 拒绝额外字段和路径逃逸。
- 底层失败退出码、SIGTERM 和审计日志符合规范。
- `deploy-go-agent` 不在 Docker 组，也不拥有通用 sudo。

## 4. Agent 与任务路径

- Agent 以 `deploy-go-agent` 用户运行，数据目录默认为 `/var/lib/deploy-go-agent`。
- 任务 staging 位于 Agent 工作目录内，业务脚本和 launcher 都只通过 `DEPLOY_OUTPUT_DIR` / `DEPLOY_ARTIFACT_DIR` 消费。
- launcher 输入文件和中间任务目录必须放在 Agent 可写、root launcher 可读的任务路径内，不能放在 systemd `PrivateTmp` 私有目录中。

## 5. 应用配置（Env）

- 运行配置统一在应用详情 → 应用配置维护，不再在部署目标上配置 Env。
- Git 两阶段首次登记由业务仓库的 `deploy-go-prepare` 随 manifest 上传，
  推荐按模块命名为 `api.env`、`worker.env`；镜像直连首次登记由管理员在
  应用配置页面登记/导入（详见 `docs/runbooks/application-templates.md`）。
- manifest 只声明 `file_name`、`module`、`format=dotenv-v1`、大小和 SHA-256。脚本日志、制品 manifest 与部署事件不得输出 Env 内容。
- 首次登记后，业务仓库再次上传同名文件不会覆盖管理员在 Deploy Go 中维护的权威版本，也不会因某次上传缺席而删除。
- 同一应用的当前 Env 版本同步到所有启用目标。每个 Agent 写入 `secrets_root/<application_slug>/<file_name>`，业务 release 脚本只读取该固定文件，不把值复制进命令行或日志。
- release 前会校验对应节点的 Env digest；离线或同步失败节点被门禁，其他节点的事实独立保留。管理员修正或重试后只收敛未成功节点。

## 6. 上线前检查

```text
□ git diff --check
□ deploy-go.yaml 与平台应用类型一致（存在时）
□ 部署目标 target_code 与已有 Compose 项目名一致（绑定既有容器时）
□ 应用环境与部署目标环境一致（应用详情环境标识核对）
□ 应用仓库 prepare/release/launcher 自测通过
□ bash -n 全部脚本通过
□ 敏感扫描不含 token、私钥、完整环境文件
□ prepare manifest 中 Env 元数据与实际文件大小、SHA-256 一致
□ 两个隔离目标节点均完成制品校验与 Env digest 门禁
□ 真实节点操作已获得当前对话明确授权
```
