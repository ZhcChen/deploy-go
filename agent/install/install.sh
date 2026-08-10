#!/usr/bin/env bash

set -euo pipefail

readonly service_name="deploy-go-agent"
readonly executor_service_name="deploy-go-agent-executor"
readonly runner_service_name="deploy-go-agent-runner"
readonly service_user="deploy-go-agent"
readonly runner_user="deploy-go-runner"
readonly root="${DEPLOY_GO_AGENT_INSTALL_ROOT:-}"
readonly bin_dir="${root}/usr/local/bin"
readonly agent_bin_path="${bin_dir}/deploy-go-agent"
readonly executor_bin_path="${bin_dir}/deploy-go-agent-executor"
readonly data_dir="${root}/var/lib/deploy-go-agent"
readonly work_root="${data_dir}/apps"
readonly secrets_root="${data_dir}/secrets"
readonly executor_data_dir="${root}/var/lib/deploy-go-agent-executor"
readonly credential_file="${data_dir}/credentials.json"
readonly config_dir="${root}/etc/deploy-go-agent"
readonly config_file="${config_dir}/config"
readonly executor_config_file="${config_dir}/executor.json"
readonly unit_dir="${root}/etc/systemd/system"
readonly agent_unit_path="${unit_dir}/deploy-go-agent.service"
readonly executor_unit_path="${unit_dir}/deploy-go-agent-executor.service"
readonly runner_unit_path="${unit_dir}/deploy-go-agent-runner.service"
readonly executor_socket_path="${root}/run/deploy-go-agent/executor.sock"
readonly runner_socket_path="${root}/run/deploy-go-agent-runner/runner.sock"

manifest_file=""
agent_binary_file=""
executor_binary_file=""
agent_unit_file=""
executor_unit_file=""
runner_unit_file=""
executor_config_template_file=""
response_file=""
rendered_executor_config_file=""
backup_dir=""
transaction_active="0"
agent_was_enabled="0"
executor_was_enabled="0"
runner_was_enabled="0"
agent_was_active="0"
executor_was_active="0"
runner_was_active="0"
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
    useradd \
      --system \
      --gid "$service_user" \
      --home-dir "$data_dir" \
      --shell /usr/sbin/nologin \
      "$service_user"
  fi
  if ! getent group "$runner_user" >/dev/null; then
    groupadd --system "$runner_user"
  fi
  if ! id "$runner_user" >/dev/null 2>&1; then
    useradd --system --gid "$runner_user" --home-dir "$data_dir" --shell /usr/sbin/nologin "$runner_user"
  fi
  usermod --append --groups "$runner_user" "$service_user"
}

service_identity() {
  if [[ -n "$root" ]]; then
    printf '%s %s\n' "${DEPLOY_GO_AGENT_TEST_UID:-1001}" "${DEPLOY_GO_AGENT_TEST_GID:-1001}"
  else
    printf '%s %s\n' "$(id -u "$service_user")" "$(id -g "$service_user")"
  fi
}

runner_identity() {
  if [[ -n "$root" ]]; then
    printf '%s %s\n' "${DEPLOY_GO_RUNNER_TEST_UID:-1002}" "${DEPLOY_GO_RUNNER_TEST_GID:-1002}"
  else
    printf '%s %s\n' "$(id -u "$runner_user")" "$(id -g "$runner_user")"
  fi
}

ensure_managed_directory() {
  local path="$1"
  local mode="$2"
  if [[ -e "$path" || -L "$path" ]]; then
    [[ -d "$path" && ! -L "$path" ]] || die "受管目录无效：$path"
    return
  fi
  install -d -m "$mode" "$path"
}

snapshot_permissions() {
  python3 - "$backup_dir/permissions.json" "$data_dir" "${data_dir}/tasks" "$work_root" "$secrets_root" <<'PY'
import json
import os
import stat
import sys

output, data_root, *trees = sys.argv[1:]
paths = [data_root]
for tree in trees:
    if not os.path.isdir(tree) or os.path.islink(tree):
        continue
    paths.append(tree)
    for current, directories, files in os.walk(tree, topdown=True, followlinks=False):
        directories[:] = [name for name in directories if not os.path.islink(os.path.join(current, name))]
        paths.extend(os.path.join(current, name) for name in directories)
        paths.extend(os.path.join(current, name) for name in files if not os.path.islink(os.path.join(current, name)))
records = []
for path in dict.fromkeys(paths):
    metadata = os.lstat(path)
    records.append({"path": path, "uid": metadata.st_uid, "gid": metadata.st_gid, "mode": stat.S_IMODE(metadata.st_mode), "directory": stat.S_ISDIR(metadata.st_mode)})
with open(output, "w") as destination:
    json.dump(records, destination, separators=(",", ":"))
PY
  chmod 0600 "$backup_dir/permissions.json"
}

set_owner() {
  local service_uid="$1"
  local service_gid="$2"
  local shared_gid="$3"
  python3 - "$data_dir" "${data_dir}/tasks" "$work_root" "$secrets_root" "$credential_file" \
    "$service_uid" "$service_gid" "$shared_gid" "$([[ -z "$root" ]] && printf 1 || printf 0)" <<'PY'
import os
import stat
import sys

data_root, tasks_root, apps_root, secrets_root, credential = sys.argv[1:6]
service_uid, service_gid, shared_gid = map(int, sys.argv[6:9])
change_owner = sys.argv[9] == "1"

def entries(root):
    yield root
    for current, directories, files in os.walk(root, topdown=True, followlinks=False):
        directories[:] = [name for name in directories if not os.path.islink(os.path.join(current, name))]
        yield from (os.path.join(current, name) for name in directories)
        yield from (os.path.join(current, name) for name in files if not os.path.islink(os.path.join(current, name)))

def apply(path, mode, uid=None, gid=None):
    metadata = os.lstat(path)
    if stat.S_ISLNK(metadata.st_mode):
        return
    if change_owner:
        os.chown(path, metadata.st_uid if uid is None else uid, metadata.st_gid if gid is None else gid)
    os.chmod(path, mode)

apply(data_root, 0o750, service_uid, shared_gid)
apply(tasks_root, 0o3710, service_uid, shared_gid)
for path in entries(tasks_root):
    if path == tasks_root:
        continue
    metadata = os.lstat(path)
    if stat.S_ISDIR(metadata.st_mode):
        apply(path, 0o3700, service_uid, shared_gid)
    else:
        mode = 0o640 if os.path.basename(path) in {"journal.json", "runner-spec.json", "git-key", "runner-launch.lock"} else 0o660
        apply(path, mode, gid=shared_gid)
for path in entries(apps_root):
    metadata = os.lstat(path)
    owner_bits = stat.S_IMODE(metadata.st_mode) & 0o700
    mode = owner_bits | (owner_bits >> 3)
    if stat.S_ISDIR(metadata.st_mode):
        mode |= 0o2000
    apply(path, mode, gid=shared_gid)
for path in entries(secrets_root):
    metadata = os.lstat(path)
    apply(path, 0o2700 if stat.S_ISDIR(metadata.st_mode) else 0o600, service_uid, service_gid)
apply(credential, 0o600, service_uid, service_gid)
PY
}

restore_permissions() {
  [[ -f "$backup_dir/permissions.json" ]] || return 0
  python3 - "$backup_dir/permissions.json" <<'PY'
import json
import os
import stat
import sys

for record in json.load(open(sys.argv[1])):
    path = record["path"]
    if not os.path.lexists(path):
        continue
    metadata = os.lstat(path)
    if stat.S_ISLNK(metadata.st_mode) or stat.S_ISDIR(metadata.st_mode) != record["directory"]:
        continue
    os.chown(path, record["uid"], record["gid"], follow_symlinks=False)
    os.chmod(path, record["mode"], follow_symlinks=False)
PY
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
  local enroll_response_file="$1"
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
    --output "$enroll_response_file" <<<"$request"
  python3 - "$enroll_response_file" "${credential_file}.new" "$DEPLOY_GO_AGENT_ID" <<'PY' || die "注册响应无效"
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

backup_path() {
  local name="$1"
  local path="$2"
  if [[ -e "$path" || -L "$path" ]]; then
    cp -a -- "$path" "$backup_dir/$name"
  else
    : >"$backup_dir/$name.absent"
  fi
}

restore_path() {
  local name="$1"
  local path="$2"
  rm -rf -- "$path"
  if [[ -e "$backup_dir/$name.absent" ]]; then
    return
  fi
  cp -a -- "$backup_dir/$name" "$path"
}

rollback_pair() {
  set +e
  service_action stop "$service_name" >/dev/null 2>&1
  service_action stop "$runner_service_name" >/dev/null 2>&1
  service_action stop "$executor_service_name" >/dev/null 2>&1
  restore_path agent_binary "$agent_bin_path"
  restore_path executor_binary "$executor_bin_path"
  restore_path agent_unit "$agent_unit_path"
  restore_path executor_unit "$executor_unit_path"
  restore_path runner_unit "$runner_unit_path"
  restore_path agent_config "$config_file"
  restore_path executor_config "$executor_config_file"
  restore_permissions
  rm -f -- "$executor_socket_path"
  rm -f -- "$runner_socket_path"
  service_action daemon-reload >/dev/null 2>&1
  if [[ "$executor_was_enabled" == "1" ]]; then
    service_action enable "$executor_service_name" >/dev/null 2>&1
  else
    service_action disable "$executor_service_name" >/dev/null 2>&1
  fi
  if [[ "$runner_was_enabled" == "1" ]]; then
    service_action enable "$runner_service_name" >/dev/null 2>&1
  else
    service_action disable "$runner_service_name" >/dev/null 2>&1
  fi
  if [[ "$agent_was_enabled" == "1" ]]; then
    service_action enable "$service_name" >/dev/null 2>&1
  else
    service_action disable "$service_name" >/dev/null 2>&1
  fi
  if [[ "$executor_was_active" == "1" && -x "$executor_bin_path" && -f "$executor_unit_path" ]]; then
    service_action restart "$executor_service_name" >/dev/null 2>&1
  fi
  if [[ "$runner_was_active" == "1" && -x "$agent_bin_path" && -f "$runner_unit_path" ]]; then
    service_action restart "$runner_service_name" >/dev/null 2>&1
  fi
  if [[ "$agent_was_active" == "1" && -x "$agent_bin_path" && -f "$agent_unit_path" ]]; then
    service_action restart "$service_name" >/dev/null 2>&1
  fi
  transaction_active="0"
  set -e
}

cleanup() {
  local status=$?
  if [[ "$status" -ne 0 && "$transaction_active" == "1" ]]; then
    rollback_pair
  fi
  rm -f -- \
    "$manifest_file" \
    "$agent_binary_file" \
    "$executor_binary_file" \
    "$agent_unit_file" \
    "$executor_unit_file" \
    "$runner_unit_file" \
    "$executor_config_template_file" \
    "$response_file" \
    "$rendered_executor_config_file" \
    "${config_file}.new" \
    "${executor_config_file}.new" \
    "${agent_unit_path}.new" \
    "${executor_unit_path}.new" \
    "${runner_unit_path}.new" \
    "${agent_bin_path}.new" \
    "${executor_bin_path}.new"
  if [[ "$transaction_active" == "0" && -n "$backup_dir" ]]; then
    rm -rf -- "$backup_dir"
  fi
  exit "$status"
}

render_executor_config() {
  local uid="$1"
  local gid="$2"
  python3 - "$executor_config_template_file" "$rendered_executor_config_file" "$uid" "$gid" <<'PY' || die "executor 配置模板无效"
import json
import os
import pwd
import re
import sys

source = open(sys.argv[1]).read()
placeholders = {
    "@DEPLOY_GO_AGENT_UID@": sys.argv[3],
    "@DEPLOY_GO_AGENT_GID@": sys.argv[4],
    "@DEPLOY_GO_NODE_ID@": os.environ["DEPLOY_GO_NODE_ID"],
    "@DEPLOY_GO_AGENT_ID@": os.environ["DEPLOY_GO_AGENT_ID"],
    "@DEPLOY_GO_TERMINAL_CAPABILITY_PUBLIC_KEY@": os.environ["DEPLOY_GO_TERMINAL_CAPABILITY_PUBLIC_KEY"],
    "@DEPLOY_GO_RELEASE_AUTHORIZATION_PUBLIC_KEY@": os.environ["DEPLOY_GO_RELEASE_AUTHORIZATION_PUBLIC_KEY"],
}
if any(source.count(key) != 1 for key in placeholders):
    raise SystemExit(1)
rendered = source
for key, value in placeholders.items():
    rendered = rendered.replace(key, value)
config = json.loads(rendered)
if set(config) != {"allowed_uid", "allowed_gid", "allowed_executable", "shell", "home", "node_id", "agent_id", "capability_public_key", "capability_replay_dir", "release_public_key", "release_jobs_dir", "release_global_storage_bytes", "release_minimum_free_bytes", "release_retention_seconds"}:
    raise SystemExit(1)
if config["allowed_uid"] != int(sys.argv[3]) or config["allowed_gid"] != int(sys.argv[4]):
    raise SystemExit(1)
if config["shell"] != "@DEPLOY_GO_ROOT_SHELL@" or config["home"] != "@DEPLOY_GO_ROOT_HOME@":
    raise SystemExit(1)
if config["allowed_executable"] != "/usr/local/bin/deploy-go-agent":
    raise SystemExit(1)
if config["node_id"] != os.environ["DEPLOY_GO_NODE_ID"] or config["agent_id"] != os.environ["DEPLOY_GO_AGENT_ID"]:
    raise SystemExit(1)
if not re.fullmatch(r"[A-Za-z0-9_-]{43}", config["capability_public_key"]):
    raise SystemExit(1)
if not re.fullmatch(r"[A-Za-z0-9_-]{43}", config["release_public_key"]):
    raise SystemExit(1)
if config["capability_replay_dir"] != "/var/lib/deploy-go-agent-executor/used-capabilities":
    raise SystemExit(1)
if config["release_jobs_dir"] != "/var/lib/deploy-go-agent-executor/release-jobs":
    raise SystemExit(1)
if config["release_global_storage_bytes"] != 2147483648 or config["release_minimum_free_bytes"] != 536870912 or config["release_retention_seconds"] != 604800:
    raise SystemExit(1)
root = pwd.getpwuid(0)
if not os.path.isabs(root.pw_shell) or not os.path.isfile(root.pw_shell) or not os.access(root.pw_shell, os.X_OK):
    raise SystemExit(1)
if not os.path.isabs(root.pw_dir) or not os.path.isdir(root.pw_dir):
    raise SystemExit(1)
config["shell"] = root.pw_shell
config["home"] = root.pw_dir
with open(sys.argv[2], "w") as output:
    json.dump(config, output, separators=(",", ":"))
    output.write("\n")
PY
}

wait_executor_ready() {
  local attempts="${DEPLOY_GO_AGENT_EXECUTOR_HEALTH_ATTEMPTS:-20}"
  local _
  for ((_=0; _<attempts; _++)); do
    if service_action is-active --quiet "$executor_service_name" \
      && [[ -S "$executor_socket_path" ]] \
      && su -s /bin/sh -c "'$agent_bin_path' executor-probe" deploy-go-agent >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  return 1
}

wait_runner_ready() {
  local attempts="${DEPLOY_GO_AGENT_RUNNER_HEALTH_ATTEMPTS:-20}"
  local expected_gid="$1"
  local metadata
  local _
  for ((_=0; _<attempts; _++)); do
    if service_action is-active --quiet "$runner_service_name" && [[ -S "$runner_socket_path" ]]; then
      metadata="$(stat -c '%a %g' "$runner_socket_path" 2>/dev/null || true)"
      if [[ "$metadata" == "660 ${expected_gid}" || (-n "$root" && "$metadata" == 660\ *) ]] \
        && su -s /bin/sh -c "'$agent_bin_path' runner-probe" deploy-go-agent >/dev/null 2>&1; then
        return 0
      fi
    fi
    sleep 1
  done
  return 1
}

uninstall_pair() {
  service_action stop "$service_name" >/dev/null 2>&1 || true
  service_action stop "$runner_service_name" >/dev/null 2>&1 || true
  service_action stop "$executor_service_name" >/dev/null 2>&1 || true
  service_action disable "$service_name" >/dev/null 2>&1 || true
  service_action disable "$runner_service_name" >/dev/null 2>&1 || true
  service_action disable "$executor_service_name" >/dev/null 2>&1 || true
  rm -f -- \
    "$agent_bin_path" \
    "$executor_bin_path" \
    "$agent_unit_path" \
    "$executor_unit_path" \
    "$runner_unit_path" \
    "$executor_config_file" \
    "$executor_socket_path"
  rm -f -- "$runner_socket_path"
  service_action daemon-reload >/dev/null 2>&1 || true
  printf 'Deploy Go Agent、runner broker 与 root executor 已卸载；节点凭证和任务数据已保留。\n'
}

main() {
  [[ "$(id -u)" -eq 0 || -n "$root" ]] || die "必须以 root 运行"
  [[ "${DEPLOY_GO_AGENT_OS:-$(uname -s)}" == "Linux" ]] || die "仅支持 Linux"
  require_command install
  if [[ -z "${DEPLOY_GO_AGENT_SYSTEMCTL:-}" ]]; then
    require_command systemctl
  fi
  if [[ "${1:-}" == "--uninstall" ]]; then
    [[ "$#" -eq 1 ]] || die "--uninstall 不接受其他参数"
    uninstall_pair
    return
  fi
  [[ "$#" -eq 0 ]] || die "未知安装参数"

  require_value DEPLOY_GO_AGENT_ID
  require_value DEPLOY_GO_NODE_ID
  require_value DEPLOY_GO_TERMINAL_CAPABILITY_PUBLIC_KEY
  require_value DEPLOY_GO_RELEASE_AUTHORIZATION_PUBLIC_KEY
  require_value DEPLOY_GO_AGENT_API_BASE_URL
  require_value DEPLOY_GO_AGENT_CONTROL_URL
  require_value DEPLOY_GO_AGENT_MANIFEST_URL
  require_command curl
  require_command python3
  require_command stat
  require_command su
  if [[ -z "$root" ]]; then
    require_command getent
    require_command groupadd
    require_command usermod
    require_command useradd
  fi

  local agent_artifact_url agent_artifact_sha executor_artifact_url executor_artifact_sha
  local agent_unit_url agent_unit_sha runner_unit_url runner_unit_sha executor_unit_url executor_unit_sha
  local executor_config_url executor_config_sha local_agent_id
  local service_uid service_gid runner_uid runner_gid
  architecture="$(normalize_architecture)"
  manifest_file="$(mktemp)"
  agent_binary_file="$(mktemp)"
  executor_binary_file="$(mktemp)"
  agent_unit_file="$(mktemp)"
  executor_unit_file="$(mktemp)"
  runner_unit_file="$(mktemp)"
  executor_config_template_file="$(mktemp)"
  response_file="$(mktemp)"
  rendered_executor_config_file="$(mktemp)"
  trap cleanup EXIT

  download "$DEPLOY_GO_AGENT_MANIFEST_URL" "$manifest_file"
  local manifest_output
  manifest_output="$(python3 - "$manifest_file" "$architecture" <<'PY'
import json
import re
import sys

manifest = json.load(open(sys.argv[1]))
architecture = sys.argv[2]
if not isinstance(manifest, dict):
    raise SystemExit(1)
version = manifest.get("agent_version")
executor_version = manifest.get("executor_version")
protocol_config = manifest.get("protocol") if isinstance(manifest.get("protocol"), dict) else {}
protocol_minimum = protocol_config.get("minimum")
protocol = protocol_config.get("maximum")
units = manifest.get("systemd_units") if isinstance(manifest.get("systemd_units"), dict) else {}
agent_unit = units.get("agent", {})
executor_unit = units.get("executor", {})
runner_unit = units.get("runner", {})
executor_config = manifest.get("executor_config", {})
artifacts = manifest.get("artifacts") if isinstance(manifest.get("artifacts"), list) else []
if not all(isinstance(item, dict) for item in [agent_unit, runner_unit, executor_unit, executor_config, *artifacts]):
    raise SystemExit(1)

def artifact(component):
    return next((item for item in artifacts if item.get("component") == component and item.get("os") == "linux" and item.get("architecture") == architecture), {})

agent = artifact("agent")
executor = artifact("executor")
values = [
    version, protocol,
    agent.get("url"), agent.get("sha256"),
    executor.get("url"), executor.get("sha256"),
    agent_unit.get("url"), agent_unit.get("sha256"),
    runner_unit.get("url"), runner_unit.get("sha256"),
    executor_unit.get("url"), executor_unit.get("sha256"),
    executor_config.get("url"), executor_config.get("sha256"),
]
artifact_keys = {(item.get("component"), item.get("os"), item.get("architecture")) for item in artifacts}
expected_keys = {
    ("agent", "linux", "x86_64"), ("agent", "linux", "aarch64"),
    ("executor", "linux", "x86_64"), ("executor", "linux", "aarch64"),
}
valid = (
    manifest.get("schema_version") == 3
    and set(manifest) == {"schema_version", "agent_version", "executor_version", "protocol", "systemd_units", "executor_config", "artifacts"}
    and isinstance(version, str) and re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?", version)
    and executor_version == version
    and set(protocol_config) == {"minimum", "maximum"}
    and isinstance(protocol_minimum, int) and not isinstance(protocol_minimum, bool)
    and isinstance(protocol, int) and not isinstance(protocol, bool)
    and protocol_minimum <= 6 <= protocol
    and set(units) == {"agent", "runner", "executor"}
    and all(set(item) == {"url", "sha256"} for item in [agent_unit, runner_unit, executor_unit, executor_config])
    and len(artifacts) == 4 and artifact_keys == expected_keys
    and all(set(item) == {"component", "os", "architecture", "url", "sha256"} for item in artifacts)
    and all(isinstance(value, str) and value.startswith("https://") and not any(character in value for character in "\r\n") for value in values[2::2])
    and all(re.fullmatch(r"[0-9a-f]{64}", value) for value in values[3::2])
)
if not valid:
    raise SystemExit(1)
print("\n".join(map(str, values)))
PY
)" || die "发布清单不兼容、未成对或缺少当前架构"
  mapfile -t manifest_values <<<"$manifest_output"
  [[ "${#manifest_values[@]}" -eq 14 ]] || die "发布清单不兼容"
  agent_version="${manifest_values[0]}"
  protocol_version="${manifest_values[1]}"
  agent_artifact_url="${manifest_values[2]}"
  agent_artifact_sha="${manifest_values[3]}"
  executor_artifact_url="${manifest_values[4]}"
  executor_artifact_sha="${manifest_values[5]}"
  agent_unit_url="${manifest_values[6]}"
  agent_unit_sha="${manifest_values[7]}"
  runner_unit_url="${manifest_values[8]}"
  runner_unit_sha="${manifest_values[9]}"
  executor_unit_url="${manifest_values[10]}"
  executor_unit_sha="${manifest_values[11]}"
  executor_config_url="${manifest_values[12]}"
  executor_config_sha="${manifest_values[13]}"

  local_agent_id="$(read_local_agent_id)"
  if [[ -n "$local_agent_id" && "$local_agent_id" != "$DEPLOY_GO_AGENT_ID" ]]; then
    die "本机已绑定其他 Agent，拒绝覆盖"
  fi
  if [[ -z "$local_agent_id" ]]; then
    require_value DEPLOY_GO_AGENT_ENROLLMENT_TOKEN
  fi

  download "$agent_artifact_url" "$agent_binary_file"
  [[ "$(sha256_file "$agent_binary_file")" == "$agent_artifact_sha" ]] ||
    die "Agent 二进制校验失败"
  download "$executor_artifact_url" "$executor_binary_file"
  [[ "$(sha256_file "$executor_binary_file")" == "$executor_artifact_sha" ]] ||
    die "executor 二进制校验失败"
  download "$agent_unit_url" "$agent_unit_file"
  [[ "$(sha256_file "$agent_unit_file")" == "$agent_unit_sha" ]] ||
    die "Agent systemd unit 校验失败"
  download "$executor_unit_url" "$executor_unit_file"
  [[ "$(sha256_file "$executor_unit_file")" == "$executor_unit_sha" ]] ||
    die "executor systemd unit 校验失败"
  download "$runner_unit_url" "$runner_unit_file"
  [[ "$(sha256_file "$runner_unit_file")" == "$runner_unit_sha" ]] ||
    die "runner systemd unit 校验失败"
  download "$executor_config_url" "$executor_config_template_file"
  [[ "$(sha256_file "$executor_config_template_file")" == "$executor_config_sha" ]] ||
    die "executor 配置模板校验失败"

  grep -Fx 'User=deploy-go-agent' "$agent_unit_file" >/dev/null || die "Agent systemd unit 无效"
  grep -Fx 'NoNewPrivileges=true' "$agent_unit_file" >/dev/null || die "Agent systemd unit 安全配置缺失"
  grep -Fx 'UMask=0007' "$agent_unit_file" >/dev/null || die "Agent systemd unit 共享文件权限无效"
  grep -Fx 'Wants=network-online.target deploy-go-agent-executor.service deploy-go-agent-runner.service' "$agent_unit_file" >/dev/null ||
    die "Agent systemd unit 缺少 executor 或 runner 依赖"
  grep -Fx 'User=root' "$executor_unit_file" >/dev/null || die "executor systemd unit 用户无效"
  grep -Fx 'User=root' "$runner_unit_file" >/dev/null || die "runner systemd unit 用户无效"
  grep -Fx 'ExecStart=/usr/local/bin/deploy-go-agent runner-service' "$runner_unit_file" >/dev/null ||
    die "runner systemd unit 启动命令无效"
  grep -Fx 'RuntimeDirectoryMode=0755' "$runner_unit_file" >/dev/null ||
    die "runner systemd unit Socket 目录权限无效"
  grep -Fx 'Delegate=yes' "$executor_unit_file" >/dev/null || die "executor systemd unit 缺少 cgroup 委派"
  grep -Fx 'KillMode=control-group' "$executor_unit_file" >/dev/null ||
    die "executor systemd unit 缺少进程组停止策略"
  if grep -Eq '^(RestrictAddressFamilies|IPAddressDeny|PrivateDevices|PrivateTmp|ProtectClock|ProtectKernelTunables|ProtectKernelModules|ProtectKernelLogs|ProtectControlGroups|ProtectHostname|RestrictSUIDSGID|LockPersonality|RestrictRealtime|SystemCallArchitectures|UMask)=' "$executor_unit_file"; then
    die "executor systemd unit 阻止完整 root 终端"
  fi
  if grep -Eq '(access_token|refresh_token|enrollment_token)=' "$agent_unit_file" "$runner_unit_file" "$executor_unit_file"; then
    die "systemd unit 包含凭证"
  fi

  install_owner
  read -r service_uid service_gid <<<"$(service_identity)"
  read -r runner_uid runner_gid <<<"$(runner_identity)"
  render_executor_config "$service_uid" "$service_gid"
  ensure_managed_directory "$data_dir" 0700
  if [[ -z "$root" ]]; then
    install -d -o root -g root -m 0700 "$executor_data_dir" "$executor_data_dir/used-capabilities"
  else
    install -d -m 0700 "$executor_data_dir" "$executor_data_dir/used-capabilities"
  fi
  ensure_managed_directory "$work_root" 0700
  ensure_managed_directory "$secrets_root" 0700
  ensure_managed_directory "${data_dir}/tasks" 0700
  install -d -m 0755 "$config_dir" "$bin_dir" "$unit_dir"
  if [[ -z "$local_agent_id" || "${DEPLOY_GO_AGENT_REBIND:-0}" == "1" ]]; then
    require_value DEPLOY_GO_AGENT_ENROLLMENT_TOKEN
    enroll "$response_file"
  fi
  backup_dir="$(mktemp -d "${data_dir}/.install-backup.XXXXXX")"
  agent_was_enabled="$(service_action is-enabled --quiet "$service_name" >/dev/null 2>&1 && printf '1' || printf '0')"
  executor_was_enabled="$(service_action is-enabled --quiet "$executor_service_name" >/dev/null 2>&1 && printf '1' || printf '0')"
  runner_was_enabled="$(service_action is-enabled --quiet "$runner_service_name" >/dev/null 2>&1 && printf '1' || printf '0')"
  agent_was_active="$(service_action is-active --quiet "$service_name" >/dev/null 2>&1 && printf '1' || printf '0')"
  executor_was_active="$(service_action is-active --quiet "$executor_service_name" >/dev/null 2>&1 && printf '1' || printf '0')"
  runner_was_active="$(service_action is-active --quiet "$runner_service_name" >/dev/null 2>&1 && printf '1' || printf '0')"
  backup_path agent_binary "$agent_bin_path"
  backup_path executor_binary "$executor_bin_path"
  backup_path agent_unit "$agent_unit_path"
  backup_path executor_unit "$executor_unit_path"
  backup_path runner_unit "$runner_unit_path"
  backup_path agent_config "$config_file"
  backup_path executor_config "$executor_config_file"
  transaction_active="1"
  if [[ "$agent_was_active" == "1" ]] \
    && (! service_action stop "$service_name" >/dev/null 2>&1 \
      || service_action is-active --quiet "$service_name" >/dev/null 2>&1); then
    die "无法停止旧 Agent，已恢复上一配对版本"
  fi
  if [[ "$runner_was_active" == "1" ]] \
    && (! service_action stop "$runner_service_name" >/dev/null 2>&1 \
      || service_action is-active --quiet "$runner_service_name" >/dev/null 2>&1); then
    die "无法停止旧 runner broker，已恢复上一配对版本"
  fi
  snapshot_permissions
  set_owner "$service_uid" "$service_gid" "$runner_gid"

  printf 'DEPLOY_GO_AGENT_CONTROL_URL=%s\nDEPLOY_GO_AGENT_DATA_DIR=%s\nDEPLOY_GO_RUNNER_SOCKET=%s\nDEPLOY_GO_RUNNER_TASK_ROOT=%s\nDEPLOY_GO_RUNNER_ALLOWED_UID=%s\nDEPLOY_GO_RUNNER_ALLOWED_GID=%s\nDEPLOY_GO_RUNNER_UID=%s\nDEPLOY_GO_RUNNER_GID=%s\n' \
    "$DEPLOY_GO_AGENT_CONTROL_URL" "/var/lib/deploy-go-agent" \
    "/run/deploy-go-agent-runner/runner.sock" "/var/lib/deploy-go-agent/tasks" \
    "$service_uid" "$service_gid" "$runner_uid" "$runner_gid" >"${config_file}.new"
  chmod 0644 "${config_file}.new"
  mv -f "${config_file}.new" "$config_file"
  install -m 0600 "$rendered_executor_config_file" "${executor_config_file}.new"
  mv -f "${executor_config_file}.new" "$executor_config_file"
  install -m 0644 "$agent_unit_file" "${agent_unit_path}.new"
  mv -f "${agent_unit_path}.new" "$agent_unit_path"
  install -m 0644 "$executor_unit_file" "${executor_unit_path}.new"
  mv -f "${executor_unit_path}.new" "$executor_unit_path"
  install -m 0644 "$runner_unit_file" "${runner_unit_path}.new"
  mv -f "${runner_unit_path}.new" "$runner_unit_path"
  install -m 0755 "$agent_binary_file" "${agent_bin_path}.new"
  mv -f "${agent_bin_path}.new" "$agent_bin_path"
  install -m 0755 "$executor_binary_file" "${executor_bin_path}.new"
  mv -f "${executor_bin_path}.new" "$executor_bin_path"

  service_action daemon-reload
  service_action enable "$executor_service_name" "$runner_service_name" "$service_name"
  service_action restart "$executor_service_name"
  if ! wait_executor_ready; then
    die "executor 健康检查失败，已恢复上一配对版本"
  fi
  service_action restart "$runner_service_name"
  if ! wait_runner_ready "$service_gid"; then
    die "runner broker 健康检查失败，已恢复上一配对版本"
  fi
  if ! service_action restart "$service_name" || ! service_action is-active --quiet "$service_name"; then
    die "Agent 健康检查失败，已恢复上一配对版本"
  fi

  transaction_active="0"
  rm -rf -- "$backup_dir"
  backup_dir=""
  printf 'Deploy Go Agent 与 root executor %s 安装完成；特权执行授权状态以管理端节点开关为准。\n' "$agent_version"
  printf '状态命令：sudo -u deploy-go-agent %s status\n' "$agent_bin_path"
  printf '诊断命令：sudo -u deploy-go-agent %s doctor\n' "$agent_bin_path"
}

main "$@"
