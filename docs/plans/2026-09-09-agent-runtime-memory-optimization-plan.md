---
title: Agent 运行态内存优化计划
status: completed
date: 2026-09-09
schema_version: 1
---

# Agent 运行态内存优化计划

## 背景

当前节点侧仍然需要低权限联网 Agent、root runner broker 和 root executor 三个服务。
经运行态采样，三个服务合计约 42.6 MiB RSS / 38.3 MiB PSS，其中：

- `deploy-go-agent`：约 34.5 MiB RSS
- `deploy-go-agent-runner`：约 5.0 MiB RSS
- `deploy-go-agent-executor`：约 3.0 MiB RSS

本次优化目标是在保持安全边界、控制协议、任务恢复和特权能力不变的前提下，降低
长驻进程内存和部署高峰期峰值内存。

## 范围

- 允许优化 Agent、runner-service 和 executor 的运行时调度。
- 允许复用 HTTP 客户端、减少重复缓冲和低效分配。
- 不允许合并低权限 Agent 与 root 服务。
- 不允许削弱 socket 权限、签名验签、cgroup、降权和任务对账。
- 不连接真实节点、不重启线上服务；优化先在本地构建、测试和隔离环境验证。

## 基线

真实基线来自 `qfy-test2` 只读采样，后续优化必须记录相同指标的差异：

| 指标 | Agent | Runner | Executor |
| --- | ---: | ---: | ---: |
| RSS | 34.54 MiB | 5.04 MiB | 3.00 MiB |
| PSS | 32.40 MiB | 2.91 MiB | 3.00 MiB |
| 匿名 RSS | 24.78 MiB | 0.79 MiB | 0.76 MiB |
| 线程数 | 22 | 21 | 21 |

不能以 systemd cgroup `MemoryCurrent` 作为优化基数，其中包含临时 child、
page cache、共享内存和 slab。

## 执行单元

### U1：运行时调度优化

- 短生命周期诊断和 runner broker 使用单线程 Tokio runtime。
- executor 外层 Unix Socket 服务改为单线程 runtime。
- 主 Agent 保留多线程，但限制 worker/blocking thread 数量。
- 验证 runner 并发、任务启动、断线恢复和 executor PTY/release 生命周期。

### U2：HTTP 客户端与缓冲收敛

- 梳理 Agent 内 artifact、Env、token refresh 的 `reqwest::Client` 重复创建。
- 对可共享配置合并客户端，避免每类请求重复初始化 TLS 与连接池。
- 检查 manifest、tar、日志和 telemetry 的大块临时缓冲，改为固定或流式处理。
- runtime probe 增加独立的 `no_proxy` 客户端并在 TaskHandler 内复用，避免每次
  探测都重建 `reqwest::Client`；请求超时仍按探测参数逐次设置。

### U3：内存轮廓与峰值验证

- 本地或隔离 Linux 环境采集空闲 RSS、PSS、线程数与部署高峰 RSS。
- 如 U2 后仍高于目标，再评估把重操作迁移到短生命周期 helper 进程，不改变安全边界。

本轮已实现 U1/U2；U3 在隔离 Linux 容器完成空闲采样，未连接真实节点：

| 服务 | 隔离环境 RSS | 隔离环境 PSS | 匿名 RSS | 线程数 |
| --- | ---: | ---: | ---: | ---: |
| runner-service | 4.84 MiB | 4.28 MiB | 1.53 MiB | 1 |
| executor | 4.30 MiB | 3.74 MiB | 2.10 MiB | 1 |

runner、executor 分别在 20 个并发 probe 下无失败，线程数保持 1。真实节点
基线中两者分别约 21 线程，说明 runtime 收敛到 current-thread 的主要收益是
避免按主机 CPU 创建整套 Tokio worker。Agent 主服务保留最多 4 个 worker 和
最多 16 个 blocking thread，用于 WSS、制品传输、任务恢复和遥测并发。

## 验证

每阶段至少执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p deploy-go-agent -p deploy-go-agent-executor
make privileged-release-check
```

本轮隔离验证结果：

```text
agent-runner-isolation-check   通过（Linux 容器身份边界）
agent-executor-cgroup-check     通过（Linux 容器 cgroup v2 生命周期）
cargo test -p deploy-go-agent -p deploy-go-agent-executor 全部通过
```

涉及真实节点内存对比时，必须先获得针对具体节点和动作的明确授权；本轮未执行。

## 结论门槛

达到以下任一结果即可停止本轮优化：

- 空闲 RSS 明显下降且主要功能回归通过。
- 继续优化的收益低于维护和回归风险。
- 已确认当前占用主要由任务、缓存或安全边界造成，而非不必要的运行时常驻。
