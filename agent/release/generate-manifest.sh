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

protocol_minimum="$(sed -n 's/^pub const MIN_SUPPORTED_PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-protocol/src/lib.rs | head -n 1)"
protocol_maximum="$(sed -n 's/^pub const PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-protocol/src/lib.rs | head -n 1)"
[[ "$protocol_minimum" =~ ^[1-9][0-9]*$ && "$protocol_maximum" =~ ^[1-9][0-9]*$ ]] || {
  printf '无法读取 Agent 协议版本\n' >&2
  exit 2
}

checksum() {
  local name="$1"
  test -s "${assets_dir}/${name}"
  sha256sum "${assets_dir}/${name}" | awk '{print $1}'
}

agent_unit="deploy-go-agent.service"
runner_unit="deploy-go-agent-runner.service"
executor_unit="deploy-go-agent-executor.service"
executor_config="executor.json.in"
agent_x86="deploy-go-agent-linux-x86_64"
agent_arm="deploy-go-agent-linux-aarch64"
executor_x86="deploy-go-agent-executor-linux-x86_64"
executor_arm="deploy-go-agent-executor-linux-aarch64"

jq -n \
  --arg version "$agent_version" \
  --argjson protocol_minimum "$protocol_minimum" \
  --argjson protocol_maximum "$protocol_maximum" \
  --arg agent_unit_url "${release_base_url}/${agent_unit}" \
  --arg agent_unit_sha "$(checksum "$agent_unit")" \
  --arg runner_unit_url "${release_base_url}/${runner_unit}" \
  --arg runner_unit_sha "$(checksum "$runner_unit")" \
  --arg executor_unit_url "${release_base_url}/${executor_unit}" \
  --arg executor_unit_sha "$(checksum "$executor_unit")" \
  --arg executor_config_url "${release_base_url}/${executor_config}" \
  --arg executor_config_sha "$(checksum "$executor_config")" \
  --arg agent_x86_url "${release_base_url}/${agent_x86}" \
  --arg agent_x86_sha "$(checksum "$agent_x86")" \
  --arg agent_arm_url "${release_base_url}/${agent_arm}" \
  --arg agent_arm_sha "$(checksum "$agent_arm")" \
  --arg executor_x86_url "${release_base_url}/${executor_x86}" \
  --arg executor_x86_sha "$(checksum "$executor_x86")" \
  --arg executor_arm_url "${release_base_url}/${executor_arm}" \
  --arg executor_arm_sha "$(checksum "$executor_arm")" \
  '{
    schema_version: 3,
    agent_version: $version,
    executor_version: $version,
    runner_protocol: 1,
    executor_protocol: 2,
    protocol: {minimum: $protocol_minimum, maximum: $protocol_maximum},
    systemd_units: {
      agent: {url: $agent_unit_url, sha256: $agent_unit_sha},
      runner: {url: $runner_unit_url, sha256: $runner_unit_sha},
      executor: {url: $executor_unit_url, sha256: $executor_unit_sha}
    },
    executor_config: {url: $executor_config_url, sha256: $executor_config_sha},
    artifacts: [
      {component: "agent", os: "linux", architecture: "x86_64", url: $agent_x86_url, sha256: $agent_x86_sha},
      {component: "agent", os: "linux", architecture: "aarch64", url: $agent_arm_url, sha256: $agent_arm_sha},
      {component: "executor", os: "linux", architecture: "x86_64", url: $executor_x86_url, sha256: $executor_x86_sha},
      {component: "executor", os: "linux", architecture: "aarch64", url: $executor_arm_url, sha256: $executor_arm_sha}
    ]
  }' >"$output_path"

jq -e '
  .schema_version == 3 and
  (.systemd_units | keys | sort == ["agent", "executor", "runner"]) and
  .agent_version == .executor_version and
  .runner_protocol == 1 and
  .executor_protocol == 2 and
  (.protocol.minimum <= .protocol.maximum) and
  ([.artifacts[] | select(.component == "agent") | .architecture] | sort == ["aarch64", "x86_64"]) and
  ([.artifacts[] | select(.component == "executor") | .architecture] | sort == ["aarch64", "x86_64"]) and
  ([.systemd_units[].sha256, .executor_config.sha256, .artifacts[].sha256] | all(test("^[a-f0-9]{64}$")))
' "$output_path" >/dev/null
