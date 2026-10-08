# 容量日志实施任务

唯一入口 spec.md/plan.md。按小闭环推进，不把整套功能作为单个实施单元。

## 阶段一：规划与门槛

- [x] T001 核对既有日志计划、容量与权限，形成 spec.md/plan.md；对应 FR-001–FR-009。
- [x] T002 只读 analyze 及独立方案核验，执行项目/上游阶段检查；migration 前运行 make setup-git-hooks、make verify-git-hooks。独立复核U1/U2恢复水位及安全回收遗漏已补入plan，发布授权按完整会话持续有效。

## 阶段二：部署输出容量（US1）

- [x] T003 在 api/src/settings/mod.rs、api/migrations/ 新增日志容量默认与事务计数；api/src/deployments/runtime.rs 容量和周期回收共用receipt完整性判据，仅删安全终态输出及副本；验证 api/tests/settings_api.rs、部署恢复、终态交付未完成/晚到重传/计数并发保护。对应 FR-001/002、SC-001。证据：settings 3、recovery 9、dispatcher 37、migrations 16项通过，独立复核补齐结果一致性、越界事件与非部署任务输出回收。
- [x] T004 更新 admin 设置、主 OpenAPI/generated client、相关测试；同步容量辅助周期语义，完成 US1 聚焦验证与独立复核。依赖 T003；对应 FR-009。证据：SystemManagement 7项、双端client漂移检查通过，旧PATCH保留容量与版本CAS通过。

## 阶段三：运行日志存储（US2）

- [x] T005 在 runtime-log/ 实现有界分段、持续序号、尾部恢复、固定目录安全、有界读写及安全诊断投影；小预算轮转/重启/恶意文件/敏感字段回归。同步 workspace/release Dockerfile。对应 FR-003/004/005、SC-002。证据：shared 6项及发布manifest契约通过；空闲读元数据优化经独立复核。
- [x] T006 在 api/src/runtime_logs.rs 持久化及限内存/队列字节，SSE 保持授权和游标，回查受管文件；重启、慢订阅、磁盘拒绝与原 runtime API 回归。依赖 T005；对应 FR-003/004、SC-004。证据：API runtime lib 4项、ingest/SSE fixture通过，队列2MiB、内存8MiB、写盘专用线程；真实取消窗口验证通过。

## 阶段四：节点集中采集（US3）

- [x] T007 接入 Agent/Broker/Executor/updater 受控运行日志及安装器权限，不读取业务日志；Linux fixture 验证 Agent 可读/root组件写、目录越界拒绝。依赖 T005；对应 FR-003/005/008。证据：隔离Docker真实身份fixture、unit静态契约、Agent短命命令2项/updater 6项通过；手工安装降级遗漏另列T012。
- [x] T008 实现 Agent 有界批次上送与 API agent access 认证接收/来源绑定/独立持久水位与epoch/重复去重/gap统计，组件游标固定路径持久化；HTTP 断线/ACK丢失/旧服务404/跨节点伪造、来源日志全部轮转后重传/epoch状态丢失/日志写后崩溃回归。依赖 T006/T007；对应 FR-005/006、SC-003。证据：Agent HTTP/cursor 3项、API ingest原12项+真实取消1项通过；独立持久水位0041，恢复失败禁止竞争追加。
- [x] T009 补充安全阶段交接/恢复/缓存/清理诊断，增加管理员节点/组件筛选，更新主 OpenAPI/client及 UI 聚焦测试。依赖 T008；对应 FR-007/009、SC-004。证据：RuntimeLogsPage 2项含gap/丢弃/写盘失败统计、typecheck/lint及OpenAPI内外契约各9项通过；白名单投影不格式化秘密。

## 阶段五：交付

- [ ] T010 同步日志容量/采集/恢复运行手册，记录独立安全/可靠性审查、真实测试及 converge；只暂存本轮文件，小闭环提交推送。对应 SC-001–SC-004。
- [ ] T011 版本 0.3.28 成对构建，按会话持续授权部署正式控制面；核对健康、发布物、正常节点串行自动升级和集中诊断，记录运行证据，不重发业务部署。依赖 T010；对应 SC-005。

## 依赖与并行

T001→T002；T003→T004 与 T005 可独立。T005→T006/T007→T008→T009→T010→T011。读密集核验可委派；并行写须独立 worktree 且文件不重合，主 agent 串行集成。所有勾选须有实际文件/验证证据，不以结构检查代替验收。

## 阶段六：Convergence

- [x] T012 修正 agent/install/install.sh 日志准备失败仍中断手工安装的遗漏；正式安装复用新版 Agent 的固定目录准备命令，兼容旧发布物，INSTALL_ROOT fixture 不操作宿主机目录。补 agent/tests/install.bats 的恶意日志路径与失败降级验证，目录准备失败不得回滚安装；与自动升级首轮 ExecStartPre 及可选 ReadWritePaths 合同共同复核。对应 FR-005/008、SC-003；依赖 T007。证据：隔离Linux Bats19/19、shell/unit契约通过，独立复核确认版本门槛及FD路径安全，原阻塞关闭。
