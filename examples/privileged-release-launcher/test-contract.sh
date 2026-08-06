#!/usr/bin/env bash
set -euo pipefail

launcher_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

task_root="$fixture/tasks"
state_root="$fixture/state"
staging="$task_root/deploy-1/staging"
mkdir -p "$staging/demo" "$state_root"
printf 'Deploy Go launcher demo\n' >"$fixture/message.txt"
tar -czf "$staging/demo/demo-app.tar.gz" -C "$fixture" message.txt

artifact_sha=$(sha256sum "$staging/demo/demo-app.tar.gz" | awk '{print $1}')
artifact_size=$(wc -c <"$staging/demo/demo-app.tar.gz" | tr -d '[:space:]')
cat >"$staging/deploy-go-artifact.json" <<EOF
{
  "schema_version": 1,
  "release_version": "demo-1",
  "commit_sha": "0123456789abcdef0123456789abcdef01234567",
  "artifacts": [
    {
      "module": "demo",
      "path": "demo/demo-app.tar.gz",
      "sha256": "$artifact_sha",
      "size": $artifact_size
    }
  ]
}
EOF

input="$staging/launcher-input.json"
cat >"$input" <<EOF
{
  "schema_version": 1,
  "app_id": "demo",
  "operation": "release",
  "task_id": "deploy-1",
  "module": "demo",
  "release_version": "demo-1",
  "staging_dir": "$staging"
}
EOF
chmod 0600 "$input"

run_launcher() {
  DEPLOY_GO_LAUNCHER_STATE_ROOT="$state_root" \
    DEPLOY_GO_LAUNCHER_ALLOWED_TASK_ROOT="$task_root" \
    bash "$launcher_dir/launcher.sh" --self-test --input "$1"
}

fail() {
  printf '[FAIL] %s\n' "$1" >&2
  exit 1
}

run_launcher "$input"
test "$(cat "$state_root/current/message.txt")" = "Deploy Go launcher demo" ||
  fail "合法发布未生效"
grep -Fq "task_id=deploy-1" "$state_root/audit/launcher.log" ||
  fail "缺少审计日志"

cat >"$staging/extra.json" <<EOF
{
  "schema_version": 1,
  "app_id": "demo",
  "operation": "release",
  "task_id": "deploy-1",
  "module": "demo",
  "release_version": "demo-1",
  "staging_dir": "$staging",
  "extra": true
}
EOF
if run_launcher "$staging/extra.json" >/dev/null 2>&1; then
  fail "额外字段未被拒绝"
fi

cat >"$staging/wrong-app.json" <<EOF
{
  "schema_version": 1,
  "app_id": "other",
  "operation": "release",
  "task_id": "deploy-1",
  "module": "demo",
  "release_version": "demo-1",
  "staging_dir": "$staging"
}
EOF
if run_launcher "$staging/wrong-app.json" >/dev/null 2>&1; then
  fail "未知应用未被拒绝"
fi

cat >"$staging/wrong-operation.json" <<EOF
{
  "schema_version": 1,
  "app_id": "demo",
  "operation": "delete",
  "task_id": "deploy-1",
  "module": "demo",
  "release_version": "demo-1",
  "staging_dir": "$staging"
}
EOF
if run_launcher "$staging/wrong-operation.json" >/dev/null 2>&1; then
  fail "未知操作未被拒绝"
fi

cat >"$staging/wrong-module.json" <<EOF
{
  "schema_version": 1,
  "app_id": "demo",
  "operation": "release",
  "task_id": "deploy-1",
  "module": "other",
  "release_version": "demo-1",
  "staging_dir": "$staging"
}
EOF
if run_launcher "$staging/wrong-module.json" >/dev/null 2>&1; then
  fail "未知模块未被拒绝"
fi

cat >"$staging/escape.json" <<EOF
{
  "schema_version": 1,
  "app_id": "demo",
  "operation": "release",
  "task_id": "deploy-1",
  "module": "demo",
  "release_version": "demo-1",
  "staging_dir": "$fixture/outside"
}
EOF
if run_launcher "$staging/escape.json" >/dev/null 2>&1; then
  fail "staging_dir 逃逸未被拒绝"
fi

mkdir -p "$fixture/outside"
ln -s "$fixture/outside" "$task_root/link-staging"
cat >"$staging/symlink.json" <<EOF
{
  "schema_version": 1,
  "app_id": "demo",
  "operation": "release",
  "task_id": "deploy-1",
  "module": "demo",
  "release_version": "demo-1",
  "staging_dir": "$task_root/link-staging"
}
EOF
if run_launcher "$staging/symlink.json" >/dev/null 2>&1; then
  fail "staging_dir 符号链接未被拒绝"
fi

cp "$staging/deploy-go-artifact.json" "$staging/manifest.json"
rm "$staging/deploy-go-artifact.json"
if run_launcher "$input" >/dev/null 2>&1; then
  fail "缺少 manifest 的发布错误地成功"
fi
mv "$staging/manifest.json" "$staging/deploy-go-artifact.json"

printf 'tampered\n' >>"$staging/demo/demo-app.tar.gz"
if run_launcher "$input" >/dev/null 2>&1; then
  fail "被篡改发布物未被拒绝"
fi

tar -czf "$staging/demo/demo-app.tar.gz" -C "$fixture" message.txt
artifact_sha=$(sha256sum "$staging/demo/demo-app.tar.gz" | awk '{print $1}')
artifact_size=$(wc -c <"$staging/demo/demo-app.tar.gz" | tr -d '[:space:]')
cat >"$staging/deploy-go-artifact.json" <<EOF
{
  "schema_version": 1,
  "release_version": "demo-1",
  "commit_sha": "0123456789abcdef0123456789abcdef01234567",
  "artifacts": [
    {
      "module": "demo",
      "path": "demo/demo-app.tar.gz",
      "sha256": "$artifact_sha",
      "size": $artifact_size
    }
  ]
}
EOF
rm -rf "$state_root"
mkdir -p "$state_root"

DEPLOY_GO_LAUNCHER_TEST_SLEEP_SECONDS=5 \
  DEPLOY_GO_LAUNCHER_STATE_ROOT="$state_root" \
  DEPLOY_GO_LAUNCHER_ALLOWED_TASK_ROOT="$task_root" \
  bash "$launcher_dir/launcher.sh" --self-test --input "$input" >/dev/null 2>&1 &
launcher_pid=$!
sleep 1
kill -TERM "$launcher_pid"
set +e
wait "$launcher_pid"
signal_status=$?
set -e
[[ "$signal_status" == "143" ]] || fail "SIGTERM 未按约定返回 143"

grep -Eq '^deploy-go-agent ALL=\(root\) NOPASSWD: /usr/local/sbin/deploy-go-release-launcher --input /var/lib/deploy-go-agent/apps/\*$' \
  "$launcher_dir/sudoers.example" || fail "sudoers 未限定固定 launcher 路径"
if grep -Eq 'ALL=\(ALL\)( NOPASSWD:)? ALL|/usr/bin/sudo|/bin/bash|docker' "$launcher_dir/sudoers.example"; then
  fail "sudoers 包含通用 shell 或 Docker 权限"
fi

printf '受控发布 launcher 契约测试通过\n'
