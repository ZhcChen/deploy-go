# deploy-go-deployer

Deploy Go 对外部署 API 的 Rust CLI / Agent Skill 执行器，只调用 `/external/v1`。

## 当前能力

| 场景 | 命令 |
| --- | --- |
| 应用查询与维护 | `list-apps`、`show-app`、`create-app`、`update-app`、`set-app-status` |
| Env 元数据、指定键检查与内容登记 | `list-env-files`、`inspect-env-file`、`register-env-file`、`update-env-file`、`delete-env-file`、`retry-env-sync` |
| 部署目标 | `list-targets`、`create-target`、`update-target`、`set-target-status` |
| 固定工作区来源 | `show-workspace-source`、`set-workspace-source` |
| Git 来源与固定分支 | `list-build-agents`、`list-git-credentials`、`show-git-source`、`set-git-source`、`refresh-git-branches`、`show-git-refresh`、`set-git-branch` |
| 配置文件元数据 | `list-config-files` |
| 部署预览与发起 | `preview-deploy`、`deploy` |
| 运行探测与状态 | `probe-runtime`、`show-runtime-status` |
| 部署观察、诊断与取消 | `status`、`diagnose`、`logs`、`cancel` |
| API 契约 | `openapi` |

使用 `deploy-go-deployer --help` 或 `deploy-go-deployer <command> --help` 查看参数；完整操作顺序、API Key 绑定和错误码见[对外 API 手册](../docs/runbooks/external-deploy-api.md)。部署预览返回固定输入的 `snapshot_hash`，发起时通过 `--snapshot-hash` 使用该预览；分支变更后重新预览。

## 配置与安装

- API 地址：`--api-base` / `DEPLOY_GO_API_BASE_URL`。
- 外部 API Key：`--api-key` / `DEPLOY_GO_API_KEY`，格式为 `dgx_...`；不得写入日志或仓库。
- Linux：从 Deploy Go API 下载发布物，见[安装步骤](../docs/runbooks/external-deploy-api.md#安装-deployer)。
- macOS：`cargo build -p deploy-go-deployer --release`，将 `target/release/deploy-go-deployer` 安装到 PATH。
- Agent Skill：见[Skill 安装说明](../docs/runbooks/external-deploy-api.md#安装-agent-skill)，写入用户技能目录需要对应授权。

## 使用边界

应用创建、编辑、来源/目标配置、Env 维护、预览/探测和部署写操作限于 `dev` / `test` / `staging`，正式环境由管理面处理。既有只读接口按各自授权契约开放，新增读取接口的非生产限制见手册。

Env 检查仅返回指定键是否存在及 UTF-8 字节长度；配置文件接口仅返回元数据。CLI 不返回 Env、Git 凭证或其他密钥明文，不执行任意命令或管理面操作。Env 内容登记从本地文件读取，避免将内容放入命令行参数。
