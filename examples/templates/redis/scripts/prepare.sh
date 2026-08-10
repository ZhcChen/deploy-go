#!/usr/bin/env bash
set -euo pipefail

template_module=redis
template_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

: "${DEPLOY_RELEASE_VERSION:?DEPLOY_RELEASE_VERSION is required}"
: "${DEPLOY_COMMIT_SHA:?DEPLOY_COMMIT_SHA is required}"
: "${DEPLOY_MODULES:?DEPLOY_MODULES is required}"
: "${DEPLOY_OUTPUT_DIR:?DEPLOY_OUTPUT_DIR is required}"

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.started"}'

command -v tar >/dev/null || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"tar is unavailable"}'
  exit 2
}
command -v jq >/dev/null || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"jq is unavailable"}'
  exit 2
}
[[ "$DEPLOY_RELEASE_VERSION" =~ ^[A-Za-z0-9._-]+$ && ${#DEPLOY_RELEASE_VERSION} -le 256 ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"release version is invalid"}'
  exit 2
}
[[ "$DEPLOY_COMMIT_SHA" =~ ^[0-9a-f]{40,64}$ ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"commit sha is invalid"}'
  exit 2
}
[[ "$DEPLOY_MODULES" == "$template_module" ]] || {
  printf '%s\n' "DEPLOY_GO_EVENT {\"schema_version\":1,\"event\":\"deploy.preflight.failed\",\"message\":\"modules must be $template_module\"}"
  exit 2
}

output_dir="$DEPLOY_OUTPUT_DIR/$template_module"
mkdir -p "$output_dir"
tar -czf "$output_dir/compose.tar.gz" -C "$template_root" compose.yaml

artifact="$output_dir/compose.tar.gz"
size=$(wc -c <"$artifact" | tr -d '[:space:]')
sha=$(sha256sum "$artifact" | awk '{print $1}')
jq -n \
  --arg release_version "$DEPLOY_RELEASE_VERSION" \
  --arg commit_sha "$DEPLOY_COMMIT_SHA" \
  --arg module "$template_module" \
  --arg path "$template_module/compose.tar.gz" \
  --arg sha256 "$sha" \
  --argjson size "$size" \
  '{schema_version:1, release_version:$release_version, commit_sha:$commit_sha, artifacts:[{module:$module,path:$path,sha256:$sha256,size:$size}]}' \
  >"$DEPLOY_OUTPUT_DIR/deploy-go-artifact.json"

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.succeeded"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.started","module":"redis","module_name":"Redis"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.started","module":"redis","step_id":"redis.package","step":"打包 Compose 配置"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded","module":"redis","step_id":"redis.package","step":"打包 Compose 配置"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"redis","module_name":"Redis"}'
