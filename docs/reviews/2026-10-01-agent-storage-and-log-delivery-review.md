# 节点存储与日志交付复核

对应计划：`docs/plans/2026-10-01-001-agent-storage-and-log-delivery-plan.md`。

## 结论

本地实现完成，未执行真实节点发布、重启、回收或业务应用修改。
旧任务兼容保留，新任务在 v18 协商后启用可靠日志；历史日志只登记 retained，不无证明删除。
自动安装仍串行，归档节点不参与，失败版本增加退避和三次暂停。

## 独立复核与修正

- 清理复用启动校验导致已销毁秘密引用被拒绝：拆出清理专用的属主、权限、类型与 no-follow 校验。
- Agent 终态不等于 updater 事务终态：增加 executor 交接标记与 root updater completion 证明；未决或不可读保留。
- ACK 等待发送锁阻塞连接：独立有界、顺序处理回执；队列满时断线，由 durable outbox 重放。
- checkpoint 恢复覆盖后续终态：只恢复交付字段；ACK 删除 checkpoint 前先保存恢复后的 journal。
- 新连接版本不能代表旧任务模式：增加可省略的任务级 `log_delivery_version`，旧任务维持原对账行为。
- 特权日志重建先截断可能破坏原始输出：先校验预算，流式写临时目录，再替换；持久化 frames 保留到结果确认。
- 未知或损坏 journal：默认保留。降权 Runner 不枚举根目录，节点准入预留缓存。
- completion 上级目录与 umask：父目录仅开放遍历，证明目录和文件显式配置可读权限，事务目录保持保护。

## 验证

- Agent、Agent protocol、updater 全套 255 项通过；追加回执与 checkpoint 回归后 Agent 单元测试 91 项通过。
- API dispatcher 与 upgrade 聚焦 53 项测试通过，覆盖重复补传、投影补偿、连续序号、新旧对账和退避。
- 安装器、manifest 与发布物同步检查通过；OpenAPI 已重新生成；migration Git hook 已验证。
- Agent/protocol library 与 updater 严格 clippy 检查通过。
- 全 all-targets 严格 clippy 存在既有 API 与 image_release 测试告警，未为本轮整理无关代码。
- API client 漂移检查未通过：已提交 Web 生成文件 header 仍为 0.3.22，而 API/spec 为 0.3.23；本轮 OpenAPI 仅改变协议最大值 17 到 18，未批量改写既有生成客户端。

## 验证边界

本机未安装 Docker。本轮没有真实 Linux 节点的跨 UID 回收、安装事务与断线负载验收；
权限契约通过本地 fixture 和源码复核验证。实际 RSS/PSS、anon/file、磁盘回收收益需发布后在同负载测量。
旧 retained 日志和无终态证明的升级目录仍可能占用磁盘，这是避免丢失日志或破坏恢复事务的保护行为。
回滚不能让旧二进制直接接管新格式活跃任务，按恢复 runbook 先完成日志确认。
