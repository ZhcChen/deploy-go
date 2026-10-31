# 当前规范目录

本目录保存当前实现必须遵守的产品、协议、安全和数据契约。规范优先级高于 `docs/runbooks/`、`docs/solutions/`、`docs/plans/` 和历史复核文档；总导航与变更流程见 [`docs/README.md`](../README.md)，权威顺序见 [`document-authority.md`](document-authority.md)。

## 访问控制、API 与数据

- [`access-control.md`](access-control.md)：用户、管理员、应用授权和 CSRF/Origin 边界。
- [`api-contract.md`](api-contract.md)：内部 API、会话、错误、分页和安全声明。
- [`sql-repository-convention.md`](sql-repository-convention.md)：SQL、migration、不可变历史和数据访问约定。
- [`client-behavior-contract.md`](client-behavior-contract.md)：Web 与 Flutter 客户端的行为边界。
- [`document-authority.md`](document-authority.md)：文档权威顺序和历史隔离规则。
- [`spec-kit-storage-register.md`](spec-kit-storage-register.md)：Spec Kit 规格正文的仓库/Obsidian 存储登记。

## Agent、协议与节点安全

- [`agent-control-protocol.md`](agent-control-protocol.md)：API、Agent 和 executor 之间的版本化控制协议。
- [`agent-credential-security.md`](agent-credential-security.md)：Agent 凭证轮换、持久化和重放保护。
- [`agent-installation-contract.md`](agent-installation-contract.md)：Agent、runner broker、executor 的安装、升级和回滚契约。
- [`privileged-agent-executor.md`](privileged-agent-executor.md)：root executor、PTY、cgroup、release admission 和本机 IPC 边界。
- [`ssh-credential-security.md`](ssh-credential-security.md)：legacy SSH 凭证的加密和退出边界。

## 应用部署与发布物

- [`deploy-script-contract.md`](deploy-script-contract.md)：业务 Make target、事件标记、退出码和日志契约。
- [`application-deployment-contract.md`](application-deployment-contract.md)：应用、目标、Env、artifact 和两阶段部署模型。
- [`application-deployment-json.md`](application-deployment-json.md)：应用部署 JSON 配置的字段和校验规则。
- [`application-manifest.md`](application-manifest.md)：应用 manifest、模块和 Env 声明。
- [`git-branch-deployment-contract.md`](git-branch-deployment-contract.md)：Git URL、分支发现、commit 固化和 materialization。
- [`privileged-release-launcher.md`](privileged-release-launcher.md)：历史 launcher 的受控兼容契约。

机器可校验的发布契约位于同一目录：

- [`deploy-artifact-manifest.schema.json`](deploy-artifact-manifest.schema.json)
- [`deploy-env-manifest.schema.json`](deploy-env-manifest.schema.json)
- [`deploy-event-marker.schema.json`](deploy-event-marker.schema.json)
- [`deploy-event.schema.json`](deploy-event.schema.json)

修改规范时必须同时核对代码、测试、OpenAPI/Schema 产物和对应 runbook；历史协议 schema 或已应用 migration 不得直接改写。
