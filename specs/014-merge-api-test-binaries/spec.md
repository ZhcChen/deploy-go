# 合并 API 集成测试二进制

状态：规格就绪
来源：用户需求（2026-11-10 会话）“构建与测试优化”第二批，“可以”确认先做原型后继续。原型数据：把 6 个最慢的 API 测试文件合成 1 个二进制，改库源码后的重建由约 17.6 秒降到 3.4 秒（约 5 倍），用例数 44 对 44 一致且全部通过，体积由约 360MB 降到 76MB。全量现状：改一个 API 源文件会重编 58 个测试二进制，累计约 232 CPU 秒；仅 API 的 56 个集成测试二进制合计 2.9GB，42 个各自重复编译一份 `mod common`。

## 目标与范围

缩短日常“改代码后跑测试”的编译与链接时间，并减少 `target/` 体积，同时保持用例集合与判定语义完全不变。

范围：把 `api/tests/` 下的 56 个集成测试文件按领域合并为少量测试二进制（`migrations` 与 `database_constraints` 保持独立），并同步所有按二进制名引用测试的命令、文档与过滤配置；新增文档门禁，防止以后文档引用不存在的 `--test` 目标。

明确不做：不修改任何测试逻辑与断言；不改产品代码、协议、migration；不合并 `agent/` 等其他 crate 的测试（另行量化后决定）；不调整长尾用例；不改变 nextest 的独占与优先级规则语义。

## 用户场景与验收

### US1：用例集合不变

- 优先级：P1
- Given：合并前后各生成一份“文件名加用例名”的清单
- When：对比两份清单
- Then：两份清单逐项一致，数量相同（合并前后 API crate 用例总数不变）；全 workspace nextest 通过用例数与基线一致（957 个通过、2 个跳过，若基线因其他改动变化则以合并前同一提交的实测为准）

### US2：日常重建更快

- 优先级：P1
- Given：改动 `api/src` 的一个文件
- When：运行 `cargo nextest run -p deploy-go-api --no-run`（或等价的测试构建）
- Then：重建墙钟明显低于合并前（合并前以同机同条件实测为基线，目标为下降一半以上）；`target/debug/deps` 中 API 测试二进制总体积明显下降

### US3：命令与文档不失效

- 优先级：P1
- Given：Makefile、runbook、standards、AGENTS.md 中按 `--test <名>` 引用 API 集成测试的命令
- When：执行这些命令
- Then：命令仍能运行到预期的用例；引用的目标名都真实存在；`make docs-check` 对不存在的 `--test` 目标会失败

## 功能要求

- FR-001：以领域分组合并为 `agents`、`deployments`、`applications`、`external`、`platform` 五个二进制（目录 `api/tests/<组>/main.rs`，子文件为原文件），`migrations` 与 `database_constraints` 保持独立文件；公共模块 `common` 在每个二进制的根上只引入一次（`#[path]` 指向 `api/tests/common/mod.rs`），子文件用 `use crate::common;` 取代 `mod common;`。
- FR-002：原文件移动使用 `git mv` 以保留历史；除 `mod common;` 一行与必要的路径引用外，不修改测试文件内容。
- FR-003：合并前后分别生成并保存“文件名加用例名”清单，对比一致后才能提交。
- FR-004：同步所有按二进制名引用的位置：Makefile（`external-deploy-check` 等）、`docs/runbooks`、`docs/standards`、AGENTS.md、`.config/nextest.toml` 的过滤；用例级过滤改为按模块路径或测试名。AGENTS.md 中的 `migrations`、`database_constraints` 命令保持不变。
- FR-005：`scripts/test/docs-check.py` 增加检查：当前有效文档与 Makefile 中的 `cargo test/nextest ... --test <名>` 引用的目标必须是工作区某个 crate 下真实存在的集成测试目标（`<crate>/tests/<名>.rs` 或 `<crate>/tests/<名>/main.rs`）；含自检的正反例。
- FR-006：若合并后 nextest 对 “leaky” 的提示数量增加，需要查清原因并记录（原型观察到 4 对 1 的差异，来源未确认）。
- FR-007：在 `docs/runbooks/development-guide.md` 与 `local-development.md` 记录新的测试布局、如何跑单个模块（按模块路径过滤）与新增测试文件应放入的分组规则；基线数据补入性能说明。

## 成功标准

- SC-001：用例清单逐项一致；`make rust-test-workspace` 全部通过，通过数与基线一致。
- SC-002：改一个 `api/src` 文件后的 API 测试重建墙钟下降一半以上（如实记录实测前后值；达不到则记录原因，不放宽其他检查）。
- SC-003：`cargo test -p deploy-go-api --test migrations --test database_constraints` 与 `make external-deploy-check` 仍可运行并通过。
- SC-004：`make docs-check`、`make spec-kit-verify`、`make whitespace-check`、`make api-check` 通过；`make check` 通过。

## 风险与验证要求

- 测试隔离：合并后同一二进制内的模块共享进程，但 nextest 每个用例单独进程；已核对 API 测试没有修改环境变量、没有全局静态状态、没有内部属性。`cargo test` 直接运行时同一二进制内的用例并发共享进程，需确认不依赖进程级状态（用 `cargo test -p deploy-go-api` 抽样验证）。
- 命名冲突：各子文件成为独立模块，同名辅助函数不会冲突；`common` 只引入一次。
- 不涉及 `cfg(target_os = "linux")` 代码、migration、协议与 OpenAPI，无需 Linux 专项 Docker 验证。
- 验证：用例清单对比；`make api-check`；`cargo test -p deploy-go-api --test migrations --test database_constraints`；`make external-deploy-check`；重建计时前后对照；docs-check 自检。

## 未决事项

无。其他 crate（`agent`、`agent-executor`）的测试二进制合并，待量化后另行决定。
