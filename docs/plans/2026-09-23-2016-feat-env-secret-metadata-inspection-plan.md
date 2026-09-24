---
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan
execution: code
title: 检查非生产 Env 密钥元数据
date: 2026-09-23
---

# 检查非生产 Env 密钥元数据

## Goal Capsule

**Objective:** 外部项目可以确认指定非生产 Env 配置项是否存在及其长度，同时不会取得密钥明文。

**Means:** External API 在服务端解密并检查调用方指定的键，只返回存在性和 UTF-8 字节长度（KTD1）。

**Authority:** 当前用户指令优先，其次为 `AGENTS.md`、External API 安全规范与 runbook；本项目只改 Deploy Go 自身。

**Stop conditions:** 若检查必须返回 Env 原文、涵盖生产环境，或需要修改业务应用仓库才能完成，则停止并请求重新定界。

## Product Contract

### Summary

为绑定应用的 External API Key 增加非生产 Env 键检查能力，并将其接入 `deploy-go-deployer` CLI 与 skill。此能力不提供 Env 明文读取。

### Problem Frame

当前 External API 只支持 Env 元数据查询和写入。外部项目无法验证必需密钥是否存在及长度，只能请求明文读取或依赖人工检查；前者超出外部 API 的安全边界。

### Requirements

- **R1:** External API 仅对该 API Key 已绑定、且环境为 `dev`、`test` 或 `staging` 的应用提供检查；`prod` 应用拒绝检查。
- **R2:** 请求必须指定 Env 文件与 1 到 50 个唯一、格式有效的键名；响应只包含请求键的存在性和字节长度，不包含键值或未请求键。
- **R3:** 值长度按 UTF-8 字节计数；dotenv 值外围成对单引号或双引号属于语法界定符，不计入值长度，值内其他字节原样计数。缺失键返回 `exists=false` 且长度为空。
- **R4:** CLI 提供只读检查命令；默认输出清晰的存在性/长度摘要，`--json` 只输出 API 的脱敏结果。skill 可使用该命令报告用户指定键的存在性和长度，仍禁止读取、输出、记录或推断密钥值。
- **R5:** 现有 Env 列表、登记、更新、删除行为保持不变；API Key 应用隔离、生产环境拒绝和无明文返回由集成测试覆盖。

### Scope Boundaries

- 不提供 Env 内容读取或下载能力。
- 不检查生产环境。
- 不解析 YAML、JSON 或其他 Env 格式；当前登记契约为 `dotenv-v1`。
- 不修改任何业务应用代码、配置、脚本或发布物。

## Planning Contract

### Key Technical Decisions

- **KTD1: 在服务端返回请求键的最小元数据。** CLI 不下载或本地处理完整 Env；只检查调用方明确传入的键名，响应不包含密钥值。
- **KTD2: 沿用 External API 的应用授权和非生产配置门禁。** 检查端点先验证 API Key 对应用的绑定，再验证 Env 文件属于该应用，并拒绝 `prod`；不增加新的授权模型。
- **KTD3: 使用 POST 检查端点承载键名列表。** 键名放在 JSON 请求体而非 URL 查询串；限制数量、格式和重复项，避免任意 Env 枚举及 URL 日志暴露。

### High-Level Technical Design

`External API Key -> 应用绑定/环境校验 -> Env 文件归属校验 -> 解密并检查请求键 -> 脱敏审计 -> 返回键名、存在性、值字节长度`

密钥值只在 API 进程内存中用于计算长度，不进入响应、审计摘要或日志。CLI 与 skill 只处理检查结果。

### Risks & Dependencies

- 解密和 dotenv 检查跨越 External API、Env 存储与 CLI；集成测试必须证明响应和审计中均无秘密值。
- CLI 内嵌 External OpenAPI，契约变更需要重新生成 `api/openapi/external.json` 并重建 CLI。
- 已安装的正式环境 CLI/skill 不会因源码更新自动变更，需按既有发布/安装流程发布后才可供其他项目使用。

## Implementation Units

### U1. 提供非生产 Env 键检查 API

**Goal:** 为 External API Key 增加最小披露的 Env 检查端点。

**Requirements:** R1, R2, R3, R5。

**Dependencies:** 无。

**Files:** `api/src/external/mod.rs`, `api/src/application_envs/mod.rs`, `api/src/application_envs/dotenv.rs`, `api/tests/external_api.rs`, `api/tests/external_openapi_contract.rs`。

**Approach:** 新增请求/响应 DTO 和检查路由；复用 API Key 应用归属、非生产配置和 Env 文件归属检查；仅处理请求中的键名；对结果写脱敏审计；不得将解密内容传给序列化 DTO。

**Execution note:** 先补集成测试，覆盖成功、跨应用、生产应用、无效键列表、缺失键及秘密值不出现在响应/审计中的情况。

**Patterns to follow:** `api/src/external/mod.rs` 中现有 Env 路由与 External OpenAPI；`api/src/application_envs/dotenv.rs` 的键名校验和内容约束；`api/tests/external_api.rs` 的 API Key fixture。

**Test scenarios:**

- 非生产应用请求已存在键，返回 `exists=true` 与符合 R3 的长度，响应不含原值。
- 请求缺失键，返回 `exists=false` 且长度为空。
- 请求含 0 个、超过上限、重复或格式非法的键，返回校验错误且不泄漏输入内容。
- 未绑定应用的 Key、其他应用的 Env 文件及 `prod` 应用无法检查。
- 审计包含检查动作和资源标识，但不包含 Env 原文或密钥值。
- External OpenAPI 描述检查请求/响应且不增加明文读取端点。

**Verification:** External API 集成测试和 OpenAPI contract 测试通过；External OpenAPI 与 Rust 定义一致。

### U2. 将检查能力接入 CLI 与 skill

**Goal:** 其他项目可经受支持的 CLI/skill 进行密钥元数据检查。

**Requirements:** R4, R5。

**Dependencies:** U1。

**Files:** `deploy-go-deployer/src/main.rs`, `deploy-go-deployer/tests/skill_package.rs`, `skills/deploy-go-deployer/SKILL.md`, `skills/deploy-go-deployer/references/commands.md`, `skills/deploy-go-deployer/references/workflows.md`, `deploy-go-deployer/README.md`。

**Approach:** 新增 `inspect-env-file` 命令，接收 application ID、Env file ID 和可重复 `--key`；调用新 External API；普通输出只展示指定键的存在性与长度，JSON 输出只含脱敏 API 响应。skill 更新触发描述、命令流程和安全边界，不允许把能力描述成读取 Env。

**Patterns to follow:** 现有 `list-env-files` 命令、`ApiClient` JSON 方法、skill 命令文档和 package contract tests。

**Test scenarios:**

- CLI help 列出命令及必填应用、文件和键参数。
- CLI 对多个键调用正确路径并显示脱敏摘要。
- 非成功 API 响应停止并保留稳定错误，不重试写操作。
- 打包 skill 说明可做元数据检查且继续禁止读取/输出明文。

**Verification:** deployer 单元/契约测试和 skill package tests 通过；命令帮助和文档与 External OpenAPI 一致。

### U3. 更新 API 产物与运行手册

**Goal:** 发布及后续接入流程能发现并安全使用新命令。

**Requirements:** R1-R5。

**Dependencies:** U1, U2。

**Files:** `api/openapi/external.json`, `docs/runbooks/external-deploy-api.md`。

**Approach:** 重新生成 External OpenAPI；补充只读命令示例、响应语义、非生产限制、安装更新说明和禁止明文读取的安全说明。

**Patterns to follow:** `docs/runbooks/external-deploy-api.md` 现有 CLI/API 示例与发布说明。

**Test scenarios:**

- OpenAPI 生成检查识别新路由和 schema，无意外 Internal API 变更。
- Runbook 示例使用新 CLI 命令且从未展示密钥值。

**Verification:** External OpenAPI 漂移检查、CLI 检查和 runbook 安全/格式检查通过。

## Verification Contract

- `cargo fmt --all -- --check`
- `cargo test -p deploy-go-api --test external_api --test external_openapi_contract`
- `cargo test -p deploy-go-deployer`
- `make api-external-openapi-check`
- `make deployer-check`
- `git diff --check`

## Definition of Done

- U1-U3 的验证均通过，External API 只返回指定键存在性和长度。
- 测试证明生产环境、跨应用 Key 和非授权路径无法读取任何 Env 元数据。
- CLI 与 skill 可指导其他项目只核实指定键的存在性/长度，不能读取或输出密钥明文。
- API 响应、审计、CLI 普通输出、JSON 输出和文档示例均不含 Env 值。
- 无遗留实验代码；正式部署只在工作区及发布源可确认包含本次改动时进行。
