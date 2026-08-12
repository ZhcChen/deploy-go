# Redis 应用模板

使用 `compose.yaml` 启动 Redis 7，开启 AOF 持久化，数据保存在命名卷
`redis-data`，默认监听 `${REDIS_PORT:-6379}`。`config/redis.conf` 以只读方式
挂载到容器，`redis.env` 提供访问密码。根目录 `deploy-go.yaml` 声明
`type: redis`、`type_version: "7"` 与模板必选 Env。模板不执行
`docker compose down -v`，不会删除持久化数据。

## 接入 Deploy Go

1. 把本目录内容复制到独立 Git 仓库并推送。
2. 在 Deploy Go 应用 Env 中登记 `compose.env`，字段参考
   `compose.env.example`；再登记 `redis.env`，字段参考
   `redis.env.example`，至少设置 `REDIS_PASSWORD`。
3. 配置应用 Git 来源并固定部署分支。
4. 创建两阶段部署目标并开启 `privileged_release`；脚本路径填写固定占位路径
   （实际由 root executor 固定执行 `make --no-print-directory deploy-go-release`）。
5. 目标参数 Schema 使用 `parameter-schema.json` 的内容。

本机预览：

```bash
cp .env.example .env
cp redis.env.example redis.env
docker compose up -d
```

发布脚本把发布物中的 `compose.yaml` 与 `config/redis.conf` 解压到
`/srv/deploy-go-apps/<DEPLOY_TARGET>/releases/<DEPLOY_RELEASE_VERSION>`，
复制 `compose.env` 与 `redis.env` 后执行：

```text
docker compose config --quiet
docker compose up -d --remove-orphans
```

然后等待容器进入 `running` 且健康检查通过。目标节点需要已安装 Docker Engine
与 Compose v2 插件；发布以 root 执行，因此不需要把用户加入 docker 组。

注意：模板把 `REDIS_PASSWORD` 用于 `redis-server --requirepass`，密码会出现在
宿主进程参数与 `docker inspect` 中。生产环境建议改为独立 Secret 管理或使用
配置挂载后继续强化。

本地验证（不执行 Docker）：

```bash
bash test-contract.sh
```

镜像直连（image）模式不需要把本目录复制到业务仓库：Deploy Go 的
`container-template` 会嵌入 `compose.yaml`、`config/redis.conf`、Makefile 与
`scripts/release.sh` 生成固定发布物，修改本目录内容后需运行
`make app-template-check` 与 `cargo test -p deploy-go-container-template`。
