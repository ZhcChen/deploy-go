#!/usr/bin/env bash

set -euo pipefail

fixture="$(mktemp -d)"
release_root="$fixture/root"
source_dir="$fixture/source"
mkdir -p "$source_dir"
trap 'rm -rf "$fixture"' EXIT

printf 'x86 fixture\n' >"$source_dir/deploy-go-agent-linux-x86_64"
printf 'arm fixture\n' >"$source_dir/deploy-go-agent-linux-aarch64"
cp agent/install/deploy-go-agent.service "$source_dir/deploy-go-agent.service"

x86_sha="$(sha256sum "$source_dir/deploy-go-agent-linux-x86_64" | awk '{print $1}')"
arm_sha="$(sha256sum "$source_dir/deploy-go-agent-linux-aarch64" | awk '{print $1}')"
unit_sha="$(sha256sum "$source_dir/deploy-go-agent.service" | awk '{print $1}')"

jq -n \
  --arg x86_sha "$x86_sha" \
  --arg arm_sha "$arm_sha" \
  --arg unit_sha "$unit_sha" \
  '{
    schema_version: 1,
    agent_version: "0.1.0",
    protocol: {minimum: 1, maximum: 1},
    systemd_unit: {url: "file:///deploy-go-agent.service", sha256: $unit_sha},
    artifacts: [
      {os: "linux", architecture: "x86_64", url: "file:///deploy-go-agent-linux-x86_64", sha256: $x86_sha},
      {os: "linux", architecture: "aarch64", url: "file:///deploy-go-agent-linux-aarch64", sha256: $arm_sha}
    ]
  }' >"$source_dir/deploy-go-agent-manifest.json"

bash scripts/sync-agent-release.sh \
  --release-dir "$release_root" \
  --version 0.1.0 \
  --base-url "file://$source_dir" \
  --allow-http

test -x "$release_root/0.1.0/deploy-go-agent-linux-x86_64"
test -x "$release_root/0.1.0/deploy-go-agent-linux-aarch64"
test -f "$release_root/0.1.0/deploy-go-agent-manifest.json"
test -f "$release_root/0.1.0/deploy-go-agent.service"
jq -e '.agent_version == "0.1.0"' \
  "$release_root/0.1.0/deploy-go-agent-manifest.json" >/dev/null
printf 'Agent release 同步脚本测试通过\n'
