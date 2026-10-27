# 独立远端开发任务

唯一入口：本目录 spec.md、plan.md。

- [x] T001 读取 Obsidian 模板、仓库规范和统一登记，盘点 qfy-test2 容器/端口/资源，记录正式环境边界。对应 FR-001/002/006。
- [x] T002 新增按 project 隔离的 Compose/运行镜像/初始化及工具入口，覆盖 owner/context/配置校验、限额、密钥权限与停止保卷。对应 FR-001/002/003/004/005。
- [x] T003 新增离线危险配置回归并渲染配置；只读复核，修正后聚焦验证并提交运行配置。对应 SC-001。
- [x] T004 登记唯一 project 和镜像、串行创建远端开发栈，执行 API/Web/Playwright 验收、停止保卷/恢复及正式环境 PID/健康对照。对应 SC-001/002/003。
- [x] T005 同步 runbook、AGENTS/CLAUDE 开发入口、Obsidian 协调索引与验收证据，运行门禁并提交推送。对应 FR-006、SC-003。

## 实施证据

T002/T003：12 项离线回归通过，包含默认 context 指向远端、无标签同名资源、具名卷 driver_opts 隐藏宿主绑定及 use_api_socket。另一个只读复核者指出上述边界和镜像覆盖风险，均已修正，最终 findings=[]。正式环境发布已先完成并健康，开发栈后续创建不会调用正式 installer。

T004/T005：真实创建、浏览器、保卷停止恢复与正式 PID/健康对照通过；Obsidian 和统一资源登记已同步。完整证据及未测边界见 `docs/reviews/2026-10-27-isolated-remote-development-review.md`。
