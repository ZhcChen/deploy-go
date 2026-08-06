#!/usr/bin/env bash
set -euo pipefail

: "${DEPLOY_RELEASE_VERSION:?DEPLOY_RELEASE_VERSION is required}"
: "${DEPLOY_COMMIT_SHA:?DEPLOY_COMMIT_SHA is required}"
: "${DEPLOY_MODULES:?DEPLOY_MODULES is required}"
: "${DEPLOY_ARTIFACT_DIR:?DEPLOY_ARTIFACT_DIR is required}"
: "${DEPLOY_DEMO_RELEASE_ROOT:?DEPLOY_DEMO_RELEASE_ROOT is required for this safe demo}"

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.started"}'

command -v jq >/dev/null || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"jq is unavailable"}'
  exit 2
}
command -v tar >/dev/null || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"tar is unavailable"}'
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
[[ "$DEPLOY_MODULES" == "demo" ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"modules are invalid"}'
  exit 2
}

manifest="$DEPLOY_ARTIFACT_DIR/deploy-go-artifact.json"
[[ -f "$manifest" ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"artifact manifest is missing"}'
  exit 2
}

jq -e \
  --arg release_version "$DEPLOY_RELEASE_VERSION" \
  --arg commit_sha "$DEPLOY_COMMIT_SHA" \
  '.schema_version == 1 and .release_version == $release_version and .commit_sha == $commit_sha and (.artifacts | length) == 1 and .artifacts[0].module == "demo"' \
  "$manifest" >/dev/null || {
    printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"artifact manifest does not match task"}'
    exit 2
  }

relative_path=$(jq -er '.artifacts[0].path' "$manifest")
expected_sha=$(jq -er '.artifacts[0].sha256' "$manifest")
[[ "$relative_path" == "demo/demo-app.tar.gz" ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"artifact path is invalid"}'
  exit 2
}
artifact_path="$DEPLOY_ARTIFACT_DIR/$relative_path"
[[ -f "$artifact_path" && ! -L "$artifact_path" ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"artifact file is invalid"}'
  exit 2
}
if command -v sha256sum >/dev/null 2>&1; then
  actual_sha=$(sha256sum "$artifact_path" | awk '{print $1}')
else
  actual_sha=$(shasum -a 256 "$artifact_path" | awk '{print $1}')
fi
[[ "$actual_sha" == "$expected_sha" ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"artifact checksum mismatch"}'
  exit 2
}
[[ "$(tar -tzf "$artifact_path")" == "message.txt" ]] || {
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.failed","message":"artifact contents are invalid"}'
  exit 2
}

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.succeeded"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.started","module":"demo","module_name":"Demo 服务"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.started","module":"demo","step_id":"demo.activate","step":"切换发布版本"}'

release_dir="$DEPLOY_DEMO_RELEASE_ROOT/releases/$DEPLOY_RELEASE_VERSION"
if ! {
  mkdir -p "$release_dir" &&
    tar -xzf "$artifact_path" -C "$release_dir" &&
    ln -sfn "$release_dir" "$DEPLOY_DEMO_RELEASE_ROOT/current.next" &&
    mv -f "$DEPLOY_DEMO_RELEASE_ROOT/current.next" "$DEPLOY_DEMO_RELEASE_ROOT/current"
}; then
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.failed","module":"demo","step_id":"demo.activate","step":"切换发布版本"}'
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.failed","module":"demo","module_name":"Demo 服务"}'
  exit 1
fi

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded","module":"demo","step_id":"demo.activate","step":"切换发布版本"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.verification.started","module":"demo","step_id":"demo.verify","step":"验证发布版本"}'
if ! test "$(cat "$DEPLOY_DEMO_RELEASE_ROOT/current/message.txt")" = "Deploy Go branch deployment demo"; then
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.verification.failed","module":"demo","step_id":"demo.verify","step":"验证发布版本"}'
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.failed","module":"demo","module_name":"Demo 服务"}'
  exit 1
fi
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.verification.succeeded","module":"demo","step_id":"demo.verify","step":"验证发布版本"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"demo","module_name":"Demo 服务"}'
