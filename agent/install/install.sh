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
  python3 - "$credential_file" <<'PY' 2>/dev/null ||
import json
import sys

value = json.load(open(sys.argv[1])).get("agent_id")
if not isinstance(value, str) or not value:
    raise SystemExit(1)
print(value)
PY
    die "本地凭证文件无效"
}

enroll() {
  local response_file="$1"
  local request
  request="$(python3 - "$agent_version" "$architecture" "$protocol_version" <<'PY'
import json
import os
import socket
import sys

print(json.dumps({
    "agent_id": os.environ["DEPLOY_GO_AGENT_ID"],
    "enrollment_token": os.environ["DEPLOY_GO_AGENT_ENROLLMENT_TOKEN"],
    "agent_version": sys.argv[1],
    "protocol_version": int(sys.argv[3]),
    "hostname": socket.gethostname(),
    "os": "linux",
    "architecture": sys.argv[2],
}, separators=(",", ":")))
PY
)"
  curl --fail --silent --show-error --proto '=https' --tlsv1.2 \
    --header 'Content-Type: application/json' \
    --data-binary @- \
    "${DEPLOY_GO_AGENT_API_BASE_URL%/}/api/v1/agent/enroll" \
    --output "$response_file" <<<"$request"
  python3 - "$response_file" "${credential_file}.new" "$DEPLOY_GO_AGENT_ID" <<'PY' || die "注册响应无效"
import json
import sys

response = json.load(open(sys.argv[1]))
agent_id = response.get("agent_id")
refresh_token = response.get("refresh_token")
if agent_id != sys.argv[3] or not isinstance(refresh_token, str) or len(refresh_token) < 32:
    raise SystemExit(1)
with open(sys.argv[2], "w") as output:
    json.dump({"agent_id": agent_id, "refresh_token": refresh_token}, output, separators=(",", ":"))
    output.write("\n")
PY
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
  require_command python3

  local artifact_url artifact_sha local_agent_id unit_url unit_sha
  architecture="$(normalize_architecture)"
  manifest_file="$(mktemp)"
  binary_file="$(mktemp)"
  response_file="$(mktemp)"
  unit_file="$(mktemp)"
  trap 'rm -f "$manifest_file" "$binary_file" "$response_file" "$unit_file"' EXIT

  download "$DEPLOY_GO_AGENT_MANIFEST_URL" "$manifest_file"
  local manifest_output
  manifest_output="$(python3 - "$manifest_file" "$architecture" <<'PY'
import json
import re
import sys

manifest = json.load(open(sys.argv[1]))
architecture = sys.argv[2]
version = manifest.get("agent_version")
protocol = manifest.get("protocol", {}).get("maximum")
unit = manifest.get("systemd_unit", {})
artifact = next((item for item in manifest.get("artifacts", []) if item.get("os") == "linux" and item.get("architecture") == architecture), None)
values = [
    version,
    protocol,
    artifact.get("url") if artifact else None,
    artifact.get("sha256") if artifact else None,
    unit.get("url"),
    unit.get("sha256"),
]
valid = (
    manifest.get("schema_version") == 1
    and isinstance(version, str) and version
    and isinstance(protocol, int) and not isinstance(protocol, bool) and protocol >= 1
    and all(isinstance(value, str) and value and not any(character in value for character in "\r\n") for value in values[2:])
    and re.fullmatch(r"[0-9a-f]{64}", values[3])
    and re.fullmatch(r"[0-9a-f]{64}", values[5])
)
if not valid:
    raise SystemExit(1)
print("\n".join(map(str, values)))
PY
)" || die "发布清单不兼容或缺少当前架构"
  mapfile -t manifest_values <<<"$manifest_output"
  [[ "${#manifest_values[@]}" -eq 6 ]] || die "发布清单不兼容"
  agent_version="${manifest_values[0]}"
  protocol_version="${manifest_values[1]}"
  artifact_url="${manifest_values[2]}"
  artifact_sha="${manifest_values[3]}"
  unit_url="${manifest_values[4]}"
  unit_sha="${manifest_values[5]}"

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
