#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/deploy-go-deployer-contract.XXXXXX")"
trap 'rm -rf -- "$TMP_DIR"' EXIT

cd "$REPO_ROOT"

for command in list-apps show-app deploy status cancel openapi; do
  cargo run -q -p deploy-go-deployer -- "$command" --help >/dev/null
done

cargo run -q -p deploy-go-deployer -- openapi --output "$TMP_DIR/external.json"
cmp -- "$TMP_DIR/external.json" "$REPO_ROOT/api/openapi/external.json"

if ! grep -q 'DEPLOY_GO_API_BASE_URL' deploy-go-deployer/src/main.rs; then
  printf 'deployer 缺少 DEPLOY_GO_API_BASE_URL 环境变量支持\n' >&2
  exit 1
fi
if ! grep -q 'DEPLOY_GO_API_KEY' deploy-go-deployer/src/main.rs; then
  printf 'deployer 缺少 DEPLOY_GO_API_KEY 环境变量支持\n' >&2
  exit 1
fi
if grep -nE 'std::process::Command|tokio::process|/bin/(ba)?sh' \
  deploy-go-deployer/src/main.rs >/dev/null; then
  printf 'deployer 不得执行任意命令\n' >&2
  exit 1
fi
missing_key_output="$(cargo run -q -p deploy-go-deployer -- list-apps 2>&1 || true)"
if [[ "$missing_key_output" != *'缺少有效的 DEPLOY_GO_API_KEY'* ]]; then
  printf 'deployer 缺少 API Key 时应明确失败\n' >&2
  exit 1
fi

printf 'deploy-go-deployer 契约检查通过\n'
