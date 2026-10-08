---
title: 分支发现 journal 初始化回归复核
date: 2026-10-08
status: completed
---

# 分支发现 journal 初始化回归复核

## 根因与范围

0.3.24 在 `JournalStore::create` 中为可靠日志先调用 `create_dir_all`，
继承 setgid 父目录与 UMask=0007 后得到 2770。随后 store 尝试修正为 3700，
与 systemd RestrictSUIDSGID 冲突，sidecar 已存在但 journal 不存在。
四个初始化残留占满 512 MiB 准入预留，Broker 启动将其当作不可信活动目录拒绝。
六次外部请求均在内部记录 journal_error，经错误归一化后表现为 git_ref_discovery_failed。

应用分支仍为 test，构建 Agent 在线；没有凭证失效证据。本轮不修改业务应用、
不重发业务部署，也不清理历史任务或日志。

## 修复与保护

- 先按 3710/3700 创建根目录与任务目录，再初始化可靠日志，避免补设 setgid。
- 仅受管 uid/gid、2770/3700、空目录或唯一 schema=1 初始零状态 sidecar 才可免计预留。
- Broker 仅跳过可信 2770 初始化残留；3770 活动目录及包含进程/spec/输出的目录不豁免。
- 不删除或修改残留文件，旧失败 task 不自动重跑；现有任务与未确认日志保持保护。
- sidecar 使用 no-follow/nonblock 文件描述符、普通单链接文件校验和限长读取；无法证明安全时不豁免。
- 复用未提交 sidecar 重新准入；损坏 JSON 保守占预留，不阻断其他任务扫描。
- 分支发现保留 journal_error 与 node_log_spool_budget_exceeded，节点记录安全的初始化错误链。
- sparse checkout 门槛固定 v16，不随最新协议升高而拒绝兼容节点。

准入依赖单 Agent 及 Executor 共享 admission_lock，不新增多进程调度。
残留识别只描述当前可见布局，不证明目录历史；不据此授权删除或复用活动任务。

## 测试与复核

- 预算耗尽与 Broker 恢复回归均在修复前失败，修复后通过。
- 本机 Agent library 97 项、API dispatcher 20 项通过。
- Agent library 严格 clippy、安装/manifest/同步检查、正式部署安全契约通过。
- API client 重新生成并通过一致性检查；0.3.25 成对版本同步。
- 独立静态复核发现两项 P2（复用 sidecar 绕过预算、损坏 sidecar 阻断扫描），均已修复并追加测试；复核未发现其他 P1/P2。
- 本机 Docker Linux 容器运行 seccomp 回归通过：隔离子进程拒绝 chmod 类调用，旧创建方式确实返回 EPERM，新 journal 创建成功。另有日志交付 12 项与 Broker 9 项 Linux 测试通过。

## 环境与发布

本机 Docker 已可用，开发测试验收可用本机容器。服务器 10808 不再作为构建前提，
远程脚本默认直连，显式网络参数隔离 builder 防止复用旧代理配置。
本机直连 Docker Hub 正常，qfy-test2 直连本次超时；没有修改或关闭系统代理。
正式发布使用已有 `DEPLOY_BUILD_MODE=local`，本机 Docker 构建 Linux amd64，上传到原控制面 qfy-test2；没有改变默认远程构建模式或服务器系统代理。首次 amd64 冷构建的 Rust release 耗时 7m23s。

## 正式发布验收

- 版本 0.3.25，发布提交 fd3a64f；发布前 SQLite 一致性备份完整性为 ok。
- API/Web active，本机 healthz/readyz 返回 ok/ready；公网 OpenAPI 版本 0.3.25。
- manifest、install.sh、Agent/executor/updater 三份发布物经公网实际下载均为 200，协议范围为 11–18。
- 测试、生产、预发布三个正常节点上报 0.3.25 / v18；三个 succeeded job，另保留一个生产节点首轮 upgrade_command_delivery_failed 历史 job，自动退避重试成功。最终升级租约与维护锁均为 0。
- 测试节点 Agent、Broker、executor 均 active；Broker NRestarts=0，doctor 的成对版本、runner IPC 与 executor IPC 均通过。WSS 身份另由控制面新鲜心跳证明，不将 doctor 的匿名 HTTPS 结果当作认证证明。
- 原初始化残留目录仍为 2770，没有删除或修改权限；Broker 已能跳过这些无业务/进程文件的初始化布局。
- 未替业务项目发起部署，因此尚未验收真实仓库分支查询与完整 prepare/release；业务项目应使用原幂等键重放验证。

## 测试遗漏与回滚

此前 macOS 权限测试允许 chmod 修正，未覆盖 Linux RestrictSUIDSGID。
新增 seccomp 子进程回归将这个环境契约作为运行测试，而不是仅依赖源码推断。
回滚保留新格式任务与日志，按 agent-recovery 手册恢复；旧 0.3.24 会再次遇到
初始化残留，不能用无验证的回滚、关闭安全保护或全局 chmod 替代修复。
