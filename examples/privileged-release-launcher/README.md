# 受控发布 launcher Demo

本目录演示应用专属 launcher 的最小实现：

- `launcher.sh`：只接受固定 JSON 输入并校验白名单；生产安装后为 `root:root` 且只能通过精确 sudo 路径调用。
- `release-entry.sh`：launcher 固定的发布动作，只消费 staging 中的 demo 发布物。
- `sudoers.example`：精确 sudo 配置样例。
- `test-contract.sh`：本地契约测试，覆盖合法输入、未知字段/模块/操作、路径逃逸、符号链接、校验失败和信号转发。

测试不执行 Docker、sudo 或真实节点操作：

```bash
make privileged-launcher-check
```

真实安装由节点管理员执行，必须复制到固定绝对路径、设置 `root:root 0755`，并按 README 中的 sudoers 示例配置。
