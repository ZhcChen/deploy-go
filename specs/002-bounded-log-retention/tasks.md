# 容量日志实施任务

唯一入口 spec.md/plan.md。按小闭环推进，不把整套功能作为单个实施单元。

## 阶段一：规划与门槛

- [x] T001 核对既有日志计划、容量与权限，形成 spec.md/plan.md；对应 FR-001–FR-009。
- [ ] T002 只读 analyze 及独立方案核验，执行项目/上游阶段检查；migration 前运行 make setup-git-hooks、make verify-git-hooks。

## 阶段二：部署输出容量（US1）

- [ ] T003 在 api/src/settings/mod.rs、api/migrations/ 新增日志容量默认与事务计数；api/src/deployments/runtime.rs 容量回收只删终态输出，验证 api/tests/settings_api.rs、部署恢复和计数/并发保护。对应 FR-001/002、SC-001。
- [ ] T004 更新 admin 设置、主 OpenAPI/generated client、相关测试；同步容量辅助周期语义，完成 US1 聚焦验证与独立复核。依赖 T003；对应 FR-009。

## 阶段三：运行日志存储（US2）

- [ ] T005 在 runtime-log/ 实现有界分段、持续序号、尾部恢复、固定目录安全、有界读写及安全诊断投影；小预算轮转/重启/恶意文件/敏感字段回归。同步 workspace/release Dockerfile。对应 FR-003/004/005、SC-002。
- [ ] T006 在 api/src/runtime_logs.rs 持久化及限内存/队列字节，SSE 保持授权和游标，回查受管文件；重启、慢订阅、磁盘拒绝与原 runtime API 回归。依赖 T005；对应 FR-003/004、SC-004。

## 阶段四：节点集中采集（US3）

- [ ] T007 接入 Agent/Broker/Executor/updater 受控运行日志及安装器权限，不读取业务日志；Linux fixture 验证 Agent 可读/root组件写、目录越界拒绝。依赖 T005；对应 FR-003/005/008。
- [ ] T008 实现 Agent 有界批次上送与 API agent access 认证接收/来源绑定/持久确认/重复去重/gap统计，组件游标固定路径持久化；HTTP 断线/ACK丢失/旧服务404/跨节点伪造回归。依赖 T006/T007；对应 FR-005/006、SC-003。
- [ ] T009 补充安全阶段交接/恢复/缓存/清理诊断，增加管理员节点/组件筛选，更新主 OpenAPI/client及 UI 聚焦测试。依赖 T008；对应 FR-007/009、SC-004。

## 阶段五：交付

- [ ] T010 同步日志容量/采集/恢复运行手册，记录独立安全/可靠性审查、真实测试及 converge；只暂存本轮文件，小闭环提交推送。对应 SC-001–SC-004。
- [ ] T011 版本 0.3.28 成对构建，按会话持续授权部署正式控制面；核对健康、发布物、正常节点串行自动升级和集中诊断，记录运行证据，不重发业务部署。依赖 T010；对应 SC-005。

## 依赖与并行

T001→T002；T003→T004 与 T005 可独立。T005→T006/T007→T008→T009→T010→T011。读密集核验可委派；并行写须独立 worktree 且文件不重合，主 agent 串行集成。所有勾选须有实际文件/验证证据，不以结构检查代替验收。
