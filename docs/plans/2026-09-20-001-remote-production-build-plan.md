---
title: 在 qfy-test2 远程构建并部署正式控制面
date: 2026-09-20
status: accepted
---

# 背景

当前 `deploy/production/deploy.sh` 在本机执行 Docker 构建，然后把 API、Web、Agent、executor 和 deployer 发布物上传到 `qfy-test2`。本机后续不再安装 Docker，构建必须迁移到正式控制面服务器；`qfy-test2` 同时仍是安装和运行目标。

# 目标与边界

- 默认在 `qfy-test2` 的独立构建根目录内完成源码快照、Web 构建、Rust 多架构构建和发布物校验。
- 运行目录 `/opt/deploy-go`、数据目录 `/var/lib/deploy-go` 与构建目录完全隔离。
- 构建输入固定为本地 Git `HEAD` 的归档快照，不上传工作区未提交文件和未跟踪文件。
- 保留现有双架构 Agent/executor、双架构 deployer、目标架构 API 和 Web 产物要求。
- 构建成功后由同一部署流程安装控制面；失败或中断时清理远程构建目录。
- 不连接、不升级、不重启业务节点 Agent。

不在本次范围内：修改 Agent 升级协议、改变控制面运行目录、修改业务应用部署逻辑、把运行服务放进构建容器。

# 技术方案

## 构建模式

增加远程构建模式，默认启用；保留本地构建模式仅作为显式兼容选项。远程模式使用 `DEPLOY_BUILD_HOST`，默认等于 `DEPLOY_HOST`，因此当前部署默认在 `qfy-test2` 构建并部署。

远程构建步骤：

1. 本地读取 API、Agent、executor、deployer 版本和协议版本，执行一致性校验。
2. 使用 `git archive HEAD` 生成源码快照，避免把本地 dirty worktree、凭证和未跟踪计划文件传到服务器。
3. 通过 SSH 在 `DEPLOY_BUILD_HOST` 的 `/var/lib/deploy-go-builder` 下创建随机、root 保护的构建目录。
4. 上传源码快照和远程构建脚本；远程脚本校验快照中的版本，并使用现有 `deploy/docker/release/Dockerfile` 构建 API、Agent/executor 和 deployer。
5. 在远程构建目录中执行 Web 依赖安装、生产构建和敏感内容扫描。
6. 远程构建脚本将产物写入独立 `output` 目录，并输出构建 commit、版本、架构和 SHA-256 清单。
7. 用 `rsync` 把已校验产物拉回本地随机 staging，再复用现有部署安装事务上传到 qfy-test2 的安装 staging。
8. 无论成功、失败还是中断，清理远程构建目录；构建缓存可使用固定的服务器级 Docker cache，但不得放到运行目录。

## 安全与隔离

- 远程构建根目录固定为 `/var/lib/deploy-go-builder`，构建实例使用随机子目录；目录由 root 创建，禁止其他用户写入。
- SSH 命令只传递不敏感的随机路径和脚本入口；部署参数仍通过现有 `0600 install.env` 传递。
- 不把 `.env`、SSH 凭证、SQLite 数据库、运行目录或完整工作区未跟踪内容纳入源码归档。
- 构建阶段使用独立用户或 root 保护的临时目录，构建完成后删除；不修改 `/opt/deploy-go` 和 `/var/lib/deploy-go`。
- 安装阶段继续使用现有远程安装锁、备份、健康检查和失败回滚。

# 需要修改的文件

- `deploy/production/deploy.sh`：增加远程构建流程、构建主机配置、源码快照上传、产物回收和统一清理。
- `deploy/production/remote-build.sh`：新增远程构建入口，负责 Docker/Rust/Web 构建、版本校验、清单和产物整理。
- `deploy/production/test-install-contract.sh`：补充远程构建路径、失败清理和不上传 dirty 文件的契约检查。
- `Makefile`：更新正式部署目标说明与检查目标，保留显式本地构建兼容入口。
- `deploy/production/README.md`：记录默认远程构建和构建目录生命周期。
- `docs/runbooks/systemd-deployment-production.md`：更新前置条件、流程、环境变量、验证和回滚说明。

# 验证

- `bash -n deploy/production/deploy.sh deploy/production/remote-build.sh`
- `bash deploy/production/test-install-contract.sh`
- 新增的远程构建契约测试：验证源码快照来自 `HEAD`、构建目录随机且不落入运行目录、成功/失败/中断均清理。
- `make deploy-production-check`
- `cargo test -p deploy-go-agent`
- API/OpenAPI 与客户端漂移检查。
- 获得明确部署授权后，在 `qfy-test2` 执行一次 `0.3.5` 远程构建部署；验证 `/healthz`、`/readyz`、Web 首页、API OpenAPI、Agent release manifest 和 systemd 状态。

# 风险与回滚

- `qfy-test2` 必须具备 Docker/buildx、网络代理、足够磁盘和构建权限；缺少时在远程构建预检阶段失败，不进入安装阶段。
- 同机编译可能消耗 CPU、内存和磁盘；构建目录和 Docker cache 必须设置容量监控，不能与运行数据混用。
- 构建失败不应影响现有控制面；只有安装阶段成功并通过健康检查才切换服务。
- 安装失败沿用现有 `install.sh` 事务回滚；远程构建失败仅清理构建目录，不回滚或重启运行服务。
