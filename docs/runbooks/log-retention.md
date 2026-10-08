# 容量日志保留与集中诊断

## 范围

本手册对应 `specs/002-bounded-log-retention/`。部署输出和组件诊断使用独立预算，不采集业务应用日志，不读取全局 journal，也不调整系统 journald 或代理。新组件诊断从新版组件重启后开始记录；历史任务日志沿用原来的可靠交付与保护规则，不执行一次性盲删。

## 容量与权限

| 对象 | 默认限制 | 回收规则 |
| --- | --- | --- |
| 控制面部署输出 | 总计 2 GiB，单部署 50 MiB | 按结束时间回收完整交付的终态部署，计入展示输出和任务输出副本 |
| 控制面组件诊断 | 32 MiB 分段，最多 16 段，总计 512 MiB | 覆盖最旧分段，包含全部节点和 API 的受控诊断 |
| 节点组件诊断 | 四组件各 16 MiB 分段、最多 2 段，总计 128 MiB | 覆盖最旧分段，上送时报告已丢失区间 |

分段预算预留少量持久元数据，不因轮转复用序号。部署输出总额是稳态目标：活动或交付不完整任务会受到保护，可能暂时超额。辅助保留天数仍默认 30 天，不批量更改已有配置。SQLite 有效载荷预算不等于数据库和 WAL 文件大小，不在线执行全库 VACUUM。

节点固定树为 `/var/lib/deploy-go-agent-runtime-logs`（`root:deploy-go-agent 0750`）：`agent/` 为 Agent 所有 `0700`，文件 `0600`；`runner/`、`executor/`、`updater/` 为 root 所有 `0750`，文件 `0640`，Agent 可读但不可写。业务身份 `deploy-go-runner` 无权访问这棵树。

0.3.28的 `/var/log/deploy-go-agent` 在Ubuntu共享目录 `root:syslog 0775` 下被安全检查拒绝，因此0.3.29改用上述独立树。不要修改 `/var/log` 权限来适配日志功能，也不要把日志树放进Agent可写的数据目录。旧树保留且不采集；新树产生新epoch，上传游标自动从新epoch的0开始，不手工清空游标。已发布0.3.28的正常节点继续按串行队列自动更新0.3.29。

首次安装器准备目录；Runner Broker unit 的 `ExecStartPre` 同步执行 `deploy-go-agent prepare-runtime-logs`，在主进程启动前准备固定目录，兼容首轮自动升级仍由旧 updater 执行。新版 Broker/Executor 启动也检查目录。正确权限只校验、不重复改属主。Agent/updater 仅增加自己目录的可选 systemd `ReadWritePaths=-/var/lib/deploy-go-agent-runtime-logs/<组件>`，目录缺失不会阻止服务启动。目录准备或日志持久化失败只降级诊断，不中止安装、升级或任务通道；采集游标读取失败每60秒重试。

控制面生产目录为 `/var/lib/deploy-go/runtime-logs`，通过 `DEPLOY_GO_RUNTIME_LOG_DIR` 配置；目录和分段只属于控制面运行用户。未配置时本地 fixture 可使用内存模式。

## 查询与诊断

管理员在运行日志页面按节点 ID、组件、级别和 Request ID 筛选。组件诊断只保留固定事件、关联 ID、错误分类和数值；原始错误链、URL、凭证、业务 stdout 和 Env 正文不进入这个通道。业务部署输出仍使用部署详情、诊断和日志接口。

Agent 通过独立 `/api/v1/agent/runtime-logs` HTTP 接口上送，控制面按 access token 绑定来源。持久水位独立于可轮转日志，重复请求不重复保存。只有持久确认后才更新本地上传游标；网络失败退避，旧 API 返回 404 时延长退避，不影响部署任务。

检查节点容量和权限（只读）：

```bash
sudo du -sh /var/lib/deploy-go-agent-runtime-logs/*
sudo find /var/lib/deploy-go-agent-runtime-logs -maxdepth 2 -type f -printf '%u:%g %m %s %p\n'
sudo -u deploy-go-agent /usr/local/bin/deploy-go-agent doctor
```

检查控制面容量（只读）：

```bash
sudo du -sh /var/lib/deploy-go/runtime-logs
sudo sqlite3 -readonly /var/lib/deploy-go/deploy-go.db 'SELECT total_bytes FROM deployment_log_capacity WHERE id=1;'
sudo sqlite3 -readonly /var/lib/deploy-go/deploy-go.db 'SELECT agent_id,component,sequence,central_sequence FROM runtime_log_source_watermarks ORDER BY agent_id,component;'
```

普通诊断磁盘故障、队列溢出和已覆盖缺口应从采集统计及组件事件查看；页面分别显示队列丢弃和持久化失败次数。不能由诊断日志缺失推断任务失败。任务 receipt 证明控制面完整接收，不证明 ACK 已到达节点；节点仍只按实际 ACK 回收可靠部署输出。

损坏处理仅允许修复组件分段中可证明的未完成尾记录；完整 JSON 损坏、读盘错误、符号链接和未知目录布局停止采集，保留文件。持久序号不回退，尾部缺口之后开启新段。不要手工清零水位或上传游标，也不要删除 task journal、receipts、结果和未确认 outbox。

## 验证与回滚

本机执行 shared runtime-log 测试、Agent/API 聚焦回归及安装器契约；Linux 容器显式执行真实身份测试：

```bash
cargo test -p deploy-go-runtime-log
cargo test -p deploy-go-agent --lib
make agent-install-check
```

`runtime-log/tests/linux_permissions.rs` 的 ignored 测试只允许 `/.dockerenv` 存在、root 身份和 `DEPLOY_GO_LOG_PERMISSION_FIXTURE=1` 的隔离容器执行；容器中准备 UID/GID 1001 的 `deploy-go-agent` 和 1002 的 `deploy-go-runner` 后执行 `cargo test -p deploy-go-runtime-log --test linux_permissions -- --ignored`。不能在真实节点运行该 fixture。

按正式部署手册回滚上一版成对发布物，保留新日志树、新 migration、来源水位及任务数据。旧 Agent 无集中诊断仍可部署，旧 API 下新 Agent 会退避。不要删除或改写已应用的 migration。自动升级仍串行且只包含正常节点；日志功能不增加业务应用自动重发行为。

发布包含新增 migration `0040_deployment_log_capacity.sql` 和 `0041_runtime_log_source_watermarks.sql`。正式切换前使用 SQLite backup API 生成权限受限的一致性备份，并记录当前最高版本；发布后检查新版本均 `success=1`、计数与输出字节汇总一致、来源水位开始增长。旧版本允许保留高版本 migration，正常回滚不回退数据库，避免丢失切换后的部署记录。日志有效载荷回收会删除已完整交付的终态输出，历史正文无法凭切回旧二进制恢复；需要历史正文时只能从备份离线只读查询，不覆盖当前运行数据库。
