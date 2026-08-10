#!/usr/bin/env bash

set -euo pipefail

assets_dir="${1:-}"
release_base_url="${2:-}"
deployer_version="${3:-}"
output_path="${4:-${assets_dir}/deploy-go-deployer-manifest.json}"

if [[ -z "$assets_dir" || -z "$release_base_url" || -z "$deployer_version" ]]; then
  printf '用法：%s <assets-dir> <release-base-url> <deployer-version> [output]\n' "$0" >&2
  exit 2
fi
if [[ ! "$release_base_url" =~ ^https:// ]] ||
  [[ ! "$deployer_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
  printf 'release base URL 或 deployer 版本无效\n' >&2
  exit 2
fi

checksum() {
  local name="$1"
  test -s "${assets_dir}/${name}"
  sha256sum "${assets_dir}/${name}" | awk '{print $1}'
}

x86_64="deploy-go-deployer-linux-x86_64"
aarch64="deploy-go-deployer-linux-aarch64"

jq -n \
  --arg version "$deployer_version" \
  --arg x86_64_url "${release_base_url}/${x86_64}" \
  --arg x86_64_sha "$(checksum "$x86_64")" \
  --arg aarch64_url "${release_base_url}/${aarch64}" \
  --arg aarch64_sha "$(checksum "$aarch64")" \
  '{
    schema_version: 1,
    deployer_version: $version,
    artifacts: [
      {component: "deployer", os: "linux", architecture: "x86_64", url: $x86_64_url, sha256: $x86_64_sha},
      {component: "deployer", os: "linux", architecture: "aarch64", url: $aarch64_url, sha256: $aarch64_sha}
    ]
  }' >"$output_path"

jq -e '
  .schema_version == 1 and
  ([.artifacts[].architecture] | sort == ["aarch64", "x86_64"]) and
  ([.artifacts[].component] | all(. == "deployer")) and
  ([.artifacts[].os] | all(. == "linux")) and
  ([.artifacts[].sha256] | all(test("^[a-f0-9]{64}$")))
' "$output_path" >/dev/null
