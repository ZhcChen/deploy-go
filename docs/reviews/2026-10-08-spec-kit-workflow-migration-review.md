---
title: Spec Kit 工作流迁移复核
status: passed
date: 2026-10-08
plan: docs/plans/2026-10-08-002-spec-kit-workflow-migration-plan.md
---

# Spec Kit 工作流迁移复核

## 结论与范围

本项目默认流程已改为 `specify -> plan -> tasks -> analyze -> implement -> converge`。Codex/MiniMax 项目级固定资产、constitution、覆盖模板、防护入口、手册和文档权威已同步。保留 main 小闭环提交、migration 门禁、本机 Docker、系统代理保护、业务应用修改禁令与会话运行授权；参考项目的业务和发布限制未移植。

历史 CE 文档保留并标注当前入口，旧计划允许原地续做，转入新规格只承接剩余任务。两个既有未跟踪计划未修改或暂存。参考仓库仅只读，未修改业务代码、运行配置或全局插件。

## 独立发现与修正

独立复核发现两项 P2：

1. 直接执行上游脚本的手册例子没有固定 `SPECIFY_INIT_DIR`，旧环境可能导向其他项目。已在仓库根目录示例中显式设置，复核注入旧变量后仍解析 deploy-go。
2. FR、验证章节和任务编号校验读取原始 Markdown，代码块/注释/引用可冒充正式结构，或误报重复编号。已统一检查过滤后的正文，同时将标题判断移到过滤后，补充五个反例与一个正例。

两项修正的独立聚焦复核通过，相关 4 组测试通过，无新的相关发现。最终文件清单还补齐 manifest 未覆盖的 bundled workflow，并加入 registry 文件存在性校验；该 bundled workflow 不作为项目默认执行入口。

## 验证

- `make spec-kit-verify`：13/13 通过，CJS 语法、32 个 managed hashes、6 个 Bash 脚本语法/执行位通过。
- 固定 Specify CLI 为 1.0.13；集成状态 ok，默认 Codex，Codex/MiniMax 均完整，缺失、修改和无效路径为 0。
- 明确目录的阶段输入、已有产物防覆盖、并发初始化、路径隔离、无活跃指针写入与真实 Git fixture 不切换/创建分支通过。
- Make 参数 dry-run 与默认 help 通过，工作分支保持 main，工作树及暂存区 diff 检查通过。

## 验证边界与恢复

结构检查不能证明语义完整、验证命令有效、实现正确或 agent 后续不会手工覆盖。新会话的原生 Skill 发现和真实功能完整闭环未验证，后续实际需求需记录调用与交付证据。

本轮没有运行产物变化，不启动业务服务、不连接节点、不部署或升级 Agent。撤销本轮提交即可恢复开发入口，不改变服务状态。全局 CE 插件继续保留，旧会话的注入规则需新会话刷新核对。
