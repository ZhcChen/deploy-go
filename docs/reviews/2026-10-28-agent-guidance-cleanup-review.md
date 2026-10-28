# 通用项目指引与专属资产清理验收

日期：2026-10-28；基线main / 551dadc，接手时工作区干净。执行正文是项目Obsidian稳定标识 `specs/009-agent-guidance-cleanup`，本记录不复制三份正文。

## 结果与引用处理

按用户指令删除CLAUDE.md、.claude十个阶段Skill、对应集成manifest和未使用的集成workflow。安装列表/设置及工作流注册均为空，初始化选项不再指定工具；Spec Kit核心12份受管理脚本/模板保留原文与hash。活动指引统一AGENTS，工程知识迁入 `docs/runbooks/development-guide.md`。

仓库README、运行手册、规范、constitution、覆盖模板及本项目Obsidian规格中的旧指引引用已更新；历史验证结果、任务编号/签收状态、006现场未验收项保留。源快照指纹不重算，引用编辑后的正文不再承诺与原迁移快照字节一致。旧安装计划标注为历史事实，当前以工作流runbook为准。

部署Skill保留为通用Agent能力，安装默认改为 `~/.agents/skills/deploy-go-deployer`，显式 `DEPLOY_GO_SKILL_INSTALL_DIR` 仍优先。本轮不安装/卸载全局工具或技能，不修改其他项目，不操作服务。

剩余同名命中仅为：安全源码中模拟外部仓库隐藏目录的测试样本/注释；Spec Kit测试中断言旧资产不存在；009规格及本记录中的删除审计清单。这些不构成活动配置或失效文档链接。安全生产逻辑及测试样本未改。

## 验证与复核

- `make spec-kit-verify`：21/21通过，核心12份资产逐hash、Shell语法和执行位检查保留；新增专属入口不存在与空集成状态断言。
- `make deployer-check`：release/schema/CLI契约通过，Rust测试38/38通过（12单元、3帮助、14Git来源CLI、5Skill包、4输出）。
- 安装脚本 `bash -n` 通过；默认路径静态核验通过。使用stub二进制和显式临时安装目录，安装、重复替换、失败保留旧目录共3场景通过，临时目录自动清理；未下载/构建或写真实用户技能目录。
- 真实项目9个feature的implement check全部通过；仓库活动文档与本项目Vault中的失效指引/集成引用扫描通过，残余类别如上；新009任务全未勾选。
- `make whitespace-check`、工作区与暂存diff检查通过（交付前执行）。

独立只读复核者实际对照原技术段与新手册，确认无实质遗漏，核对权限/门禁、集成清理和安装失败保护，并独立运行21组测试通过，findings=[]。未将主会话自审称为独立复核。

未运行API/Agent/Admin全量门禁或Linux权限容器测试（没有相应生产源码变化）；未验证全局安装、外部原生集成或真实服务。Vault正文不在本仓库提交内，备份/同步由Vault维护。
