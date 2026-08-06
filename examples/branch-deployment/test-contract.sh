#!/usr/bin/env bash
set -euo pipefail

example_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

output_dir="$fixture/output"
release_root="$fixture/runtime"
prepare_log="$fixture/prepare.log"
release_log="$fixture/release.log"
commit_sha=0123456789abcdef0123456789abcdef01234567

env \
  DEPLOY_RELEASE_VERSION=demo-1 \
  DEPLOY_COMMIT_SHA="$commit_sha" \
  DEPLOY_MODULES=demo \
  DEPLOY_OUTPUT_DIR="$output_dir" \
  make --no-print-directory -C "$example_root" deploy-go-prepare >"$prepare_log"

jq -e \
  --arg commit_sha "$commit_sha" \
  '.schema_version == 1 and .release_version == "demo-1" and .commit_sha == $commit_sha and .artifacts[0].module == "demo"' \
  "$output_dir/deploy-go-artifact.json" >/dev/null
grep -Fx 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"demo","module_name":"Demo 服务"}' "$prepare_log" >/dev/null

env \
  DEPLOY_RELEASE_VERSION=demo-1 \
  DEPLOY_COMMIT_SHA="$commit_sha" \
  DEPLOY_MODULES=demo \
  DEPLOY_ARTIFACT_DIR="$output_dir" \
  DEPLOY_DEMO_RELEASE_ROOT="$release_root" \
  make --no-print-directory -C "$example_root" deploy-go-release >"$release_log"

test "$(cat "$release_root/current/message.txt")" = "Deploy Go branch deployment demo"
grep -Fx 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.verification.succeeded","module":"demo","step_id":"demo.verify","step":"验证发布版本"}' "$release_log" >/dev/null

env \
  DEPLOY_RELEASE_VERSION=demo-1 \
  DEPLOY_COMMIT_SHA="$commit_sha" \
  DEPLOY_MODULES=demo \
  DEPLOY_ARTIFACT_DIR="$output_dir" \
  DEPLOY_DEMO_RELEASE_ROOT="$release_root" \
  make --no-print-directory -C "$example_root" deploy-go-release >>"$release_log"
test "$(cat "$release_root/current/message.txt")" = "Deploy Go branch deployment demo"

if env \
  DEPLOY_RELEASE_VERSION=demo-2 \
  DEPLOY_COMMIT_SHA=main \
  DEPLOY_MODULES=demo \
  DEPLOY_OUTPUT_DIR="$fixture/invalid-output" \
  make --no-print-directory -C "$example_root" deploy-go-prepare >/dev/null 2>&1; then
  printf '%s\n' '非法 commit SHA 未被拒绝' >&2
  exit 1
fi

printf 'tampered\n' >>"$output_dir/demo/demo-app.tar.gz"
if env \
  DEPLOY_RELEASE_VERSION=demo-1 \
  DEPLOY_COMMIT_SHA="$commit_sha" \
  DEPLOY_MODULES=demo \
  DEPLOY_ARTIFACT_DIR="$output_dir" \
  DEPLOY_DEMO_RELEASE_ROOT="$fixture/tampered-runtime" \
  make --no-print-directory -C "$example_root" deploy-go-release >/dev/null 2>&1; then
  printf '%s\n' '被篡改发布物未被拒绝' >&2
  exit 1
fi

printf '%s\n' '分支部署 Demo 契约测试通过'
