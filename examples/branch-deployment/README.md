# 分支部署接入 Demo

本目录展示业务应用接入 Deploy Go 时需要提供的最小 Makefile、准备脚本、发布脚本、artifact manifest 和结构化事件。Demo 不执行 Git、网络请求、Docker、sudo 或 systemd 操作。

真实执行时 Git checkout、环境变量注入、发布物校验和阶段串联由 Agent 完成。业务仓库只需要提供：

```text
make deploy-go-prepare
make deploy-go-release
```

准备脚本将发布物写入 `DEPLOY_OUTPUT_DIR`，发布脚本只读取 `DEPLOY_ARTIFACT_DIR`。示例额外要求 `DEPLOY_DEMO_RELEASE_ROOT`，确保演示只能写入调用方显式指定的安全目录；真实业务使用自身经过权限设计的 release 根目录。

本地验证：

```bash
make deploy-contract-demo-check
```

该命令全部使用临时目录，不连接真实 Git 仓库、Agent 或服务器。
