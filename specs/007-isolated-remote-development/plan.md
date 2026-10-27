# 独立远端开发实施方案

状态：完成；规格：spec.md。

## 设计与范围

新增 deploy/development/ 的 Compose、运行镜像和卷内初始化脚本，以及 scripts/dev/remote-compose.py 的受控入口。API 与 Web 共用按 SHA 构建的工具镜像，SQLite/制品/日志在 data 卷，密钥在独立 keys 卷，低权限服务运行；不增加 PostgreSQL/Redis/NATS。所有服务限额、只读根和受限网络；默认不发布任何宿主端口，SSH 隧道连接 project Web 容器地址。

构建复用 deploy/docker/release/Dockerfile 的现有缓存，源码来自干净 Git archive；不改正式发布入口。基础镜像明确版本，运行镜像 SHA tag 与 image ID 入登记。生成配置仅含非敏感标识，合成密钥在初始化容器内部生成。

工具只支持本 project 的 build/up/status/stop/tunnel；不提供全局清理、卷删除、业务 Agent 接入或正式服务控制。创建前核验登记和 Docker 归属；复用必须匹配 owner/分支/源码/镜像。配置与证据位于 gitignore 的 .data/remote-development，统一登记位于用户既有 ai-space/server/dev-environments/REGISTRY.md，Obsidian 只链接。

## 验证

离线 unittest 覆盖非法名称、context/owner、越界挂载、端口和固定名称等；docker compose config 渲染审查；qfy-test2 上合成空库启动、ready、Web 代理、Playwright 首管页面，停止保卷/恢复；正式 PID/健康不变。运行 make whitespace-check、Spec Kit 检查与匹配开发契约入口。API/Agent 源码不变，不重复全 workspace 门禁。

## 发布与恢复

先提交运行配置以固定源码 SHA，再构建、登记和创建独立 project；只读复核工具与隔离边界后修正复验。失败保留本 project 事实，不停止别人服务。停止保留卷，镜像更新需记录旧 ID 与数据兼容性；卷/远端配置删除另需具体授权。
