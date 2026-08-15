# Agent v11 特权终端与回退

> v11 Agent 将 root PTY 作为标准配对能力，不存在节点级启用开关。对真实节点安装、升级、重启或连接终端前，仍须获得针对节点和动作的明确授权。

## 适用范围

本手册用于在 Linux systemd 节点启用节点详情中的“SSH”终端。页面名称沿用运维习惯，实际不开放 SSH 端口，也不使用 SSH key 或本地 SSH config。链路为：

```text
浏览器 -> API WebSocket -> Agent WSS v11 -> Unix Socket -> root executor -> PTY
```

真实节点的安装、升级、重启、开关切换和回退都属于运行态操作，必须在当前对话中获得针对具体环境和节点的明确授权。

## 安全边界

- `deploy-go-agent` 继续以低权限用户运行并负责联网；`deploy-go-agent-executor` 以 root 运行，但只监听本机 Unix Socket。
- executor 不读取 Agent token、不实现网络客户端，也不接受 shell、用户、环境变量或任意命令作为打开会话参数；PTY 子进程是完整 root 登录终端，可以联网和管理主机。
- 只有管理员可发现和连接终端；控制面只接受在线、身份有效、PTy executor 健康的 v11 Agent。
- API、Agent、数据库、审计和浏览器存储均不得持久化终端输入输出正文。
- 同一节点最多一个活动终端会话。浏览器、Agent、API 或 executor 任一链路断开时必须收敛会话并清理进程组。
- API 使用独立 Ed25519 私钥为每次 open 签发 15 秒单次 capability；Agent 仅透传，executor 使用公钥离线验签并持久化消费标记。
- Linux 上每个 PTY 使用 executor systemd cgroup 下的独立 cgroup v2 子组；普通 `setsid`、double-fork 和忽略 TERM 后代由 `cgroup.kill` 收敛。完整 root 可以主动迁出 cgroup，因此该机制只保证正常断线回收，不是 root 沙箱。
- 业务部署仍使用结构化任务和应用脚本，不得改用特权终端作为常规部署通道。

## 版本与能力门禁

终端可用必须同时满足：

1. 当前用户是唯一管理员。
2. Agent 身份有效且节点在线。
3. Agent 协商协议版本为 v11。
4. Agent 上报 `pty_terminal`，表示本机 executor 探测健康。

v10 及更早 Agent 不得注册、连接或执行部署任务，必须使用平台安装命令重新安装 v11 配对 Agent/runner/executor。不得为了显示终端入口而伪造能力。

## 上线前检查

在仓库根目录执行：

```bash
make privileged-terminal-check
make agent-executor-cgroup-check
npm run check --workspace deploy-go-admin
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Linux 隔离环境还必须执行：

```bash
bats agent/tests/install.bats
systemd-analyze verify \
  agent/install/deploy-go-agent.service \
  agent/install/deploy-go-agent-runner.service \
  agent/install/deploy-go-agent-executor.service
```

完成信号：v11 终端和部署、executor 权限与 PTY 生命周期、Agent 桥接、API 授权与审计、Admin 门禁测试全部通过；隔离 Linux cgroup v2 测试覆盖脱离进程组的后代清理和 reader 有界退出；安装器测试覆盖首装、幂等升级和整对回滚。

## 灰度启用

### 1. 更新主控

先部署包含 v11 协议、终端 API 和签名密钥配置的主控。控制面重启后旧 Agent 会因协议不兼容断开，必须逐节点重新安装 v11 配对发布物。确认：

```bash
curl --fail http://127.0.0.1:30100/readyz
systemctl is-active deploy-go-api deploy-go-web
test "$(stat -c '%U %G %a' /etc/deploy-go/terminal-signing.key)" = 'root deploy-go 440'
```

API 环境必须设置 `DEPLOY_GO_TERMINAL_SIGNING_KEY_FILE=/etc/deploy-go/terminal-signing.key`。私钥只能由 API 读取，不得写入 Agent 安装命令、日志、数据库或浏览器响应；安装命令只包含对应公钥。

不得保留旧 Agent 继续承担部署任务；确认每个需要使用的节点已重新安装并以 v11 在线。

### 2. 升级单个非关键节点

使用平台生成的安装命令升级一个非关键节点。manifest v3 必须配对校验并安装相同版本的 Agent、runner broker 和 executor。节点上检查：

```bash
systemctl is-active deploy-go-agent-executor deploy-go-agent-runner deploy-go-agent
systemctl show deploy-go-agent-executor -p User -p Group -p NoNewPrivileges
systemctl show deploy-go-agent-executor -p Delegate -p KillMode -p ControlGroup
systemctl show deploy-go-agent-runner -p User -p Group -p NoNewPrivileges
systemctl show deploy-go-agent -p User -p Group
stat -c '%U %G %a %n' /run/deploy-go-agent /run/deploy-go-agent/executor.sock
stat -c '%U %G %a %n' /var/lib/deploy-go-agent-executor/used-capabilities
journalctl -u deploy-go-agent-executor -u deploy-go-agent-runner -u deploy-go-agent --since '10 minutes ago' --no-pager
```

预期：executor 为 root、`Delegate=yes` 且 `KillMode=control-group`，主机使用 cgroup v2；unit 不设置 `IPAddressDeny`、`RestrictAddressFamilies`、`PrivateDevices`、`PrivateTmp`、`UMask` 等阻止完整 root 登录行为的隔离项；Agent 仍为 `deploy-go-agent`；Socket 权限为安装合同规定的 root/Agent 组边界；capability replay 目录为 `root root 700`；`InaccessiblePaths` 降低误读 Agent 凭证的概率但不能防御完整 root；日志没有 token、capability 或终端正文。

### 3. 验证部署兼容

先在该节点执行一次普通业务部署。确认任务事件、日志、取消和恢复行为正确；协议低于 v11 的 Agent 不得被回退为兼容执行器。

### 4. 单节点终端验证

管理员确认节点已是在线有效的 v11 Agent 后，直接切换到“SSH”页连接。终端内仅执行无副作用检查：

```bash
id -u
printf 'deploy-go-terminal-ready\n'
stty size
```

预期：`id -u` 为 `0`，输出与 resize 正常；关闭终端后数据库只保留会话起止时间、字节计数、退出原因等元数据。

检查审计时不得搜索或复制终端正文，只核对会话 ID、节点 ID、管理员 ID、状态和退出原因。

## 停用

撤销节点 Agent 身份或停止 Agent/executor 会拒绝新会话并终止当前活动会话。随后确认：

1. “SSH”页显示 Agent 不可用状态，不能创建会话。
2. executor 中没有遗留 PTY 子进程。
3. 普通部署任务仍可执行。

需要停止本机能力时，顺序固定为：

```bash
systemctl stop deploy-go-agent
systemctl stop deploy-go-agent-runner
systemctl stop deploy-go-agent-executor
```

不得先停止 executor 后继续保留声称 `pty_terminal` 的 v11 Agent 连接。

## 回退

### 仅回退功能

1. 确认活动会话均进入终态。
2. 停止 Agent，再停止 executor。
3. 使用安装器成对恢复受支持的 v11 Agent/runner broker/executor，并先启动 executor 与 runner broker、后启动 Agent。
4. 验证节点重新在线且普通部署成功；不得回退到旧协议 Agent。

### 回退主控

先完成上述功能停用，再回退 API/Web。已经应用的 migration 不回滚、不删除；旧二进制必须先验证能够容忍新增表和字段。若不兼容，应前滚修复，不能修改历史 migration 或清理生产数据。

## 故障排查

- **节点在线但 executor 不可用**：检查两个 unit 的版本是否一致、Socket 所有者/组/权限以及 Agent 是否有连接 Socket 的组权限。
- **协议版本不支持**：主控仅接受 v11；使用平台安装命令配对升级 Agent/executor，不能只替换一个二进制。
- **capability 验签失败**：核对 API 签名私钥与安装命令中的公钥是否配对、executor 配置的节点/Agent ID 是否与主控一致，并检查系统时间；不得跳过验签或清空消费目录后直接重试同一 capability。
- **cgroup 创建或清理失败**：确认 `/sys/fs/cgroup/cgroup.controllers` 存在、executor unit 为 `Delegate=yes`，并检查 `systemctl show deploy-go-agent-executor -p ControlGroup -p Delegate`。不得回退到仅进程组或 `/proc` 扫描后继续开放终端。
- **打开后立即关闭**：检查 executor peer credential 拒绝、单会话冲突、输入序号和会话 ID；不要记录或转储终端正文。
- **浏览器 WebSocket 失败**：核对 HTTPS 反向代理是否透传 Upgrade，以及 Origin、Cookie 和 CSRF 子协议是否通过；CSRF 不得放入 URL query。
- **疑似残留 root shell**：立即撤销节点身份并停止 Agent/executor，核对 executor 管理的进程组；确认清理后才能重新安装。
- **升级失败**：安装器应恢复整对旧版本。若恢复版本低于 v11 或 Agent/executor 版本不一致，节点保持不可用并重新执行配对安装。

## 验收记录

每次灰度至少记录：主控版本、Agent/executor 版本、节点、协议版本、能力状态、普通部署结果、终端元数据验证、停用结果和回退演练结果。不得记录 token、命令正文或终端输出。
