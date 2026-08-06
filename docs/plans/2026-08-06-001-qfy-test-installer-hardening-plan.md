---
title: qfy-test 安装器安全加固计划
date: 2026-08-06
status: completed
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: code
---

# qfy-test 安装器安全加固计划

## Goal Capsule

收紧 qfy-test systemd 部署链路的 root 与服务账号权限边界，避免 staging 替换、并发安装、SSH 参数注入和密钥篡改，并让失败部署可以恢复上一版运行状态。

## Requirements

- R1. 本地和远端 staging 每次部署独立创建，远端目录固定在 root 专用根目录下。
- R2. 部署参数不得拼入 SSH 远端 shell 命令，安装器只解析白名单配置字段。
- R3. `/opt/deploy-go` 由 root 管理，`deploy-go` 只拥有运行数据目录写权限。
- R4. 安装器使用固定跨进程锁，不能通过环境变量选择其他锁绕过互斥。
- R5. 主密钥异常时停止安装；正常运行时服务可读但无法修改主密钥。
- R6. unit、环境文件和发布产物使用临时文件或目录替换，失败后恢复上一版。
- R7. 不可捕获中断留下恢复材料时，后续部署必须停止并要求人工恢复。
- R8. 提供聚焦检查入口，覆盖随机 staging、配置隔离、失败清理和关键权限契约。

## Implementation Units

### U1. 隔离部署输入

修改 `deploy/qfy-test/deploy.sh`，使用本地和远端随机 staging，通过 `install.env` 上传部署参数，并收敛远端 owner/mode。

### U2. 收紧安装权限与事务

修改 `deploy/qfy-test/install.sh`，固定安装路径和锁文件，校验 staging 与主密钥，调整目录所有权，并实现备份、恢复和遗留事务检测。

### U3. systemd 与密钥边界

保持主密钥 `0400 deploy-go:deploy-go`，通过 `ProtectSystem=strict` 和 `ReadOnlyPaths` 在服务 mount namespace 中强制只读；unit 使用同目录临时文件替换。

### U4. 验证与文档

增加 `make deploy-qfy-test-check` 和部署行为测试，更新 README、runbook，并记录 WSL 实装、权限、故障注入与公网验证结果。

## Verification

- `make deploy-qfy-test-check`
- `bash -n deploy/qfy-test/deploy.sh deploy/qfy-test/install.sh`
- ShellCheck 聚焦检查
- WSL 正常安装和重复安装
- 安装锁竞争验证
- systemd 只读主密钥验证
- 坏 API 健康检查失败后的回滚验证
- 公网页面与 API readiness 验证
