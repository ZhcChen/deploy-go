# 合并 API 集成测试二进制任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：T001–T005 已实施并验证。全部任务待用户签收。勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-003 在合并前记录基线：用例清单、全 workspace 通过数、改 `api/src` 后测试重建墙钟、API 测试二进制体积。无前置依赖。
- [ ] T002 FR-001 FR-002 按分组 `git mv` 并改写 `mod common;`，生成各组 `main.rs`；编译并运行，对比用例清单一致。依赖 T001。
- [ ] T003 FR-004 FR-005 同步 Makefile、docs、AGENTS、nextest 过滤中的二进制名引用，并在 docs-check 增加 `--test` 目标存在性检查与自检。依赖 T002。
- [ ] T004 FR-006 FR-007 调查 leaky 差异，更新开发手册（布局、单模块运行方式、分组规则、基线数据），登记 014。依赖 T002。
- [ ] T005 SC-001 至 SC-004 验证：api-check、migrations 与 database_constraints 命令、external-deploy-check、`cargo test -p deploy-go-api`、重建与体积前后对照、docs-check、spec-kit-verify、whitespace-check、make check；自审最终 diff，执行 converge；提交并推送。依赖 T003、T004。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已实现 | 通过 | 不适用 | 待用户签收 | 合并前基线：API 515 个用例、57 个二进制（56 个集成测试加库内测试），集成测试二进制合计 2.95GB，改 `api/src` 后 `nextest --no-run` 重建 85.0 / 86.7 / 94.6 秒；全 workspace 957 个通过、2 个跳过 |
| T002 | 已实现 | 通过 | 不适用 | 待用户签收 | 56 个文件按 5 个领域 `git mv` 并改写 `mod common;`，migrations 与 database_constraints 独立；两处 `include_*!` 相对路径多一层 `../`，三个未实际使用 common 的文件去掉了引用。用例清单合并前后 515 对 515 逐项一致；全量 API 测试 513 通过、2 跳过 |
| T003 | 已实现 | 通过 | 不适用 | 待用户签收 | 同步 Makefile 3 处与 4 份 runbook/solution 的 `--test` 引用，改写后三条 Makefile 命令选中的用例数与原文件合计逐一相同（24、92、39）；`make external-deploy-check` 与 AGENTS 第 5 节 migrations/database_constraints 命令通过。docs-check 新增 `--test <名>` 目标存在性检查（限定同行含 cargo，避免误报 node --test），含自检，并因此发现 development-guide 中 3 处过期引用并修正 |
| T004 | 已实现 | 通过 | 不适用 | 待用户签收 | leaky：合并后连续 3 遍全量 API 测试均无 LEAK，原型中的 “4 对 1” 是运行间浮动（合并前同样有 1 个偶发）；普通 `cargo test` 连续 2 遍通过（同进程多线程）。development-guide 记录新布局、单模块运行方式与新增测试的分组规则；登记 014 |
| T005 | 已完成 | api-check 通过（957 通过、2 跳过）；make check 并行 55 秒全部通过；docs-check、spec-kit-verify、whitespace-check 通过 | 不适用 | 待用户签收 | 重建 85–95 秒降到 16.7–17.7 秒（约 5 倍）；API 集成测试二进制 56 个 2.95GB 降到 7 个 0.38GB；首次 fmt 检查发现 include 路径折行差异，已用 cargo fmt 修正；自审 diff：仅测试文件移动加一行改写、Makefile 与文档引用、docs-check 门禁，未触及产品代码、协议、migration、OpenAPI |
