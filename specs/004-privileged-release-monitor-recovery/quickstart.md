# 特权 Release 终态收敛本地验收

所有场景使用本机测试 fixture，不连接节点或外部控制面。生产部署、重启、清理及业务任务重发均不属于验收步骤。

## Agent / executor

```bash
cargo test -p deploy-go-agent --lib privileged_bridge_tests
cargo test -p deploy-go-agent --lib
cargo test -p deploy-go-agent --test privileged_release_recovery
cargo test -p deploy-go-agent --test image_release
cargo test -p deploy-go-agent-executor --test release_lifecycle
```

预期覆盖：

- 从 Agent 已持久化序号 699 接管同一 job，读取 700–721 后再处理 executor terminal；只产生一个逻辑 TaskResult，不发送第二次 ReleaseStart。
- RPC 暂时失败、Agent 重连/重启、monitor 锁竞争后恢复，都复用原 job 和 durable 游标。
- 本地 frame/journal/event/outbox 故障不会无声丢弃 monitor；可恢复故障恢复后补齐输出，无法安全恢复的完整性冲突形成非成功中断诊断。
- monitor 诊断在 Agent 内部可关联 deployment、target run、task 和原 job，并记录可确认的 Agent/executor 序号、终态与恢复动作；在 executor 未返回终态时不推断。外部诊断不公开内部 task/job 标识，所有诊断样本均不含 Env、密钥、授权正文或业务输出正文。
- 新 journal 的 deployment/target run 标识须先于 ReleaseStart durable 保存；旧 journal 缺字段时从有效 descriptor 回填。对核心 journal 可解析但 descriptor/关联字段均缺失/损坏的 fixture，验证 reconcile/取消恢复发出明确诊断，只对原 job 拉取输出/状态，并在其 terminal 后报告 interrupted；没有新的 ReleaseStart，也没有静默跳过。
- 对核心 journal 缺失/不可解析的 fixture，验证只产生 keyed-by-task 的 Unknown/恢复诊断，不向 executor 发送无可信 payload digest 的请求，不合成 TaskResult、不标记成功或重新启动。
- `cargo test -p deploy-go-agent --lib` 必须包含 `agent/src/journal.rs` 的旧格式反序列化兼容测试；旧 Agent 不接管带新关联字段的活跃 journal，遵守现有 `docs/runbooks/agent-recovery.md` 回滚边界。
- executor 明确 `output_limit_exceeded` 且输出截断时，Agent 消费可读取帧后报告失败，不无限等待未生成的帧。

## API / target run

```bash
cargo test -p deploy-go-api --test agent_dispatcher
cargo test -p deploy-go-api --test two_stage_deployment
```

预期覆盖：

- dispatch 初始仍为 `downloading / artifact_download`；通用 TaskState Running 不会过早改变该含义。
- 首个 stage=release 的远端脚本开始事件把 run 推进到 `running / release`，重复进度事件不重复递增 version。
- 成功、失败、中断和取消结果正确终结 run 与 deployment。
- 同结果重放、receipt 提交失败后重放、同序号内容冲突及终态后的迟到消息符合 `data-model.md` 的幂等约束。

## 聚焦质量门

```bash
cargo fmt --all -- --check
cargo clippy -p deploy-go-agent -p deploy-go-agent-executor -p deploy-go-api --all-targets -- -D warnings
git diff --check
```

若实现触及 executor Linux cgroup/进程回收分支，再按 `docs/runbooks/local-development.md` 使用本机隔离 Docker 执行对应 Linux 验收。单测通过不能替代高风险状态机的独立代码复核；任何真实节点运行验证都必须另有当前会话的具体授权。
