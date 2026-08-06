#!/usr/bin/env bash

set -euo pipefail

readonly service_name="deploy-go-agent"
readonly service_user="deploy-go-agent"
readonly root="${DEPLOY_GO_AGENT_INSTALL_ROOT:-}"
readonly bin_path="${root}/usr/local/bin/deploy-go-agent"
readonly previous_path="${bin_path}.previous"
readonly data_dir="${root}/var/lib/deploy-go-agent"
readonly work_root="${data_dir}/apps"
readonly secrets_root="${data_dir}/secrets"
readonly credential_file="${data_dir}/credentials.json"
readonly config_dir="${root}/etc/deploy-go-agent"
readonly config_file="${config_dir}/config"
readonly unit_path="${root}/etc/systemd/system/deploy-go-agent.service"
manifest_file=""
binary_file=""
response_file=""
unit_file=""
agent_version=""
protocol_version=""
architecture=""

die() {
  printf '安装失败：%s\n' "$1" >&2
  exit 1
}

require_value() {
  local name="$1"
  [[ -n "${!name:-}" ]] || die "缺少 ${name}"
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "缺少命令：$1"
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    die "缺少 SHA-256 校验工具"
  fi
}

normalize_architecture() {
  case "${DEPLOY_GO_AGENT_ARCHITECTURE:-$(uname -m)}" in
    x86_64 | amd64) printf 'x86_64\n' ;;
    aarch64 | arm64) printf 'aarch64\n' ;;
    *) die "不支持的架构" ;;
  esac
}

install_owner() {
  if [[ -n "$root" ]]; then
    return
  fi
  if ! getent group "$service_user" >/dev/null; then
    groupadd --system "$service_user"
  fi
  if ! id "$service_user" >/dev/null 2>&1; then
    useradd --system --gid "$service_user" --home-dir "$data_dir" --shell /usr/sbin/nologin "$service_user"
  fi
}

set_owner() {
  if [[ -z "$root" ]]; then
    chown -R "${service_user}:${service_user}" "$data_dir"
  fi
}

service_action() {
  if [[ -n "${DEPLOY_GO_AGENT_SYSTEMCTL:-}" ]]; then
    "${DEPLOY_GO_AGENT_SYSTEMCTL}" "$@"
  else
    systemctl "$@"
  fi
}

download() {
  curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 "$1" --output "$2"
}

read_local_agent_id() {
  [[ -f "$credential_file" ]] || return 0
  jq -er '.agent_id | select(type == "string" and length > 0)' "$credential_file" 2>/dev/null ||
    die "本地凭证文件无效"
}

enroll() {
  local response_file="$1"
  local request
  request="$(jq -cn \
    --arg agent_id "$DEPLOY_GO_AGENT_ID" \
    --arg enrollment_token "$DEPLOY_GO_AGENT_ENROLLMENT_TOKEN" \
    --arg agent_version "$agent_version" \
    --arg architecture "$architecture" \
    --arg hostname "$(hostname)" \
    --argjson protocol_version "$protocol_version" \
    '{agent_id: $agent_id, enrollment_token: $enrollment_token, agent_version: $agent_version, protocol_version: $protocol_version, hostname: $hostname, os: "linux", architecture: $architecture}')"
  curl --fail --silent --show-error --proto '=https' --tlsv1.2 \
    --header 'Content-Type: application/json' \
    --data-binary @- \
    "${DEPLOY_GO_AGENT_API_BASE_URL%/}/api/v1/agent/enroll" \
    --output "$response_file" <<<"$request"
  jq -e --arg id "$DEPLOY_GO_AGENT_ID" \
    '.agent_id == $id and (.refresh_token | type == "string" and length >= 32)' \
    "$response_file" >/dev/null || die "注册响应无效"
  jq -c '{agent_id, refresh_token}' "$response_file" >"${credential_file}.new"
  chmod 0600 "${credential_file}.new"
  mv -f "${credential_file}.new" "$credential_file"
}

rollback() {
  if [[ -f "$previous_path" ]]; then
    mv -f "$previous_path" "$bin_path"
    service_action restart "$service_name" >/dev/null 2>&1 || true
  else
    rm -f "$bin_path"
    service_action stop "$service_name" >/dev/null 2>&1 || true
  fi
}

main() {
  [[ "$(id -u)" -eq 0 || -n "$root" ]] || die "必须以 root 运行"
  [[ "${DEPLOY_GO_AGENT_OS:-$(uname -s)}" == "Linux" ]] || die "仅支持 Linux"
  require_value DEPLOY_GO_AGENT_ID
  require_value DEPLOY_GO_AGENT_API_BASE_URL
  require_value DEPLOY_GO_AGENT_CONTROL_URL
  require_value DEPLOY_GO_AGENT_MANIFEST_URL
  require_command curl
  require_command jq

  local artifact_url artifact_sha local_agent_id unit_url unit_sha
  architecture="$(normalize_architecture)"
  manifest_file="$(mktemp)"
  binary_file="$(mktemp)"
  response_file="$(mktemp)"
  unit_file="$(mktemp)"
  trap 'rm -f "$manifest_file" "$binary_file" "$response_file" "$unit_file"' EXIT

  download "$DEPLOY_GO_AGENT_MANIFEST_URL" "$manifest_file"
  jq -e '.schema_version == 1 and (.agent_version | type == "string" and length > 0) and (.protocol.maximum | type == "number" and . >= 1 and floor == .)' "$manifest_file" >/dev/null ||
    die "发布清单不兼容"
  agent_version="$(jq -er '.agent_version' "$manifest_file")"
  protocol_version="$(jq -er '.protocol.maximum' "$manifest_file")"
  artifact_url="$(jq -er --arg arch "$architecture" '.artifacts[] | select(.os == "linux" and .architecture == $arch) | .url' "$manifest_file")" ||
    die "发布清单缺少当前架构"
  artifact_sha="$(jq -er --arg arch "$architecture" '.artifacts[] | select(.os == "linux" and .architecture == $arch) | .sha256' "$manifest_file")" ||
    die "发布清单缺少当前架构校验值"
  unit_url="$(jq -er '.systemd_unit.url' "$manifest_file")" || die "发布清单缺少 systemd unit"
  unit_sha="$(jq -er '.systemd_unit.sha256' "$manifest_file")" || die "发布清单缺少 systemd unit 校验值"

  local_agent_id="$(read_local_agent_id)"
  if [[ -n "$local_agent_id" && "$local_agent_id" != "$DEPLOY_GO_AGENT_ID" ]]; then
    die "本机已绑定其他 Agent，拒绝覆盖"
  fi
  if [[ -z "$local_agent_id" ]]; then
    require_value DEPLOY_GO_AGENT_ENROLLMENT_TOKEN
  fi

  download "$artifact_url" "$binary_file"
  [[ "$(sha256_file "$binary_file")" == "$artifact_sha" ]] || die "Agent 二进制校验失败"
  download "$unit_url" "$unit_file"
  [[ "$(sha256_file "$unit_file")" == "$unit_sha" ]] || die "systemd unit 校验失败"
  grep -Fx 'User=deploy-go-agent' "$unit_file" >/dev/null || die "systemd unit 无效"
  grep -Fx 'NoNewPrivileges=true' "$unit_file" >/dev/null || die "systemd unit 安全配置缺失"
  if grep -Eq '(access_token|refresh_token|enrollment_token)=' "$unit_file"; then
    die "systemd unit 包含凭证"
  fi

  install_owner
  install -d -m 0700 "$data_dir"
  install -d -m 0700 "$work_root" "$secrets_root"
  install -d -m 0755 "$config_dir" "$(dirname "$bin_path")" "$(dirname "$unit_path")"
  if [[ -z "$local_agent_id" || "${DEPLOY_GO_AGENT_REBIND:-0}" == "1" ]]; then
    require_value DEPLOY_GO_AGENT_ENROLLMENT_TOKEN
    enroll "$response_file"
  fi
  set_owner

  printf 'DEPLOY_GO_AGENT_CONTROL_URL=%s\nDEPLOY_GO_AGENT_DATA_DIR=%s\n' \
    "$DEPLOY_GO_AGENT_CONTROL_URL" "/var/lib/deploy-go-agent" >"$config_file"
  chmod 0644 "$config_file"
  install -m 0644 "$unit_file" "$unit_path"

  rm -f "$previous_path"
  if [[ -f "$bin_path" ]]; then
    mv "$bin_path" "$previous_path"
  fi
  install -m 0755 "$binary_file" "${bin_path}.new"
  mv -f "${bin_path}.new" "$bin_path"

  service_action daemon-reload
  service_action enable "$service_name"
  if ! service_action restart "$service_name" || ! service_action is-active --quiet "$service_name"; then
    rollback
    die "服务健康检查失败，已恢复上一版本"
  fi
  rm -f "$previous_path"
  printf 'Deploy Go Agent %s 安装完成。\n' "$agent_version"
}

main "$@"
