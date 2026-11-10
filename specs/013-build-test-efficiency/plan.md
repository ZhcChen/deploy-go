# 构建与检查效率优化（第一批）实施方案

规格：[spec.md](spec.md)。状态：方案就绪；证据见 [tasks.md](tasks.md)。

## Technical Context

本机 GNU Make 3.81（无 `--output-sync`），因此并行由自写的 Bash 运行器完成而不是 `make -j`。已测基线：`make check` 串行约 376 秒；`api-client-check` 约 214 秒，其中 `dart pub get` 约 124 秒、`build_runner` 约 252 秒（两者是同一次运行里的两段，之和大于总墙钟说明另一次浮动，取实测日志）；Java 生成器仅约 3 秒。clippy 20 秒、nextest 约 45 秒（预热后）、admin 四项约 23 秒、Flutter 两项约 18 秒。部署日志目前不留存。

## Constitution Check

遵守 [AGENTS.md](../../AGENTS.md)：不放松门禁、不改产品代码；Makefile、脚本与部署脚本日志行为属于验证与运维策略，走完整产物；不运行真实部署；日志不含密钥。

## 实现方案与文件范围

1. 记忆化（FR-001、FR-002）：`scripts/generate-api-clients.mjs`。新增 `computeFingerprint()`：对列出的输入逐个取 sha256，目录按排序后的相对路径与内容聚合；读写 `target/.api-client-check.stamp`（`target/` 已被忽略，`cargo clean` 会使其失效，这是期望的）。`--check` 下先比对，一致则打印“输入未变，已跳过”并退出 0；完整检查通过且 `process.exitCode` 未被置为失败后才写入。`--force` 跳过比对。`dart pub get` 包装为先 `--offline` 再联网。
2. 并行运行器（FR-003、FR-004）：新增 `scripts/test/run-checks.sh`。检查组：
   - rust：`make api-check`（fmt、clippy、nextest+doctest、openapi-check），串行；
   - contracts：`agent-install-check agent-manifest-check deploy-contract-demo-check privileged-launcher-check app-template-static deployer-static external-deploy-static migration-git-guard-self-test deploy-production-check dev-remote-check`；
   - docs：`spec-kit-verify docs-check whitespace-check client-sensitive-check`；
   - web：`admin-check`；flutter：`admin-app-check`；client：`api-client-check`。
   各组把输出写入临时文件，完成后整体打印；汇总每组耗时和失败。`--serial` 顺序执行同一批目标。Makefile：`check` 调用运行器，`check-serial` 保留旧的依赖链；`deployer-check` 等拆出 `*-static` 子目标，原目标由静态部分加原有 `cargo test` 组成，行为不变。
3. 部署耗时与日志（FR-005、FR-006）：`deploy/production/deploy.sh` 顶部增加日志留存（`.data/deploy-logs/`，目录不可写时降级为提示）、`stage` 计时函数与 EXIT trap 汇总；`remote-build.sh` 为两个构建各打一行耗时；`test-install-contract.sh` 增加断言与降级用例。保留最近 20 份日志。
4. 文档（FR-007）：`development-guide.md`、`local-development.md`、`systemd-deployment-production.md`、Makefile help；`specs/README.md` 与存储登记登记 013。

## 阶段与依赖

1. 记忆化与离线 `pub get`，先实测命中与未命中。
2. Makefile 拆分与并行运行器，并与串行结果对照。
3. 部署脚本日志与契约测试，依赖现有契约测试框架。
4. 文档、登记、全量验证、自审 diff、converge、提交推送。

## 验证计划

- 记忆化：首次完整跑、第二次命中（计时）、改 `openapi.json`/手改生成文件/改脚本各触发一次重跑，检查失败不写记忆；`--force`。
- 并行：`make check` 与 `make check-serial` 各一次，对照通过结果与耗时；注入一个失败（临时改坏一个文档链接或格式）确认失败组被列出，随后还原。
- 部署：`make deploy-production-check`；契约测试覆盖日志目录不可写降级。
- 文档与门禁：`make docs-check`、`make spec-kit-verify`、`make whitespace-check`。

## 风险与回退

记忆化指纹漏项是首要风险，以逐类输入变更验证；运行器与记忆文件都可单独回退，`check-serial` 与 `--force` 提供旧语义的逃生口。
