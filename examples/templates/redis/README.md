# Redis 应用模板

使用 `compose.yaml` 启动 Redis 7，开启 AOF 持久化，数据保存在命名卷
`redis-data`，默认监听 `${REDIS_PORT:-6379}`。模板不执行
`docker compose down -v`，不会删除持久化数据。

## 接入 Deploy Go

1. 把本目录内容复制到独立 Git 仓库并推送。
2. 在 Deploy Go 应用 Env 中登记 `compose.env`，字段参考
   `compose.env.example`，至少设置 `REDIS_PASSWORD`。
3. 配置应用 Git 来源并固定部署分支。
4. 创建两阶段部署目标并开启 `privileged_release`；脚本路径填写固定占位路径
   （实际由 root executor 固定执行 `make --no-print-directory deploy-go-release`）。
5. 目标参数 Schema 使用 `parameter-schema.json` 的内容。

发布脚本把发布物中的 `compose.yaml` 解压到
`/srv/deploy-go-apps/<DEPLOY_TARGET>/releases/<DEPLOY_RELEASE_VERSION>`，
复制 `DEPLOY_ENV_DIR/compose.env` 后执行：

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
