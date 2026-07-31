---
title: GitHub Actions API 构建与发布计划
date: 2026-07-31
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# GitHub Actions API 构建与发布计划

## Goal Capsule

- 为当前 Rust API 建立可重复的 GitHub Actions 检查、Linux 多架构构建和 GitHub Release 发布链路。
- 参考 `redcode-im` 的 API Release 产物组织方式，但以本仓库的 Cargo workspace、SQLite 和 OpenSSH 运行依赖为准。
- 本计划只覆盖 `api/`；尚未创建的 `admin/` 和 `admin-app/` 不进入构建矩阵。
- 工作流不得连接真实节点、执行远程脚本或持有任何运行环境凭证。

## Product Contract

### Summary

代码提交和 Pull Request 应自动执行全仓质量检查；版本 tag 应构建 Linux `x86_64` 与 `arm64` API 产物，并发布可校验的 GitHub Release。

### Requirements

- R1. `main` push 和 Pull Request 自动执行 `make check`，相同 ref 的旧检查可被取消。
- R2. `v*.*.*` tag 和手动触发可以构建 Linux `x86_64`、`arm64` API Docker 镜像。
- R3. 每个平台导出 API 可执行文件压缩包、Docker 镜像归档和 SHA-256 校验文件。
- R4. tag 自动发布 GitHub Release；手动触发默认只生成 Actions artifact，只有显式选择后才发布指定 tag。
- R5. Release tag 必须符合语义版本格式，发布说明列出各架构下载地址和与上一稳定版本的比较链接。
- R6. 运行镜像使用非 root 用户，包含 OpenSSH、CA 证书、时区和健康检查所需依赖，不包含运行密钥或数据库。
- R7. 本地提供与 Actions 一致的镜像构建入口及可执行 runbook。

### Scope Boundaries

- 不构建未落地的 Web 和 Flutter 正式客户端。
- 不推送容器注册表，不签名产物，不执行生产部署。
- 不在 CI 中连接真实 SSH 节点。

## Planning Contract

- KTD1. **CI 与 Release 分离。** `ci.yml` 负责高频质量门禁，`release-artifacts.yml` 负责低频多架构产物，避免每次提交都执行镜像导出。
- KTD2. **通过 Docker 原生架构 runner 构建。** 沿用参考项目的 `ubuntu-latest` 和 `ubuntu-24.04-arm` runner，减少 QEMU 交叉构建的不确定性。
- KTD3. **仓库根目录作为 Docker context。** Cargo workspace 的 manifest 和 lockfile 位于根目录，Dockerfile 位于 `api/docker/release/Dockerfile`。
- KTD4. **运行配置外部注入。** 镜像只封装二进制与运行依赖；SQLite 文件、主密钥、Origin、监听地址和初始化 token 由部署环境提供。

实施顺序为先建立 Docker 可重复构建，再接入 Release artifact，最后补齐 CI、文档和静态验证。

## Implementation Units

### U1. 建立 API release 镜像

- **Goal：** 生成同时可供运行和导出二进制的最小 API 镜像。
- **Requirements：** R2、R6、R7。
- **Files：** `api/docker/release/Dockerfile`、`.dockerignore`、`Makefile`。
- **Approach：** 使用 Rust 1.94 Alpine 多阶段构建 workspace package；运行阶段安装 OpenSSH 客户端并以非 root 用户运行。
- **Test Scenarios：** Dockerfile 可解析并完成 release build；镜像存在 API 二进制与 `ssh`/`ssh-keyscan`；镜像配置不包含敏感值。
- **Verification：** `make api-image`；条件允许时检查镜像内命令和架构。

### U2. 建立 GitHub Actions CI 与 Release 工作流

- **Goal：** 自动执行检查、构建多架构产物并按受控条件发布 Release。
- **Requirements：** R1-R5。
- **Files：** `.github/workflows/ci.yml`、`.github/workflows/release-artifacts.yml`、`.github/scripts/generate-release-notes.sh`。
- **Approach：** CI 使用项目固定 Rust toolchain 和 Cargo cache；Release 在原生 runner 上构建镜像，导出压缩产物与校验文件，并用 `gh release` 创建或更新版本。
- **Test Scenarios：** 非法手动 tag 被拒绝；手动未勾选发布时不创建 Release；tag 构建两个架构且发布 job 等待全部产物；发布说明能处理首个版本和已有上一版本两种情况。
- **Verification：** YAML 和 shell 静态解析；本地 fixture 验证发布说明输出；检查 Actions 表达式及权限最小化。

### U3. 补齐构建运行手册

- **Goal：** 让维护者可以从本地构建映射到 Actions 和 Release 行为。
- **Requirements：** R4、R7。
- **Files：** `docs/runbooks/github-actions-release.md`、`docs/runbooks/README.md`、`README.md`。
- **Approach：** 说明触发方式、产物、手动发布条件、运行配置边界和失败排查入口。
- **Test Scenarios：** 命令与 workflow 名称、文件名和实际路径一致；明确构建不等于部署授权。
- **Verification：** 文档路径与命令检查，`git diff --check`。

## Verification Contract

| Gate | Command or evidence | Applies to |
| --- | --- | --- |
| 全仓质量门禁 | `make check` | U1-U3 |
| Docker 构建 | `make api-image` | U1 |
| Workflow 语法 | Ruby YAML 解析或可用的 `actionlint` | U2 |
| Shell 语法 | `bash -n .github/scripts/generate-release-notes.sh` | U2 |
| 发布说明 | 临时 Git 仓库与模拟 artifact 文件生成结果 | U2 |
| Git 格式 | `git diff --check`、`git diff --cached --check` | U1-U3 |

## Definition of Done

- `main` 与 Pull Request 有明确 CI 门禁，tag 与手动触发有明确 Release 构建入口。
- 两个 Linux 架构均生成命名稳定、带 SHA-256 的 binary 与 Docker image 产物。
- 镜像具备 API 实际运行所需的 OpenSSH 依赖且不携带环境密钥。
- 发布链路、产物和本地构建命令有可执行 runbook。
- 相关检查通过，计划范围外没有客户端构建或生产部署逻辑。
