# prepare 发布物交接修复实施复核

日期：2026-10-08。唯一需求入口：specs/001-prepare-artifact-transfer/。

## 结果与证据边界

本地实现完成：构建成功但上传失败不再只返回 agent_error；Agent 持久化安全传输诊断，API 从白名单字段重建已有 summary。初始化、查询与 finalize 响应丢失可有限恢复；PUT 失败先核对服务端进度，禁止未知 offset 盲传。只有 verified 且长度一致才交接 release。

历史部署 deployment_01M4CW07DQ9BM2RXN0A2VM6756 的原始故障为 artifact_transfer_failed、exit0、offset0。初始化曾到达控制面不证明 Agent 收到响应；其底层网络原因仍未确认。本地故障注入证明旧实现至少存在初始化响应丢失、恢复 GET 响应中断后提前退出两个缺口，不能据此断言历史故障与 fixture 完全同因。

## 验证

- cargo test -p deploy-go-agent --test artifact_transfer --test two_stage --lib：98 个 unit、27 个上传、5 个 two_stage 测试通过。
- cargo test -p deploy-go-api --test artifacts_api --test external_api --test external_openapi_contract --test two_stage_deployment：14、24、9、23 个测试通过。
- cargo test -p deploy-go-deployer：21 个测试通过。共 221 个，不重复累计再次运行的用例。
- 约 60 MiB 合成归档经 Agent 到真实 API ArtifactStore 的正常/已提交 PUT 响应丢失两条 TCP 路径通过；对象字节、长度、SHA-256、verified 和 lease 消费一致。
- two_stage 合成业务脚本验证恢复成功只构建/发布一次，断控制连接重发同任务与终态重放一致；拒绝、取消和 deadline 不发布。
- make deployer-check、make api-external-openapi-check、cargo fmt --all --check、cargo clippy -p deploy-go-agent --lib -- -D warnings 通过。

API 响应复用既有 summary，不公开 data/result_json，不生成额外 OpenAPI 字段或 CLI 命令。旧记录缺失详情固定标为未记录，未知错误保持 agent_error；旧协议 fixture、CLI 历史无摘要 fixture 与跨应用隔离测试通过。历史版本二进制未实际运行，兼容结论限于现有契约和 fixture。

## 独立审查与修正

Newton 独立只读审查部署可靠性及诊断安全，主 agent 运行验证。发现三项 P2 并修复：永久拒绝响应头后等待正文会误报 timeout；外层 deadline 会丢失阶段/offset/cause 并编造 attempts0；校验失败硬编码尝试次数和状态码，丢失真实阶段耗时。第二轮发现恢复退避可能越过 deadline，再次裁剪并增加回归。最终限定复核确认这些发现关闭，未发现新问题。

所有错误只记录内部枚举、HTTP 状态、请求次数、阶段耗时和确认 offset；不输出原始 reqwest 错误、URL、凭证、响应正文或内部路径。原 PUT 失败与恢复 GET 失败保留因果。总 deadline/取消约束不变；请求最多三次、恢复最多三轮，无无限重试。

## 交付与未完成事项

正式发布按 docs/runbooks/systemd-deployment-production.md 执行，维持完整成对发布物及正常节点串行自动升级。没有自动重发业务部署。T019 须保留至取得授权真实 prepare、release、健康检查成功证据；本地通过不关闭现场故障。

speckit-converge 已显式选择需求目录并执行项目/上游前置检查；独立只读核对无新增本地遗漏，不追加重复任务。原 T018 收尾与 T019 现场验收沿用现有编号。最后摘要格式调整后，持久化/重放聚焦 unit 测试再次通过，fmt 与 diff 检查通过。

回滚以本轮独立代码提交 revert 和正式安装事务备份为边界，无 migration，不清理数据库、制品状态或业务目录。

本地修复提交 abe8f25 已推送 main。正式发布准备同步五个组件为 0.3.27；manifest、安装器静态契约、release 同步及正式部署安全契约检查通过，5 个 Python 代理测试通过。macOS 本机未运行 Linux Bats 动态安装测试，检查工具已明确报告该项限制。真实发布及节点版本证据随后补充。

## 正式发布证据

2026-10-08（Asia/Shanghai），版本提交 2952966 已推送 main。默认 qfy-test2 远程构建在 Dockerfile frontend 元数据请求阶段超时：registry-1.docker.io 的 HEAD 连接失败，尚未进入安装。未修改代理或远程网络配置，改用脚本已有 DEPLOY_BUILD_MODE=local 模式，本机 Docker 生成 linux/amd64 产物后安装到 qfy-test2。release 编译 5m04s，本轮 sccache 3 个实际请求均未命中；没有足够证据将全部时间归因于单一因素。

正式安装完成，API/Web systemd 均 active，启动时间 14:15:49。运行 API 的 /proc/MainPID/exe SHA-256 与本地发布 image 的 /out/deploy-go-api 一致：96314add737bb15418c3212b3ddf0908972227e09976c76db4a29757f0a7bc4e。公网独立 API 域名 https://deploy-api.quanxinfu.com 的 /healthz、/readyz 分别返回 ok、ready；本机 30100 相同。Web 域名根路径 /healthz、/readyz 返回 SPA，未将其 HTTP200 冒充 API 检查通过。

https://deploy.quanxinfu.com/api/v1/agent/download/0_3_27/manifest.json 返回 schema4、Agent/executor 0.3.27、最低 v11/最高 v18、executor protocol4，Agent/executor/updater 三份 x86_64 发布物齐全。

控制面数据库只读核对，三个节点升级均 succeeded，无 error_code，随后 heartbeat 均为 Agent0.3.27/protocol18：

| 节点 | validating 时间 | 完成时间 |
| --- | --- | --- |
| 测试环境节点01 | 14:16:04 | 14:16:08 |
| 生产节点01 | 14:16:19 | 14:16:24 |
| 预发布环境01 | 14:16:34 | 14:16:39 |

progress 依次包含 validating、downloading、staged、installing，三个时间窗无重叠；未手工安装或重启业务节点。未接入的离线预发布节点02保持离线，已撤销的旧测试 Agent未升级。14:25:30 只读复核仍在线。

T019 的控制面发布与自动升级部分完成，但没有新业务部署 ID、prepare/release/健康检查证据，真实故障关闭仍未完成。请业务方在确认来源、目标及幂等键后按原流程发起一笔授权验证；不要复活历史失败任务。
