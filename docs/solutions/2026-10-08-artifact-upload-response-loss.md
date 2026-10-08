# 发布物上传中响应丢失的恢复与诊断

## 适用场景

prepare 脚本退出 0，但发布物上传交接失败。服务端收到请求或提交内容，不等于发送端收到完整响应；须把脚本结果与平台交接结果分开判断。

## 已验证的处理原则

- PUT 失败后先查询确认 offset，只接受已知发送范围内的进度；查询也可能暂时失联，恢复预算须覆盖它，但必须有总上限和任务 deadline。
- 查询接口只支持 uploading 时，不用 GET 判断 finalize 已成功。复用同一 lease 的幂等 finalize POST，要求 verified 和完整长度，不能把 HTTP 200 当成功。
- 永久拒绝从响应头收敛，不为读取诊断正文等到 timeout。只对白名单 409 做有限恢复，未知冲突保持失败。
- 校验错误复用真实请求元数据；deadline 前保留已确认 offset、阶段与原失败 cause。缺失次数使用 null，不编造零次。
- 内部结构化结果经 API 白名单投影到现有摘要，避免泄露原始网络错误、URL、凭证和响应正文。

## 复现与验收

使用 agent/tests/artifact_transfer.rs 的 TCP 响应截断 fixture、agent/tests/two_stage.rs 的重连与终态重放 fixture，以及 api/tests/artifacts_api.rs 的约 60 MiB 真实 ArtifactStore 上传。分别证明上传字节/SHA-256、任务交接和失败不发布，不能仅以格式化函数测试认定可靠。

历史线上底层网络原因未确认时仍保留该结论，不能以本地同类响应丢失复现冒充现场同因。排障和操作边界见 docs/runbooks/deployment-recovery.md；实施证据见 docs/reviews/2026-10-08-prepare-artifact-transfer-implementation-review.md。
