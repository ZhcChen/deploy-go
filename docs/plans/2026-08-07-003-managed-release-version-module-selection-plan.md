# 发布版本托管与模块多选计划

## 目标

完善两阶段部署的发起流程：发布版本由主控自动生成，模块由管理员从业务目标声明的白名单中多选，默认全选，不再要求操作人员记忆或填写内部参数。

## 决策

- `release-version` 在生成预览时由 API 创建，确认请求复用预览返回值；snapshot 校验保证预览与最终部署使用同一版本。
- 继续兼容旧客户端显式传入 `release-version`，新管理端不展示该字段。
- 业务模块清单由目标 `parameter_schema` 的 `modules.x-options` 声明，平台不写死具体业务模块。
- `modules` 对业务脚本仍保持逗号分隔字符串，避免改变 Agent 与现有脚本协议。
- API 同时校验 `x-options` 声明及用户选择，前端多选不是唯一安全边界。

## 执行单元

1. **API 契约与校验（已完成）**
   - 自动生成 17 位 UTC 毫秒时间版本。
   - 确认请求增加可选 `release_version`，并重新生成 OpenAPI 与双端客户端。
   - 参数 schema 支持受限的 `x-options`，校验数量、唯一性、字符与提交值白名单。
2. **Web 交互（已完成）**
   - 两阶段部署隐藏 `release-version` 与通用 `modules` 输入。
   - 模块选项默认全选，支持逐项选择、全选和取消全选；未选择模块时禁止生成预览。
   - 更新目标编辑提示与视觉样式。
3. **规范与验证（已完成）**
   - 更新应用部署契约及 schema 示例。
   - 覆盖自动版本、确认复用、模块默认全选、取消全选门禁及 API 白名单测试。

## 验收结果

- `cargo test -p deploy-go-api --test two_stage_deployment`：11 项通过。
- `cargo test -p deploy-go-api execution_spec::tests::parameter_schema`：2 项通过。
- `npm run check --workspace deploy-go-admin`：70 项测试、lint、typecheck、production build 全部通过。
- `make api-client-check` 与 `git diff --check` 通过。
