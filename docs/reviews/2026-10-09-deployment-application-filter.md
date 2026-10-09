# 部署记录应用筛选复核

需求唯一入口：specs/003-deployment-application-filter/。

## 变更与边界

部署记录新增可搜索应用下拉，默认全部应用，与状态联合筛选。应用选项自动加载后续页，失败提示重试。服务端在分页前绑定应用和状态条件，保留授权与旧记录应用关联；查询缓存隔离，切换条件清除页码，旧翻页结果不得推进新条件页码。

无 migration、Agent 或业务应用改动。新增参数可选，原列表调用兼容；同步 docs/standards/api-contract.md 和标准生成双端客户端。

## 审查发现与修正

独立只读审查确认权限、稳定 cursor、客户端参数映射一致；发现组合输入期间 Enter/ArrowDown 会误选，已跳过 isComposing/keyCode 229 事件。回归在存在匹配项时验证不选择、不关闭、不转移焦点。共享非搜索 Select 的键盘行为也有回归覆盖。

analyze 复核规格、方案和任务一致；converge 对照 FR-001–005、SC-001、计划实现点与原则，无剩余实现缺口，不追加空阶段。

## 验证

- cargo test -p deploy-go-api --test deployments_api：9/9，覆盖管理员/普通用户跨页、相同时间 ID 排序、旧 NULL 关联、不可访问/不存在应用与非法状态。
- npm --prefix admin run check：lint/typecheck、165/165 单元/组件测试、构建通过。
- 最后调整输入法回归后 SearchableSelect 聚焦测试 2/2。
- Playwright deployment-application-filter、deployment-flow、responsive-layout：10/10，包含桌面/窄屏、搜索键盘选择、联合筛选、页码重置、axe 与原部署流程。
- make api-openapi-check、make api-client-check：通过。客户端生成使用隔离 Docker Dart，未手改 generated 或修改本机 SDK。
- git diff --check：通过。

## 发布

正式控制面 qfy-test2 身份 DESKTOP-H0KSULB 已核对，API/Web active，发布前无 queued/running/canceling 部署。
按会话已有授权使用 DEPLOY_BUILD_MODE=local DEPLOY_AGENT_SYNC=0 make deploy-production，只更新本项目控制面与工具，无业务应用部署。
安装与线上验收待完成后补记。
