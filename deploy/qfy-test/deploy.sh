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
DEPLOY_API_BIND="${DEPLOY_API_BIND:-0.0.0.0}"
DEPLOY_WEB_PORT="${DEPLOY_WEB_PORT:-30101}"
DEPLOY_WEB_BIND="${DEPLOY_WEB_BIND:-0.0.0.0}"
DEPLOY_GO_COOKIE_SECURE="${DEPLOY_GO_COOKIE_SECURE:-false}"
DEPLOY_GO_MASTER_KEY_VERSION="${DEPLOY_GO_MASTER_KEY_VERSION:-1}"
DEPLOY_GO_PUBLIC_BASE_URL="${DEPLOY_GO_PUBLIC_BASE_URL:-}"
DEPLOY_GO_ALLOWED_ORIGIN="${DEPLOY_GO_ALLOWED_ORIGIN:-}"
DEPLOY_AGENT_SYNC="${DEPLOY_AGENT_SYNC:-}"
DEPLOY_GITHUB_REPOSITORY="${DEPLOY_GITHUB_REPOSITORY:-ZhcChen/deploy-go}"
remote_host="$DEPLOY_HOST"
REMOTE_STAGING_ROOT="/var/lib/deploy-go-installer"
REMOTE_STAGING=""
LOCAL_STAGING=""
API_IMAGE="${DEPLOY_API_IMAGE:-deploy-go-api:qfy-test}"

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

require_command ssh
require_command rsync
require_command curl
require_command mktemp
require_command openssl

case "$DEPLOY_SOURCE" in
  build | release) ;;
  *) die "DEPLOY_SOURCE 只支持 build 或 release" ;;
esac

API_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' api/Cargo.toml | head -n 1 | tr -d '\r')"
AGENT_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' agent/Cargo.toml | head -n 1 | tr -d '\r')"
[[ -n "$API_VERSION" && "$API_VERSION" == "$AGENT_VERSION" ]] ||
  die "API 与 Agent 版本不一致：$API_VERSION != $AGENT_VERSION"

if [[ -z "$DEPLOY_ARCH" ]]; then
  remote_arch="$(ssh "$DEPLOY_HOST" 'uname -m' 2>/dev/null | tr -d '\r\n' || true)"
  case "$remote_arch" in
    x86_64) DEPLOY_ARCH="x86_64" ;;
    aarch64 | arm64) DEPLOY_ARCH="arm64" ;;
    *) die "无法识别 qfy-test 架构：${remote_arch:-未知}" ;;
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

if [[ -z "$DEPLOY_AGENT_SYNC" ]]; then
  if [[ "$DEPLOY_SOURCE" == "release" ]]; then
    DEPLOY_AGENT_SYNC="1"
  else
    DEPLOY_AGENT_SYNC="0"
  fi
fi
case "$DEPLOY_AGENT_SYNC" in
  0 | 1) ;;
  *) die "DEPLOY_AGENT_SYNC 必须为 0 或 1" ;;
esac

[[ "$DEPLOY_API_PORT" =~ ^[0-9]+$ ]] || die "DEPLOY_API_PORT 无效"
[[ "$DEPLOY_WEB_PORT" =~ ^[0-9]+$ ]] || die "DEPLOY_WEB_PORT 无效"
[[ "$DEPLOY_GO_COOKIE_SECURE" == "true" || "$DEPLOY_GO_COOKIE_SECURE" == "false" ]] ||
  die "DEPLOY_GO_COOKIE_SECURE 必须为 true 或 false"
[[ "$DEPLOY_GO_MASTER_KEY_VERSION" =~ ^[1-9][0-9]*$ ]] ||
  die "DEPLOY_GO_MASTER_KEY_VERSION 必须为正整数"
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

LOCAL_STAGING="$(mktemp -d "${TMPDIR:-/tmp}/deploy-go-qfy-test.XXXXXX")"
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
fi

cp deploy/qfy-test/web_server.py "$LOCAL_STAGING/web_server.py"
cp deploy/qfy-test/install.sh "$LOCAL_STAGING/install.sh"
if [[ "$DEPLOY_AGENT_SYNC" == "1" ]]; then
  cp scripts/sync-agent-release.sh "$LOCAL_STAGING/sync-agent-release.sh"
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
  if [[ "$DEPLOY_AGENT_SYNC" == "1" ]]; then
    printf 'DEPLOY_GO_AGENT_VERSION=%s\n' "$AGENT_VERSION"
  else
    printf 'DEPLOY_GO_AGENT_VERSION=\n'
  fi
  printf 'DEPLOY_GO_GITHUB_REPOSITORY=%s\n' "$DEPLOY_GITHUB_REPOSITORY"
} >"$LOCAL_STAGING/install.env"
chmod 0600 "$LOCAL_STAGING/install.env"

rsync -az --delete "$LOCAL_STAGING/" "$DEPLOY_HOST:$REMOTE_STAGING/"
ssh "$DEPLOY_HOST" \
  "chown -R root:root '$REMOTE_STAGING' && chmod 0700 '$REMOTE_STAGING' && chmod 0600 '$REMOTE_STAGING/install.env'"

ssh "$DEPLOY_HOST" "bash '$REMOTE_STAGING/install.sh'"
ssh "$DEPLOY_HOST" "rm -rf -- '$REMOTE_STAGING'"
remote_staging_created="0"

printf 'qfy-test 部署完成\n'
printf '  API：http://%s:%s\n' "$remote_host" "$DEPLOY_API_PORT"
printf '  Web：http://%s:%s\n' "$remote_host" "$DEPLOY_WEB_PORT"
