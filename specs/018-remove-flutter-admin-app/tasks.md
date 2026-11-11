# 移除 Flutter 管理端任务

输入：[spec.md](spec.md)、[plan.md](plan.md)。状态：已实施并通过本地门禁，待用户签收。勾选仅由用户更新。

## 实施与验证

- [ ] T001 FR-001 删除 `admin-app/`（已跟踪文件与本机被忽略的产物）。
- [ ] T002 FR-002 FR-003 FR-004 FR-005 Makefile、生成脚本、检查脚本、文档检查与 `.dockerignore` 清理。
- [ ] T003 FR-006 FR-007 当前有效文档修订与计划索引说明。
- [ ] T004 FR-008 FR-009 确认后端与 Web 无改动，登记 018。
- [ ] T005 SC-001 至 SC-004 全量验证、清扫、自审、提交推送。

## 验收证据

| 任务 | 实施进度 | 验证结果 | 组件发布 | 人工签收 | 证据 |
| --- | --- | --- | --- | --- | --- |
| T001 | 已完成 | 已验证 | 不适用 | 待用户签收 | 删除前核对：跟踪文件 434 个；被忽略的本机产物约 72MB（`build/` 52MB、`.dart_tool/` 11MB 等）；检索 `*.jks`、`*.keystore`、`key.properties`、`*.p12`、`.env*`、`*.pem`、`google-services.json` 等均为空。删除后 `git ls-files admin-app` 为 0，目录不存在 |
| T002 | 已完成 | 已验证 | 不适用 | 待用户签收 | `node --check` 两个 mjs、`bash -n run-checks.sh`、`py_compile docs-check.py` 通过；`make help` 无 Flutter 条目，`make -n check-serial` 可解析；去掉 `dart`/`flutter` 的 PATH 下 `node scripts/generate-api-clients.mjs --check --force` 通过，约 2.1 秒（2026-11-10 实测旧流程里 `dart pub get` 123.8 秒、`build_runner` 251.6 秒）；`make api-client-generate` 前后 `admin/src/api/generated/` 174 个文件的目录摘要同为 `ede3e8ab1a826178`；变异验证：改动 `models/NodeResponse.ts` 后检查退出 1 并报“内容不同”，还原后通过 |
| T003 | 已完成 | 已验证 | 不适用 | 待用户签收 | 修订 README、AGENTS.md、`docs/runbooks/` 3 份、`docs/standards/` 6 份，另改 `admin/src/api/README.md` 与 `ClientBehaviorContract.test.tsx` 的措辞；`docs/plans/README.md` 补废弃说明。SC-002 清扫（忽略大小写检索 `admin-app`、`flutter`、`dart-define`、`pubspec`、`dart`、“双端”“两端”）只剩 018 自身的 feature 标识与登记 |
| T004 | 已完成 | 已验证 | 不适用 | 待用户签收 | 本规格范围的 diff 不含 `api/`、`agent*/`、migration、协议与版本号；`/api/v1/auth/preferences` 仍由 Web `ProfilePage` 使用，后端无 app 专属代码。登记 `specs/README.md` 与存储登记；`spec-kit-check` 的 plan、tasks、implement 三个阶段通过 |
| T005 | 已完成 | 已验证 | 不适用 | 待用户签收 | 合并工作区（含同期 017 未提交改动）`make check` 通过：rust 180s、contracts 26s、docs 6s、web 23s、client 0s（命中记忆化），总墙钟 180s，检查组不再含 flutter；Rust 983 个用例通过、2 个跳过，Web 27 个文件 174 个用例，`docs-check` 覆盖 211 个 Markdown，spec-kit 21 项。`whitespace-check`、`client-sensitive-check`（141 个文件）通过。提交与推送见 Git 历史 |
