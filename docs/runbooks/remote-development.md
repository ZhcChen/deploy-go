# 独立远端开发环境

## 环境选择与授权

本机负责编辑、格式化、静态检查、单元测试及轻量调试；API/Web 完整装配和浏览器验收默认使用 qfy-test2 的独立 Compose project。远端不可用时可用本机隔离 fixture 临时排障，不能把它报告成远端验收。

**qfy-test2 同时承载 Deploy Go 正式环境。** 正式 systemd 使用回环 30100/30101，运行目录 `/opt/deploy-go`、数据 `/var/lib/deploy-go`、密钥 `/etc/deploy-go` 均不属于开发 project。开发命令不得调用正式 installer、修改 systemd、复制正式库/密钥或接入业务 Agent。旧 `deploy-go-test` 和 `deploy-go-business-temp-*` 容器归属与本栈不同，不因名字相似而接管或清理。

创建、启动、停止和更新远端环境仍需要当前对话对该开发环境具体操作的授权；模板、登记和“开始开发”本身不扩张授权。2026-10-27 用户明确要求参考 Obsidian 模板调整开发环境、注意 qfy-test2 同机冲突，本次创建和合成验证按此执行。正式发布继续单独走 `docs/runbooks/systemd-deployment-production.md`。

## 结构与隔离

栈仅有初始化容器、Rust API（含进程内 worker）和 Web。SQLite 无独立数据库进程，不启动 PG/Redis/NATS。所有容器不发布任何宿主端口；容器内部 30100/8080 可重复，通过独立内部网络连接。浏览器使用 SSH 隧道连接本 project Web 容器地址。

project 为 `deploy-go-<实际分支 slug>-<unit>-<dev|int|accept>-<owner末8位>`，完整 owner 必填。无固定 container_name、外部网络/卷、host network、Docker socket 或宿主目录挂载。网络和卷自动带 project 前缀与完整归属标签；即使同名资源没有 Compose 标签也要拒绝接管。SHA 镜像 tag 含完整 project，不覆盖旧 tag。

API/Web 以 1000:1000 运行，只读根文件系统、cap_drop=ALL、no-new-privileges、临时 /tmp；SQLite/制品/日志在独立 data 卷，合成密钥仅在 keys 卷中生成且只读供 API 使用。初始化容器没有网络，仅有修正卷属主所需权限。禁止卷 driver_opts 间接挂入宿主路径。常驻总上限 1.25 CPU/640MiB，初始化另 0.25CPU/64MiB，每容器 PID 128、Docker 日志 3×10MiB。开发制品最多 64MiB，不继承正式数据或身份。

## 构建和登记

本机已有 `orbstack` context（必须是 Unix socket）；远端 `qfy-test2` context 必须指向 `ssh://qfy-test2`。工具显式固定两个 context，不依赖 docker context use、DOCKER_CONTEXT 或目录推导 project。构建传 Git archive 的干净 HEAD，使用现有 release Dockerfile 缓存；本机构建镜像通过 image save/load 传输，远端不需要访问 Docker Hub，不把本机源码 bind mount 到远端。

```bash
make dev-remote-check
# owner 由当前 CODEX_THREAD_ID 提供；手工运行时显式传完整 session ID。
python3 scripts/dev/remote-compose.py build --owner <完整-session-id> --unit environment --purpose dev --local-port 30880
```

build 要求工作区干净；不会覆盖已有 project 构建记录。镜像身份与配置保存在 `.data/remote-development/<project>/record.json`，Compose 副本和非敏感基础设施配置在同目录。凭据不进入这些文件。统一资源事实源为用户既有 `~/ai-space/server/dev-environments/REGISTRY.md`，build 在远端镜像传输前串行预留归属、SHA、镜像 ID、限额和责任；Obsidian 只维护链接。

执行前核对远端容器、网络、卷、端口、CPU/内存/磁盘与登记。新环境不自动继承旧 owner；交接需明确移交责任并另行核实。不运行全局 prune，不停其他环境腾资源。构建失败保留已生成镜像/登记事实待核，不掩盖成已清理。

## 启动、访问和验收

每个 project 一名写入负责人，工具用本地文件锁串行操作。所有底层 Compose 命令显式传 `--context qfy-test2 -p -f --env-file`，启动前复验配置哈希、渲染后的受限合同、远端镜像 ID 与资源标签。以下命令的 owner/unit/purpose 必须与构建一致：

```bash
python3 scripts/dev/remote-compose.py up --owner <完整-session-id> --unit environment --purpose dev
python3 scripts/dev/remote-compose.py status --owner <完整-session-id> --unit environment --purpose dev
python3 scripts/dev/remote-compose.py tunnel --owner <完整-session-id> --unit environment --purpose dev
```

tunnel 只输出经核对的 SSH 命令，人工运行后访问 `http://127.0.0.1:30880`；SSH 使用 ExitOnForwardFailure，端口占用立即失败，不抢占本机服务。Origin 绑定构建时的本机端口；更换端口须新建经核验的配置，不临时覆盖环境变量绕过哈希。默认不开放公网、不改防火墙。

空库显示首管初始化页；需要账号时只创建本 project 的合成账号，不导入正式身份。真实 Agent、业务发布、Env、迁移或远程命令不在开发页面验收范围。浏览器自动化使用 Playwright 管理的 Chromium，并在退出时关闭页面/context/browser；只做初始化页时不必创建账号。

验收分别记录 API ready、Web/代理、浏览器和专项测试，不能用进程 healthy 替代业务链路。创建、停止及恢复前后核对正式 API/Web 的 PID、健康和 30100/30101 监听，确保开发操作未重启正式服务。记录源 SHA、镜像 ID、Compose/配置哈希、项目归属和迁移事实，禁止存凭据或完整渲染后的敏感配置。

## 停止、恢复和更新

```bash
python3 scripts/dev/remote-compose.py stop --owner <完整-session-id> --unit environment --purpose dev
```

stop 仅停止本 project，保留 data/keys 卷、网络和镜像。再次 up 复用相同卷，初始化校验已有密钥而不轮换。没有 down -v、卷删除或全局清理入口。删除数据/配置需要具体授权、保留策略核对及精确资源归属验证。

更新源码用新的 unit/project 与不可变 SHA 镜像，不覆盖旧镜像或自动迁移旧卷；需保留旧数据时先设计独立兼容/备份步骤。status/stop 不依赖镜像 tag 仍在，但始终核对归属与配置，不因 tag 丢失阻止安全停止自己的环境。
