#!/usr/bin/env bash

set -euo pipefail

assets_dir="${1:-}"
release_base_url="${2:-}"
agent_version="${3:-}"
output_path="${4:-${assets_dir}/deploy-go-agent-manifest.json}"

if [[ -z "$assets_dir" || -z "$release_base_url" || -z "$agent_version" ]]; then
  printf '用法：%s <assets-dir> <release-base-url> <agent-version> [output]\n' "$0" >&2
  exit 2
fi
if [[ ! "$release_base_url" =~ ^https:// ]] ||
  [[ ! "$agent_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
  printf 'release base URL 或 Agent 版本无效\n' >&2
  exit 2
fi

checksum() {
  local name="$1"
  test -s "${assets_dir}/${name}"
  sha256sum "${assets_dir}/${name}" | awk '{print $1}'
}

unit_name="deploy-go-agent.service"
x86_name="deploy-go-agent-linux-x86_64"
arm_name="deploy-go-agent-linux-aarch64"

jq -n \
  --arg version "$agent_version" \
  --arg unit_url "${release_base_url}/${unit_name}" \
  --arg unit_sha "$(checksum "$unit_name")" \
  --arg x86_url "${release_base_url}/${x86_name}" \
  --arg x86_sha "$(checksum "$x86_name")" \
  --arg arm_url "${release_base_url}/${arm_name}" \
  --arg arm_sha "$(checksum "$arm_name")" \
  '{
    schema_version: 1,
    agent_version: $version,
    protocol: {minimum: 1, maximum: 1},
    systemd_unit: {url: $unit_url, sha256: $unit_sha},
    artifacts: [
      {os: "linux", architecture: "x86_64", url: $x86_url, sha256: $x86_sha},
      {os: "linux", architecture: "aarch64", url: $arm_url, sha256: $arm_sha}
    ]
  }' >"$output_path"

jq -e '
  .schema_version == 1 and
  (.protocol.minimum <= .protocol.maximum) and
  ([.artifacts[].architecture] | sort == ["aarch64", "x86_64"]) and
  ([.systemd_unit.sha256, .artifacts[].sha256] | all(test("^[a-f0-9]{64}$")))
' "$output_path" >/dev/null
