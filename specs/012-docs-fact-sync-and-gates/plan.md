# 文档事实修正与文档门禁增强实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

纯文档与脚本改动。文档门禁是 `scripts/test/docs-check.py`（Python 3 标准库，`git ls-files` 取文件，现仅检查必需入口与相对链接）。Makefile 的 `docs-check`、`whitespace-check` 是入口。事实来源已由只读核查给出文件与行号，实施时逐条回到代码再核对，不凭报告直接改。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md)：不动历史文档正文，不改协作规则，不改运行代码；检查脚本的改动属于验证策略，因此走完整产物；提交只暂存本轮文件，运行 `make docs-check`、`make spec-kit-verify`、`make whitespace-check`。

## 实现方案与文件范围

1. 事实修正（FR-001 至 FR-007）：逐条核对代码依据后修改 runbooks、standards、solutions 与入口文档；所有“当前版本/数字”改为占位或指向来源文件。
2. 门禁（FR-008、FR-009）：在 `docs-check.py` 增加检查函数，范围由 `CURRENT_DOCS` 前缀列表限定；Makefile 目标用 `git ls-files` 读取，避免硬编码；锚点按 GitHub 规则生成（小写、去标点、空格转 `-`，保留中文）；make 目标集合由解析 Makefile 的 `^target:` 得到。自检用临时仓库目录构造正反例，通过 `--self-test` 运行，并由 `make docs-check` 先自检再检查仓库。
3. 先写门禁并在当前仓库上跑，用其输出作为修正清单，再修文档直到通过，避免只凭报告修改。
4. `whitespace-check`（FR-010）扩展扫描范围。

## 阶段与依赖

1. 门禁实现与自检（先在现状上得到真实失败清单）。
2. 文档修正，依赖阶段 1 的清单。
3. whitespace 范围、验证、自审 diff、converge、提交并推送。

## 验证计划

`make docs-check`（含自检）、`make spec-kit-verify`、`make whitespace-check`；构造坏例子确认失败；`git diff --stat` 确认只涉及文档、脚本与 Makefile。

## 风险与回退

误报是主要风险，以“当前仓库全量通过且不靠大量白名单”为准；回退为恢复 `docs-check.py` 与 Makefile 的改动，文档修正互相独立。
