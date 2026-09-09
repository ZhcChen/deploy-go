---
date: 2026-09-09
topic: workspace-reserved-module-admission
plan: docs/plans/2026-09-02-two-stage-script-mode-plan.md
---

# workspace 发布物保留模块必须三层一致处理

## 问题

workspace 两阶段模式由 Agent prepare 阶段在发布物 manifest 中追加平台级
`deploy-go-workspace` 模块（固定路径 `deploy-go-workspace.tar.gz`），但
`claims.modules` 只包含业务模块。控制面 `validate_prepared_manifest` 与 Agent
`staging::validate_modules` 已过滤该保留模块，而 executor 的
`release::validate_artifact_manifest` 仍直接比较 manifest 模块集合与
`claims.modules`，导致 release 准入返回 `release_digest_mismatch`，业务
`deploy-go-release` 尚未执行就失败。

## 结论

- 保留模块常量统一放在 `agent-protocol`：`RESERVED_WORKSPACE_MODULE` 与
  `RESERVED_WORKSPACE_ARTIFACT`。
- 三层都必须把该模块排除出业务模块集合：控制面 manifest 校验、Agent
  staging 校验、executor release 准入校验。
- executor 侧只允许最多一个保留模块条目，且路径必须等于
  `RESERVED_WORKSPACE_ARTIFACT`；该文件的 SHA-256 与大小仍通过
  `claims.artifacts` 校验，不能绕过完整性校验。
- 业务模块集合仍必须与 `claims.modules` 完全一致，重复模块继续拒绝。

## 验证

```bash
cargo test -p deploy-go-agent-executor --lib
make agent-check
make api-check
```

## 影响

修复需要同步升级目标节点 Agent（executor 随 Agent 发布物成对安装）；Agent
控制协议仍为 v15，executor 本机协议仍为 v3。
