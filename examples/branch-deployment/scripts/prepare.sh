#!/usr/bin/env bash
set -euo pipefail

: "${DEPLOY_RELEASE_VERSION:?DEPLOY_RELEASE_VERSION is required}"
: "${DEPLOY_COMMIT_SHA:?DEPLOY_COMMIT_SHA is required}"
: "${DEPLOY_MODULES:?DEPLOY_MODULES is required}"
: "${DEPLOY_OUTPUT_DIR:?DEPLOY_OUTPUT_DIR is required}"

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.started"}'

[[ "$DEPLOY_RELEASE_VERSION" =~ ^[A-Za-z0-9._-]+$ && ${#DEPLOY_RELEASE_VERSION} -le 256 ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"release version is invalid"}'
  exit 2
}
[[ "$DEPLOY_COMMIT_SHA" =~ ^[0-9a-f]{40,64}$ ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"commit sha is invalid"}'
  exit 2
}
[[ "$DEPLOY_MODULES" == "demo" ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"modules are invalid"}'
  exit 2
}

command -v jq >/dev/null || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"jq is unavailable"}'
  exit 2
}
command -v tar >/dev/null || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"tar is unavailable"}'
  exit 2
}
mkdir -p "$DEPLOY_OUTPUT_DIR/demo" || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"output directory is unavailable"}'
  exit 1
}

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.succeeded"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.started","module":"demo","module_name":"Demo 服务"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.started","module":"demo","step_id":"demo.package","step":"打包发布物"}'

artifact_path="$DEPLOY_OUTPUT_DIR/demo/demo-app.tar.gz"
tar -czf "$artifact_path" -C app message.txt || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.failed","module":"demo","step_id":"demo.package","step":"打包发布物"}'
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.failed","module":"demo","module_name":"Demo 服务"}'
  exit 1
}

if command -v sha256sum >/dev/null 2>&1; then
  artifact_sha=$(sha256sum "$artifact_path" | awk '{print $1}')
else
  artifact_sha=$(shasum -a 256 "$artifact_path" | awk '{print $1}')
fi
if stat -c %s "$artifact_path" >/dev/null 2>&1; then
  artifact_size=$(stat -c %s "$artifact_path")
else
  artifact_size=$(stat -f %z "$artifact_path")
fi

if ! jq -n \
  --arg release_version "$DEPLOY_RELEASE_VERSION" \
  --arg commit_sha "$DEPLOY_COMMIT_SHA" \
  --arg sha256 "$artifact_sha" \
  --argjson size "$artifact_size" \
  '{
    schema_version: 1,
    release_version: $release_version,
    commit_sha: $commit_sha,
    artifacts: [{
      module: "demo",
      path: "demo/demo-app.tar.gz",
      sha256: $sha256,
      size: $size
    }]
  }' >"$DEPLOY_OUTPUT_DIR/deploy-go-artifact.json"; then
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.failed","module":"demo","step_id":"demo.package","step":"打包发布物"}'
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.failed","module":"demo","module_name":"Demo 服务"}'
  exit 1
fi

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded","module":"demo","step_id":"demo.package","step":"打包发布物"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"demo","module_name":"Demo 服务"}'
