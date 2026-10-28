# 特权发布制品交接修复复核与验证

范围：`specs/006-privileged-release-artifact-handoff/`。实施基线 `6e8657c`，当前写入者完成实现和验证；另外的只读对抗复核者检查授权、恢复、事务、历史任务和配置边界，不修改文件。本记录不将实现者自审称为独立复核。

## 修复范围

旧单目标 Git prepare 统一携带上传授权，release 必须从 verified 记录取得制品摘要和下载 lease；同节点不例外。预览、确认和调度均对禁用传输返回稳定错误。首次投递校验任务、快照、Agent、节点、target run、制品与 lease；已投递任务保留 ACK 丢失恢复路径。历史缺下载 queued/delivered 任务失败化，不重写 payload/digest，不误伤已接单/运行/取消任务。外部接口精确公开错误码，未知仍隐藏。

没有修改业务应用、migration、线协议、公开 DTO、生成产物或版本号。`api/Cargo.toml` 仅新增已有 executor crate 的测试依赖（关闭默认 feature），用于类型化模拟 IPC。

## 只读复核

复核者 `/root/handoff_review` 第一轮发现四项并已修复、补回归：

1. claim 为 canceling 后 API 退出可能永久等待：新增限定错误码的恢复 sweep。
2. 多目标传输禁用可能影响已运行目标：仅失败未调度 run。
3. 已投递 legacy prepare 的 ACK 丢失可能被误判：以 delivered_at 限定首次投递门禁。
4. 镜像制品过期/摘要失配可能循环 500：事务回滚后稳定失败，无半成品 release。

最终只读复核返回 `findings=[]`、`testing_gaps=[]`。按 payload 撤销 lease 依赖平台原子生成的可信任务；当前没有发现正常创建路径可产生跨任务授权引用。

## 验证

- 修复前单目标 Git 上传授权回归失败，修复后通过。
- `cargo test -p deploy-go-api --lib artifact_handoff_tests`：2/2 通过；真实 dispatcher、本地 HTTP、Agent 上传/下载与签名准入，对接合成 executor，同节点/不同节点各一次 release；另验证多目标运行保护。
- `cargo test -p deploy-go-api --test two_stage_deployment --test agent_end_to_end`：36/36 通过。
- `make privileged-release-check`：通过，含 Docker cgroup v2、运行身份隔离、授权/协议/Agent/executor/API、OpenAPI/客户端一致性及管理面发布流程 19 项测试。
- `make client-sensitive-check`：4 个 canary 自检、185 个客户端文件扫描通过。
- Spec Kit converge 结构检查和 `make spec-kit-verify`（13 项）通过。
- `make api-check`：最终退出 0，fmt/clippy、957/957 workspace 测试、doc-tests、OpenAPI 校验通过，2 项既有跳过。nextest 对既有 `empty_database_reaches_agent_deployment_and_resumable_sse_without_ssh` 标记 1 leaky（通过但子进程持有输出句柄的非阻断提示）。

第一次完整门禁的既有 `runtime_log_ingest::durable_append_before_database_commit_is_recovered_before_rotation` 出现 `runtime_log_writer_busy`；该测试释放后台文件线程后立即重新打开存在时序窗口，本轮没有修改相关实现。单独复查和完整门禁重跑均通过。

测试迭代失败来自 fixture 缺 verified_at、目标镜像 CHECK 组合不完整，以及试图修改 verified 不可变摘要，均通过补全合法事实/模拟历史坏绑定修正，没有更改数据库约束。坏任务不投递，Agent 既有缺下载拒单用例证明无 executor 连接。

## 现场边界与交接

本轮未连接真实节点、读取现场配置或 Env、发布组件、升级节点、重试/取消业务部署，也未修改任何业务应用。历史现场版本与配置仍未归因，T011 保持未完成。恢复与后续授权要求见 `docs/runbooks/deployment-recovery.md` 和 `docs/runbooks/external-deploy-api.md`。

本地代码可通过独立 revert 回退；不手改任务状态、不删除制品/lease/journal、不复活 failed 部署。提交后以 Git HEAD 和工作区状态为交接事实。

## 正式环境发布（2026-10-27，后续明确授权）

用户随后明确要求“部署正式环境”。已核对 qfy-test2 身份为历史记录中的 DESKTOP-H0KSULB、x86_64，API/Web 发布前健康，无活动部署、任务或 pending/installing 升级。`make deploy-production-check` 通过。

默认远程构建因 Docker Hub 访问超时失败，未进入安装；按正式 runbook 使用 `DEPLOY_BUILD_MODE=local make deploy-production` 构建上传，最终退出 0。发布源码为 `7751015d00d305fe121abd5c4e8820f28e058ca6`，版本保持 0.3.33。安装器完成备份、发布包校验、服务重启和健康检查；API 启动恢复约两分钟，期间未监听，随后在安装器 180 秒等待窗口内 ready，没有回滚或重启循环。

发布后 API/Web active，readyz 成功，正式域名首页、internal/external OpenAPI 均 HTTP 200。安装 API SHA-256 为 `c3e8028e640af103df3804da3e913752ed2bd47b8ea5b8a0f0f5230feba3d8b2`，已核对二进制包含本次缺上传授权门禁。已有三个 Agent 报告 0.3.33，未手动重启或升级业务节点。

此次授权不包含业务应用重试：未重发、取消或新建业务部署。T011 中组件发布已完成，具体业务交接验收和历史现场归因仍未完成。
