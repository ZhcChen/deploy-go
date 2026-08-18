#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$REPO_ROOT/api/Cargo.toml" | head -n 1 | tr -d '\r')"
AGENT_PROTOCOL_VERSION="$(sed -n 's/^pub const PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' "$REPO_ROOT/agent-protocol/src/lib.rs" | head -n 1)"
DEPLOY_SCRIPT="$REPO_ROOT/deploy/production/deploy.sh"
INSTALL_SCRIPT="$REPO_ROOT/deploy/production/install.sh"
UNIFIED_DOCKERFILE="$REPO_ROOT/deploy/docker/release/Dockerfile"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/deploy-go-contract.XXXXXX")"
MOCK_BIN="$TEST_ROOT/bin"
MOCK_LOG="$TEST_ROOT/mock.log"
CAPTURE_DIR="$TEST_ROOT/captures"

cleanup() {
  rm -rf -- "$TEST_ROOT"
}
trap cleanup EXIT
mkdir -p "$MOCK_BIN" "$CAPTURE_DIR" "$TEST_ROOT/local"

assert_contains() {
  local file="$1"
  local expected="$2"
  grep -F -- "$expected" "$file" >/dev/null || {
    printf '缺少部署安全契约：%s\n' "$expected" >&2
    exit 1
  }
}

assert_not_contains() {
  local file="$1"
  local needle="$2"
  if grep -F -- "$needle" "$file" >/dev/null; then
    printf '不应包含会破坏构建分层的内容：%s（%s）\n' "$needle" "$file" >&2
    exit 1
  fi
}

assert_line_before() {
  local file="$1"
  local earlier="$2"
  local later="$3"
  local earlier_line later_line
  earlier_line="$(grep -nF -- "$earlier" "$file" | head -n 1 | cut -d: -f1)"
  later_line="$(grep -nF -- "$later" "$file" | head -n 1 | cut -d: -f1)"
  if [[ -z "$earlier_line" || -z "$later_line" || "$earlier_line" -ge "$later_line" ]]; then
    printf '构建分层顺序无效：%s 必须早于 %s（%s）\n' "$earlier" "$later" "$file" >&2
    exit 1
  fi
}

cat >"$MOCK_BIN/ssh" <<'EOF'
#!/usr/bin/env bash
printf 'ssh %s\n' "$*" >>"$MOCK_LOG"
if [[ "$*" == *"uname -m"* ]]; then
  printf 'x86_64\n'
fi
EOF

cat >"$MOCK_BIN/rsync" <<'EOF'
#!/usr/bin/env bash
printf 'rsync %s\n' "$*" >>"$MOCK_LOG"
if [[ "${MOCK_RSYNC_FAIL:-0}" == "1" ]]; then
  exit 23
fi
source_dir="${@: -2:1}"
capture_count="$(find "$CAPTURE_DIR" -maxdepth 1 -name 'install.env.*' | wc -l | tr -d '[:space:]')"
cp "${source_dir%/}/install.env" "$CAPTURE_DIR/install.env.$capture_count"
EOF

cat >"$MOCK_BIN/curl" <<'EOF'
#!/usr/bin/env bash
output=""
while (($#)); do
  if [[ "$1" == "--output" ]]; then
    output="$2"
    shift 2
  else
    shift
  fi
done
case "$(basename "$output")" in
  api.sha256) printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  deploy-go-api-linux-x86_64\n' >"$output" ;;
  SHA256SUMS) printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  deploy-go-admin-web.tar.gz\n' >"$output" ;;
  deploy-go-deployer-linux-x86_64.sha256) \
    printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  deploy-go-deployer-linux-x86_64\n' >"$output" ;;
  deploy-go-deployer-linux-aarch64.sha256) \
    printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  deploy-go-deployer-linux-aarch64\n' >"$output" ;;
  *) printf 'fixture\n' >"$output" ;;
esac
EOF

cat >"$MOCK_BIN/sha256sum" <<'EOF'
#!/usr/bin/env bash
printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  %s\n' "$1"
EOF

cat >"$MOCK_BIN/tar" <<'EOF'
#!/usr/bin/env bash
destination=""
while (($#)); do
  if [[ "$1" == "-C" ]]; then
    destination="$2"
    shift 2
  else
    shift
  fi
done
printf '<html></html>\n' >"$destination/index.html"
EOF

chmod +x "$MOCK_BIN"/*
export MOCK_LOG CAPTURE_DIR

for _ in 1 2; do
  PATH="$MOCK_BIN:$PATH" \
    TMPDIR="$TEST_ROOT/local" \
    DEPLOY_SOURCE=release \
    DEPLOY_RELEASE_TAG="v$API_VERSION" \
    DEPLOY_ARCH=x86_64 \
    DEPLOY_AGENT_SYNC=0 \
    DEPLOY_GO_ALLOWED_ORIGIN=https://deploy.example.test \
    DEPLOY_GO_COOKIE_SECURE=true \
    DEPLOY_API_BIND='127.0.0.1;touch /tmp/should-not-run' \
    bash "$DEPLOY_SCRIPT" >/dev/null
done

remote_staging_paths=()
while IFS= read -r remote_staging_path; do
  remote_staging_paths+=("$remote_staging_path")
done < <(
  grep '^ssh .*install -d ' "$MOCK_LOG" |
    grep -o '/var/lib/deploy-go-installer/staging\.[0-9a-f]\{24\}'
)
[[ "${#remote_staging_paths[@]}" -eq 2 ]] || {
  printf '未记录两次远端随机 staging\n' >&2
  exit 1
}
[[ "${remote_staging_paths[0]}" != "${remote_staging_paths[1]}" ]] || {
  printf '两次部署复用了远端 staging\n' >&2
  exit 1
}

local_staging_paths=()
while IFS= read -r local_staging_path; do
  local_staging_paths+=("$local_staging_path")
done < <(
  grep '^rsync ' "$MOCK_LOG" | sed -n 's#^rsync -az --delete \([^ ]*\)/ .*#\1#p'
)
[[ "${#local_staging_paths[@]}" -eq 2 ]] || {
  printf '未记录两次本地随机 staging\n' >&2
  exit 1
}
[[ "${local_staging_paths[0]}" != "${local_staging_paths[1]}" ]] || {
  printf '两次部署复用了本地 staging\n' >&2
  exit 1
}
for local_staging in "${local_staging_paths[@]}"; do
  [[ ! -e "$local_staging" ]] || {
    printf '正常退出后遗留本地 staging：%s\n' "$local_staging" >&2
    exit 1
  }
done
for remote_staging in "${remote_staging_paths[@]}"; do
  grep -F "rm -rf -- '$remote_staging'" "$MOCK_LOG" >/dev/null || {
    printf '正常退出后未清理远端 staging：%s\n' "$remote_staging" >&2
    exit 1
  }
done

set +e
PATH="$MOCK_BIN:$PATH" \
  TMPDIR="$TEST_ROOT/local" \
  MOCK_RSYNC_FAIL=1 \
  DEPLOY_SOURCE=release \
  DEPLOY_RELEASE_TAG="v$API_VERSION" \
  DEPLOY_ARCH=x86_64 \
  DEPLOY_AGENT_SYNC=0 \
  DEPLOY_GO_ALLOWED_ORIGIN=https://deploy.example.test \
  bash "$DEPLOY_SCRIPT" >/dev/null 2>&1
failure_status=$?
set -e
[[ "$failure_status" -ne 0 ]] || {
  printf 'rsync 失败时部署脚本应返回非零\n' >&2
  exit 1
}
failed_remote_staging="$(grep '^ssh .*install -d ' "$MOCK_LOG" | tail -n 1 | grep -o '/var/lib/deploy-go-installer/staging\.[0-9a-f]\{24\}')"
grep -F "rm -rf -- '$failed_remote_staging'" "$MOCK_LOG" >/dev/null || {
  printf 'rsync 失败后未清理远端 staging\n' >&2
  exit 1
}

set +e
PATH="$MOCK_BIN:$PATH" \
  DEPLOY_SOURCE=release \
  DEPLOY_RELEASE_TAG="v$API_VERSION" \
  DEPLOY_ARCH=x86_64 \
  DEPLOY_PLATFORM=linux/arm64 \
  DEPLOY_AGENT_SYNC=0 \
  bash "$DEPLOY_SCRIPT" >/dev/null 2>&1
platform_mismatch_status=$?
set -e
[[ "$platform_mismatch_status" -ne 0 ]] || {
  printf 'DEPLOY_ARCH 与 DEPLOY_PLATFORM 不一致时必须拒绝部署\n' >&2
  exit 1
}

if grep -E 'ssh .*env .*DEPLOY_GO_' "$MOCK_LOG" >/dev/null; then
  printf '部署配置不得拼入 SSH 远端 shell 命令\n' >&2
  exit 1
fi
assert_contains "$CAPTURE_DIR/install.env.0" 'DEPLOY_GO_API_BIND=127.0.0.1;touch /tmp/should-not-run'
assert_contains "$CAPTURE_DIR/install.env.0" 'DEPLOY_GO_ALLOWED_ORIGIN=https://deploy.example.test'
assert_contains "$CAPTURE_DIR/install.env.0" 'DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED=true'
assert_contains "$CAPTURE_DIR/install.env.0" 'DEPLOY_GO_ARTIFACTS_ROOT=/var/lib/deploy-go/artifacts'
assert_contains "$CAPTURE_DIR/install.env.0" "DEPLOY_GO_AGENT_PROTOCOL_VERSION=$AGENT_PROTOCOL_VERSION"
assert_contains "$CAPTURE_DIR/install.env.0" "DEPLOY_GO_DEPLOYER_VERSION=$API_VERSION"

cat >"$MOCK_BIN/docker" <<'EOF'
#!/usr/bin/env bash
printf 'docker %s\n' "$*" >>"$MOCK_LOG"
case "$1" in
  build)
    exit 0
    ;;
  create)
    printf 'mock-container\n'
    ;;
  cp)
    destination="$3"
    mkdir -p "$(dirname "$destination")"
    printf 'mock release artifact\n' >"$destination"
    ;;
  rm)
    exit 0
    ;;
  *)
    printf 'unexpected docker command: %s\n' "$*" >&2
    exit 1
    ;;
esac
EOF
chmod +x "$MOCK_BIN/docker"
: >"$MOCK_LOG"

agent_output="$TEST_ROOT/agent-release"
PATH="$MOCK_BIN:$PATH" \
  DEPLOY_SOURCE=build \
  DEPLOY_AGENT_BUILD_ONLY=1 \
  DEPLOY_AGENT_OUTPUT_DIR="$agent_output" \
  bash "$DEPLOY_SCRIPT" >/dev/null

[[ "$(grep -c '^docker build ' "$MOCK_LOG")" -eq 2 ]] || {
  printf 'Agent build-only 必须每架构仅执行一次统一构建\n' >&2
  exit 1
}
assert_contains "$MOCK_LOG" 'docker build --platform linux/amd64 --build-arg BUILD_API=0 --build-arg BUILD_AGENT=1 --build-arg BUILD_DEPLOYER=0'
assert_contains "$MOCK_LOG" 'docker build --platform linux/arm64 --build-arg BUILD_API=0 --build-arg BUILD_AGENT=1 --build-arg BUILD_DEPLOYER=0'
[[ "$(grep -c '^docker cp .*:/out/deploy-go-agent ' "$MOCK_LOG")" -eq 2 ]] || {
  printf 'Agent build-only 未导出双架构 Agent\n' >&2
  exit 1
}
[[ "$(grep -c '^docker cp .*:/out/deploy-go-agent-executor ' "$MOCK_LOG")" -eq 2 ]] || {
  printf 'Agent build-only 未导出双架构 executor\n' >&2
  exit 1
}
[[ "$(grep -c '^docker create ' "$MOCK_LOG")" -eq 2 ]] || {
  printf 'Agent build-only 必须为每个架构创建独立产物容器\n' >&2
  exit 1
}
[[ "$(grep -c '^docker rm -f ' "$MOCK_LOG")" -eq 2 ]] || {
  printf 'Agent build-only 必须清理每个架构的产物容器\n' >&2
  exit 1
}
if grep -E '^(ssh|rsync) ' "$MOCK_LOG" >/dev/null; then
  printf 'Agent build-only 不得连接远端或上传文件\n' >&2
  exit 1
fi
for artifact in \
  deploy-go-agent-linux-x86_64 \
  deploy-go-agent-linux-aarch64 \
  deploy-go-agent-executor-linux-x86_64 \
  deploy-go-agent-executor-linux-aarch64 \
  deploy-go-agent.service \
  deploy-go-agent-runner.service \
  deploy-go-agent-executor.service \
  executor.json.in \
  deploy-go-agent-manifest.json; do
  [[ -f "$agent_output/$artifact" ]] || {
    printf 'Agent build-only 缺少产物：%s\n' "$artifact" >&2
    exit 1
  }
done

assert_contains "$DEPLOY_SCRIPT" 'REMOTE_STAGING_ROOT="/var/lib/deploy-go-installer"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_API_BIND="${DEPLOY_API_BIND:-127.0.0.1}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_WEB_BIND="${DEPLOY_WEB_BIND:-127.0.0.1}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_GO_COOKIE_SECURE="${DEPLOY_GO_COOKIE_SECURE:-true}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_GO_PUBLIC_BASE_URL="${DEPLOY_GO_PUBLIC_BASE_URL:-https://deploy.quanxinfu.com}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_GO_ALLOWED_ORIGIN="${DEPLOY_GO_ALLOWED_ORIGIN:-https://deploy.quanxinfu.com}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_AGENT_SYNC="${DEPLOY_AGENT_SYNC:-1}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_AGENT_BUILD_ONLY="${DEPLOY_AGENT_BUILD_ONLY:-0}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_AGENT_OUTPUT_DIR="${DEPLOY_AGENT_OUTPUT_DIR:-target/deploy-release/agent}"'
assert_contains "$DEPLOY_SCRIPT" 'build_rust_releases'
assert_contains "$DEPLOY_SCRIPT" 'deploy/docker/release/Dockerfile'
assert_contains "$DEPLOY_SCRIPT" 'docker cp "$container_id:/out/deploy-go-api"'
assert_contains "$DEPLOY_SCRIPT" 'docker cp "$container_id:/out/deploy-go-agent"'
assert_contains "$DEPLOY_SCRIPT" 'docker cp "$container_id:/out/deploy-go-agent-executor"'
assert_contains "$DEPLOY_SCRIPT" 'docker cp "$container_id:/out/deploy-go-deployer"'
assert_contains "$DEPLOY_SCRIPT" 'deploy-go-agent-executor-linux-$arch'
assert_contains "$DEPLOY_SCRIPT" 'deploy-go-agent-executor.service'
assert_contains "$DEPLOY_SCRIPT" 'deploy-go-agent-runner.service'
assert_contains "$DEPLOY_SCRIPT" 'executor.json.in'
assert_contains "$DEPLOY_SCRIPT" 'agent/release/generate-manifest.sh'
assert_contains "$DEPLOY_SCRIPT" '.protocol.minimum <= $protocol and .protocol.maximum >= $protocol'
assert_not_contains "$DEPLOY_SCRIPT" '--file api/docker/release/Dockerfile'
assert_not_contains "$DEPLOY_SCRIPT" '--file agent/docker/release/Dockerfile'
assert_not_contains "$DEPLOY_SCRIPT" '--file deploy-go-deployer/docker/release/Dockerfile'
assert_contains "$DEPLOY_SCRIPT" 'deploy-go-deployer/release/generate-manifest.sh'
assert_contains "$DEPLOY_SCRIPT" 'deploy-go-deployer-linux-$arch'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_PLATFORM 与 DEPLOY_ARCH 不一致'
assert_contains "$UNIFIED_DOCKERFILE" 'ARG TARGETARCH'
assert_contains "$UNIFIED_DOCKERFILE" 'ARG BUILD_API=0'
assert_contains "$UNIFIED_DOCKERFILE" 'ARG BUILD_AGENT=0'
assert_contains "$UNIFIED_DOCKERFILE" 'ARG BUILD_DEPLOYER=0'
for manifest in agent agent-executor agent-protocol api container-template deploy-go-deployer release-authorization terminal-capability; do
  assert_contains "$UNIFIED_DOCKERFILE" "COPY $manifest/Cargo.toml $manifest/Cargo.toml"
done
for source in agent agent-executor agent-protocol api container-template deploy-go-deployer release-authorization terminal-capability; do
  assert_contains "$UNIFIED_DOCKERFILE" "COPY $source/src $source/src"
done
assert_contains "$UNIFIED_DOCKERFILE" 'COPY api/migrations api/migrations'
assert_contains "$UNIFIED_DOCKERFILE" 'COPY api/openapi/external.json api/openapi/external.json'
assert_contains "$UNIFIED_DOCKERFILE" 'COPY examples/templates examples/templates'
assert_contains "$UNIFIED_DOCKERFILE" 'COPY docs/standards/deploy-artifact-manifest.schema.json docs/standards/deploy-artifact-manifest.schema.json'
assert_contains "$UNIFIED_DOCKERFILE" 'id=deploy-go-cargo-registry'
assert_contains "$UNIFIED_DOCKERFILE" 'id=deploy-go-cargo-git'
assert_contains "$UNIFIED_DOCKERFILE" 'id=deploy-go-rust-target-${TARGETARCH}'
assert_contains "$UNIFIED_DOCKERFILE" 'cargo fetch --locked'
assert_contains "$UNIFIED_DOCKERFILE" 'cargo build --locked --release $packages'
assert_contains "$UNIFIED_DOCKERFILE" 'cp target/release/deploy-go-api /out/deploy-go-api'
assert_contains "$UNIFIED_DOCKERFILE" 'cp target/release/deploy-go-agent /out/deploy-go-agent'
assert_contains "$UNIFIED_DOCKERFILE" 'cp target/release/deploy-go-agent-executor /out/deploy-go-agent-executor'
assert_contains "$UNIFIED_DOCKERFILE" 'cp target/release/deploy-go-deployer /out/deploy-go-deployer'
assert_contains "$REPO_ROOT/agent/docker/release/Dockerfile" \
  'COPY docs/standards/deploy-artifact-manifest.schema.json docs/standards/deploy-artifact-manifest.schema.json'
assert_contains "$REPO_ROOT/agent/docker/release/Dockerfile" 'COPY agent-executor/src agent-executor/src'
assert_contains "$REPO_ROOT/api/docker/release/Dockerfile" \
  'COPY docs/standards/deploy-artifact-manifest.schema.json docs/standards/deploy-artifact-manifest.schema.json'
assert_contains "$REPO_ROOT/api/docker/release/Dockerfile" 'COPY agent-executor/src agent-executor/src'
assert_contains "$REPO_ROOT/.dockerignore" \
  '!docs/standards/deploy-artifact-manifest.schema.json'
assert_contains "$INSTALL_SCRIPT" 'LOCK_FILE="/run/lock/deploy-go-install.lock"'
assert_contains "$INSTALL_SCRIPT" '"install_locked"'
assert_contains "$INSTALL_SCRIPT" 'install_agent_release'
assert_contains "$INSTALL_SCRIPT" 'install_deployer_release'
assert_contains "$INSTALL_SCRIPT" 'deploy-go-agent-executor-linux-x86_64'
assert_contains "$INSTALL_SCRIPT" 'deploy-go-agent-executor-linux-aarch64'
assert_contains "$INSTALL_SCRIPT" 'deploy-go-agent-executor.service'
assert_contains "$INSTALL_SCRIPT" 'deploy-go-agent-runner.service'
assert_contains "$INSTALL_SCRIPT" 'executor.json.in'
assert_contains "$INSTALL_SCRIPT" 'manifest.get("protocol", {}).get("minimum", 0) <= protocol'
assert_contains "$INSTALL_SCRIPT" 'manifest.get("protocol", {}).get("maximum", 0) >= protocol'
assert_contains "$INSTALL_SCRIPT" 'deploy-go-deployer-linux-x86_64'
assert_contains "$INSTALL_SCRIPT" 'deploy-go-deployer-linux-aarch64'
assert_contains "$INSTALL_SCRIPT" 'deploy-go-deployer-manifest.json'
assert_contains "$INSTALL_SCRIPT" 'manifest.get("deployer_version") == sys.argv[2]'
assert_contains "$INSTALL_SCRIPT" 'chown deploy-go:deploy-go "$MASTER_KEY_FILE"'
assert_contains "$INSTALL_SCRIPT" 'chmod 0400 "$MASTER_KEY_FILE"'
assert_contains "$INSTALL_SCRIPT" 'ReadOnlyPaths=$MASTER_KEY_FILE'
assert_contains "$INSTALL_SCRIPT" 'TERMINAL_SIGNING_KEY_FILE="/etc/deploy-go/terminal-signing.key"'
assert_contains "$INSTALL_SCRIPT" 'openssl rand -base64 32 >"$key_tmp"'
assert_contains "$INSTALL_SCRIPT" 'chown root:deploy-go "$TERMINAL_SIGNING_KEY_FILE"'
assert_contains "$INSTALL_SCRIPT" 'chmod 0440 "$TERMINAL_SIGNING_KEY_FILE"'
assert_contains "$INSTALL_SCRIPT" 'DEPLOY_GO_TERMINAL_SIGNING_KEY_FILE=$TERMINAL_SIGNING_KEY_FILE'
assert_contains "$INSTALL_SCRIPT" 'ReadOnlyPaths=$TERMINAL_SIGNING_KEY_FILE'
assert_contains "$INSTALL_SCRIPT" 'RELEASE_SIGNING_KEY_FILE="/etc/deploy-go/release-signing.key"'
assert_contains "$INSTALL_SCRIPT" '特权发布签名密钥必须是普通文件'
assert_contains "$INSTALL_SCRIPT" 'mktemp /etc/deploy-go/.release-signing.key.XXXXXX'
assert_contains "$INSTALL_SCRIPT" 'chown root:deploy-go "$RELEASE_SIGNING_KEY_FILE"'
assert_contains "$INSTALL_SCRIPT" 'chmod 0440 "$RELEASE_SIGNING_KEY_FILE"'
assert_contains "$INSTALL_SCRIPT" 'DEPLOY_GO_RELEASE_SIGNING_KEY_FILE=$RELEASE_SIGNING_KEY_FILE'
assert_contains "$INSTALL_SCRIPT" 'ReadOnlyPaths=$RELEASE_SIGNING_KEY_FILE'
assert_contains "$INSTALL_SCRIPT" 'restore_backup release_signing_key "$RELEASE_SIGNING_KEY_FILE"'
release_key_line="$(grep -n 'mktemp /etc/deploy-go/.release-signing.key' "$INSTALL_SCRIPT" | head -n 1 | cut -d: -f1)"
rollback_armed_line="$(grep -n 'rollback_armed="1"' "$INSTALL_SCRIPT" | head -n 1 | cut -d: -f1)"
if [[ -z "$release_key_line" || -z "$rollback_armed_line" || "$release_key_line" -le "$rollback_armed_line" ]]; then
  printf '特权发布签名密钥必须在回滚备份建立后生成\n' >&2
  exit 1
fi
if grep -F 'cat "$RELEASE_SIGNING_KEY_FILE"' "$INSTALL_SCRIPT" >/dev/null; then
  printf 'install.sh 不得输出特权发布私钥正文\n' >&2
  exit 1
fi
assert_contains "$INSTALL_SCRIPT" 'StateDirectoryMode=0750'
assert_contains "$INSTALL_SCRIPT" 'ReadWritePaths=$DATA_DIR'
assert_contains "$INSTALL_SCRIPT" 'DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS=$ARTIFACT_RETENTION_TTL_SECONDS'
assert_contains "$INSTALL_SCRIPT" 'DEPLOY_GO_DEPLOYER_RELEASE_DIR=$DATA_DIR/deployer-releases'
assert_contains "$INSTALL_SCRIPT" 'wait_for_url "http://127.0.0.1:$API_PORT/api/v1/deployer/download/$deployer_download_version/manifest.json"'
assert_contains "$INSTALL_SCRIPT" 'restore_backup web "$WEB_DIR"'
assert_contains "$INSTALL_SCRIPT" 'restore_backup agent_release "$DATA_DIR/agent-releases/$AGENT_VERSION"'
assert_contains "$INSTALL_SCRIPT" 'restore_backup deployer_release "$DATA_DIR/deployer-releases/$DEPLOYER_VERSION"'
assert_contains "$INSTALL_SCRIPT" 'cp -a -- "$agent_release_path" "$rollback_dir/agent_release"'
assert_contains "$INSTALL_SCRIPT" 'cp -a -- "$deployer_release_path" "$rollback_dir/deployer_release"'
assert_contains "$INSTALL_SCRIPT" ': >"$rollback_dir/agent_release.absent"'
assert_contains "$INSTALL_SCRIPT" 'curl --fail --silent --connect-timeout 1 --max-time 2'
assert_contains "$INSTALL_SCRIPT" 'rollback_armed="1"'
API_DOCKERFILE="$REPO_ROOT/api/docker/release/Dockerfile"
AGENT_DOCKERFILE="$REPO_ROOT/agent/docker/release/Dockerfile"
DEPLOYER_DOCKERFILE="$REPO_ROOT/deploy-go-deployer/docker/release/Dockerfile"
assert_contains "$API_DOCKERFILE" 'COPY release-authorization/src release-authorization/src'
assert_contains "$API_DOCKERFILE" 'COPY agent/release agent/release'
assert_contains "$API_DOCKERFILE" 'COPY agent/install/install.sh agent/install/install.sh'
assert_contains "$API_DOCKERFILE" 'COPY deploy-go-deployer/release/manifest.schema.json deploy-go-deployer/release/manifest.schema.json'
assert_contains "$API_DOCKERFILE" 'COPY deploy-go-deployer/src deploy-go-deployer/src'
assert_contains "$AGENT_DOCKERFILE" 'COPY release-authorization/src release-authorization/src'
assert_contains "$AGENT_DOCKERFILE" 'COPY deploy-go-deployer/Cargo.toml deploy-go-deployer/Cargo.toml'
assert_contains "$AGENT_DOCKERFILE" 'COPY deploy-go-deployer/src deploy-go-deployer/src'
assert_contains "$DEPLOYER_DOCKERFILE" 'COPY agent/src agent/src'
assert_contains "$DEPLOYER_DOCKERFILE" 'COPY agent-executor/src agent-executor/src'
assert_contains "$DEPLOYER_DOCKERFILE" 'COPY agent-protocol/src agent-protocol/src'
assert_contains "$DEPLOYER_DOCKERFILE" 'COPY api/src api/src'
assert_contains "$DEPLOYER_DOCKERFILE" 'COPY release-authorization/src release-authorization/src'
assert_contains "$DEPLOYER_DOCKERFILE" 'COPY terminal-capability/src terminal-capability/src'
for source in \
  'COPY agent/src agent/src' \
  'COPY agent-executor/src agent-executor/src' \
  'COPY agent-protocol/src agent-protocol/src' \
  'COPY release-authorization/src release-authorization/src' \
  'COPY terminal-capability/src terminal-capability/src' \
  'COPY container-template/src container-template/src' \
  'COPY examples/templates examples/templates' \
  'COPY deploy-go-deployer/src deploy-go-deployer/src' \
  'COPY api/src api/src'; do
  assert_contains "$API_DOCKERFILE" "$source"
  assert_contains "$AGENT_DOCKERFILE" "$source"
done
for source in \
  'COPY api/migrations api/migrations' \
  'COPY agent/release agent/release' \
  'COPY agent/install/install.sh agent/install/install.sh' \
  'COPY deploy-go-deployer/release/manifest.schema.json deploy-go-deployer/release/manifest.schema.json'; do
  assert_contains "$API_DOCKERFILE" "$source"
done
for source in \
  'COPY deploy-go-deployer/src deploy-go-deployer/src' \
  'COPY deploy-go-deployer/release deploy-go-deployer/release' \
  'COPY container-template/src container-template/src' \
  'COPY examples/templates examples/templates' \
  'COPY api/openapi/external.json api/openapi/external.json'; do
  assert_contains "$DEPLOYER_DOCKERFILE" "$source"
done
for dockerfile in "$API_DOCKERFILE" "$AGENT_DOCKERFILE" "$DEPLOYER_DOCKERFILE"; do
  assert_contains "$dockerfile" 'ARG TARGETARCH'
  assert_contains "$dockerfile" 'COPY agent/Cargo.toml agent/Cargo.toml'
  assert_contains "$dockerfile" 'COPY agent-executor/Cargo.toml agent-executor/Cargo.toml'
  assert_contains "$dockerfile" 'COPY agent-protocol/Cargo.toml agent-protocol/Cargo.toml'
  assert_contains "$dockerfile" 'COPY api/Cargo.toml api/Cargo.toml'
  assert_contains "$dockerfile" 'COPY container-template/Cargo.toml container-template/Cargo.toml'
  assert_contains "$dockerfile" 'COPY deploy-go-deployer/Cargo.toml deploy-go-deployer/Cargo.toml'
  assert_contains "$dockerfile" 'COPY release-authorization/Cargo.toml release-authorization/Cargo.toml'
  assert_contains "$dockerfile" 'COPY terminal-capability/Cargo.toml terminal-capability/Cargo.toml'
  assert_contains "$dockerfile" 'touch agent/src/lib.rs agent/src/main.rs'
  assert_contains "$dockerfile" 'id=deploy-go-cargo-registry'
  assert_contains "$dockerfile" 'id=deploy-go-cargo-git'
  assert_contains "$dockerfile" 'id=deploy-go-rust-target-${TARGETARCH}'
  assert_contains "$dockerfile" 'sharing=locked'
  assert_contains "$dockerfile" 'cargo fetch --locked'
  assert_line_before "$dockerfile" 'cargo fetch --locked' 'COPY agent/src'
  assert_line_before "$dockerfile" 'cargo fetch --locked' 'cargo build --locked --release'
  assert_not_contains "$dockerfile" 'COPY agent agent'
  assert_not_contains "$dockerfile" 'COPY agent-executor agent-executor'
  assert_not_contains "$dockerfile" 'COPY agent-protocol agent-protocol'
  assert_not_contains "$dockerfile" 'COPY container-template container-template'
  assert_not_contains "$dockerfile" 'COPY deploy-go-deployer deploy-go-deployer'
  assert_not_contains "$dockerfile" 'COPY release-authorization release-authorization'
  assert_not_contains "$dockerfile" 'COPY terminal-capability terminal-capability'
done
assert_contains "$INSTALL_SCRIPT" '检测到未完成部署，请先按 runbook 恢复'
assert_contains "$INSTALL_SCRIPT" 'DEPLOY_ERROR code=%s message=%s'
if grep -F 'sync-agent-release.sh' "$INSTALL_SCRIPT" >/dev/null; then
  printf 'install.sh 不应再引用 GitHub 同步脚本\n' >&2
  exit 1
fi
if grep -F 'require_command jq' "$INSTALL_SCRIPT" >/dev/null; then
  printf 'install.sh 不应依赖服务器 jq\n' >&2
  exit 1
fi
if grep -F 'DEPLOY_GO_GITHUB_REPOSITORY' "$CAPTURE_DIR/install.env.0" >/dev/null; then
  printf 'install.env 不应再包含 GitHub 仓库配置\n' >&2
  exit 1
fi

printf '正式环境部署安全契约检查通过\n'
