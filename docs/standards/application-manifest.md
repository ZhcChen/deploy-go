---
date: 2026-08-12
topic: application-manifest
status: accepted
schema_version: 1
---

# 应用清单 deploy-go.yaml

## 目标

业务应用或平台模板根目录提供 `deploy-go.yaml`，用结构化字段声明应用类型与
模板版本，避免 Deploy Go 通过命令、可执行文件或任意字段推断发布能力。
平台侧应用元数据与仓库清单保持一致；清单不携带命令、参数或 env map。

## 文件位置与格式

文件固定位于仓库根目录，名称固定为 `deploy-go.yaml`。示例：

```yaml
schema_version: 1
type: redis
type_version: "7"
modules:
  - redis
env_files:
  - compose.env
  - redis.env
```

## 字段

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `schema_version` | 是 | 固定 `1` |
| `type` | 是 | 平台白名单：`redis` / `postgres` / `binary` |
| `type_version` | 是 | 模板版本字符串；`redis=7`，`postgres=16/18`，`binary=1` |
| `modules` | 是 | 1-32 个安全模块标识；模板应用必须与模板模块一致 |
| `env_files` | 否 | 0-16 个 `*.env` 白名单文件名，模板应用必须包含模板必选文件 |

`type_version` 是模板/应用类型版本，不是发布版本；发布版本仍由 Deploy Go
生成并保存在 deployment snapshot。

## 类型白名单

- `redis`：平台 Redis 模板，`type_version` 当前为 `7`，模块 `redis`。
- `postgres`：平台 PostgreSQL 模板，`type_version` 当前为 `18`（兼容 `16`），
  模块 `postgres`。
- `binary`：普通二进制业务应用，`type_version` 固定 `1`，模块由业务仓库声明。

未知类型、未知字段、路径逃逸、命令片段和任意 env map 一律拒绝。

## 平台侧约束

- 应用详情保存的 `app_type` / `type_version` 是控制面权威值，业务仓库清单
  用于接入时核对；不一致时部署门禁应拒绝。
- executor 仍只执行固定 `make --no-print-directory deploy-go-release`；
  应用类型不引入任何新的可执行输入。
- `deploy-go.yaml` 是只读声明，不作为环境变量或 Shell 来源。
