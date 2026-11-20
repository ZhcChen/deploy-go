# 本机编辑、qfy-test2 构建测试实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

- 通用协议：`remote_sync.py` 协议版本 1。本机生成排除后的源码快照与清单（`snapshot_id`），经 SSH 在远端安装同摘要的工具包（协议、合同、适配器），rsync 单向增量同步到 `source/`，远端校验清单一致后在操作锁内执行适配器；结果、日志与 `runtime-result.json` 写入 `evidence/<run_id>/`，取回本机 `tmp/remote-dev/<run_id>/`。
- 适配器在 qfy-test2 上以 root 执行，经本机 Docker 套接字（私有客户端配置、显式 `qfy-test2` context）启动一次性容器。
- 现场（2026-11-20 只读）：20 核、可用内存约 36 GiB、磁盘约 933 GiB 可用；`dev-shared-network` 存在（`0414fd05…`，10.255.0.0/24）；Docker 有 `docker.1ms.run`、`docker.m.daocloud.io` 加速；`/opt/qfy-dev/deploy-go` 不存在；`rust:1.99.0-bookworm` 与 `node:22-alpine` 可经加速拉取。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md) 第 3.1 节与远端开发手册：开发容器不发布宿主端口，不挂载正式目录或密钥，不接入业务 Agent，不接管旧环境；Docker 命令显式 context；创建前登记并核对资源。本次授权来自用户当前对话的明确要求，范围为建立目录、同步源码、构建工具镜像、运行测试容器；不含删卷、prune、正式部署或更改已有远端开发栈。

## 实现与文件范围

1. `scripts/dev/remote_sync.py`：原样复制（正文逐字节一致）。
2. `scripts/dev/remote-test.json`：合同（项目、主机、根目录、适配器、排除与模板放行、超时）。
3. `scripts/dev/remote-test.py`：薄入口，映射 Makefile 目标到 `test`、`dry-run`、`verify`、`status`。
4. `scripts/dev/deploy_go_remote_runtime.py`：命令白名单、工具镜像构建（经典 builder、共享网络）、缓存目录、容器启动参数、Rust 与 Web 步骤、证据。
5. `deploy/dev-tools/Dockerfile.rust`、`Dockerfile.node`。
6. `Makefile`：`remote-*` 目标与帮助；`.gitignore`：`/tmp/`（本机结果目录）。
7. `scripts/dev/test_remote_test.py`：白名单、合同、镜像固定、资源上限的本地回归。
8. 文档：`docs/runbooks/remote-development.md` 新增“远端构建与测试”，`docs/runbooks/development-guide.md` 与 `AGENTS.md` 更新默认执行位置；登记 021；`REGISTRY.md` 新增条目。

## 验证计划

- 本地：`make dev-remote-check`（含新回归）、`make docs-check`、`make whitespace-check`、`make spec-kit-verify`。
- 远端：`make remote-dry-run`（同步预览）；`make remote-test REMOTE_COMMAND=rust-fmt`（含首次镜像构建）；`make remote-check`（Rust 全量与 Web 全量），对照本机用例数；运行前后核对正式 API/Web PID 与健康、残留容器、网络数量。

## 风险与回退

- 回退：删除入口文件即可回到本机执行；远端目录与缓存保留（清理需授权，只删 `/opt/qfy-dev/deploy-go` 下本项目内容与本项目工具镜像）。
