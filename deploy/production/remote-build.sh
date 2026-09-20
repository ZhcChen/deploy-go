#!/usr/bin/env bash

set -euo pipefail

source_dir=""
output_dir=""
deploy_platform=""
expected_commit=""
api_version=""
agent_version=""
executor_version=""
deployer_version=""
agent_sync="1"
proxy_url=""
builder_name=""
web_dockerfile=""

die() {
  printf 'REMOTE_BUILD_ERROR %s\n' "$1" >&2
  exit 1
}

while (($# > 0)); do
  case "$1" in
    --source-dir) source_dir="$2"; shift 2 ;;
    --output-dir) output_dir="$2"; shift 2 ;;
    --deploy-platform) deploy_platform="$2"; shift 2 ;;
    --expected-commit) expected_commit="$2"; shift 2 ;;
    --api-version) api_version="$2"; shift 2 ;;
    --agent-version) agent_version="$2"; shift 2 ;;
    --executor-version) executor_version="$2"; shift 2 ;;
    --deployer-version) deployer_version="$2"; shift 2 ;;
    --agent-sync) agent_sync="$2"; shift 2 ;;
    --proxy-url) proxy_url="$2"; shift 2 ;;
    *) die "未知参数：$1" ;;
  esac
done

[[ -d "$source_dir" && ! -L "$source_dir" ]] || die "源码目录无效"
[[ "$output_dir" == /var/lib/deploy-go-builder/* ]] || die "构建输出目录必须位于独立构建根目录"
[[ "$deploy_platform" == linux/amd64 || "$deploy_platform" == linux/arm64 ]] || die "目标平台无效"
[[ "$expected_commit" =~ ^[0-9a-f]{40}$ ]] || die "源码 commit 无效"
[[ "$agent_sync" == 0 || "$agent_sync" == 1 ]] || die "Agent 同步参数无效"
[[ "$api_version" == "$agent_version" && "$agent_version" == "$executor_version" && "$executor_version" == "$deployer_version" ]] ||
  die "发布版本不一致"

command -v docker >/dev/null 2>&1 || die "qfy-test2 缺少 docker"
docker info >/dev/null 2>&1 || die "qfy-test2 Docker daemon 不可用"
docker buildx version >/dev/null 2>&1 || die "qfy-test2 缺少 docker buildx"
command -v jq >/dev/null 2>&1 || die "qfy-test2 缺少 jq"
command -v sha256sum >/dev/null 2>&1 || die "qfy-test2 缺少 sha256sum"
command -v curl >/dev/null 2>&1 || die "qfy-test2 缺少 curl"
[[ "$(cat "$source_dir/.deploy-go-source-commit")" == "$expected_commit" ]] || die "源码快照 commit 校验失败"

if [[ -z "$proxy_url" ]]; then
  for candidate in http://127.0.0.1:10800 http://127.0.0.1:10808; do
    if curl --fail --silent --show-error --max-time 5 \
      --proxy "$candidate" --output /dev/null https://registry-1.docker.io/v2/; then
      proxy_url="$candidate"
      break
    fi
  done
fi
[[ -n "$proxy_url" ]] || die "qfy-test2 未找到可访问 Docker Registry 的构建代理（尝试 10800、10808）"

builder_name="deploy-go-remote-${expected_commit:0:12}-$$"
docker buildx create \
  --name "$builder_name" \
  --driver docker-container \
  --driver-opt network=host \
  --driver-opt "env.http_proxy=$proxy_url" \
  --driver-opt "env.https_proxy=$proxy_url" \
  --driver-opt "env.all_proxy=$proxy_url" \
  >/dev/null
cleanup_builder() {
  [[ -z "$builder_name" ]] || docker buildx rm --force "$builder_name" >/dev/null 2>&1 || true
}
trap cleanup_builder EXIT
docker buildx inspect --bootstrap "$builder_name" >/dev/null

mkdir -p "$output_dir/deployer-release" "$output_dir/web"
if [[ "$agent_sync" == 1 ]]; then
  mkdir -p "$output_dir/agent-release"
fi
export BUILDKIT_NO_CLIENT_TOKEN="${BUILDKIT_NO_CLIENT_TOKEN:-1}"
build_proxy_args=(
  --build-arg "HTTP_PROXY=$proxy_url"
  --build-arg "HTTPS_PROXY=$proxy_url"
  --build-arg "ALL_PROXY=$proxy_url"
  --build-arg "http_proxy=$proxy_url"
  --build-arg "https_proxy=$proxy_url"
  --build-arg "all_proxy=$proxy_url"
)

build_rust_release() {
  local arch="$1"
  local platform="$2"
  local image="deploy-go-rust-release-remote-$expected_commit-$arch"
  local container_id=""

  trap '[[ -z "$container_id" ]] || docker rm -f "$container_id" >/dev/null 2>&1 || true' RETURN
  docker buildx build --builder "$builder_name" \
    --platform "$platform" \
    --network host \
    "${build_proxy_args[@]}" \
    --build-arg BUILD_API=1 \
    --build-arg "BUILD_AGENT=$agent_sync" \
    --build-arg BUILD_DEPLOYER=1 \
    --tag "$image" \
    --load \
    --file "$source_dir/deploy/docker/release/Dockerfile" \
    "$source_dir"
  container_id="$(docker create "$image")"
  if [[ "$platform" == "$deploy_platform" ]]; then
    docker cp "$container_id:/out/deploy-go-api" "$output_dir/deploy-go-api"
    chmod 0755 "$output_dir/deploy-go-api"
  fi
  if [[ "$agent_sync" == 1 ]]; then
    docker cp "$container_id:/out/deploy-go-agent" \
      "$output_dir/agent-release/deploy-go-agent-linux-$arch"
    docker cp "$container_id:/out/deploy-go-agent-executor" \
      "$output_dir/agent-release/deploy-go-agent-executor-linux-$arch"
  fi
  docker cp "$container_id:/out/deploy-go-deployer" \
    "$output_dir/deployer-release/deploy-go-deployer-linux-$arch"
  chmod 0755 \
    "$output_dir/deployer-release/deploy-go-deployer-linux-$arch"
  if [[ "$agent_sync" == 1 ]]; then
    chmod 0755 \
      "$output_dir/agent-release/deploy-go-agent-linux-$arch" \
      "$output_dir/agent-release/deploy-go-agent-executor-linux-$arch"
  fi
  docker rm -f "$container_id" >/dev/null
  container_id=""
  trap - RETURN
}

build_rust_release x86_64 linux/amd64
build_rust_release aarch64 linux/arm64

web_dockerfile="$source_dir/deploy/production/web-build.Dockerfile"
cat > "$web_dockerfile" <<'EOF'
# syntax=docker/dockerfile:1.7
FROM node:22-alpine AS builder
ARG HTTP_PROXY
ARG HTTPS_PROXY
ARG ALL_PROXY
ARG http_proxy
ARG https_proxy
ARG all_proxy
ENV HTTP_PROXY=$HTTP_PROXY HTTPS_PROXY=$HTTPS_PROXY ALL_PROXY=$ALL_PROXY \
    http_proxy=$http_proxy https_proxy=$https_proxy all_proxy=$all_proxy
WORKDIR /workspace
COPY . .
RUN npm ci && npm run build --workspace deploy-go-admin && \
    node scripts/check-client-sensitive-data.mjs admin/dist && \
    mkdir -p /web && cp -R admin/dist/. /web/
FROM scratch
COPY --from=builder /web/ /
EOF
docker buildx build --builder "$builder_name" \
  --platform "$deploy_platform" \
  --network host \
  "${build_proxy_args[@]}" \
  --output "type=local,dest=$output_dir/web" \
  --file "$web_dockerfile" \
  "$source_dir"

rm -f "$web_dockerfile"

cp "$source_dir/deploy-go-deployer/release/generate-manifest.sh" "$output_dir/deployer-release/"

if [[ "$agent_sync" == 1 ]]; then
  cp "$source_dir/agent/install/deploy-go-agent.service" "$output_dir/agent-release/"
  cp "$source_dir/agent/install/deploy-go-agent-runner.service" "$output_dir/agent-release/"
  cp "$source_dir/agent/install/deploy-go-agent-executor.service" "$output_dir/agent-release/"
  cp "$source_dir/agent/install/executor.json.in" "$output_dir/agent-release/"
  cp "$source_dir/agent/release/generate-manifest.sh" "$output_dir/agent-release/"
  (
    cd "$output_dir/agent-release"
    bash generate-manifest.sh . "https://deploy-go.invalid/agent-releases/$agent_version" "$agent_version"
    rm generate-manifest.sh
  )
fi
(
  cd "$output_dir/deployer-release"
  bash generate-manifest.sh . "https://deploy-go.invalid/deployer-releases/$deployer_version" "$deployer_version"
  rm generate-manifest.sh
)

if [[ "$agent_sync" == 1 ]]; then
  jq -e --arg version "$agent_version" \
    '.agent_version == $version and .executor_version == $version' \
    "$output_dir/agent-release/deploy-go-agent-manifest.json" >/dev/null || die "Agent manifest 校验失败"
fi
jq -e --arg version "$deployer_version" \
  '.deployer_version == $version' \
  "$output_dir/deployer-release/deploy-go-deployer-manifest.json" >/dev/null || die "deployer manifest 校验失败"
printf '%s\n' "$expected_commit" > "$output_dir/build-commit"
printf '远程构建完成：%s\n' "$expected_commit"
