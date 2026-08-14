# PostgreSQL 应用模板

使用 `compose.yaml` 启动 PostgreSQL 18，数据保存在命名卷 `postgres-data`，
默认监听 `${POSTGRES_PORT:-5432}`。`config/postgresql.conf` 以只读方式挂载到
容器，`postgres.env` 提供数据库名、用户与密码。根目录 `deploy-go.yaml` 声明
`type: postgres`、`type_version: "18"` 与模板必选 Env。模板不执行
`docker compose down -v`，不会删除持久化数据。

## 接入 Deploy Go

1. 把本目录内容复制到独立 Git 仓库并推送。
2. 在 Deploy Go 应用 Env 中登记 `compose.env`，字段参考
   `compose.env.example`；再登记 `postgres.env`，字段参考
   `postgres.env.example`，至少设置 `POSTGRES_PASSWORD`。
3. 配置应用 Git 来源并固定部署分支。
4. 创建两阶段部署目标；平台固定使用 Agent 原生特权 release，脚本路径填写固定
   占位路径（实际由 root executor 固定执行
   `make --no-print-directory deploy-go-release`）。
5. 目标参数 Schema 使用 `parameter-schema.json` 的内容。

本机预览：

```bash
cp .env.example .env
cp postgres.env.example postgres.env
docker compose up -d
```

发布脚本把发布物中的 `compose.yaml` 与 `config/postgresql.conf` 解压到
`/srv/deploy-go-apps/<DEPLOY_TARGET>/releases/<DEPLOY_RELEASE_VERSION>`，
复制 `compose.env` 与 `postgres.env` 后执行：

```text
docker compose config --quiet
docker compose up -d --remove-orphans
```

然后等待容器进入 `running` 且健康检查通过。目标节点需要已安装 Docker Engine
与 Compose v2 插件；发布以 root 执行，因此不需要把用户加入 docker 组。

本地验证（不执行 Docker）：

```bash
bash test-contract.sh
```

镜像直连（image）模式不需要把本目录复制到业务仓库：Deploy Go 的
`container-template` 会嵌入 `compose.yaml`、`config/postgresql.conf`、Makefile
与 `scripts/release.sh` 生成固定发布物，修改本目录内容后需运行
`make app-template-check` 与 `cargo test -p deploy-go-container-template`。
