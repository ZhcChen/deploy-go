#!/usr/bin/env bash
set -euo pipefail

template_module=postgres
module_name=PostgreSQL
release_root_base=/srv/deploy-go-apps
current_step_id=
current_step_name=
module_active=0

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
  if [[ -n "$current_step_id" ]]; then
    printf 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.failed","module":"%s","step_id":"%s","step":"%s","message":"%s","failure_stage":"%s"}\n' "$template_module" "$current_step_id" "$current_step_name" "$message" "$current_step_id"
  fi
  printf 'DEPLOY_GO_EVENT {"schema_version":1,"event":"%s","message":"%s"}\n' "$event" "$message"
  exit 1
}

begin_step() {
  current_step_id=$1
  current_step_name=$2
  printf 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.started","module":"%s","step_id":"%s","step":"%s","failure_stage":"%s"}\n' "$template_module" "$current_step_id" "$current_step_name" "$current_step_id"
}

end_step() {
  local step_id=$1
  local step_name=$2
  printf 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded","module":"%s","step_id":"%s","step":"%s","failure_stage":"%s"}\n' "$template_module" "$step_id" "$step_name" "$step_id"
  current_step_id=
  current_step_name=
}

canceled() {
  if [[ -n "$current_step_id" ]]; then
    printf 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.failed","module":"%s","step_id":"%s","step":"%s","message":"canceled","failure_stage":"%s"}\n' "$template_module" "$current_step_id" "$current_step_name" "$current_step_id"
  fi
  if [[ "$module_active" == "1" ]]; then
    printf 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.failed","module":"%s","module_name":"%s","message":"canceled"}\n' "$template_module" "$module_name"
  fi
  exit 130
}

trap canceled TERM INT

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
[[ "$artifact_relative" == "$template_module/template.tar.gz" ]] || die deploy.preflight.failed "artifact path is invalid"
[[ -f "$artifact" && ! -L "$artifact" ]] || die deploy.preflight.failed "artifact is missing"
actual_sha=$(sha256sum "$artifact" | awk '{print $1}')
actual_size=$(wc -c <"$artifact" | tr -d '[:space:]')
[[ "$actual_sha" == "$expected_sha" && "$actual_size" == "$expected_size" ]] || die deploy.preflight.failed "artifact checksum mismatch"
command -v tar >/dev/null || die deploy.preflight.failed "tar is unavailable"
archive_contents=$(tar -tzf "$artifact")
[[ "$archive_contents" == "compose.yaml
config/postgresql.conf
deploy-go.yaml" || "$archive_contents" == "Makefile
compose.yaml
config/postgresql.conf
deploy-go.yaml
scripts/release.sh" ]] || die deploy.preflight.failed "artifact contents are invalid"

env_file="$DEPLOY_ENV_DIR/compose.env"
[[ -f "$env_file" && ! -L "$env_file" ]] || die deploy.preflight.failed "compose.env is missing"
service_env_file="$DEPLOY_ENV_DIR/postgres.env"
[[ -f "$service_env_file" && ! -L "$service_env_file" ]] || die deploy.preflight.failed "postgres.env is missing"
[[ -f "$DEPLOY_CANCEL_FILE" ]] && die deploy.preflight.failed "deployment is canceled"

command -v docker >/dev/null || die deploy.preflight.failed "docker is unavailable"
docker compose version >/dev/null 2>&1 || die deploy.preflight.failed "docker compose plugin is unavailable"

printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.preflight.succeeded"}'
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.started","module":"postgres","module_name":"PostgreSQL"}'
module_active=1

if [[ "${DEPLOY_GO_TEMPLATE_DRY_RUN:-0}" == "1" ]]; then
  printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"postgres","module_name":"PostgreSQL"}'
  exit 0
fi

project_name=$(printf '%s' "deploy-go-$DEPLOY_TARGET" | tr '[:upper:]' '[:lower:]' | tr -c 'a-z0-9_-' '_')
release_dir="$release_root_base/$DEPLOY_TARGET/releases/$DEPLOY_RELEASE_VERSION"
mkdir -p "$release_dir"
tar -xzf "$artifact" -C "$release_dir" compose.yaml config/postgresql.conf deploy-go.yaml
install -m 0600 "$env_file" "$release_dir/compose.env"
install -m 0600 "$service_env_file" "$release_dir/postgres.env"

compose() {
  docker compose \
    --env-file "$release_dir/compose.env" \
    --project-name "$project_name" \
    --file "$release_dir/compose.yaml" \
    "$@"
}

begin_step "$template_module.compose_config" "校验 Compose 配置"
compose config --quiet || die deploy.module.failed "compose config validation failed"
end_step "$template_module.compose_config" "校验 Compose 配置"

[[ -f "$DEPLOY_CANCEL_FILE" ]] && die deploy.module.failed "deployment is canceled"
begin_step "$template_module.up" "启动容器"
compose up -d --remove-orphans --wait --timeout 180 || die deploy.module.failed "docker compose up failed"
end_step "$template_module.up" "启动容器"

begin_step "$template_module.verify" "等待健康检查"
healthy=0
if status=$(compose ps --format json 2>/dev/null); then
  if jq -e 'if type == "array" then (length > 0 and all(.[]; .State == "running" and ((.Health // "") == "healthy" or (.Health // "") == ""))) else (.State == "running" and ((.Health // "") == "healthy" or (.Health // "") == "")) end' <<<"$status" >/dev/null 2>&1; then
    healthy=1
  fi
fi
[[ "$healthy" == "1" ]] || die deploy.module.failed "postgres did not become healthy in time"
end_step "$template_module.verify" "等待健康检查"
printf '%s\n' 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.succeeded","module":"postgres","module_name":"PostgreSQL"}'
