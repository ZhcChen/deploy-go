#!/usr/bin/env bash
set -euo pipefail

template_module=redis
module_name=Redis
release_root_base=/srv/deploy-go-apps

: "${DEPLOY_ID:?DEPLOY_ID is required}"
: "${DEPLOY_ENVIRONMENT:?DEPLOY_ENVIRONMENT is required}"
: "${DEPLOY_RELEASE_VERSION:?DEPLOY_RELEASE_VERSION is required}"
: "${DEPLOY_COMMIT_SHA:?DEPLOY_COMMIT_SHA is required}"
: "${DEPLOY_MODULES:?DEPLOY_MODULES is required}"
: "${DEPLOY_TARGET:?DEPLOY_TARGET is required}"
: "${DEPLOY_ARTIFACT_DIR:?DEPLOY_ARTIFACT_DIR is required}"
: "${DEPLOY_ENV_DIR:?DEPLOY_ENV_DIR is required}"
: "${DEPLOY_CANCEL_FILE:?DEPLOY_CANCEL_FILE is required}"

die() {
  local event=$1
  local message=$2
  printf 'DEPLOY_GO_EVENT {"schema_version":1,"event":"%s","message":"%s"}\n' "$event" "$message"
  exit 1
}

trap 'printf '"'"'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.failed","module":"redis","module_name":"Redis","message":"canceled"}'"'"'\n; exit 130' TERM INT

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.started"}'

[[ "$DEPLOY_RELEASE_VERSION" =~ ^[A-Za-z0-9._-]+$ && ${#DEPLOY_RELEASE_VERSION} -le 256 ]] || die deploy.preflight.failed "release version is invalid"
[[ "$DEPLOY_COMMIT_SHA" =~ ^[0-9a-f]{40,64}$ ]] || die deploy.preflight.failed "commit sha is invalid"
[[ "$DEPLOY_MODULES" == "$template_module" ]] || die deploy.preflight.failed "modules must be $template_module"
[[ "$DEPLOY_TARGET" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || die deploy.preflight.failed "target is invalid"

manifest="$DEPLOY_ARTIFACT_DIR/deploy-go-artifact.json"
[[ -f "$manifest" && ! -L "$manifest" ]] || die deploy.preflight.failed "artifact manifest is missing"
jq -e \
  --arg release_version "$DEPLOY_RELEASE_VERSION" \
  --arg commit_sha "$DEPLOY_COMMIT_SHA" \
  --arg module "$template_module" \
  '.schema_version == 1 and .release_version == $release_version and .commit_sha == $commit_sha and .artifacts[0].module == $module and (.artifacts | length) == 1' \
  "$manifest" >/dev/null || die deploy.preflight.failed "artifact manifest does not match task"

artifact_relative=$(jq -er '.artifacts[0].path' "$manifest")
artifact="$DEPLOY_ARTIFACT_DIR/$artifact_relative"
expected_sha=$(jq -er '.artifacts[0].sha256' "$manifest")
expected_size=$(jq -er '.artifacts[0].size' "$manifest")
[[ "$artifact_relative" == "$template_module/compose.tar.gz" ]] || die deploy.preflight.failed "artifact path is invalid"
[[ -f "$artifact" && ! -L "$artifact" ]] || die deploy.preflight.failed "artifact is missing"
actual_sha=$(sha256sum "$artifact" | awk '{print $1}')
actual_size=$(wc -c <"$artifact" | tr -d '[:space:]')
[[ "$actual_sha" == "$expected_sha" && "$actual_size" == "$expected_size" ]] || die deploy.preflight.failed "artifact checksum mismatch"
command -v tar >/dev/null || die deploy.preflight.failed "tar is unavailable"
[[ "$(tar -tzf "$artifact")" == "compose.yaml" ]] || die deploy.preflight.failed "artifact contents are invalid"

env_file="$DEPLOY_ENV_DIR/compose.env"
[[ -f "$env_file" && ! -L "$env_file" ]] || die deploy.preflight.failed "compose.env is missing"
[[ -f "$DEPLOY_CANCEL_FILE" ]] && die deploy.preflight.failed "deployment is canceled"

command -v docker >/dev/null || die deploy.preflight.failed "docker is unavailable"
docker compose version >/dev/null 2>&1 || die deploy.preflight.failed "docker compose plugin is unavailable"

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.succeeded"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.started","module":"redis","module_name":"Redis"}'

if [[ "${DEPLOY_GO_TEMPLATE_DRY_RUN:-0}" == "1" ]]; then
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"redis","module_name":"Redis"}'
  exit 0
fi

project_name=$(printf '%s' "deploy-go-$DEPLOY_TARGET" | tr '[:upper:]' '[:lower:]' | tr -c 'a-z0-9_-' '_')
release_dir="$release_root_base/$DEPLOY_TARGET/releases/$DEPLOY_RELEASE_VERSION"
mkdir -p "$release_dir"
tar -xzf "$artifact" -C "$release_dir" compose.yaml
install -m 0600 "$env_file" "$release_dir/compose.env"

compose() {
  docker compose \
    --env-file "$release_dir/compose.env" \
    --project-name "$project_name" \
    --file "$release_dir/compose.yaml" \
    "$@"
}

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.started","module":"redis","step_id":"redis.compose_config","step":"校验 Compose 配置"}'
compose config --quiet || die deploy.module.failed "compose config validation failed"
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded","module":"redis","step_id":"redis.compose_config","step":"校验 Compose 配置"}'

[[ -f "$DEPLOY_CANCEL_FILE" ]] && die deploy.module.failed "deployment is canceled"
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.started","module":"redis","step_id":"redis.up","step":"启动容器"}'
compose up -d --remove-orphans || die deploy.module.failed "docker compose up failed"
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded","module":"redis","step_id":"redis.up","step":"启动容器"}'

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.started","module":"redis","step_id":"redis.verify","step":"等待健康检查"}'
healthy=0
for _ in $(seq 1 30); do
  [[ -f "$DEPLOY_CANCEL_FILE" ]] && {
    compose stop >/dev/null 2>&1 || true
    die deploy.module.failed "deployment is canceled"
  }
  if status=$(compose ps --format json 2>/dev/null); then
    if jq -e 'length > 0 and all(.[]; .State == "running" and ((.Health // "") == "healthy" or (.Health // "") == ""))' <<<"$status" >/dev/null 2>&1; then
      healthy=1
      break
    fi
  fi
  sleep 2
done
[[ "$healthy" == "1" ]] || die deploy.module.failed "redis did not become healthy in time"
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded","module":"redis","step_id":"redis.verify","step":"等待健康检查"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"redis","module_name":"Redis"}'
