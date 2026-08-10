#!/usr/bin/env bash
set -euo pipefail

template_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

output_dir="$fixture/output"
env_dir="$fixture/env"
prepare_log="$fixture/prepare.log"
release_log="$fixture/release.log"
commit_sha=0123456789abcdef0123456789abcdef01234567
mkdir -p "$env_dir"
cat >"$env_dir/compose.env" <<'EOF'
REDIS_PORT=6379
TZ=Asia/Shanghai
EOF
cat >"$env_dir/redis.env" <<'EOF'
REDIS_PASSWORD=test-password
TZ=Asia/Shanghai
EOF

env \
  DEPLOY_RELEASE_VERSION=redis-1 \
  DEPLOY_COMMIT_SHA="$commit_sha" \
  DEPLOY_MODULES=redis \
  DEPLOY_OUTPUT_DIR="$output_dir" \
  make --no-print-directory -C "$template_root" deploy-go-prepare >"$prepare_log"

jq -e \
  --arg commit_sha "$commit_sha" \
  '.schema_version == 1 and .release_version == "redis-1" and .commit_sha == $commit_sha and .artifacts[0].module == "redis"' \
  "$output_dir/deploy-go-artifact.json" >/dev/null
grep -Fx 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"redis","module_name":"Redis"}' "$prepare_log" >/dev/null

env \
  DEPLOY_ID=deploy-test-1 \
  DEPLOY_ENVIRONMENT=test \
  DEPLOY_RELEASE_VERSION=redis-1 \
  DEPLOY_COMMIT_SHA="$commit_sha" \
  DEPLOY_MODULES=redis \
  DEPLOY_TARGET=node-test \
  DEPLOY_ARTIFACT_DIR="$output_dir" \
  DEPLOY_ENV_DIR="$env_dir" \
  DEPLOY_CANCEL_FILE="$fixture/cancel" \
  DEPLOY_GO_TEMPLATE_DRY_RUN=1 \
  make --no-print-directory -C "$template_root" deploy-go-release >"$release_log"

grep -Fx 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"redis","module_name":"Redis"}' "$release_log" >/dev/null

if env \
  DEPLOY_RELEASE_VERSION=redis-2 \
  DEPLOY_COMMIT_SHA=main \
  DEPLOY_MODULES=redis \
  DEPLOY_OUTPUT_DIR="$fixture/invalid-output" \
  make --no-print-directory -C "$template_root" deploy-go-prepare >/dev/null 2>&1; then
  printf '%s\n' '非法 commit SHA 未被拒绝' >&2
  exit 1
fi

printf 'tampered\n' >>"$output_dir/redis/template.tar.gz"
if env \
  DEPLOY_ID=deploy-test-2 \
  DEPLOY_ENVIRONMENT=test \
  DEPLOY_RELEASE_VERSION=redis-1 \
  DEPLOY_COMMIT_SHA="$commit_sha" \
  DEPLOY_MODULES=redis \
  DEPLOY_TARGET=node-test \
  DEPLOY_ARTIFACT_DIR="$output_dir" \
  DEPLOY_ENV_DIR="$env_dir" \
  DEPLOY_CANCEL_FILE="$fixture/cancel" \
  DEPLOY_GO_TEMPLATE_DRY_RUN=1 \
  make --no-print-directory -C "$template_root" deploy-go-release >/dev/null 2>&1; then
  printf '%s\n' '被篡改发布物未被拒绝' >&2
  exit 1
fi

printf '%s\n' 'Redis 模板契约测试通过'
