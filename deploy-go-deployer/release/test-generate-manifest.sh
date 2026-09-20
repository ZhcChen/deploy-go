#!/usr/bin/env bash

set -euo pipefail

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
printf 'x86 fixture\n' >"$fixture/deploy-go-deployer-linux-x86_64"
deploy-go-deployer/release/generate-manifest.sh \
  "$fixture" \
  "https://github.com/ZhcChen/deploy-go/releases/download/v0.3.5" \
  "0.3.5"
jq -e '
  .schema_version == 1 and
  .deployer_version == "0.3.5" and
  (.artifacts | length == 1) and
  ([.artifacts[] | select(.component == "deployer") | .architecture] == ["x86_64"]) and
  ([.artifacts[].sha256] | all(test("^[a-f0-9]{64}$")))
' "$fixture/deploy-go-deployer-manifest.json" >/dev/null
