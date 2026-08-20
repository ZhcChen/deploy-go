# Valkey 9 应用模板

使用 `compose.yaml` 启动 Valkey 9，开启 AOF 持久化，数据保存在命名卷
`valkey-data`，默认监听 `${VALKEY_PORT:-6379}`。`config/valkey.conf` 以只读
方式挂载到容器，`valkey.env` 提供访问密码。根目录 `deploy-go.yaml` 声明
`type: valkey`、`type_version: "9"` 与模板必选 Env。模板不执行
`docker compose down -v`，不会删除持久化数据。

Valkey 是 Redis 协议兼容的开源数据存储，模板可作为 Redis 7 的替代部署，但
业务侧需要确认所使用的客户端版本与 Valkey 9 兼容。

## 接入 Deploy Go

1. 在 Deploy Go 从“Valkey 9”模板创建应用：平台会把 `compose.yaml`、
   `compose.env`、`valkey.env`、`config/valkey.conf` 克隆为应用配置副本。
2. 在应用配置工作区替换 `VALKEY_PASSWORD=change-me` 为强密码并保存；敏感
   文件会重新验证管理员密码后加密保存，并同步到目标节点。
3. 创建镜像直连目标：模板、镜像与宿主端口固定，Env 文件白名单自动来自模板。
4. 发起部署：preview 固化当前配置版本，确认后生成包含配置副本的发布物并
   执行 Compose release。

两阶段 Git 仓库模式仍支持：把本目录内容复制到独立仓库并推送，登记
`compose.env` 与 `valkey.env` 后固定部署分支。

本机预览：

```bash
cp .env.example .env
cp valkey.env.example valkey.env
docker compose up -d
```

发布脚本把发布物中的 `compose.yaml` 与 `config/valkey.conf` 解压到
`/srv/deploy-go-apps/<DEPLOY_TARGET>/releases/<DEPLOY_RELEASE_VERSION>`，
复制 `compose.env` 与 `valkey.env` 后执行：

```text
docker compose config --quiet
docker compose up -d --remove-orphans
```

然后等待容器进入 `running` 且健康检查通过。目标节点需要已安装 Docker Engine
与 Compose v2 插件；发布以 root 执行，因此不需要把用户加入 docker 组。

注意：模板把 `VALKEY_PASSWORD` 用于 `valkey-server --requirepass`，密码会
出现在宿主进程参数与 `docker inspect` 中。生产环境建议改为独立 Secret
管理或使用配置挂载后继续强化。

本地验证（不执行 Docker）：

```bash
bash test-contract.sh
```

镜像直连（image）模式不需要把本目录复制到业务仓库：Deploy Go 的
`container-template` 会嵌入 `compose.yaml`、`config/valkey.conf`、Makefile
与 `scripts/release.sh` 生成固定发布物，修改本目录内容后需运行
`make app-template-check` 与 `cargo test -p deploy-go-container-template`。
