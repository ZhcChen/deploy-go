#!/usr/bin/env bash

set -euo pipefail

release_tag="${RELEASE_TAG:-${1:-}}"
github_repository="${GITHUB_REPOSITORY:-${2:-}}"
assets_dir="${3:-.artifacts/publish-assets}"
output_path="${4:-.artifacts/release-notes.md}"

if [[ -z "$release_tag" || -z "$github_repository" ]]; then
  echo "RELEASE_TAG and GITHUB_REPOSITORY are required" >&2
  exit 1
fi

mkdir -p "$(dirname "$output_path")"
release_base_url="https://github.com/${github_repository}/releases/download/${release_tag}"

asset_link() {
  local name="$1"
  if [[ -f "${assets_dir}/${name}" ]]; then
    printf -- '  - [%s](%s/%s)\n' "$name" "$release_base_url" "$name"
  fi
}

previous_tag="$({
  git tag --list 'v*.*.*' --sort=-version:refname |
    grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' || true
} | awk -v current="$release_tag" 'found { print; exit } $0 == current { found=1 }')"

{
  printf '## 下载\n\n'
  printf '### API\n\n'
  for arch in x86_64 arm64; do
    printf -- '- Linux `%s`\n' "$arch"
    asset_link "deploy-go-api-linux-${arch}"
    asset_link "deploy-go-api-linux-${arch}.binary.tar.gz"
    asset_link "deploy-go-api-linux-${arch}.docker.tar.gz"
    asset_link "deploy-go-api-linux-${arch}.sha256"
  done

  if [[ -n "$previous_tag" ]]; then
    printf '\n## 变更\n\n'
    printf -- '- [查看完整变更](https://github.com/%s/compare/%s...%s)\n' \
      "$github_repository" "$previous_tag" "$release_tag"
  fi
} > "$output_path"

printf 'Generated release notes at %s\n' "$output_path"
