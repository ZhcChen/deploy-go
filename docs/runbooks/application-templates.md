# Docker Compose 应用模板接入 Runbook

## 目标

管理员快速接入 PostgreSQL、Redis 等基础中间件，使用 Docker Compose 在两阶段
部署目标上发布。模板只提供业务仓库骨架；Compose、Env 与发布脚本仍然由业务
仓库维护和审查，Deploy Go 不解析或接管 `compose.yaml`。

管理端「应用模板」页面（`/templates`）可只读查看模板的 Compose、Env 示例、
应用配置与参数 Schema；页面不提供直接创建应用或复制文件的入口，正式接入仍按
下方步骤复制到独立 Git 仓库。

## 前置条件

- Deploy Go API / Web 0.2.0 以上，目标节点 Agent 0.2.0、控制协议 v7、
  executor v2，并且目标节点 `PRIVILEGED_RELEASE` capability 可用。
- 目标节点已安装 Docker Engine 与 Compose v2 插件；发布脚本以 root 运行，
  不需要把 `deploy-go-agent` 加入 docker 组。
- 管理员已准备独立 Git 仓库（模板不允许直接在 `deploy-go` 仓库部署）。

## 步骤

1. 复制模板到业务仓库：

   ```bash
   cp -R examples/templates/postgres /srv/git/my-postgres
   cd /srv/git/my-postgres
   git init
   git add .
   git commit -m "init postgres template"
   ```

2. 在 Deploy Go 中创建应用并配置 Git 来源，固定部署分支。
3. 在应用 Env 中登记：
   - `compose.env`：Compose 插值，内容参考 `compose.env.example`；
   - `postgres.env` 或 `redis.env`：服务级容器 Env，内容参考对应
     `<service>.env.example`；密码使用真实值，禁止提交到仓库。
4. 创建两阶段部署目标：
   - 执行模式：`two_stage`
   - 脚本路径：固定占位路径，例如 `/srv/apps/my-postgres/placeholder`
     （实际由 root executor 固定执行 `make deploy-go-release`）
   - 开启 `privileged_release` 并完成 root 信任确认
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
- 开启 `privileged_release` 意味着该仓库和固定分支的写入者获得目标节点 root
  发布能力；仓库 URL、分支、节点变化后必须重新确认。
- 模板不执行 `eval`、`sudo docker` 或 `docker compose down -v`。
- Redis 模板的密码会出现在宿主进程参数与 `docker inspect` 中，仅适合内部
  测试；生产环境应改为独立 Secret 管理后再接入。
