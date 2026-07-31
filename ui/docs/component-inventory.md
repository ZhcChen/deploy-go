# UI 组件清单

## 共享组件

| 组件 | 用途 | 必须覆盖的状态 |
| --- | --- | --- |
| `Status Badge` | 节点、应用和部署状态 | 在线 / 离线、正常 / 异常、排队 / 运行 / 成功 / 失败 / 取消 |
| `Primary Button` | 发起和确认明确命令 | Web：默认、悬停、聚焦、按下、禁用；App：默认、按下、聚焦、禁用 |
| `Danger Button` | 取消部署等高影响命令 | 默认、确认中、禁用 |
| `Icon Button` | 复制、跟随、跳到末尾和返回 | Web：默认、悬停、聚焦、选中；App：默认、按下、聚焦、选中 |
| `Search Field` | 列表关键字过滤 | 空、输入、聚焦、无结果 |
| `Segmented Control` | 状态快速筛选 | 默认、当前项、键盘聚焦 |
| `Continuous List / Table` | 部署、应用和节点目录 | 默认、悬停、密集、空 |
| `Resource Mark` | 稳定的资源识别锚点 | 应用缩写、节点 icon、部署编号 |
| `Summary Strip` | 详情页关键元数据 | 2 列移动布局、4 列桌面布局 |
| `Timeline` | 部署阶段 | 待处理、已完成、当前、失败结束 |
| `Log Workspace` | 脚本输出阅读与工具操作 | 连接、跟随、暂停、断连、结束、空日志 |
| `Empty State` | 没有资源或过滤无结果 | 有下一步、只读说明 |
| `Notice` | 阻断、警告和异常摘要 | 警告、危险 |
| `Confirmation Modal` | 部署和取消的二次确认 | 打开、返回、确认 |
| `Toast` | 短时操作反馈 | 成功、信息 |

## Web 复合组件

- `Web Shell`：固定导航、页面标题、主要动作和内容工作区。
- `Metric Strip`：概览中的运行数量、成功率、失败和异常节点。
- `Deployment Table`：应用、状态、目标、版本、发起人与时间的连续表格。
- `Deployment Detail`：状态时间线、日志主区和执行信息侧栏。
- `Resource Detail`：资源摘要、部署目标或关联记录、基础信息侧栏。
- `Resource Editor`：节点、应用和部署目标的连续表单、检查状态和右侧约束说明。
- `Settings Workspace`：唯一管理员的用户、默认值和审计工作区。
- `Settings Subnav`：系统设置、用户管理和审计记录的常驻二级菜单与当前项。
- `Login Panel`：登录失败、会话失效和管理员分配账号说明。

## App 复合组件

- `Mobile Shell`：设备状态区、页面内容和五项底部导航。
- `Mobile Resource Row`：资源标记、主摘要、状态和时间。
- `Mobile Deployment Flow`：应用选择、目标核对、确认与详情跳转。
- `Mobile Log Reader`：全宽日志阅读面及固定操作区。
- `Mobile Account Workspace`：固定身份栏、连续设置列表、用户管理和退出确认。

## 交互约束

- Web 可以使用 hover 辅助扫描；App 不定义 hover，只使用 pressed、selected、disabled 和 focus。
- 所有可见命令必须进入页面、改变 Mock 状态或打开确认，不保留仅说明“后续实现”的占位 Toast。
- 未知资源、权限不足和会话失效使用独立状态，不复用空数据页面。

## 状态契约

- 部署：`queued`、`running`、`success`、`failed`、`canceling`、`canceled`。
- 应用：`healthy`、`deploying`、`error`、`archived`。
- 节点：`online`、`offline`、`checking`、`disabled`。
- 日志：连接状态独立于部署状态；日志断开不能自动显示部署失败。
