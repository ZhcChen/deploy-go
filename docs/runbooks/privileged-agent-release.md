# Agent 原生结构化特权 Release 手册

## 适用范围

本手册用于开发、隔离验证和灰度 Agent 原生 `privileged_release`。它不授权连接真实节点；升级、重启或执行 self-test 前，仍需当前对话对具体测试节点的明确授权。

本能力是两阶段与镜像直连部署固定使用的 release 执行后端：prepare 始终由低权限 `deploy-go-runner` 执行。平台已移除目标级 `privileged_release` 开关，launcher 仅作为历史兼容参考，不自动选择或回退。

## 版本和能力

- Agent 控制协议：v7+（当前 v11；镜像直连要求 v11 通用 artifact checkout）。
- executor 本机协议：v2。
- Agent capability：`privileged_release`。
- 部署目标不再暴露 `privileged_release` 配置；release 固定特权，内部固定为 1。
- 节点终端字段 `privileged_execution` 和 capability `pty_terminal` 与本能力无关。

Agent 只有在 executor v3 release probe 健康时才上报 `privileged_release`。终端 probe 与 release probe 独立，任一失败不能伪造另一项能力。

## 主控生产签名密钥

API 通过 `DEPLOY_GO_RELEASE_SIGNING_KEY_FILE` 读取 release 专属私钥并签发短期授权，不通过 Agent 安装命令、日志、数据库或浏览器响应传递私钥正文。正式部署的 `deploy/production/install.sh` 会在首次部署时生成独立文件 `/etc/deploy-go/release-signing.key`，与终端签名密钥分离：

- 文件为 base64 编码的 32 字节 seed，权限 `0440 root:deploy-go`，普通文件且禁止符号链接。
- 重复部署只复用，不覆盖；为空、符号链接或非普通文件时安装器拒绝继续并提示恢复。
- 密钥纳入安装事务备份与回滚；API systemd unit 只注入文件路径并配合 `ProtectSystem=strict` 与 `ReadOnlyPaths` 只读访问。
- 安装命令只包含对应公钥并写入 executor 配置的 `release_public_key`；任何输出不得包含私钥正文。

生产环境未生成或未注入该密钥时，API 会在加载签名器阶段失败，任何特权 release 都不能签发；修复前不要部署当前 main。

## 固定执行合同

executor 只允许执行：

```text
make --no-print-directory deploy-go-release
```

请求不得指定 command、shell、executable、args、Make target 或任意环境变量 map。root child 清空继承环境，只保留本机固定最小 `PATH` 和：

```text
DEPLOY_ID
DEPLOY_ENVIRONMENT
DEPLOY_RELEASE_VERSION
DEPLOY_COMMIT_SHA
DEPLOY_MODULES
DEPLOY_TARGET
DEPLOY_ARTIFACT_DIR
DEPLOY_ENV_DIR
DEPLOY_CANCEL_FILE
```

业务仓库仍需提供 `Makefile`、`deploy-go-release` target 和业务 release 脚本，不再需要为原生模式安装应用专属 launcher、sudoers 或系统目录脚本。

## 本地和隔离验证

先执行聚焦检查：

```bash
make privileged-release-check
```

Linux cgroup 与身份隔离必须在隔离 Linux 环境执行：

```bash
make agent-runner-isolation-check
make agent-executor-cgroup-check
```

提交前执行：

```bash
make api-openapi-check
make api-client-check
make check
git diff --check
git diff --cached --check
```

聚焦测试必须证明：目标 API/界面不再暴露 `privileged_release` 开关；snapshot 变化使旧 preview 失效；v6 普通 release 兼容；任意命令、路径逃逸、symlink、hardlink、非普通文件、额外环境和错误签名在 spawn 前拒绝；成功、非零退出、超时、取消与断线恢复保持唯一终态；root job cgroup 最终为空。

## 测试节点灰度

只有 U1-U8 已完成、验证通过，并且用户再次明确授权“测试环境节点（WSL）”后才能执行。先确认节点 ID、Agent ID 和环境不是生产节点。

WSL 测试节点必须为 WSL 2 且已启用 systemd/cgroup v2；无 systemd 或控制器为空时，安装器以 `cgroup_v2_missing` 拒绝并回滚，不得绕过该检查或用无 systemd 节点继续灰度。

1. 使用当前主控生成的幂等安装命令配对升级 Agent、runner 和 executor。
2. 在节点执行无敏感输出的诊断：

   ```bash
   sudo -u deploy-go-agent /usr/local/bin/deploy-go-agent status
   sudo -u deploy-go-agent /usr/local/bin/deploy-go-agent doctor
   systemctl status deploy-go-agent deploy-go-agent-runner deploy-go-agent-executor --no-pager
   ```

3. 在主控确认节点在线、控制协议 v11（最低 v7），并上报 `privileged_release`。
4. 运行 Deploy Go 自带的 privileged release self-test：

   ```bash
   sudo -u deploy-go-agent /usr/local/bin/deploy-go-agent privileged-release-self-test
   ```

   self-test 通过独立 executor v3 operation 使用平台固定 checkout/Makefile，只输出测试事件和 `privileged-release-self-test uid=0` 后退出，用于确认固定 Make 入口、root UID、环境白名单、日志、退出码和 cgroup 清理。请求不接受 command、args、path 或 env；fixture 不读取业务 Env，不调用 Docker，不修改 systemd 业务服务或生产数据。
5. 确认未创建或修改 `qfy-voucher-hub` 部署目标，未发起任何业务 prepare/release，也未操作生产节点。

仅看到 capability 不足以证明执行链路可用；必须同时通过 self-test。self-test 不是业务部署授权。

## 失败处理

- **安装器报 `cgroup_v2_missing`**：先确认 `systemd-detect-virt`、`/proc/1/comm`、`mount | grep cgroup` 和 `cat /sys/fs/cgroup/cgroup.controllers`；控制器为空、缺少 cgroup2 挂载或 `systemd` 未托管时需修复环境。sysfs 伪文件 `stat` size 为 0，安装器按文件内容判断，不能以 `test -s` 判定。WSL 2 节点需启用 systemd 并重启 WSL；enrollment token 若已消费需重新签发。不得跳过该检查或放宽 executor 运行条件。
- **协议低于 v7 或缺少 capability**：停止发起新 deployment，重新执行配对安装；不得让任务自动回退 launcher。所有镜像任务还需协商到 v11 的通用 artifact checkout。
- **executor v3 probe 失败**：检查三个服务版本、executor Socket、配置公钥、cgroup v2 和 `Delegate=yes`。Agent 可以保持普通部署在线，但不得声明特权 release。
- **doctor 显示 `EXECUTOR_PROTOCOL`/`PRIVILEGED_RELEASE` 不可用且 executor journal 反复 `unauthorized local peer`**：通常是旧 executor 的 peer PID 绑定未随连接关闭释放，Agent 服务进程挡住了一次性 doctor/self-test。重新安装当前 0.2.0 发布物并重启 executor；不要放宽 Socket 权限或跳过 peer 校验。
- **授权验签失败**：核对 API release authorization 私钥与 executor 公钥配对、节点/Agent/snapshot/commit/deadline 绑定和系统时间；不得跳过验签或清空 nonce 后重放任务。
- **bundle 校验失败**：保留源任务和脱敏元数据用于诊断，不从低权限 checkout 直接执行；检查 symlink/hardlink、digest、文件类型和并发改写。
- **日志或磁盘预算触发**：任务应以稳定错误失败并清理 cgroup；不得扩大上限后重放同一有副作用 job。
- **cgroup 清理失败**：停止接受新特权 release，按 executor 日志确认残留进程并人工恢复；不得把清理错误标记成功。

诊断日志不得包含签名授权、nonce、token、Env 正文、Git 私钥或业务 Secret。

## 回退

升级失败时，安装器必须成对恢复上一版 Agent/executor 和三个 unit，并确认普通 Agent 重新在线。若升级已完成但 self-test 失败：

1. 停止发起新的 deployment；平台不存在目标级 `privileged_release` 开关，无需也无法关闭。
2. 停止 Agent，再停止 runner 和 executor。
3. 成对恢复上一版发布物与配置，按 executor、runner、Agent 顺序启动。
4. 确认普通低权限部署能力和原 launcher 兼容路径未改变。

已创建 deployment 的 snapshot 不受回退影响；已选择 executor 的失败任务不得转由 launcher 自动重跑。

## 灰度记录

只记录以下非敏感事实：主控版本、Agent/executor 版本、控制/本机协议、节点与 Agent ID、capability、self-test 结果、升级与回退结果。不得记录 Env、token、签名授权正文或业务日志中的 Secret。
