# 业务应用接入部署手册

本手册说明业务应用接入 Deploy Go 两阶段部署时的准备、launcher 安装与本地验证步骤。手册不授权连接真实 Agent、执行真实部署、migration、重启、切流或清理；真实节点操作必须由当前对话明确授权。

## 1. 应用仓库要求

业务仓库根目录必须提供：

```makefile
.PHONY: deploy-go-prepare deploy-go-release

deploy-go-prepare:
	# 只生成发布物到 DEPLOY_OUTPUT_DIR

deploy-go-release:
	# 只消费 DEPLOY_ARTIFACT_DIR 中的已校验发布物
```

接口与事件要求见 `docs/standards/application-deployment-contract.md` 和 `docs/standards/deploy-script-contract.md`。最小参考实现位于 `examples/branch-deployment/`。

## 2. 特权发布 launcher

业务脚本需要 Docker、root 或系统级操作时，应用仓库必须提供：

- launcher 源脚本，遵守 `docs/standards/privileged-release-launcher.md`。
- root 所有、固定绝对路径的发布入口或内部固定动作。
- sudoers 配置样例和本地契约测试。

本地验证：

```bash
make privileged-launcher-check
```

## 3. 安装与校验

由节点管理员在目标节点执行：

1. 复制 launcher 到固定路径，例如 `/usr/local/sbin/qfy-voucher-hub-release-launcher`。
2. 复制发布入口及依赖到固定 root-owned 目录，例如 `/usr/local/lib/qfy-voucher-hub/`。
3. 设置 `root:root` 和 `0755`，确认目录链不被 `deploy-go-agent` 写。
4. 创建 `/etc/sudoers.d/` 精确规则，权限 `0440`。
5. 执行 `visudo -c` 和 `sudo -l -U deploy-go-agent` 核对。

安装前必须确认：

- launcher 不接受 shell、Docker 参数、URL、环境文件内容或任意命令路径。
- 输入 JSON 只包含规范字段，launcher 拒绝额外字段和路径逃逸。
- 底层失败退出码、SIGTERM 和审计日志符合规范。
- `deploy-go-agent` 不在 Docker 组，也不拥有通用 sudo。

## 4. Agent 与任务路径

- Agent 以 `deploy-go-agent` 用户运行，数据目录默认为 `/var/lib/deploy-go-agent`。
- 任务 staging 位于 Agent 工作目录内，业务脚本和 launcher 都只通过 `DEPLOY_OUTPUT_DIR` / `DEPLOY_ARTIFACT_DIR` 消费。
- launcher 输入文件和中间任务目录必须放在 Agent 可写、root launcher 可读的任务路径内，不能放在 systemd `PrivateTmp` 私有目录中。

## 5. 上线前检查

```text
□ git diff --check
□ 应用仓库 prepare/release/launcher 自测通过
□ bash -n 全部脚本通过
□ 敏感扫描不含 token、私钥、完整环境文件
□ 真实节点操作已获得当前对话明确授权
```
