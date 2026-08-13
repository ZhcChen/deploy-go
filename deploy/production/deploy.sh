#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

DEPLOY_HOST="${DEPLOY_HOST:-qfy-test}"
DEPLOY_SOURCE="${DEPLOY_SOURCE:-build}"
DEPLOY_RELEASE_TAG="${DEPLOY_RELEASE_TAG:-}"
DEPLOY_ARCH="${DEPLOY_ARCH:-}"
DEPLOY_PLATFORM="${DEPLOY_PLATFORM:-}"
DEPLOY_API_PORT="${DEPLOY_API_PORT:-30100}"
DEPLOY_API_BIND="${DEPLOY_API_BIND:-127.0.0.1}"
DEPLOY_WEB_PORT="${DEPLOY_WEB_PORT:-30101}"
DEPLOY_WEB_BIND="${DEPLOY_WEB_BIND:-127.0.0.1}"
DEPLOY_GO_COOKIE_SECURE="${DEPLOY_GO_COOKIE_SECURE:-true}"
DEPLOY_GO_MASTER_KEY_VERSION="${DEPLOY_GO_MASTER_KEY_VERSION:-1}"
DEPLOY_GO_PUBLIC_BASE_URL="${DEPLOY_GO_PUBLIC_BASE_URL:-https://deploy.quanxinfu.com}"
DEPLOY_GO_ALLOWED_ORIGIN="${DEPLOY_GO_ALLOWED_ORIGIN:-https://deploy.quanxinfu.com}"
DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED="${DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED:-true}"
DEPLOY_GO_ARTIFACTS_ROOT="${DEPLOY_GO_ARTIFACTS_ROOT:-/var/lib/deploy-go/artifacts}"
DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES="${DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES:-536870912}"
DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES="${DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES:-2147483648}"
DEPLOY_GO_ARTIFACT_MAX_FILES="${DEPLOY_GO_ARTIFACT_MAX_FILES:-256}"
DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES="${DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES:-8388608}"
DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS="${DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS:-1800}"
DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS="${DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS:-86400}"
DEPLOY_AGENT_SYNC="${DEPLOY_AGENT_SYNC:-}"
DEPLOY_AGENT_BUILD_ONLY="${DEPLOY_AGENT_BUILD_ONLY:-0}"
DEPLOY_AGENT_OUTPUT_DIR="${DEPLOY_AGENT_OUTPUT_DIR:-target/deploy-release/agent}"
DEPLOY_GITHUB_REPOSITORY="${DEPLOY_GITHUB_REPOSITORY:-ZhcChen/deploy-go}"
remote_host="$DEPLOY_HOST"
REMOTE_STAGING_ROOT="/var/lib/deploy-go-installer"
REMOTE_STAGING=""
LOCAL_STAGING=""
API_IMAGE="${DEPLOY_API_IMAGE:-deploy-go-api:production}"
AGENT_IMAGE="${DEPLOY_AGENT_IMAGE:-deploy-go-agent:production}"
DEPLOYER_IMAGE="${DEPLOY_DEPLOYER_IMAGE:-deploy-go-deployer:production}"

die() {
  printf 'DEPLOY_ERROR code=%s message=%s\n' "${2:-deploy_failed}" "$1" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "缺少命令：$1"
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

build_agent_release() {
  local output_dir="$1"
  local arch platform image container_id spec

  mkdir -p "$output_dir"
  for spec in "x86_64 linux/amd64" "aarch64 linux/arm64"; do
    arch="${spec%% *}"
    platform="${spec##* }"
    image="$AGENT_IMAGE-$arch"
    container_id=""
    trap '[[ -z "$container_id" ]] || docker rm -f "$container_id" >/dev/null 2>&1 || true' RETURN
    docker build \
      --platform "$platform" \
      --tag "$image" \
      --file agent/docker/release/Dockerfile \
      .
    container_id="$(docker create "$image")"
    docker cp "$container_id:/deploy-go-agent" "$output_dir/deploy-go-agent-linux-$arch"
    docker cp "$container_id:/deploy-go-agent-executor" \
      "$output_dir/deploy-go-agent-executor-linux-$arch"
    docker rm -f "$container_id" >/dev/null
    container_id=""
    trap - RETURN
    chmod 0755 \
      "$output_dir/deploy-go-agent-linux-$arch" \
      "$output_dir/deploy-go-agent-executor-linux-$arch"
  done

  cp agent/install/deploy-go-agent.service "$output_dir/deploy-go-agent.service"
  cp agent/install/deploy-go-agent-runner.service \
    "$output_dir/deploy-go-agent-runner.service"
  cp agent/install/deploy-go-agent-executor.service \
    "$output_dir/deploy-go-agent-executor.service"
  cp agent/install/executor.json.in "$output_dir/executor.json.in"

  local manifest_base="https://deploy-go.invalid/agent-releases/$AGENT_VERSION"
  agent/release/generate-manifest.sh \
    "$output_dir" "$manifest_base" "$AGENT_VERSION"
  jq -e --arg version "$AGENT_VERSION" \
    --argjson protocol "$AGENT_PROTOCOL_VERSION" \
    --argjson executor_protocol "$EXECUTOR_PROTOCOL_VERSION" \
    '.schema_version == 3 and .agent_version == $version and .executor_version == $version and (.systemd_units | keys | sort == ["agent","executor","runner"]) and .runner_protocol == 1 and .executor_protocol == $executor_protocol and .protocol.minimum <= $protocol and .protocol.maximum >= $protocol and ([.artifacts[] | select(.component == "agent") | .architecture] | sort == ["aarch64","x86_64"]) and ([.artifacts[] | select(.component == "executor") | .architecture] | sort == ["aarch64","x86_64"])' \
    "$output_dir/deploy-go-agent-manifest.json" >/dev/null ||
    die "本地构建 Agent manifest 校验失败"
  printf 'Agent %s 已在本机构建\n' "$AGENT_VERSION"
}

build_deployer_release() {
  local output_dir="$1"
  local arch platform image container_id spec

  mkdir -p "$output_dir"
  for spec in "x86_64 linux/amd64" "aarch64 linux/arm64"; do
    arch="${spec%% *}"
    platform="${spec##* }"
    image="$DEPLOYER_IMAGE-$arch"
    container_id=""
    trap '[[ -z "$container_id" ]] || docker rm -f "$container_id" >/dev/null 2>&1 || true' RETURN
    docker build \
      --platform "$platform" \
      --tag "$image" \
      --file deploy-go-deployer/docker/release/Dockerfile \
      .
    container_id="$(docker create "$image")"
    docker cp "$container_id:/deploy-go-deployer" \
      "$output_dir/deploy-go-deployer-linux-$arch"
    docker rm -f "$container_id" >/dev/null
    container_id=""
    trap - RETURN
    chmod 0755 "$output_dir/deploy-go-deployer-linux-$arch"
  done

  local manifest_base="https://deploy-go.invalid/deployer-releases/$DEPLOYER_VERSION"
  deploy-go-deployer/release/generate-manifest.sh \
    "$output_dir" "$manifest_base" "$DEPLOYER_VERSION"
  jq -e --arg version "$DEPLOYER_VERSION" \
    '.schema_version == 1 and .deployer_version == $version and
     ([.artifacts[].architecture] | sort == ["aarch64","x86_64"]) and
     ([.artifacts[] | select(.component == "deployer")] | length == 2)' \
    "$output_dir/deploy-go-deployer-manifest.json" >/dev/null ||
    die "本地构建 deployer manifest 校验失败"
  printf 'Deployer %s 已在本机构建\n' "$DEPLOYER_VERSION"
}

case "$DEPLOY_SOURCE" in
  build | release) ;;
  *) die "DEPLOY_SOURCE 只支持 build 或 release" ;;
esac

API_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' api/Cargo.toml | head -n 1 | tr -d '\r')"
AGENT_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' agent/Cargo.toml | head -n 1 | tr -d '\r')"
EXECUTOR_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' agent-executor/Cargo.toml | head -n 1 | tr -d '\r')"
DEPLOYER_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' deploy-go-deployer/Cargo.toml | head -n 1 | tr -d '\r')"
AGENT_PROTOCOL_VERSION="$(sed -n 's/^pub const PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-protocol/src/lib.rs | head -n 1)"
AGENT_PROTOCOL_MINIMUM="$(sed -n 's/^pub const MIN_SUPPORTED_PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-protocol/src/lib.rs | head -n 1)"
EXECUTOR_PROTOCOL_VERSION="$(sed -n 's/^pub const PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-executor/src/protocol.rs | head -n 1)"
[[ -n "$API_VERSION" && "$API_VERSION" == "$AGENT_VERSION" ]] ||
  die "API 与 Agent 版本不一致：$API_VERSION != $AGENT_VERSION"
[[ -n "$EXECUTOR_VERSION" && "$EXECUTOR_VERSION" == "$AGENT_VERSION" ]] ||
  die "Agent 与 executor 版本不一致：$AGENT_VERSION != $EXECUTOR_VERSION"
[[ -n "$DEPLOYER_VERSION" && "$DEPLOYER_VERSION" == "$API_VERSION" ]] ||
  die "API 与 deployer 版本不一致：$API_VERSION != $DEPLOYER_VERSION"
[[ "$AGENT_PROTOCOL_VERSION" =~ ^[1-9][0-9]*$ && "$AGENT_PROTOCOL_MINIMUM" =~ ^[1-9][0-9]*$ ]] ||
  die "无法读取 Agent 协议版本"
[[ "$EXECUTOR_PROTOCOL_VERSION" =~ ^[2-9][0-9]*$ ]] ||
  die "无法读取 executor 本机协议版本"
((AGENT_PROTOCOL_MINIMUM <= AGENT_PROTOCOL_VERSION)) || die "Agent 协议范围无效"

case "$DEPLOY_AGENT_BUILD_ONLY" in
  0 | 1) ;;
  *) die "DEPLOY_AGENT_BUILD_ONLY 必须为 0 或 1" ;;
esac
if [[ "$DEPLOY_AGENT_BUILD_ONLY" == "1" ]]; then
  [[ "$DEPLOY_SOURCE" == "build" ]] ||
    die "DEPLOY_AGENT_BUILD_ONLY 只支持 DEPLOY_SOURCE=build"
  require_command docker
  require_command jq
  build_agent_release "$DEPLOY_AGENT_OUTPUT_DIR"
  printf 'Agent %s 本机构建完成，产物目录：%s\n' \
    "$AGENT_VERSION" "$DEPLOY_AGENT_OUTPUT_DIR"
  exit 0
fi

require_command ssh
require_command rsync
require_command curl
require_command mktemp
require_command openssl
require_command jq

if [[ -z "$DEPLOY_ARCH" ]]; then
  remote_arch="$(ssh "$DEPLOY_HOST" 'uname -m' 2>/dev/null | tr -d '\r\n' || true)"
  case "$remote_arch" in
    x86_64) DEPLOY_ARCH="x86_64" ;;
    aarch64 | arm64) DEPLOY_ARCH="arm64" ;;
    *) die "无法识别正式服务器架构：${remote_arch:-未知}" ;;
  esac
fi

case "$DEPLOY_ARCH" in
  x86_64)
    API_ASSET_ARCH="x86_64"
    DEPLOY_PLATFORM="${DEPLOY_PLATFORM:-linux/amd64}"
    ;;
  arm64 | aarch64)
    API_ASSET_ARCH="arm64"
    DEPLOY_PLATFORM="${DEPLOY_PLATFORM:-linux/arm64}"
    ;;
  *) die "不支持的 DEPLOY_ARCH：$DEPLOY_ARCH" ;;
esac

if [[ "$DEPLOY_SOURCE" == "release" ]]; then
  [[ -n "$DEPLOY_RELEASE_TAG" ]] || die "release 模式必须设置 DEPLOY_RELEASE_TAG"
  [[ "$DEPLOY_RELEASE_TAG" == "v$API_VERSION" ]] ||
    die "DEPLOY_RELEASE_TAG 必须与 API 版本一致：v$API_VERSION"
fi

DEPLOY_AGENT_SYNC="${DEPLOY_AGENT_SYNC:-1}"
case "$DEPLOY_AGENT_SYNC" in
  0 | 1) ;;
  *) die "DEPLOY_AGENT_SYNC 必须为 0 或 1" ;;
esac
if [[ "$DEPLOY_AGENT_SYNC" == "1" ]]; then
  require_command docker
  require_command jq
fi

[[ "$DEPLOY_API_PORT" =~ ^[0-9]+$ ]] || die "DEPLOY_API_PORT 无效"
[[ "$DEPLOY_WEB_PORT" =~ ^[0-9]+$ ]] || die "DEPLOY_WEB_PORT 无效"
[[ "$DEPLOY_GO_COOKIE_SECURE" == "true" || "$DEPLOY_GO_COOKIE_SECURE" == "false" ]] ||
  die "DEPLOY_GO_COOKIE_SECURE 必须为 true 或 false"
[[ "$DEPLOY_GO_MASTER_KEY_VERSION" =~ ^[1-9][0-9]*$ ]] ||
  die "DEPLOY_GO_MASTER_KEY_VERSION 必须为正整数"
[[ "$DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED" == "true" ]] ||
  die "正式部署必须启用 DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED"
[[ "$DEPLOY_GO_ARTIFACTS_ROOT" == /var/lib/deploy-go/artifacts ]] ||
  die "正式制品目录必须为 /var/lib/deploy-go/artifacts"
for artifact_limit in \
  "$DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES" \
  "$DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES" \
  "$DEPLOY_GO_ARTIFACT_MAX_FILES" \
  "$DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES" \
  "$DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS" \
  "$DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS"; do
  [[ "$artifact_limit" =~ ^[1-9][0-9]*$ ]] || die "制品限额与 TTL 必须为正整数"
done
[[ "$DEPLOY_GITHUB_REPOSITORY" =~ ^[0-9A-Za-z_.-]+/[0-9A-Za-z_.-]+$ ]] ||
  die "DEPLOY_GITHUB_REPOSITORY 无效"

if [[ -z "$DEPLOY_GO_ALLOWED_ORIGIN" ]]; then
  remote_host="$(ssh -G "$DEPLOY_HOST" 2>/dev/null |
    awk '$1 == "hostname" {print $2; exit}')"
  remote_host="${remote_host:-$DEPLOY_HOST}"
  DEPLOY_GO_ALLOWED_ORIGIN="http://${remote_host}:${DEPLOY_WEB_PORT}"
fi
[[ "$DEPLOY_GO_ALLOWED_ORIGIN" =~ ^https?://[^/[:space:]]+$ ]] ||
  die "DEPLOY_GO_ALLOWED_ORIGIN 必须是 http(s) origin"
if [[ -n "$DEPLOY_GO_PUBLIC_BASE_URL" ]]; then
  [[ "$DEPLOY_GO_PUBLIC_BASE_URL" =~ ^https://[^/[:space:]]+/?$ ]] ||
    die "DEPLOY_GO_PUBLIC_BASE_URL 必须是 HTTPS origin"
fi
for config_value in "$DEPLOY_API_BIND" "$DEPLOY_WEB_BIND"; do
  [[ "$config_value" != *$'\n'* && "$config_value" != *$'\r'* ]] ||
    die "监听地址不得包含换行符"
done

container_id=""
remote_staging_created="0"
cleanup() {
  if [[ -n "$container_id" ]]; then
    docker rm -f "$container_id" >/dev/null 2>&1 || true
  fi
  if [[ -n "$LOCAL_STAGING" ]]; then
    rm -rf -- "$LOCAL_STAGING"
  fi
  if [[ "$remote_staging_created" == "1" ]]; then
    ssh "$DEPLOY_HOST" "rm -rf -- '$REMOTE_STAGING'" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

LOCAL_STAGING="$(mktemp -d "${TMPDIR:-/tmp}/deploy-go-production.XXXXXX")"
mkdir -p "$LOCAL_STAGING/web"

if [[ "$DEPLOY_SOURCE" == "release" ]]; then
  release_base="https://github.com/${DEPLOY_GITHUB_REPOSITORY}/releases/download/${DEPLOY_RELEASE_TAG}"
  curl --fail --silent --show-error --location --retry 3 \
    --proto '=https' --tlsv1.2 \
    --output "$LOCAL_STAGING/deploy-go-api-linux-$API_ASSET_ARCH" \
    "$release_base/deploy-go-api-linux-$API_ASSET_ARCH"
  curl --fail --silent --show-error --location --retry 3 \
    --proto '=https' --tlsv1.2 \
    --output "$LOCAL_STAGING/api.sha256" \
    "$release_base/deploy-go-api-linux-$API_ASSET_ARCH.sha256"
  expected_api_sha="$(awk -v name="deploy-go-api-linux-$API_ASSET_ARCH" \
    '$2 == name {print $1; exit}' "$LOCAL_STAGING/api.sha256")"
  [[ -n "$expected_api_sha" ]] || die "无法读取 API SHA-256"
  [[ "$(sha256_of "$LOCAL_STAGING/deploy-go-api-linux-$API_ASSET_ARCH")" == "$expected_api_sha" ]] ||
    die "API 二进制 SHA-256 校验失败"
  mv "$LOCAL_STAGING/deploy-go-api-linux-$API_ASSET_ARCH" "$LOCAL_STAGING/deploy-go-api"
  chmod 0755 "$LOCAL_STAGING/deploy-go-api"

  curl --fail --silent --show-error --location --retry 3 \
    --proto '=https' --tlsv1.2 \
    --output "$LOCAL_STAGING/deploy-go-admin-web.tar.gz" \
    "$release_base/deploy-go-admin-web.tar.gz"
  curl --fail --silent --show-error --location --retry 3 \
    --proto '=https' --tlsv1.2 \
    --output "$LOCAL_STAGING/SHA256SUMS" \
    "$release_base/SHA256SUMS"
  expected_web_sha="$(awk '$2 == "deploy-go-admin-web.tar.gz" {print $1; exit}' \
    "$LOCAL_STAGING/SHA256SUMS")"
  [[ -n "$expected_web_sha" ]] || die "无法读取 Web SHA-256"
  [[ "$(sha256_of "$LOCAL_STAGING/deploy-go-admin-web.tar.gz")" == "$expected_web_sha" ]] ||
    die "Web 归档 SHA-256 校验失败"
  tar -xzf "$LOCAL_STAGING/deploy-go-admin-web.tar.gz" -C "$LOCAL_STAGING/web"
  [[ -f "$LOCAL_STAGING/web/index.html" ]] || die "Web 归档缺少 index.html"

  mkdir -p "$LOCAL_STAGING/deployer-release"
  for arch in x86_64 aarch64; do
    curl --fail --silent --show-error --location --retry 3 \
      --proto '=https' --tlsv1.2 \
      --output "$LOCAL_STAGING/deployer-release/deploy-go-deployer-linux-$arch" \
      "$release_base/deploy-go-deployer-linux-$arch"
    curl --fail --silent --show-error --location --retry 3 \
      --proto '=https' --tlsv1.2 \
      --output "$LOCAL_STAGING/deployer-release/deploy-go-deployer-linux-$arch.sha256" \
      "$release_base/deploy-go-deployer-linux-$arch.sha256"
    expected_deployer_sha="$(awk \
      -v name="deploy-go-deployer-linux-$arch" \
      '$2 == name {print $1; exit}' \
      "$LOCAL_STAGING/deployer-release/deploy-go-deployer-linux-$arch.sha256")"
    [[ -n "$expected_deployer_sha" ]] || die "无法读取 deployer $arch SHA-256"
    [[ "$(sha256_of "$LOCAL_STAGING/deployer-release/deploy-go-deployer-linux-$arch")" == \
      "$expected_deployer_sha" ]] ||
      die "deployer $arch 二进制 SHA-256 校验失败"
    chmod 0755 "$LOCAL_STAGING/deployer-release/deploy-go-deployer-linux-$arch"
  done
  deploy-go-deployer/release/generate-manifest.sh \
    "$LOCAL_STAGING/deployer-release" \
    "https://deploy-go.invalid/deployer-releases/$DEPLOYER_VERSION" \
    "$DEPLOYER_VERSION"
  jq -e --arg version "$DEPLOYER_VERSION" \
    '.schema_version == 1 and .deployer_version == $version and
     ([.artifacts[].architecture] | sort == ["aarch64","x86_64"])' \
    "$LOCAL_STAGING/deployer-release/deploy-go-deployer-manifest.json" >/dev/null ||
    die "release deployer manifest 校验失败"
else
  require_command docker
  require_command npm
  require_command node

  npm ci
  npm run build --workspace deploy-go-admin
  node scripts/check-client-sensitive-data.mjs admin/dist
  cp -R admin/dist/. "$LOCAL_STAGING/web/"

  docker build \
    --platform "$DEPLOY_PLATFORM" \
    --tag "$API_IMAGE" \
    --file api/docker/release/Dockerfile \
    .
  container_id="$(docker create "$API_IMAGE")"
  docker cp "$container_id:/app/deploy-go-api" "$LOCAL_STAGING/deploy-go-api"
  docker rm -f "$container_id" >/dev/null
  container_id=""
  chmod 0755 "$LOCAL_STAGING/deploy-go-api"
  build_deployer_release "$LOCAL_STAGING/deployer-release"
fi

cp deploy/production/web_server.py "$LOCAL_STAGING/web_server.py"
cp deploy/production/install.sh "$LOCAL_STAGING/install.sh"
if [[ "$DEPLOY_AGENT_SYNC" == "1" ]]; then
  build_agent_release "$LOCAL_STAGING/agent-release"
fi

REMOTE_STAGING="$REMOTE_STAGING_ROOT/staging.$(openssl rand -hex 12)"
remote_staging_created="1"
ssh "$DEPLOY_HOST" \
  "install -d -m 0700 -o root -g root '$REMOTE_STAGING_ROOT' '$REMOTE_STAGING'"

{
  printf 'DEPLOY_GO_API_PORT=%s\n' "$DEPLOY_API_PORT"
  printf 'DEPLOY_GO_API_BIND=%s\n' "$DEPLOY_API_BIND"
  printf 'DEPLOY_GO_WEB_PORT=%s\n' "$DEPLOY_WEB_PORT"
  printf 'DEPLOY_GO_WEB_BIND=%s\n' "$DEPLOY_WEB_BIND"
  printf 'DEPLOY_GO_ALLOWED_ORIGIN=%s\n' "$DEPLOY_GO_ALLOWED_ORIGIN"
  printf 'DEPLOY_GO_COOKIE_SECURE=%s\n' "$DEPLOY_GO_COOKIE_SECURE"
  printf 'DEPLOY_GO_MASTER_KEY_VERSION=%s\n' "$DEPLOY_GO_MASTER_KEY_VERSION"
  printf 'DEPLOY_GO_PUBLIC_BASE_URL=%s\n' "$DEPLOY_GO_PUBLIC_BASE_URL"
  printf 'DEPLOY_GO_AGENT_PROTOCOL_VERSION=%s\n' "$AGENT_PROTOCOL_VERSION"
  printf 'DEPLOY_GO_DEPLOYER_VERSION=%s\n' "$DEPLOYER_VERSION"
  printf 'DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED=%s\n' "$DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED"
  printf 'DEPLOY_GO_ARTIFACTS_ROOT=%s\n' "$DEPLOY_GO_ARTIFACTS_ROOT"
  printf 'DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES=%s\n' "$DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES"
  printf 'DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES=%s\n' "$DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES"
  printf 'DEPLOY_GO_ARTIFACT_MAX_FILES=%s\n' "$DEPLOY_GO_ARTIFACT_MAX_FILES"
  printf 'DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES=%s\n' "$DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES"
  printf 'DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS=%s\n' "$DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS"
  printf 'DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS=%s\n' "$DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS"
  if [[ "$DEPLOY_AGENT_SYNC" == "1" ]]; then
    printf 'DEPLOY_GO_AGENT_VERSION=%s\n' "$AGENT_VERSION"
  else
    printf 'DEPLOY_GO_AGENT_VERSION=\n'
  fi
} >"$LOCAL_STAGING/install.env"
chmod 0600 "$LOCAL_STAGING/install.env"

rsync -az --delete "$LOCAL_STAGING/" "$DEPLOY_HOST:$REMOTE_STAGING/"
ssh "$DEPLOY_HOST" \
  "chown -R root:root '$REMOTE_STAGING' && chmod 0700 '$REMOTE_STAGING' && chmod 0600 '$REMOTE_STAGING/install.env'"

ssh "$DEPLOY_HOST" "bash '$REMOTE_STAGING/install.sh'"
ssh "$DEPLOY_HOST" "rm -rf -- '$REMOTE_STAGING'"
remote_staging_created="0"

printf '正式环境部署完成\n'
printf '  API：http://%s:%s\n' "$remote_host" "$DEPLOY_API_PORT"
printf '  Web：http://%s:%s\n' "$remote_host" "$DEPLOY_WEB_PORT"
if [[ "$DEPLOY_AGENT_SYNC" == "1" ]]; then
  printf '  Agent：%s（本机构建上传）\n' "$AGENT_VERSION"
fi
