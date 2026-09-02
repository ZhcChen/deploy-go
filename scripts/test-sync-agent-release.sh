#!/usr/bin/env bash

set -euo pipefail

fixture="$(mktemp -d)"
release_root="$fixture/root"
source_dir="$fixture/source"
mkdir -p "$source_dir"
trap 'rm -rf "$fixture"' EXIT

printf 'x86 fixture\n' >"$source_dir/deploy-go-agent-linux-x86_64"
printf 'arm fixture\n' >"$source_dir/deploy-go-agent-linux-aarch64"
printf 'executor x86 fixture\n' >"$source_dir/deploy-go-agent-executor-linux-x86_64"
printf 'executor arm fixture\n' >"$source_dir/deploy-go-agent-executor-linux-aarch64"
cp agent/install/deploy-go-agent.service "$source_dir/deploy-go-agent.service"
cp agent/install/deploy-go-agent-runner.service "$source_dir/deploy-go-agent-runner.service"
cp agent/install/deploy-go-agent-executor.service "$source_dir/deploy-go-agent-executor.service"
cp agent/install/executor.json.in "$source_dir/executor.json.in"

x86_sha="$(sha256sum "$source_dir/deploy-go-agent-linux-x86_64" | awk '{print $1}')"
arm_sha="$(sha256sum "$source_dir/deploy-go-agent-linux-aarch64" | awk '{print $1}')"
executor_x86_sha="$(sha256sum "$source_dir/deploy-go-agent-executor-linux-x86_64" | awk '{print $1}')"
executor_arm_sha="$(sha256sum "$source_dir/deploy-go-agent-executor-linux-aarch64" | awk '{print $1}')"
agent_unit_sha="$(sha256sum "$source_dir/deploy-go-agent.service" | awk '{print $1}')"
runner_unit_sha="$(sha256sum "$source_dir/deploy-go-agent-runner.service" | awk '{print $1}')"
executor_unit_sha="$(sha256sum "$source_dir/deploy-go-agent-executor.service" | awk '{print $1}')"
executor_config_sha="$(sha256sum "$source_dir/executor.json.in" | awk '{print $1}')"
protocol_minimum="$(sed -n 's/^pub const MIN_SUPPORTED_PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-protocol/src/lib.rs | head -n 1)"
protocol_maximum="$(sed -n 's/^pub const PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-protocol/src/lib.rs | head -n 1)"

jq -n \
  --arg x86_sha "$x86_sha" \
  --arg arm_sha "$arm_sha" \
  --arg executor_x86_sha "$executor_x86_sha" \
  --arg executor_arm_sha "$executor_arm_sha" \
  --arg agent_unit_sha "$agent_unit_sha" \
  --arg runner_unit_sha "$runner_unit_sha" \
  --arg executor_unit_sha "$executor_unit_sha" \
  --arg executor_config_sha "$executor_config_sha" \
  --argjson protocol_minimum "$protocol_minimum" \
  --argjson protocol_maximum "$protocol_maximum" \
  '{
    schema_version: 3,
    agent_version: "0.2.0",
    executor_version: "0.2.0",
    runner_protocol: 1,
    executor_protocol: 3,
    protocol: {minimum: $protocol_minimum, maximum: $protocol_maximum},
    systemd_units: {
      agent: {url: "file:///deploy-go-agent.service", sha256: $agent_unit_sha},
      runner: {url: "file:///deploy-go-agent-runner.service", sha256: $runner_unit_sha},
      executor: {url: "file:///deploy-go-agent-executor.service", sha256: $executor_unit_sha}
    },
    executor_config: {url: "file:///executor.json.in", sha256: $executor_config_sha},
    artifacts: [
      {component: "agent", os: "linux", architecture: "x86_64", url: "file:///deploy-go-agent-linux-x86_64", sha256: $x86_sha},
      {component: "agent", os: "linux", architecture: "aarch64", url: "file:///deploy-go-agent-linux-aarch64", sha256: $arm_sha},
      {component: "executor", os: "linux", architecture: "x86_64", url: "file:///deploy-go-agent-executor-linux-x86_64", sha256: $executor_x86_sha},
      {component: "executor", os: "linux", architecture: "aarch64", url: "file:///deploy-go-agent-executor-linux-aarch64", sha256: $executor_arm_sha}
    ]
  }' >"$source_dir/deploy-go-agent-manifest.json"

bash scripts/sync-agent-release.sh \
  --release-dir "$release_root" \
  --version 0.2.0 \
  --base-url "file://$source_dir" \
  --allow-http

test -x "$release_root/0.2.0/deploy-go-agent-linux-x86_64"
test -x "$release_root/0.2.0/deploy-go-agent-linux-aarch64"
test -x "$release_root/0.2.0/deploy-go-agent-executor-linux-x86_64"
test -x "$release_root/0.2.0/deploy-go-agent-executor-linux-aarch64"
test -f "$release_root/0.2.0/deploy-go-agent-manifest.json"
test -f "$release_root/0.2.0/deploy-go-agent.service"
test -f "$release_root/0.2.0/deploy-go-agent-runner.service"
test -f "$release_root/0.2.0/deploy-go-agent-executor.service"
test -f "$release_root/0.2.0/executor.json.in"
jq -e '.agent_version == "0.2.0"' \
  "$release_root/0.2.0/deploy-go-agent-manifest.json" >/dev/null
printf 'Agent release 同步脚本测试通过\n'
