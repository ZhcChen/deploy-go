# 对外部署来源可观测性复核

范围：`docs/plans/2026-10-08-001-external-source-observability-plan.md`，版本 0.3.26。

## 行为与边界

- `show-app` 分别返回 `sources.git` / `sources.workspace`；来源与执行模式的对应关系由文档和 CLI 明示，不把配置分支当实时 HEAD。
- 缺失来源为 null；draft、archived 与未选择分支如实展示；构建节点信息采用 LEFT JOIN，关联缺失不隐藏来源。
- 沿用现有应用授权，未新增写端点、凭证接口、远端 Git 解析或数据库迁移。
- 部署详情仅公开快照白名单，当前应用分支变化不改变历史固定分支；工作区摘要不作为 Git SHA 返回。
- 本轮只修改 Deploy Go，不修改或重新部署任何业务应用。

## 验证

- 来源回归先失败（原接口返回 null 而期望 test），实现后通过。
- API 聚焦 47 项通过：external_api 23、external_api_keys 3、external_openapi_contract 9、deployer_release 3、openapi_contract 9。
- CLI 20 项通过：单元 9、帮助 3、skill 包 6、来源输出 2；覆盖新字段文本/JSON、旧控制面响应、发布版本与提交区分。
- OpenAPI、双端 API client 生成物一致性检查通过；Flutter 生成结果无差异，Web 仅版本头更新。
- CLI 契约、manifest、正式部署安全契约、Agent release 同步脚本、fmt 和 diff 检查通过。
- 普通 Clippy 通过；严格 `-D warnings` 被 7 项现有警告阻断，位置为 CLI 原有部署响应验证、API Agent dispatcher/upgrades/manifest 与 deployer 目录检查；未增加警告，不声明严格检查通过。

## 简化

三项独立复核已完成：复用无发现；采纳一次 branch 判断返回分支/SHA 元组的可读性建议。仅投影 JSON 字段的 SQL 优化暂不采用：缺少性能测量，且会增加类型/损坏 JSON 处理复杂度；保留现有 Rust 白名单读取，未放松安全检查。

## 独立代码审查

`ce-code-review` 已完成（receipt run_id `20261008-source-observability-review`）。最初发现 P1：应用详情遗漏工作区模式还原；P2：镜像发布版本路径遗漏。均补充修改前失败的回归用例，再修正；API external_api 23 与 external_openapi_contract 9 项重新通过。另一独立上下文只读复核确认两项关闭、无新增发现。审查未改业务应用。

跨模型审查因提供方余额不足未成功，审查使用父上下文 fallback；不宣称完成跨模型验证。最终覆盖包含独立复核、实际 API/CLI 测试与源码白名单检查。

## 发布验收与恢复

正式发布及公网契约验收完成后补充结果。发布前已生成 SQLite 一致性备份 `pre-0.3.26-20261008-035622.db`，完整性为 ok。

发布使用本机 Docker Linux amd64 构建并上传 qfy-test2，不使用 10808，不改变系统代理。回滚沿用 `docs/runbooks/systemd-deployment-production.md` 的安装产物备份恢复流程；无新增 migration，无需回滚数据库 schema 或重放业务部署。

发布后观察 API/Web 健康、公开 OpenAPI/下载 manifest 版本、正常节点串行升级结果；若新增字段导致现有读取失败，应恢复上一版控制面和成对发布物并保留任务/日志。本轮不凭 UI 推断实际 Agent 状态。
