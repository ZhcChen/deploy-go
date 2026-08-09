#!/usr/bin/env bash

set -euo pipefail

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
printf 'x86 fixture\n' >"$fixture/deploy-go-agent-linux-x86_64"
printf 'arm fixture\n' >"$fixture/deploy-go-agent-linux-aarch64"
printf 'executor x86 fixture\n' >"$fixture/deploy-go-agent-executor-linux-x86_64"
printf 'executor arm fixture\n' >"$fixture/deploy-go-agent-executor-linux-aarch64"
cp agent/install/deploy-go-agent.service "$fixture/"
cp agent/install/deploy-go-agent-executor.service "$fixture/"
cp agent/install/executor.json.in "$fixture/"
agent/release/generate-manifest.sh \
  "$fixture" \
  "https://github.com/ZhcChen/deploy-go/releases/download/v0.1.0" \
  "0.1.0"
jq -e '
  .schema_version == 2 and
  .agent_version == "0.1.0" and
  .executor_version == "0.1.0" and
  .protocol == {minimum: 1, maximum: 6} and
  (.artifacts | length == 4) and
  ([.artifacts[] | select(.component == "agent") | .architecture] | sort == ["aarch64", "x86_64"]) and
  ([.artifacts[] | select(.component == "executor") | .architecture] | sort == ["aarch64", "x86_64"]) and
  (.systemd_units | keys | sort == ["agent", "executor"]) and
  (.executor_config.sha256 | test("^[a-f0-9]{64}$"))
' "$fixture/deploy-go-agent-manifest.json" >/dev/null
