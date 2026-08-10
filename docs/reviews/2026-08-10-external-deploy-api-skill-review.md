# 对外部署 OpenAPI 与 deploy-go-deployer Skill 复核

复核日期：2026-08-11

## 结论

对外部署 OpenAPI、`deploy-go-deployer` CLI、skill 模块与发布下载链路已按计划完成，
`make external-deploy-check` 与 `make check` 均通过。未发现阻断性正确性、安全性、
可靠性或 API 契约问题。

本次只完成本地实现、测试与契约验证；未连接或修改正式环境节点。正式发布需用户另行授权。

## 复核范围

- 外部 API Key 数据模型、系统账号隔离与管理接口
- `/external/v1` 只读、部署创建、状态、取消端点
- 对外 OpenAPI 独立契约与安全边界
- Rust CLI `deploy-go-deployer`
- `skills/deploy-go-deployer/SKILL.md` 与 runbook
- deployer release manifest、API 下载路由、生产构建/安装/回滚链路

## 关键验证

- `make external-deploy-check` 通过：
  - deployer CLI 契约、manifest 生成器、外部 OpenAPI 校验
  - `external_api` 5 项、`external_api_keys` 3 项、
    `external_openapi_contract` 4 项、`deployer_release` 2 项
- `make check` 通过：
  - workspace clippy `-D warnings`
  - workspace 全部测试
  - 内部/外部 OpenAPI 产物一致性
  - Agent/executor、生产部署、UI/Admin/Flutter、敏感数据扫描
- `make deploy-production-check` 通过，release 与 build 路径均覆盖 deployer 产物

## Review 结论

### Correctness

- 外部部署复用内部 preview/校验与幂等键逻辑，`Idempotency-Key` 以 API Key 为作用域。
- 对外部署 DTO 不包含 `requested_by`、`external_api_key_id`、`snapshot_json` 等内部字段。
- deployer manifest 版本必须与 API 版本一致，API 启动时校验当前 release 存在。
- 配置兼容性：未设置 deployer release 目录时下载路由禁用，不影响既有 Agent 发布配置。

### Security

- API Key 只保存 SHA-256 hash，明文仅创建时返回一次。
- 外部接口只接受 Bearer `dgx_...`，不进入 Cookie/CSRF 路径。
- 对外 OpenAPI 契约测试强制无 `/api/v1`、Env、凭证、用户、节点、审计路径。
- deployer 二进制下载不携带任何 API Key 或私钥；manifest 只含版本、URL 与 SHA-256。
- 生产安装脚本不输出私钥/API Key 正文，release 目录安装采用原子替换并纳入回滚。

### Reliability

- deployer release 安装先校验 manifest、SHA-256 与文件类型，再原子替换；
  失败路径保留旧版本并恢复。
- 下载路由支持点号与下划线版本号，manifest URL 由 API 重写为公开下载地址。
- 生产部署健康检查增加 deployer manifest 可达性检查。

### API contract

- 对外 OpenAPI 只含 5 个白名单路径，Bearer security 全部覆盖。
- 内部 OpenAPI 不含 `/external/v1`；对外 OpenAPI 不含内部管理面。
- CLI 与外部 OpenAPI 产物保持一致，`openapi` 可导出本地契约。

### Testing

- 新增 deployer release 下载正反向测试与 manifest 生成器测试。
- 生产部署 mock 契约覆盖 release 模式的 deployer 下载、SHA 校验与 install.env。
- 新增 config 兼容性单测，防止后续误改外部发布配置。

## 低风险后续项（不阻塞）

- `external_api_keys::update_applications` 移除应用时未同步清理不再被任何 active Key
  引用的共享系统账号 grant；当前无外部利用路径，后续可增加清理逻辑。
- deployer 发布物当前只构建 Linux x86_64/aarch64；若 skill 需在本机 macOS 直接运行，
  需要另行增加 Darwin 发布物与 manifest `os` 维度。
- deployer 下载路由未提供 release 列表接口；当前按固定版本 manifest 下载即可满足
  skill 安装，后续可按需补充。

## 提交

- `19d0e2e` 计划
- `8fb2462` API Key 数据模型与系统账号隔离
- `c93ba90` API Key 管理接口与审计
- `9f013c7` 对外只读 API
- `c44e40a` 对外部署创建/状态/取消
- `bb5b8d1` 对外 OpenAPI 契约
- `3321550` deploy-go-deployer CLI
- `fd4a6d3` skill 与 runbook
- `d539773` deployer 发布物 API 下载与生产安装链路
- `1578e66` deployer 发布配置兼容性修正
