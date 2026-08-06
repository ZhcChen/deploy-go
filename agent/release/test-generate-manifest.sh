#!/usr/bin/env bash

set -euo pipefail

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
printf 'x86 fixture\n' >"$fixture/deploy-go-agent-linux-x86_64"
printf 'arm fixture\n' >"$fixture/deploy-go-agent-linux-aarch64"
cp agent/install/deploy-go-agent.service "$fixture/"
agent/release/generate-manifest.sh \
  "$fixture" \
  "https://github.com/ZhcChen/deploy-go/releases/download/v0.1.0" \
  "0.1.0"
jq -e '
  .agent_version == "0.1.0" and
  .protocol == {minimum: 1, maximum: 4} and
  (.artifacts | length == 2) and
  ([.artifacts[].architecture] | sort == ["aarch64", "x86_64"])
' "$fixture/deploy-go-agent-manifest.json" >/dev/null
