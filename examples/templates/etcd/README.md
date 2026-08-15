# etcd 应用模板

此模板以 Docker Compose 启动单节点 etcd 3.6，数据保存在命名卷 `etcd-data`，
客户端端口默认只绑定节点回环地址 `${ETCD_CLIENT_PORT:-2379}`。`etcd.env` 声明
成员名、初始集群和自动压缩参数；根目录 `deploy-go.yaml` 声明 `type: etcd`、
`type_version: "3.6"` 与模板必选 Env。模板不会执行 `docker compose down -v`，
不会删除持久化数据。

## 适用范围

这是开发、测试或可接受 etcd 暂时不可用场景的单节点模板。它不启用 TLS、认证或
RBAC，且仅将 client URL 发布到 `127.0.0.1`，不能作为生产配置中心或生产分布式
锁集群。生产场景必须使用独立三节点拓扑、mTLS、认证/RBAC、备份恢复与监控方案，
不要通过修改本模板暴露未加密的 client 或 peer 端口。

配置值、分布式锁和租约不应承载密码、令牌或私钥；敏感值继续使用 Deploy Go 的加密
Env/secret 文件边界。

## 接入 Deploy Go

1. 把本目录内容复制到独立 Git 仓库并推送。
2. 在 Deploy Go 应用 Env 中登记 `compose.env`，字段参考
   `compose.env.example`；再登记 `etcd.env`，字段参考 `etcd.env.example`。
3. 配置应用 Git 来源并固定部署分支。
4. 创建两阶段部署目标；平台固定使用 Agent 原生特权 release，脚本路径填写固定
   占位路径（实际由 root executor 固定执行
   `make --no-print-directory deploy-go-release`）。
5. 目标参数 Schema 使用 `parameter-schema.json` 的内容。

本机预览：

```bash
cp compose.env.example compose.env
cp etcd.env.example etcd.env
docker compose up -d
ETCDCTL_API=3 etcdctl --endpoints=http://127.0.0.1:2379 endpoint health
```

发布脚本将发布物中的 `compose.yaml` 与 `deploy-go.yaml` 解压到
`/srv/deploy-go-apps/<DEPLOY_TARGET>/releases/<DEPLOY_RELEASE_VERSION>`，复制
`compose.env` 与 `etcd.env` 后执行：

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
`container-template` 会嵌入 `compose.yaml`、Makefile 与 `scripts/release.sh`
生成固定发布物。镜像直连同样固定仅绑定 `127.0.0.1`；修改本目录内容后需运行
`make app-template-check` 与 `cargo test -p deploy-go-container-template`。
