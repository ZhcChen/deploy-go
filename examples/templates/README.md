# Deploy Go 应用模板

本目录提供可直接复制到独立业务仓库的 Docker Compose 应用模板。模板遵守
`docs/standards/application-deployment-contract.md`：Deploy Go 不接管
Compose 文件，业务仓库继续负责 `compose.yaml`、环境文件与发布脚本。

当前模板：

- `postgres/`：PostgreSQL 16，持久化数据卷、健康检查与 Compose Env 插值。
- `redis/`：Redis 7，AOF 持久化、健康检查与 Compose Env 插值。

每个模板包含：

- `compose.yaml`：容器编排定义。
- `compose.env.example`：注册到 Deploy Go 应用 Env 的模板；复制为
  `compose.env` 后登记，正式值不要提交到仓库。
- `<service>.env.example`：服务级容器 Env，例如 `postgres.env` / `redis.env`。
- `config/`：对应应用的配置文件，例如 `postgresql.conf` / `redis.conf`。
- `.env.example`：本机 `docker compose up` 预览用。
- `parameter-schema.json`：部署目标参数 Schema，可直接粘贴到目标编辑器。
- `Makefile`：固定 `deploy-go-prepare` / `deploy-go-release`。
- `scripts/prepare.sh`：生成发布物与 artifact manifest。
- `scripts/release.sh`：校验 manifest、Compose 配置，执行
  `docker compose up -d --remove-orphans` 并等待健康。
- `test-contract.sh`：本地契约测试，不执行 Docker 或真实节点操作。

部署步骤和节点要求见 `docs/runbooks/application-templates.md`。

## 管理端入口

管理员可在 Deploy Go Web 的「应用模板」页点击「从模板创建应用」，使用本目录
模板预填应用、Git 来源与两阶段部署目标。向导只编排现有 API，不会上传 Env
明文，也不会创建业务 Git 仓库；模板文件仍需复制到独立仓库审查后再部署。
