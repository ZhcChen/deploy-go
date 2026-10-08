# 容量日志与集中诊断复核

唯一规格：`specs/002-bounded-log-retention/`。发布目标 0.3.28，基线 0.3.27。本轮只修改 Deploy Go，未修改业务应用、全局 journald 或系统代理。

## 独立复核及修正

Kierkegaard 核对回收、鉴权和恢复；Aristotle 在隔离 worktree 实施/验证 API、回收 fixture，并复核节点链路。主 agent 集成、修正与最终验证，未把自审称为独立复核。

- 回收必须验证最终结果与 `last_sequence`、完整 committed receipts、无越界事件，且结果正文一致；容量与周期共用判据，保护旧无 receipt 任务。终态后新 N+1 事件拒绝，已提交重传继续 ACK。
- 两份输出正文均计入容量；没有 deployment 的终态任务输出同样按完整交付判据回收。状态、progress/result/diagnostic 和去重 receipt 保留。
- Settings 使用版本 CAS；旧 PATCH 省略总容量时保留现值，避免并发覆盖或恢复为默认值。
- 来源水位独立于可覆盖的 JSONL；落盘先于数据库水位提交。真实取消 fixture 在 fsync 后、水位提交前 abort，竞争写入先恢复水位；恢复失败拒绝追加，恢复后重传不重复。
- updater 日志目录失败不回滚升级；cursor 初始读取失败定期重试。Runner unit 的 ExecStartPre 同步准备目录，Agent/updater 可选 ReadWritePaths 不因目录缺失拒绝启动。
- 独立复核另发现手工安装器仍把日志准备失败视为致命错误，列为 T012；已改为新版binary的固定目录helper，旧版本不调用未知命令，fixture采用目录FD隔离。Linux Bats19/19通过，独立复核确认原阻塞关闭。
- 空闲上传和 SSE bounds 查询只校验日志元数据，不反复扫描当前整个分段；独立复核确认 epoch 切换、边界和去重语义不变。
- 安全投影不格式化原始 error/message/URL，关联 ID 长度有界；节点根目录和文件拒绝不安全 owner/mode、symlink/hardlink。普通 runner 不获得日志树权限。

## 已验证证据

- API：dispatcher 37、deployment_recovery 9、migrations 16、settings 3、runtime ingest 原12项及新增真实取消1项通过；enrollment 15项通过。包含旧配置、并发计数、保护输出、HTTP鉴权、重传/覆盖/重启、慢 SSE 与实时撤权。
- shared runtime-log 6项、Agent runtime-log delivery 3项、Agent短命命令2项、updater 6项通过；早返回优化后 shared 6项重新通过。
- 本机隔离 Docker：真实 Linux UID/GID 权限 fixture 1项、安装 Bats 原15项通过。真实 systemd mount namespace 尚未独立模拟，发布时核对受管服务健康与专属日志目录；不把静态 unit 检查等同于真实 systemd 验证。
- Web：运行日志/系统设置9项、typecheck、lint、build、双端 API client 漂移检查通过；主/外部OpenAPI契约各9项通过；正式部署脚本安全契约及代理服务5项通过。
- `cargo fmt --all -- --check`、分段/发布 manifest 契约及 `git diff --check` 通过。

API 严格 clippy 存在既有 `collapsible_if/too_many_arguments/unnecessary_map_or/single_element_loop/manual_unwrap_or` 类别告警，允许这些基线类别后 lib/bins 检查通过；未为消除告警改动原有安全错误白名单。Agent 全 targets 的既有 image_release 测试 lint 仍是基线缺口，不宣称全仓严格 clippy 通过。

runtime-log、Agent、Executor、updater 的 lib/bins 严格 `-D warnings` clippy 通过。Spec Kit converge 发现的安装器遗漏已完成，剩余只有既有 T010 提交交付与 T011 正式发布运行验收，不追加重复任务。

## 回滚与证据边界

回滚保留 migration、来源水位、任务数据和新日志树，不清空游标或删除未确认 outbox。容量预算是有效载荷/受管分段的边界，不等于 SQLite 文件大小、主机 page cache 或进程实际峰值。正式发布与自动升级实测结果另行追加；当前测试不能代替生产观察。

## 首轮发布发现与修正

实现提交 `2fb53b9` 已推送，0.3.28正式控制面发布成功，migration40/41成功，部署输出有效载荷31227897字节。三个正常节点自动升级均succeeded、protocol18；测试节点doctor确认Agent/runner/executor配对健康。两个归档节点未创建0.3.28升级任务。

运行验收发现Ubuntu `/var/log=root:syslog 0775`，日志树初始化被严格祖先安全检查拒绝。组件正确降级，任务服务保持正常，但集中诊断未启用，不能把升级成功当功能完整验收。T013将专属树迁到 `/var/lib/deploy-go-agent-runtime-logs`；真实控制面 `/var/lib=root:root 0755`。不修改共享目录权限、不放宽安全校验、不扫描旧日志树；发布新的0.3.29，不覆盖已发布0.3.28。

Kierkegaard独立确认权限边界，Aristotle独立确认旧updater mount namespace不传播到systemd启动的新Executor/Runner，同步helper可创建新树。新增隔离Linux fixture设置 `/var/log uid=0,gid=1003,mode=0775`，验证其保持不变，新树初始化和真实身份隔离通过；shared6项、unit契约再次通过。

构建证据：qfy-test2直连Hub超时；显式镜像源可拉镜像，但apk安装在IPv6已建立连接上停滞。本次仅停止自己的构建client，确认构建进程退出，API/Web继续active。采用已授权本机Docker回退，0.3.28 Rust release编译4m31s；全五组件单架构amd64成对安装，未修改系统代理或业务应用。

数据库切换前的一致性备份为 `/var/lib/deploy-go/backups/pre-0.3.28-20261008153349.db`，权限0600、136130560字节、integrity_check=ok、最高migration39；备份不随产物回滚删除。
