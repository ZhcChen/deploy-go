# Spec Kit 项目级外部存储整改验证

日期：2026-10-28。基线main / 82dcd99；本轮只改deploy-go工具、测试与工作流文档，不连接节点或操作服务。盘点与逐项存量权威登记见 `docs/standards/spec-kit-storage-register.md`，执行正文为稳定标识 `specs/008-obsidian-spec-kit-storage` 对应的项目Obsidian三份产物。

## 实现与复核

保留官方 `.specify/`、默认 `specs/` 和固定1.0.13受管理资产/hash；新增项目相对配置、显式单次覆盖、项目Vault候选目录与历史仓库兼容。输出真实FEATURE_DIR/STORAGE/REQUIRED_READS，向上游传实际绝对目录且不写隐式指针。目录/产物链接、非目录、越界、重复feature和缺失产物失败；排他初始化保留已有正文。001–007共29个历史文件未改写，006现场未验收项保持。

另一只读agent按adversarial-reviewer方法复核，发现根目录字符串别名在规范化前比较可绕过限制，`/./`、`//`、`///`会定位文件系统根。已改为规范化后拒绝并补反例；复核者实际复验根别名、21组测试及29份当前/源HEAD原文，最终findings=[]。这是独立只读代码复核，不声称Claude Code独立复核。

## 实际验证

- 修改前 `make spec-kit-verify`：13/13通过；原007 implement check通过。
- 修改后 `make spec-kit-verify`：21/21通过，包括默认仓库模式、项目配置+Vault、单次覆盖、新建外部目录、四类历史、重复、越界、符号链接含悬空链接、缺失和非目录、已有产物保护及并发；原manifest/hash、Shell语法/执行位、无分支/指针副作用检查保留通过。
- 001–007在默认和项目Vault模式共14次implement检查均通过、均报告repository；29份冻结文件SHA256与82dcd99一致。
- 真实Vault执行008的 `make spec-kit-init` 成功、STORAGE=obsidian；补齐三份正文后implement check成功；再次init返回EXISTING=true，单次Spec Kit根覆盖的converge check成功。仓库没有同名008目录。
- `make -n spec-kit-init FEATURE=specs/009-example`、`make -n spec-kit-check FEATURE=specs/008-obsidian-spec-kit-storage STAGE=implement` 参数正确；`make whitespace-check`、工作区/暂存diff检查通过（交付前执行）。

## 外部正文与边界

本轮仅在Vault相对路径 `QFY/项目/deploy-go/10-项目文档/03-计划与任务/Spec Kit/008-obsidian-spec-kit-storage` 创建三份正文及本项目索引，未批量修改其他项目。以下为提交前正文快照指纹，仅供核对，不建立第二份正文权威；后续补证据或用户签收会自然改变摘要。

- `spec.md`：`4d4b7d294f4b5e3033a886549a270b439c35f97152e3a1a89f33a92001fff7a1`
- `plan.md`：`a2d832b43f1a747a47a2c9d4dcaf9bf4bcb8d68b4e74d961a703959dedc7ab6e`
- `tasks.md`：`04fa5ed117bcac88f703639291d8981604c06227fdc585cafa48e1cb1eef72a8`

任务复选框只由用户签收，008保持全部未勾选；实现、测试、复核证据已经追加。代码、JSON/OpenAPI、发布工件、受管理资产及历史审核证据仍留仓库。外部正文不包含在本仓库Git提交中，Vault备份/同步不在本轮验证范围。

未执行原生Skill真实交互闭环、业务功能测试、API/Agent/Admin构建测试或真实运行环境验证；无运行源码变化，不把结构检查当作业务验收。检查不能防御有权限进程主动在校验后替换祖先目录，仍要求Vault单写入。未搬迁历史规格；后续须按登记逐项核对，不为清目录重写历史。
