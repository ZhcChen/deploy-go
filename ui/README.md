# UI 设计源

`ui/` 是 `admin/` 与 `admin-app/` 的统一设计源，参考同类静态 UI 预览模块，使用纯 `HTML + CSS + JavaScript` 构建可交互原型。

## 目标

- 在正式实现前收敛信息架构、视觉 tokens、组件状态和关键业务流程。
- 同时展示桌面 Web 与移动 App 形态，但不把桌面页面直接缩放成手机页面。
- 使用确定性 mock 数据覆盖正常、空、加载、失败、断连和长日志状态。
- 为 Web 与 Flutter 提供明确的组件和交互交付基线。

## 技术约束

- 单页 hash router。
- 本地静态资源和统一 mock store。
- 不连接真实 API，不执行真实部署。
- 预览运行不依赖 bundler 或前端框架；自动化回归使用 npm 安装 Playwright。
- 预览状态可按需写入 `localStorage`，不得包含真实凭证。

## 启动方式

在仓库根目录启动，默认端口为 `30102`：

```bash
make ui
```

启动后访问：

```text
http://127.0.0.1:30102/#/entry
```

`make ui` 底层执行 Python 静态服务器，并对预览资源返回 `Cache-Control: no-store`，避免设计调整后仍看到旧的 CSS 或 JavaScript。设计源必须在该启动方式下正常工作，不依赖 npm、bundler 或框架开发服务器。

提交前安装锁定依赖并执行统一检查：

```bash
npm ci
make ui-test
make ui-check
```

`make ui-test` 会自行启动 `30102` 预览并运行 Chromium 回归；`make ui-check` 检查 JavaScript、Python、尾随空格和 Git diff 格式。Playwright 规格保存在 `ui/tests/ui-preview.spec.js`。

## 计划入口

- `#/entry`：设计源总入口。
- `#/spec`：设计 tokens、组件和状态规范。
- `#/web`：Web 管理端预览。
- `#/app`：App 管理端设备预览。
- `#/web/deployments`：Web 部署列表。
- `#/web/deployments/:id`：Web 部署详情与日志。
- `#/web/apps/new`：Web 应用与默认目标配置。
- `#/web/settings`：Web 系统管理。
- `#/app/deployments`：App 部署列表。
- `#/app/deployments/:id`：App 部署详情与日志。
- `#/app/mine`：App 账号、身份与设置入口。
- `#/app/mine/users`：用户管理。
- `#/app/mine/users/:id`：用户详情与启停操作。
- `#/app/mine/users/new`：管理员新增普通用户。
- `#/app/mine/profile`：个人资料与安全摘要。
- `#/app/mine/preferences`：部署与节点通知偏好。
- `#/app/mine/about`：产品信息、脚本契约与服务状态。
- `#/web/login`、`#/app/login`：登录与会话失效预览。

完整页面范围见 `ui/docs/page-map.md`。

## 第一轮设计重点

1. 全局导航和运行状态语言。
2. 部署列表的信息密度与过滤方式。
3. 部署前的高风险确认。
4. 运行中日志、进度与取消操作。
5. 成功、失败、节点断连和空数据状态。

## 推荐实现顺序

1. 设计 tokens 与共享状态组件。
2. Web 工作台外壳与 App 导航外壳。
3. 部署列表、部署详情和部署确认。
4. 节点列表 / 详情与应用列表 / 详情。
5. 空状态、错误状态、长内容和响应式复核。
6. 补齐组件清单与 Web / Flutter handoff 文档。

## 设计文档

- `ui/docs/design-tokens.md`
- `ui/docs/component-inventory.md`
- `ui/docs/page-map.md`
- `ui/docs/web-handoff.md`
- `ui/docs/flutter-handoff.md`
