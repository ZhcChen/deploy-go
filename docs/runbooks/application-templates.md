# Docker Compose 应用模板接入 Runbook

## 目标

管理员快速接入 PostgreSQL、Redis 等基础中间件，使用 Docker Compose 发布。
模板支持两种接入方式：

- **Git 两阶段**：模板只提供业务仓库骨架；Compose、Env 与发布脚本由业务
  仓库维护和审查，Deploy Go 不解析或接管 `compose.yaml`。
- **镜像直连（image）**：不需要业务 Git 仓库。Deploy Go 使用
  `container-template` 共享模板在平台侧生成固定发布物，目标 Agent 下载并
  复验后由 root executor 固定执行
  `make --no-print-directory deploy-go-release`。

管理端「应用模板」页面（`/templates`）可只读查看模板的 Compose、Env 示例、
应用配置与参数 Schema。管理员可从页面进入「从模板创建应用」向导，按模板预填
应用与部署目标；选择「镜像直连（无需仓库）」时跳过 Git 来源，选择
「Git 两阶段」时仍需由管理员把模板复制到独立 Git 仓库审查后正式接入。

## 从模板创建应用/目标（管理端向导）

1. 管理员在 `/templates` 点击「从模板创建应用」，选择 PostgreSQL 或 Redis
   模板。
2. 确认应用名称与 slug，创建应用后进入部署方式步骤。
3. 选择「镜像直连（无需仓库）」：直接进入镜像部署目标配置，选择在线节点、
   模板、镜像引用、宿主端口与已登记 Env 文件；必须勾选 root 信任边界确认，
   不需要 Git 来源、固定分支或构建节点。
4. 选择「Git 两阶段」：填写业务仓库地址，选择只读 Git 凭证与在线 Agent；
   保存来源后等待分支发现并固定部署分支。来源失败不会回滚，已创建应用会
   保留并提供应用详情入口。
5. 结果页展示已创建资源与 Env 示例（`compose.env.example`、
   `postgres.env.example` / `redis.env.example`）。向导不上传 Env 明文，
   真实值按下方步骤登记。

普通用户只能只读查看模板，不能访问 `/templates/new`。

## 前置条件

- Deploy Go API / Web 0.2.0 以上，目标节点 Agent 0.2.0、控制协议 v9、
  executor v3，并且目标节点 `PRIVILEGED_RELEASE` / `RUNTIME_STATUS`
  capability 可用。
- 目标节点已安装 Docker Engine 与 Compose v2 插件；发布脚本以 root 运行，
  不需要把 `deploy-go-agent` 加入 docker 组。
- 镜像直连模式不需要管理员准备业务 Git 仓库；Git 两阶段模式需要独立业务
  仓库（模板不允许直接在 `deploy-go` 仓库部署）。

## 镜像直连部署步骤（image）

1. 创建应用：在 `/templates` 从模板创建应用，部署方式选择「镜像直连（无需
   仓库）」；应用不需要配置 Git 来源。
2. 登记 Env：在应用详情 → 应用配置登记 `compose.env` 与
   `postgres.env` / `redis.env`（内容参考对应 `*.env.example`，密码使用真实
   值）。镜像模式要求 Env 已登记并同步到目标节点，部署前 Env 门禁会校验
   版本与摘要。
3. 创建镜像部署目标：执行模式选择「镜像直连模式」，选择模板、镜像引用、
   宿主端口与 1-16 个已登记 Env 文件。平台固定使用 Agent 原生特权 release，
   不再提供 `privileged_release` 开关或 root 信任确认。
4. 发起部署：主控使用 `container-template` 生成固定发布物（模板压缩包与
   artifact manifest），Agent 下载并复验后生成固定 checkout；release 由
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
3. 在应用详情 → 应用配置中登记：
   - `compose.env`：Compose 插值，内容参考 `compose.env.example`；
   - `postgres.env` 或 `redis.env`：服务级容器 Env，内容参考对应
     `<service>.env.example`；密码使用真实值，禁止提交到仓库。
4. 创建两阶段部署目标：
   - 执行模式：`two_stage`
   - 脚本路径：固定占位路径，例如 `/srv/apps/my-postgres/placeholder`
     （实际由 root executor 固定执行 `make deploy-go-release`）
   - 参数 Schema 使用模板目录中的 `parameter-schema.json`，`modules.x-options`
     只保留 `postgres` 或 `redis`
5. 发起部署。prepare 由低权限 runner 打包 `compose.yaml`、`config/`
   下的应用配置与 manifest；release 由目标节点 root executor 执行：

   ```text
   docker compose config --quiet
   docker compose up -d --remove-orphans
   ```

   然后等待容器进入 `running` 且健康检查通过。

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
