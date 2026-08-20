# Deploy Go 应用模板

本目录提供 Docker Compose 应用模板，同时服务两条接入路径：

- **Git 两阶段**：把模板目录复制到独立业务仓库，业务仓库继续负责
  `compose.yaml`、环境文件与发布脚本；模板遵守
  `docs/standards/application-deployment-contract.md`，Deploy Go 不接管
  Compose 文件。
- **镜像直连（image）**：不需要业务仓库。Deploy Go 的 `container-template`
  crate 通过 `include_str!` 嵌入本目录的 `compose.yaml`、`config/`、Makefile
  与 `scripts/release.sh`，在平台侧生成固定发布物；修改模板文件后必须同步
  运行模板契约与 Rust 测试，保证两套来源一致。

当前模板：

- `postgres/`：PostgreSQL 18，持久化数据卷、健康检查与 Compose Env 插值。
- `redis/`：Redis 7，AOF 持久化、健康检查与 Compose Env 插值。
- `valkey/`：Valkey 9，AOF 持久化、健康检查与 Compose Env 插值。

每个模板包含：

- `compose.yaml`：容器编排定义。
- `compose.env.example`：注册到 Deploy Go 应用 Env 的模板；复制为
  `compose.env` 后登记，正式值不要提交到仓库。
- `<service>.env.example`：服务级容器 Env，例如 `postgres.env` / `redis.env` / `valkey.env`。
- `config/`：对应应用的配置文件，例如 `postgresql.conf` / `redis.conf` / `valkey.conf`。
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
模板预填应用与部署目标。向导支持「镜像直连（无需仓库）」与「Git 两阶段」
两种方式：镜像模式直接创建特权镜像目标，Git 模式仍需把模板文件复制到独立
仓库审查后再部署。向导不会上传 Env 明文。
