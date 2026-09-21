#!/usr/bin/env bash

set -euo pipefail

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
protocol_minimum="$(sed -n 's/^pub const MIN_SUPPORTED_PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-protocol/src/lib.rs | head -n 1)"
protocol_maximum="$(sed -n 's/^pub const PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-protocol/src/lib.rs | head -n 1)"
executor_protocol="$(sed -n 's/^pub const PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-executor/src/protocol.rs | head -n 1)"
printf 'x86 fixture\n' >"$fixture/deploy-go-agent-linux-x86_64"
printf 'executor x86 fixture\n' >"$fixture/deploy-go-agent-executor-linux-x86_64"
printf 'updater x86 fixture\n' >"$fixture/deploy-go-agent-updater-linux-x86_64"
cp agent/install/deploy-go-agent.service "$fixture/"
cp agent/install/deploy-go-agent-runner.service "$fixture/"
cp agent/install/deploy-go-agent-executor.service "$fixture/"
cp agent/install/deploy-go-agent-updater.service "$fixture/"
cp agent/install/executor.json.in "$fixture/"
agent/release/generate-manifest.sh \
  "$fixture" \
  "https://github.com/ZhcChen/deploy-go/releases/download/v0.1.0" \
  "0.1.0"
jq -e --argjson protocol_minimum "$protocol_minimum" --argjson protocol_maximum "$protocol_maximum" --argjson executor_protocol "$executor_protocol" '
  .schema_version == 4 and
  .agent_version == "0.1.0" and
  .executor_version == "0.1.0" and
  .protocol == {minimum: $protocol_minimum, maximum: $protocol_maximum} and
  .runner_protocol == 1 and
  .executor_protocol == $executor_protocol and
  (.artifacts | length == 3) and
  ([.artifacts[] | select(.component == "agent") | .architecture] == ["x86_64"]) and
  ([.artifacts[] | select(.component == "executor") | .architecture] == ["x86_64"]) and
  ([.artifacts[] | select(.component == "updater") | .architecture] == ["x86_64"]) and
  (.systemd_units | keys | sort == ["agent", "executor", "runner", "updater"]) and
  (.executor_config.sha256 | test("^[a-f0-9]{64}$"))
' "$fixture/deploy-go-agent-manifest.json" >/dev/null
