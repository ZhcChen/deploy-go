#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEPLOY_SCRIPT="$REPO_ROOT/deploy/production/deploy.sh"
INSTALL_SCRIPT="$REPO_ROOT/deploy/production/install.sh"
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
  api.sha256) printf 'abc  deploy-go-api-linux-x86_64\n' >"$output" ;;
  SHA256SUMS) printf 'abc  deploy-go-admin-web.tar.gz\n' >"$output" ;;
  *) printf 'fixture\n' >"$output" ;;
esac
EOF

cat >"$MOCK_BIN/sha256sum" <<'EOF'
#!/usr/bin/env bash
printf 'abc  %s\n' "$1"
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
    DEPLOY_RELEASE_TAG=v0.1.0 \
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
  DEPLOY_RELEASE_TAG=v0.1.0 \
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

if grep -E 'ssh .*env .*DEPLOY_GO_' "$MOCK_LOG" >/dev/null; then
  printf '部署配置不得拼入 SSH 远端 shell 命令\n' >&2
  exit 1
fi
assert_contains "$CAPTURE_DIR/install.env.0" 'DEPLOY_GO_API_BIND=127.0.0.1;touch /tmp/should-not-run'
assert_contains "$CAPTURE_DIR/install.env.0" 'DEPLOY_GO_ALLOWED_ORIGIN=https://deploy.example.test'
assert_contains "$CAPTURE_DIR/install.env.0" 'DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED=true'
assert_contains "$CAPTURE_DIR/install.env.0" 'DEPLOY_GO_ARTIFACTS_ROOT=/var/lib/deploy-go/artifacts'
assert_contains "$CAPTURE_DIR/install.env.0" 'DEPLOY_GO_AGENT_PROTOCOL_VERSION=5'

assert_contains "$DEPLOY_SCRIPT" 'REMOTE_STAGING_ROOT="/var/lib/deploy-go-installer"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_API_BIND="${DEPLOY_API_BIND:-127.0.0.1}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_WEB_BIND="${DEPLOY_WEB_BIND:-127.0.0.1}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_GO_COOKIE_SECURE="${DEPLOY_GO_COOKIE_SECURE:-true}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_GO_PUBLIC_BASE_URL="${DEPLOY_GO_PUBLIC_BASE_URL:-https://deploy.quanxinfu.com}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_GO_ALLOWED_ORIGIN="${DEPLOY_GO_ALLOWED_ORIGIN:-https://deploy.quanxinfu.com}"'
assert_contains "$DEPLOY_SCRIPT" 'DEPLOY_AGENT_SYNC="${DEPLOY_AGENT_SYNC:-1}"'
assert_contains "$DEPLOY_SCRIPT" 'build_agent_release "$LOCAL_STAGING/agent-release"'
assert_contains "$DEPLOY_SCRIPT" 'deploy-go-agent-executor-linux-$arch'
assert_contains "$DEPLOY_SCRIPT" 'deploy-go-agent-executor.service'
assert_contains "$DEPLOY_SCRIPT" 'executor.json.in'
assert_contains "$DEPLOY_SCRIPT" 'agent/release/generate-manifest.sh'
assert_contains "$DEPLOY_SCRIPT" '.protocol.minimum <= $protocol and .protocol.maximum >= $protocol'
assert_contains "$DEPLOY_SCRIPT" 'agent/docker/release/Dockerfile'
assert_contains "$REPO_ROOT/agent/docker/release/Dockerfile" \
  'COPY docs/standards/deploy-artifact-manifest.schema.json docs/standards/deploy-artifact-manifest.schema.json'
assert_contains "$REPO_ROOT/agent/docker/release/Dockerfile" 'COPY agent-executor agent-executor'
assert_contains "$REPO_ROOT/api/docker/release/Dockerfile" \
  'COPY docs/standards/deploy-artifact-manifest.schema.json docs/standards/deploy-artifact-manifest.schema.json'
assert_contains "$REPO_ROOT/api/docker/release/Dockerfile" 'COPY agent-executor agent-executor'
assert_contains "$REPO_ROOT/.dockerignore" \
  '!docs/standards/deploy-artifact-manifest.schema.json'
assert_contains "$INSTALL_SCRIPT" 'LOCK_FILE="/run/lock/deploy-go-install.lock"'
assert_contains "$INSTALL_SCRIPT" '"install_locked"'
assert_contains "$INSTALL_SCRIPT" 'install_agent_release'
assert_contains "$INSTALL_SCRIPT" 'deploy-go-agent-executor-linux-x86_64'
assert_contains "$INSTALL_SCRIPT" 'deploy-go-agent-executor-linux-aarch64'
assert_contains "$INSTALL_SCRIPT" 'deploy-go-agent-executor.service'
assert_contains "$INSTALL_SCRIPT" 'executor.json.in'
assert_contains "$INSTALL_SCRIPT" 'manifest.get("protocol", {}).get("minimum", 0) <= protocol'
assert_contains "$INSTALL_SCRIPT" 'manifest.get("protocol", {}).get("maximum", 0) >= protocol'
assert_contains "$INSTALL_SCRIPT" 'chown deploy-go:deploy-go "$MASTER_KEY_FILE"'
assert_contains "$INSTALL_SCRIPT" 'chmod 0400 "$MASTER_KEY_FILE"'
assert_contains "$INSTALL_SCRIPT" 'ReadOnlyPaths=$MASTER_KEY_FILE'
assert_contains "$INSTALL_SCRIPT" 'StateDirectoryMode=0750'
assert_contains "$INSTALL_SCRIPT" 'ReadWritePaths=$DATA_DIR'
assert_contains "$INSTALL_SCRIPT" 'DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS=$ARTIFACT_RETENTION_TTL_SECONDS'
assert_contains "$INSTALL_SCRIPT" 'restore_backup web "$WEB_DIR"'
assert_contains "$INSTALL_SCRIPT" 'curl --fail --silent --connect-timeout 1 --max-time 2'
assert_contains "$INSTALL_SCRIPT" 'rollback_armed="1"'
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
