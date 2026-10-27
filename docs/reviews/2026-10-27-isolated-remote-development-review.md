# 独立远端开发环境验收（2026-10-27）

## 结果与边界

独立 project `deploy-go-main-environment-dev-70a2bc8f` 在 qfy-test2 保留运行，API/Web healthy，初始化容器退出 0。正式环境也在本机：开发创建、停止、恢复前后，正式 API PID `3638112`、Web PID `3638118`、active 状态与 ready 响应精确对照一致。正式版本 0.3.33 已另外按正式发布流程完成，见 `docs/reviews/2026-10-27-privileged-release-artifact-handoff-review.md`。

owner 为 `01a17b1e-aa08-7b70-82b6-0b4970a2bc8f`，checkout 为本仓库 main。运行镜像源码固定 `8d2755b12484041d77235ae31d513da7086300bb`；之后的主机工具修正截至 `1994aef`，没有更改运行镜像。文档提交不触发重建。

## 隔离与镜像证据

- 零宿主发布端口，无固定 container_name、宿主 bind mount、Docker socket 或正式数据/密钥挂载；独立 internal 网络 `10.245.124.48/28`，独立 data/keys 具名卷。
- API/Web 1000:1000、只读根、cap_drop ALL、no-new-privileges；常驻上限 1.25 CPU / 640MiB，初始化另 0.25 CPU / 64MiB。
- 远端镜像 ID `sha256:d2f5ceadf2a36a2cd87b6cf8d8a2eba68f8fd39727dcff31cf97cf01c90f473a`；本机/归档配置 ID `sha256:9c6019a5aeaf055cbbb5e02c812c0d4c1a105abd2a29b33545e7388e9ecffafd`。
- 跨引擎有效内容指纹 `sha256:bea4c4af5f2101da5181d613f233f3c3f8f177b15d5317297e5353e72d98febb`，绑定完整非 null Config、RootFS、OS/架构/Variant；远端实际 ID 与内容同时核验。
- Compose SHA256 `7fef1319604346f350868387e63d59d44d977630a5c7337e5c7141344d9ab7d2`；非敏感 config.env SHA256 `98f8ae792bbf324d5f39b76f203f87ae4d2bb90d81080f0ac1cfbaf73139cfc4`。

## 验证

真实 API ready、Web 代理 OpenAPI 200 / 0.3.33；Playwright 管理的 Chromium 打开 SSH 隧道入口 `/setup`，可见“创建管理员”和密码表单，pageErrors=0，未提交首管表单。浏览器/context 已关闭；验收隧道已关闭，本机 30880 无监听。开发容器保留运行，使用 `make dev-remote-tunnel` 获取再次访问命令。

执行 stop 后全部容器退出且两个卷保留，再 up 成功恢复。恢复前后密钥 inode/mtime/属主/权限/大小，以及用户/Agent/部署数量、migration 状态精确一致；data 可写、keys 不可写。现场 Agent 和部署数量均为 0；未导入正式数据、接入业务 Agent 或重试业务部署。此项验收不覆盖登录后管理功能、真实节点发布或升级。

本机证据位于 `.data/remote-development/deploy-go-main-environment-dev-70a2bc8f/evidence/`：browser.json、setup.png、volumes-before.json、volumes-after.json、isolation.json；均为忽略的现场证据，不存密钥正文。统一资源事实源是用户既有 `~/ai-space/server/dev-environments/REGISTRY.md`；Obsidian 只做协调索引。

## 失败迭代与复核

初始包索引下载失败，改用自带 Python 的明确版本运行基镜像；经典 Docker 导入重写历史 JSON，改为同时校验完整有效内容和远端实际 ID；默认 Docker 地址池耗尽，新增核对全部网络、主机路由及登记后分配不冲突 /28，不调整 daemon 或清理他人资源。

只读复核指出默认 context 漂移、无标签同名卷接管、driver_opts/use_api_socket 越界、tag 覆盖和重试未复验子网等问题，均已修正并加入回归；最终 findings=[]。该复核不是 Claude Code 独立复核。

最终门禁：`make dev-remote-check` 12 项；Spec Kit converge、`make spec-kit-verify` 13 项与 `make whitespace-check`。API/Agent 生产源码未变，本轮不重复全 workspace 门禁。
