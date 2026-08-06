#!/usr/bin/env bash
set -euo pipefail

staging_dir=$1
release_version=$2
module=$3
state_root=${DEPLOY_GO_DEMO_RELEASE_ROOT:?DEPLOY_GO_DEMO_RELEASE_ROOT is required}

[[ "$module" == "demo" ]] || {
  printf 'unknown module: %s\n' "$module" >&2
  exit 2
}
[[ "$release_version" =~ ^[A-Za-z0-9._-]+$ && ${#release_version} -le 256 ]] || {
  printf 'invalid release version\n' >&2
  exit 2
}

manifest="$staging_dir/deploy-go-artifact.json"
[[ -f "$manifest" ]] || {
  printf 'missing artifact manifest\n' >&2
  exit 2
}

artifact_relative=$(jq -er '.artifacts[0].path' "$manifest")
artifact_sha=$(jq -er '.artifacts[0].sha256' "$manifest")
artifact_size=$(jq -er '.artifacts[0].size' "$manifest")
[[ "$artifact_relative" == "demo/demo-app.tar.gz" ]] || {
  printf 'invalid artifact path\n' >&2
  exit 2
}

artifact="$staging_dir/$artifact_relative"
[[ -f "$artifact" && ! -L "$artifact" ]] || {
  printf 'artifact is missing or a symlink\n' >&2
  exit 2
}

actual_sha=$(sha256sum "$artifact" | awk '{print $1}')
actual_size=$(wc -c <"$artifact" | tr -d '[:space:]')
[[ "$actual_sha" == "$artifact_sha" && "$actual_size" == "$artifact_size" ]] || {
  printf 'artifact checksum mismatch\n' >&2
  exit 2
}

release_dir="$state_root/releases/$release_version"
mkdir -p "$release_dir"
tar -xzf "$artifact" -C "$release_dir"
ln -sfn "$release_dir" "$state_root/current.next"
mv -f "$state_root/current.next" "$state_root/current"

[[ "$(cat "$state_root/current/message.txt")" == "Deploy Go launcher demo" ]] || {
  printf 'release verification failed\n' >&2
  exit 1
}

if [[ "${DEPLOY_GO_LAUNCHER_SELF_TEST:-0}" == "1" ]]; then
  sleep "${DEPLOY_GO_LAUNCHER_TEST_SLEEP_SECONDS:-0}"
fi

printf 'released demo %s from %s\n' "$release_version" "$staging_dir"
