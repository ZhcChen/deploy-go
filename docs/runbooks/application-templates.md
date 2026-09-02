# Docker Compose 应用模板接入 Runbook

## 目标

管理员快速接入 PostgreSQL、Redis、Valkey、etcd 等基础中间件，使用 Docker Compose 发布。
模板支持三种接入路径：

- **Git 两阶段**：模板只提供业务仓库骨架；Compose、Env 与发布脚本由业务
  仓库维护和审查，Deploy Go 不解析或接管 `compose.yaml`。
- **脚本两阶段（two_stage_script）**：模板/应用不必进入 Git 仓库，管理员在
  构建节点登记固定工作区路径，Agent 快照该工作区并执行 prepare/release。
  适合内部已有镜像与 Compose 编排、暂时不需要平台接管 Git 来源的应用。
- **镜像直连（image）**：不需要业务 Git 仓库。Deploy Go 使用
  `container-template` 共享模板在平台侧生成固定发布物，目标 Agent 下载并
  复验后由 root executor 固定执行
  `make --no-print-directory deploy-go-release`。

管理端「应用模板」页面（`/templates`）可只读查看模板的 Compose、Env 示例、
应用配置与参数 Schema。管理员可从页面进入「从模板创建应用」向导，按模板预填
应用与部署目标。向导直接支持「Git 两阶段」与「镜像直连（无需仓库）」；
「脚本两阶段（本地工作区）」在应用创建后配置，不依赖模板向导。

## 从模板创建应用/目标（管理端向导）

1. 管理员在 `/templates` 点击「从模板创建应用」，选择 PostgreSQL、Redis、
   Valkey 或 etcd 模板。
2. 确认应用名称与 slug，创建应用后进入部署方式步骤。
   部署方式步骤会直接展示已克隆的模板配置文件（Compose、Env 与应用配置），
   可在带语法高亮的代码编辑器中调整预设值并保存；保存结果会进入应用配置
   版本历史，后续部署 preview 固化当前版本摘要。
3. 选择「镜像直连（无需仓库）」：直接进入镜像部署目标配置，选择在线节点、
   模板、镜像引用、宿主端口与已登记 Env 文件；必须勾选 root 信任边界确认，
   不需要 Git 来源、固定分支或构建节点。
4. 选择「Git 两阶段」：填写业务仓库地址，选择只读 Git 凭证与在线 Agent；
   保存来源后等待分支发现并固定部署分支。来源失败不会回滚，已创建应用会
   保留并提供应用详情入口。
5. 结果页展示已创建资源与 Env 示例（`compose.env.example`、
   `postgres.env.example` / `redis.env.example` / `valkey.env.example` /
   `etcd.env.example`）。向导不上传 Env 明文，真实值按下方步骤登记。

普通用户只能只读查看模板，不能访问 `/templates/new`。

## 前置条件

- Deploy Go API / Web 0.2.0 以上，目标节点具备 executor v3 与
  `PRIVILEGED_RELEASE` capability；所有镜像模板均要求控制协议 v11 的通用
  artifact checkout 能力。
- 目标节点已安装 Docker Engine 与 Compose v2 插件；发布脚本以 root 运行，
  不需要把 `deploy-go-agent` 加入 docker 组。
- 镜像直连模式不需要管理员准备业务 Git 仓库；Git 两阶段模式需要独立业务
  仓库（模板不允许直接在 `deploy-go` 仓库部署）。

## 镜像直连部署步骤（image）

1. 创建应用：在 `/templates` 从模板创建应用，部署方式选择「镜像直连（无需
   仓库）」；应用不需要配置 Git 来源。
2. 调整 Env：创建应用的部署方式步骤已把 `compose.env` 与
   `postgres.env` / `redis.env` / `valkey.env` / `etcd.env` 克隆为应用配置
   副本，可直接在代码编辑器调整并保存；也可以稍后在应用详情 → 应用配置继续
   编辑（密码使用真实值）。镜像模式要求 Env 已同步到目标节点，部署前 Env
   门禁会校验版本与摘要。
3. 创建镜像部署目标：执行模式选择「镜像直连模式」，选择模板、镜像引用、
   宿主端口与 1-16 个已登记 Env 文件。平台固定使用 Agent 原生特权 release，
   不再提供 `privileged_release` 开关或 root 信任确认。
4. 发起部署：主控使用 `container-template` 生成固定发布物（模板压缩包、固定
   checkout 文件与 artifact manifest），Agent 下载并复验后只按固定文件清单
   生成 checkout；release 由
   root executor 固定执行 `make --no-print-directory deploy-go-release`。
5. 模板内固定动作：`docker compose config --quiet` →
   `docker compose up -d --remove-orphans` → 等待容器 `running` 且健康检查
   通过。模板不接受任意 Compose、命令、参数或环境变量表。

## Git 两阶段步骤

1. 复制模板到业务仓库：

   ```bash
   cp -R examples/templates/postgres /srv/git/my-postgres
   cd /srv/git/my-postgres
   git init
   git add .
   git commit -m "init postgres template"
   ```

2. 在 Deploy Go 中创建应用并配置 Git 来源，固定部署分支。
3. 在部署方式步骤或应用详情 → 应用配置中编辑模板克隆的配置副本：
   - `compose.env`：Compose 插值，内容参考 `compose.env.example`；
   - `postgres.env` / `redis.env` / `valkey.env`：服务级容器 Env，内容参考对应
     `<service>.env.example`；密码使用真实值，禁止提交到仓库。
4. 创建两阶段部署目标：
   - 执行模式：`two_stage`
   - 脚本路径：固定占位路径，例如 `/srv/apps/my-postgres/placeholder`
     （实际由 root executor 固定执行 `make deploy-go-release`）
   - 参数 Schema 使用模板目录中的 `parameter-schema.json`，`modules.x-options`
     只保留所选模板的模块名（`postgres`、`redis`、`valkey` 或 `etcd`）；
     可选用 `modules.x-default-selected` 设置默认选中模块，省略时默认全选
   - 部署后验证配置使用模板提供的默认值；镜像模板默认使用 TCP 端口检查
   - JSON 完整约束与示例见
     `docs/standards/application-deployment-json.md`
5. 发起部署。prepare 由低权限 runner 打包 `compose.yaml`、`config/`
   下的应用配置与 manifest；etcd 模板仅打包 `compose.yaml` 与 manifest；
   release 由目标节点 root executor 执行：

   ```text
   docker compose config --quiet
   docker compose up -d --remove-orphans
   ```

   然后等待容器进入 `running` 且健康检查通过。

## 脚本两阶段（two_stage_script）步骤

1. 在构建节点准备固定工作区，根目录提供：

   ```makefile
   .PHONY: deploy-go-prepare deploy-go-release

   deploy-go-prepare:

   deploy-go-release:
   ```

   prepare 只生成发布物到 `DEPLOY_OUTPUT_DIR`，release 只消费
   `DEPLOY_ARTIFACT_DIR` 中的已校验发布物，规则与
   `docs/standards/application-deployment-contract.md` 一致。

2. 在 Deploy Go 创建应用，并在应用详情 → 工作区来源选择在线 v14 构建 Agent、
   填写固定绝对路径；保存即生成 `workspace_version=1`。
3. 创建部署目标：执行模式选择「脚本两阶段模式（本地工作区）」，配置模块、参数
   Schema 与部署后验证（与 Git 两阶段一致）。
4. 发起部署。prepare 在 Agent 快照的固定工作区中执行，发布物会同时包含业务
   模块与 `deploy-go-workspace.tar.gz`；release 在目标节点解压还原后由 root
   executor 固定执行 `make --no-print-directory deploy-go-release`。

工作区安全边界：

- 工作区路径必须是构建 Agent 本机绝对路径，禁止相对路径、`..`、前缀路径和
  控制字符。
- Agent 快照拒绝符号链接、硬链接、非普通文件和路径逃逸，并受 staging 大小与
  文件数限制；业务 release 不应依赖工作区中的绝对路径。
- `two_stage_script` 是平台固定两阶段脚本模式，不接受任意命令、任意 Make
  target 或 Git 分支来源；平台仍固定执行 `deploy-go-prepare` /
  `deploy-go-release`。

## 验证

- 本地契约测试（不执行 Docker）：

  ```bash
  make app-template-check
  ```

- 部署完成后在节点检查：

  ```bash
  docker compose -p deploy-go-<DEPLOY_TARGET> ps
  ```

- 数据卷名称以 Compose 项目名 `deploy-go-<DEPLOY_TARGET>` 为前缀；回滚时创建
  指向旧 commit 的新部署，不要执行 `docker compose down -v`。

## 安全边界

- 模板发布脚本只接受平台固定环境变量，不接受任意命令、参数或 Make target。
- `compose.env` 只写入 release 目录，权限 `0600`；脚本不输出密码、连接串或
  完整 Env 文件内容。
- release 固定特权意味着该仓库和固定分支的写入者获得目标节点 root
  发布能力；仓库 URL、分支、节点变化后必须重新确认。
- 镜像直连目标不接受任意 `command`、`executable`、`args`、Make target 或
  env map；`image_spec` 只允许安全字符，端口 1-65535，Env 文件必须来自应用
  配置白名单。
- 镜像直连不需要 launcher、sudoers 或系统目录安装脚本；平台 release 固定
  使用 Agent 原生特权 executor，历史 Git 两阶段模板如仍使用 launcher，仅
  作为兼容参考并遵守 `privileged-launcher-check`。
- 模板不执行 `eval`、`sudo docker` 或 `docker compose down -v`。
- Redis 模板的密码会出现在宿主进程参数与 `docker inspect` 中，仅适合内部
  测试；生产环境应改为独立 Secret 管理后再接入。
- etcd 模板是单节点、非 TLS 且仅发布到 `127.0.0.1` 的开发/测试模板。不得把
  `2379` 或 `2380` 暴露到不受控网络；生产配置中心或分布式锁必须独立部署三节点
  mTLS 集群，并配置认证/RBAC、备份恢复与监控。
