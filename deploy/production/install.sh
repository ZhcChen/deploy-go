#!/usr/bin/env bash

set -euo pipefail

die() {
  printf 'DEPLOY_ERROR code=%s message=%s\n' "${2:-install_failed}" "$1" >&2
  exit 1
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

wait_for_url() {
  local url="$1"
  local _
  for _ in {1..30}; do
    if curl --fail --silent --connect-timeout 1 --max-time 2 "$url" >/dev/null; then
      return 0
    fi
    sleep 1
  done
  die "健康检查超时：$url"
}

install_agent_release() {
  local source_dir="$STAGING_DIR/agent-release"
  local manifest_file="$source_dir/deploy-go-agent-manifest.json"
  local agent_unit_file="$source_dir/deploy-go-agent.service"
  local executor_unit_file="$source_dir/deploy-go-agent-executor.service"
  local executor_config_file="$source_dir/executor.json.in"
  local agent_x86_file="$source_dir/deploy-go-agent-linux-x86_64"
  local agent_arm_file="$source_dir/deploy-go-agent-linux-aarch64"
  local executor_x86_file="$source_dir/deploy-go-agent-executor-linux-x86_64"
  local executor_arm_file="$source_dir/deploy-go-agent-executor-linux-aarch64"
  local release_root target_dir staging_dir old_dir
  local expected_agent_unit_sha expected_executor_unit_sha expected_executor_config_sha
  local expected_agent_x86_sha expected_agent_arm_sha
  local expected_executor_x86_sha expected_executor_arm_sha

  [[ -d "$source_dir" && ! -L "$source_dir" ]] ||
    die "缺少本地构建的 Agent release 目录：$source_dir" "agent_release_invalid"
  for required_file in \
    "$manifest_file" "$agent_unit_file" "$executor_unit_file" "$executor_config_file" \
    "$agent_x86_file" "$agent_arm_file" "$executor_x86_file" "$executor_arm_file"; do
    [[ -f "$required_file" && ! -L "$required_file" ]] ||
      die "缺少 Agent release 文件：$required_file" "agent_release_invalid"
  done
  if ! python3 - "$manifest_file" "$AGENT_VERSION" "$AGENT_PROTOCOL_VERSION" <<'PY'; then
import json
import sys

manifest = json.load(open(sys.argv[1]))
protocol = int(sys.argv[3])
artifacts = {
    (item.get("component"), item.get("architecture"))
    for item in manifest.get("artifacts", [])
}
valid = (
    manifest.get("schema_version") == 2
    and manifest.get("agent_version") == sys.argv[2]
    and manifest.get("executor_version") == sys.argv[2]
    and manifest.get("protocol", {}).get("minimum", 0) <= protocol
    and manifest.get("protocol", {}).get("maximum", 0) >= protocol
    and artifacts == {
        ("agent", "x86_64"), ("agent", "aarch64"),
        ("executor", "x86_64"), ("executor", "aarch64"),
    }
)
sys.exit(0 if valid else 1)
PY
    die "Agent manifest 与目标版本不一致或协议不兼容" "agent_release_invalid"
  fi
  expected_agent_unit_sha="$(python3 -c \
    'import json,sys; print(json.load(open(sys.argv[1]))["systemd_units"]["agent"]["sha256"])' \
    "$manifest_file")"
  expected_executor_unit_sha="$(python3 -c \
    'import json,sys; print(json.load(open(sys.argv[1]))["systemd_units"]["executor"]["sha256"])' \
    "$manifest_file")"
  expected_executor_config_sha="$(python3 -c \
    'import json,sys; print(json.load(open(sys.argv[1]))["executor_config"]["sha256"])' \
    "$manifest_file")"
  expected_agent_x86_sha="$(python3 -c \
    'import json,sys; m=json.load(open(sys.argv[1])); print(next(i["sha256"] for i in m["artifacts"] if i["component"] == "agent" and i["architecture"] == "x86_64"))' \
    "$manifest_file")"
  expected_agent_arm_sha="$(python3 -c \
    'import json,sys; m=json.load(open(sys.argv[1])); print(next(i["sha256"] for i in m["artifacts"] if i["component"] == "agent" and i["architecture"] == "aarch64"))' \
    "$manifest_file")"
  expected_executor_x86_sha="$(python3 -c \
    'import json,sys; m=json.load(open(sys.argv[1])); print(next(i["sha256"] for i in m["artifacts"] if i["component"] == "executor" and i["architecture"] == "x86_64"))' \
    "$manifest_file")"
  expected_executor_arm_sha="$(python3 -c \
    'import json,sys; m=json.load(open(sys.argv[1])); print(next(i["sha256"] for i in m["artifacts"] if i["component"] == "executor" and i["architecture"] == "aarch64"))' \
    "$manifest_file")"
  [[ "$(sha256_file "$agent_unit_file")" == "$expected_agent_unit_sha" ]] ||
    die "Agent systemd unit 校验失败" "agent_release_invalid"
  [[ "$(sha256_file "$executor_unit_file")" == "$expected_executor_unit_sha" ]] ||
    die "executor systemd unit 校验失败" "agent_release_invalid"
  [[ "$(sha256_file "$executor_config_file")" == "$expected_executor_config_sha" ]] ||
    die "executor 配置模板校验失败" "agent_release_invalid"
  [[ "$(sha256_file "$agent_x86_file")" == "$expected_agent_x86_sha" ]] ||
    die "Agent x86_64 二进制校验失败" "agent_release_invalid"
  [[ "$(sha256_file "$agent_arm_file")" == "$expected_agent_arm_sha" ]] ||
    die "Agent aarch64 二进制校验失败" "agent_release_invalid"
  [[ "$(sha256_file "$executor_x86_file")" == "$expected_executor_x86_sha" ]] ||
    die "executor x86_64 二进制校验失败" "agent_release_invalid"
  [[ "$(sha256_file "$executor_arm_file")" == "$expected_executor_arm_sha" ]] ||
    die "executor aarch64 二进制校验失败" "agent_release_invalid"
  grep -Fx 'User=deploy-go-agent' "$agent_unit_file" >/dev/null ||
    die "Agent systemd unit 缺少专用用户" "agent_release_invalid"
  grep -Fx 'NoNewPrivileges=true' "$agent_unit_file" >/dev/null ||
    die "Agent systemd unit 缺少 NoNewPrivileges" "agent_release_invalid"
  grep -Fx 'User=root' "$executor_unit_file" >/dev/null ||
    die "executor systemd unit 必须以 root 运行" "agent_release_invalid"
  if grep -Eq '^(RestrictAddressFamilies|IPAddressDeny|PrivateDevices|PrivateTmp|ProtectClock|ProtectKernelTunables|ProtectKernelModules|ProtectKernelLogs|ProtectControlGroups|ProtectHostname|RestrictSUIDSGID|LockPersonality|RestrictRealtime|SystemCallArchitectures|UMask)=' "$executor_unit_file"; then
    die "executor systemd unit 阻止完整 root 终端" "agent_release_invalid"
  fi
  grep -Fq '@DEPLOY_GO_AGENT_UID@' "$executor_config_file" ||
    die "executor 配置模板缺少 UID 占位符" "agent_release_invalid"
  grep -Fq '@DEPLOY_GO_AGENT_GID@' "$executor_config_file" ||
    die "executor 配置模板缺少 GID 占位符" "agent_release_invalid"
  grep -Fq '@DEPLOY_GO_ROOT_SHELL@' "$executor_config_file" ||
    die "executor 配置模板缺少 root shell 占位符" "agent_release_invalid"
  grep -Fq '@DEPLOY_GO_ROOT_HOME@' "$executor_config_file" ||
    die "executor 配置模板缺少 root home 占位符" "agent_release_invalid"

  release_root="$DATA_DIR/agent-releases"
  target_dir="$release_root/$AGENT_VERSION"
  staging_dir="$release_root/.deploy-go-agent.$$"
  old_dir="$release_root/.deploy-go-agent.old.$$"
  rm -rf -- "$staging_dir" "$old_dir"
  mkdir -p "$staging_dir"
  cp -a "$source_dir"/. "$staging_dir/"
  chmod 0755 \
    "$staging_dir/deploy-go-agent-linux-x86_64" \
    "$staging_dir/deploy-go-agent-linux-aarch64" \
    "$staging_dir/deploy-go-agent-executor-linux-x86_64" \
    "$staging_dir/deploy-go-agent-executor-linux-aarch64"
  chmod 0644 \
    "$staging_dir/deploy-go-agent-manifest.json" \
    "$staging_dir/deploy-go-agent.service" \
    "$staging_dir/deploy-go-agent-executor.service" \
    "$staging_dir/executor.json.in"
  if [[ -e "$target_dir" || -L "$target_dir" ]]; then
    mv -- "$target_dir" "$old_dir"
  fi
  if ! mv -- "$staging_dir" "$target_dir"; then
    rm -rf -- "$staging_dir"
    if [[ -e "$old_dir" ]]; then
      mv -- "$old_dir" "$target_dir"
    fi
    die "Agent release 安装失败" "agent_release_install_failed"
  fi
  rm -rf -- "$old_dir"
  chown -R deploy-go:deploy-go "$release_root"
  echo "已安装本机构建 Agent $AGENT_VERSION"
}

[[ "$(id -u)" -eq 0 ]] || die "install.sh 必须以 root 运行"
require_command stat

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/install.env"
if [[ -e "$CONFIG_FILE" ]]; then
  [[ -f "$CONFIG_FILE" && ! -L "$CONFIG_FILE" ]] || die "安装配置必须是普通文件：$CONFIG_FILE"
  [[ "$(stat -c '%u:%a' "$CONFIG_FILE")" == "0:600" ]] || die "安装配置必须是 0600 root 所有：$CONFIG_FILE"
  declare -A seen_config=()
  while IFS='=' read -r config_key config_value || [[ -n "$config_key" ]]; do
    case "$config_key" in
      DEPLOY_GO_API_PORT|DEPLOY_GO_API_BIND|DEPLOY_GO_WEB_PORT|DEPLOY_GO_WEB_BIND|DEPLOY_GO_ALLOWED_ORIGIN|DEPLOY_GO_COOKIE_SECURE|DEPLOY_GO_MASTER_KEY_VERSION|DEPLOY_GO_PUBLIC_BASE_URL|DEPLOY_GO_AGENT_VERSION|DEPLOY_GO_AGENT_PROTOCOL_VERSION|DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED|DEPLOY_GO_ARTIFACTS_ROOT|DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES|DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES|DEPLOY_GO_ARTIFACT_MAX_FILES|DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES|DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS|DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS) ;;
      *) die "安装配置包含未知字段：$config_key" ;;
    esac
    [[ -z "${seen_config[$config_key]:-}" ]] || die "安装配置包含重复字段：$config_key"
    seen_config[$config_key]=1
    export "$config_key=$config_value"
  done <"$CONFIG_FILE"
fi

API_PORT="${DEPLOY_GO_API_PORT:-30100}"
API_BIND="${DEPLOY_GO_API_BIND:-127.0.0.1}"
WEB_PORT="${DEPLOY_GO_WEB_PORT:-30101}"
WEB_BIND="${DEPLOY_GO_WEB_BIND:-127.0.0.1}"
ALLOWED_ORIGIN="${DEPLOY_GO_ALLOWED_ORIGIN:-https://deploy.quanxinfu.com}"
COOKIE_SECURE="${DEPLOY_GO_COOKIE_SECURE:-true}"
MASTER_KEY_VERSION="${DEPLOY_GO_MASTER_KEY_VERSION:-1}"
PUBLIC_BASE_URL="${DEPLOY_GO_PUBLIC_BASE_URL:-https://deploy.quanxinfu.com}"
AGENT_VERSION="${DEPLOY_GO_AGENT_VERSION:-}"
AGENT_PROTOCOL_VERSION="${DEPLOY_GO_AGENT_PROTOCOL_VERSION:-}"
ARTIFACTS_ENABLED="${DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED:-true}"
ARTIFACTS_ROOT="${DEPLOY_GO_ARTIFACTS_ROOT:-/var/lib/deploy-go/artifacts}"
ARTIFACT_MAX_FILE_BYTES="${DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES:-536870912}"
ARTIFACT_MAX_TOTAL_BYTES="${DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES:-2147483648}"
ARTIFACT_MAX_FILES="${DEPLOY_GO_ARTIFACT_MAX_FILES:-256}"
ARTIFACT_MAX_CHUNK_BYTES="${DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES:-8388608}"
ARTIFACT_UPLOAD_TTL_SECONDS="${DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS:-1800}"
ARTIFACT_RETENTION_TTL_SECONDS="${DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS:-86400}"
STAGING_DIR="$SCRIPT_DIR"
INSTALL_DIR="/opt/deploy-go"
DATA_DIR="/var/lib/deploy-go"
LOCK_FILE="/run/lock/deploy-go-install.lock"
API_DIR="$INSTALL_DIR/api"
WEB_DIR="$INSTALL_DIR/web"
ENV_FILE="/etc/deploy-go/api.env"
MASTER_KEY_FILE="/etc/deploy-go/master.key"
TERMINAL_SIGNING_KEY_FILE="/etc/deploy-go/terminal-signing.key"

[[ "$API_PORT" =~ ^[0-9]+$ ]] || die "API 端口无效：$API_PORT"
[[ "$WEB_PORT" =~ ^[0-9]+$ ]] || die "Web 端口无效：$WEB_PORT"
[[ "$ALLOWED_ORIGIN" =~ ^https?://[^/]+$ ]] ||
  die "ALLOWED_ORIGIN 必须是 http(s) origin：$ALLOWED_ORIGIN"
[[ "$COOKIE_SECURE" == "true" || "$COOKIE_SECURE" == "false" ]] ||
  die "COOKIE_SECURE 必须为 true 或 false"
[[ "$MASTER_KEY_VERSION" =~ ^[1-9][0-9]*$ ]] ||
  die "MASTER_KEY_VERSION 必须为正整数"
if [[ -n "$PUBLIC_BASE_URL" ]]; then
  [[ "$PUBLIC_BASE_URL" =~ ^https://[^/]+/?$ ]] ||
    die "PUBLIC_BASE_URL 必须是 HTTPS origin"
fi
if [[ -n "$AGENT_VERSION" ]]; then
  [[ "$AGENT_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]] ||
    die "AGENT_VERSION 无效：$AGENT_VERSION"
  [[ "$AGENT_PROTOCOL_VERSION" =~ ^[1-9][0-9]*$ ]] ||
    die "AGENT_PROTOCOL_VERSION 必须为正整数"
fi
[[ "$ARTIFACTS_ENABLED" == "true" ]] || die "正式部署必须启用跨节点制品"
[[ "$ARTIFACTS_ROOT" == "$DATA_DIR/artifacts" ]] ||
  die "正式制品目录必须位于 $DATA_DIR/artifacts"
for artifact_limit in \
  "$ARTIFACT_MAX_FILE_BYTES" "$ARTIFACT_MAX_TOTAL_BYTES" "$ARTIFACT_MAX_FILES" \
  "$ARTIFACT_MAX_CHUNK_BYTES" "$ARTIFACT_UPLOAD_TTL_SECONDS" \
  "$ARTIFACT_RETENTION_TTL_SECONDS"; do
  [[ "$artifact_limit" =~ ^[1-9][0-9]*$ ]] || die "制品限额与 TTL 必须为正整数"
done

require_command flock
exec 9>"$LOCK_FILE"
flock -n 9 || die "已有 deploy-go 安装任务正在执行" "install_locked"

api_tmp=""
web_tmp=""
web_old=""
env_tmp=""
key_tmp=""
api_unit_tmp=""
web_unit_tmp=""
rollback_dir=""
rollback_armed="0"
deployment_committed="0"
services_touched="0"
systemd_state_touched="0"
api_was_enabled="0"
web_was_enabled="0"

restore_backup() {
  local name="$1"
  local destination="$2"
  local restore_tmp="${destination}.restore.$$"
  rm -rf -- "$restore_tmp"
  if [[ -e "$rollback_dir/$name.absent" ]]; then
    rm -rf -- "$destination"
    return 0
  fi
  cp -a -- "$rollback_dir/$name" "$restore_tmp" || return 1
  rm -rf -- "$destination" || return 1
  mv -- "$restore_tmp" "$destination"
}

cleanup() {
  local exit_status=$?
  set +e
  [[ -z "$api_tmp" ]] || rm -f -- "$api_tmp"
  [[ -z "$web_tmp" ]] || rm -rf -- "$web_tmp"
  [[ -z "$web_old" ]] || rm -rf -- "$web_old"
  [[ -z "$env_tmp" ]] || rm -f -- "$env_tmp"
  [[ -z "$key_tmp" ]] || rm -f -- "$key_tmp"
  [[ -z "$api_unit_tmp" ]] || rm -f -- "$api_unit_tmp"
  [[ -z "$web_unit_tmp" ]] || rm -f -- "$web_unit_tmp"
  local rollback_failed="0"
  if [[ "$deployment_committed" == "0" && "$rollback_armed" == "1" && -d "$rollback_dir" ]]; then
    restore_backup api "$API_DIR/deploy-go-api" || rollback_failed="1"
    restore_backup web "$WEB_DIR" || rollback_failed="1"
    restore_backup web_server "$INSTALL_DIR/web_server.py" || rollback_failed="1"
    restore_backup env "$ENV_FILE" || rollback_failed="1"
    restore_backup api_unit /etc/systemd/system/deploy-go-api.service || rollback_failed="1"
    restore_backup web_unit /etc/systemd/system/deploy-go-web.service || rollback_failed="1"
    if [[ "$api_was_enabled" == "1" ]]; then
      systemctl enable deploy-go-api >/dev/null || rollback_failed="1"
    else
      systemctl disable deploy-go-api >/dev/null 2>&1 || true
    fi
    if [[ "$web_was_enabled" == "1" ]]; then
      systemctl enable deploy-go-web >/dev/null || rollback_failed="1"
    else
      systemctl disable deploy-go-web >/dev/null 2>&1 || true
    fi
    if [[ "$systemd_state_touched" == "1" ]]; then
      systemctl daemon-reload || rollback_failed="1"
    fi
    if [[ "$services_touched" == "1" ]]; then
      systemctl restart deploy-go-api deploy-go-web || rollback_failed="1"
      systemctl is-active --quiet deploy-go-api || rollback_failed="1"
      systemctl is-active --quiet deploy-go-web || rollback_failed="1"
    fi
  fi
  if [[ "$rollback_failed" == "1" ]]; then
    printf '部署回滚失败，已保留恢复材料：%s\n' "$rollback_dir" >&2
  elif [[ -n "$rollback_dir" ]]; then
    rm -rf -- "$rollback_dir"
  fi
  exit "$exit_status"
}
trap cleanup EXIT

[[ -d "$STAGING_DIR" && ! -L "$STAGING_DIR" ]] || die "staging 必须是普通目录：$STAGING_DIR" "staging_invalid"
[[ "$(stat -c '%u' "$STAGING_DIR")" == "0" ]] || die "staging 必须归 root 所有：$STAGING_DIR" "staging_invalid"
staging_mode="$(stat -c '%a' "$STAGING_DIR")"
(( (8#$staging_mode & 8#022) == 0 )) || die "staging 不得允许 group/other 写入：$STAGING_DIR" "staging_invalid"
[[ -f "$STAGING_DIR/deploy-go-api" && ! -L "$STAGING_DIR/deploy-go-api" ]] || die "缺少安全的 API 二进制：$STAGING_DIR/deploy-go-api"
[[ -f "$STAGING_DIR/web/index.html" ]] || die "缺少 Web 构建产物：$STAGING_DIR/web/index.html"
[[ -f "$STAGING_DIR/web_server.py" && ! -L "$STAGING_DIR/web_server.py" ]] || die "缺少安全的 Web 服务脚本：$STAGING_DIR/web_server.py"
if find "$STAGING_DIR" -xdev -type l -print -quit | grep -q .; then
  die "staging 不得包含符号链接：$STAGING_DIR"
fi

require_command useradd
require_command install
require_command systemctl
require_command curl
require_command openssl
require_command python3

PYTHON_BIN="$(command -v python3)"

if ! id deploy-go >/dev/null 2>&1; then
  useradd \
    --system \
    --user-group \
    --home-dir "$DATA_DIR" \
    --shell /usr/sbin/nologin \
    --comment "Deploy Go service" \
    deploy-go
fi

install -d -m 0750 -o root -g deploy-go /etc/deploy-go
key_tmp=""
if [[ -e "$MASTER_KEY_FILE" || -L "$MASTER_KEY_FILE" ]]; then
  [[ -f "$MASTER_KEY_FILE" && ! -L "$MASTER_KEY_FILE" ]] || die "主密钥必须是普通文件：$MASTER_KEY_FILE" "master_key_invalid"
  [[ -s "$MASTER_KEY_FILE" ]] || die "主密钥文件为空，拒绝自动覆盖：$MASTER_KEY_FILE" "master_key_invalid"
else
  umask 077
  key_tmp="$(mktemp /etc/deploy-go/.master.key.XXXXXX)"
  openssl rand -base64 32 >"$key_tmp"
  chown deploy-go:deploy-go "$key_tmp"
  chmod 0400 "$key_tmp"
  mv -- "$key_tmp" "$MASTER_KEY_FILE"
  key_tmp=""
  echo "已生成主密钥文件：$MASTER_KEY_FILE"
fi
chown deploy-go:deploy-go "$MASTER_KEY_FILE"
chmod 0400 "$MASTER_KEY_FILE"

if [[ -e "$TERMINAL_SIGNING_KEY_FILE" || -L "$TERMINAL_SIGNING_KEY_FILE" ]]; then
  [[ -f "$TERMINAL_SIGNING_KEY_FILE" && ! -L "$TERMINAL_SIGNING_KEY_FILE" ]] ||
    die "终端签名密钥必须是普通文件：$TERMINAL_SIGNING_KEY_FILE" "terminal_signing_key_invalid"
  [[ -s "$TERMINAL_SIGNING_KEY_FILE" ]] ||
    die "终端签名密钥为空，拒绝自动覆盖：$TERMINAL_SIGNING_KEY_FILE" "terminal_signing_key_invalid"
else
  umask 077
  key_tmp="$(mktemp /etc/deploy-go/.terminal-signing.key.XXXXXX)"
  openssl rand -base64 32 >"$key_tmp"
  chown root:deploy-go "$key_tmp"
  chmod 0440 "$key_tmp"
  mv -- "$key_tmp" "$TERMINAL_SIGNING_KEY_FILE"
  key_tmp=""
  echo "已生成终端 capability 签名密钥：$TERMINAL_SIGNING_KEY_FILE"
fi
chown root:deploy-go "$TERMINAL_SIGNING_KEY_FILE"
chmod 0440 "$TERMINAL_SIGNING_KEY_FILE"

for managed_dir in "$INSTALL_DIR" "$API_DIR" "$WEB_DIR" "$DATA_DIR"; do
  [[ ! -L "$managed_dir" ]] || die "受管目录不得是符号链接：$managed_dir"
done

shopt -s nullglob
unfinished_rollbacks=("$INSTALL_DIR"/.rollback.*)
shopt -u nullglob
if ((${#unfinished_rollbacks[@]} > 0)); then
  die "检测到未完成部署，请先按 runbook 恢复：${unfinished_rollbacks[0]}" "rollback_required"
fi

install -d -m 0750 -o root -g deploy-go "$INSTALL_DIR" "$API_DIR" "$WEB_DIR"
install -d -m 0750 -o deploy-go -g deploy-go "$DATA_DIR" "$DATA_DIR/agent-releases"
chown -R deploy-go:deploy-go "$DATA_DIR"

rollback_dir="$(mktemp -d "$INSTALL_DIR/.rollback.XXXXXX")"
api_was_enabled="$(systemctl is-enabled --quiet deploy-go-api && printf '1' || printf '0')"
web_was_enabled="$(systemctl is-enabled --quiet deploy-go-web && printf '1' || printf '0')"
for backup_spec in \
  "api:$API_DIR/deploy-go-api" \
  "web:$WEB_DIR" \
  "web_server:$INSTALL_DIR/web_server.py" \
  "env:$ENV_FILE" \
  "api_unit:/etc/systemd/system/deploy-go-api.service" \
  "web_unit:/etc/systemd/system/deploy-go-web.service"; do
  backup_name="${backup_spec%%:*}"
  backup_source="${backup_spec#*:}"
  if [[ -e "$backup_source" || -L "$backup_source" ]]; then
    cp -a -- "$backup_source" "$rollback_dir/$backup_name"
  else
    : >"$rollback_dir/$backup_name.absent"
  fi
done
rollback_armed="1"

api_tmp="$(mktemp "$API_DIR/.deploy-go-api.XXXXXX")"
web_tmp="$(mktemp -d "$INSTALL_DIR/.web.XXXXXX")"

install -m 0550 -o root -g deploy-go "$STAGING_DIR/deploy-go-api" "$api_tmp"
mv -f -- "$api_tmp" "$API_DIR/deploy-go-api"
api_tmp=""

cp -a "$STAGING_DIR/web/." "$web_tmp/"
chown -R root:deploy-go "$web_tmp"
find "$web_tmp" -type d -exec chmod 0750 {} +
find "$web_tmp" -type f -exec chmod 0640 {} +
if [[ -d "$WEB_DIR" ]]; then
  web_old="$(mktemp -d "$INSTALL_DIR/.web-old.XXXXXX")"
  rmdir "$web_old"
  mv -- "$WEB_DIR" "$web_old"
fi
mv -- "$web_tmp" "$WEB_DIR"
web_tmp=""
if [[ -n "$web_old" ]]; then
  rm -rf -- "$web_old"
  web_old=""
fi

install -m 0550 -o root -g deploy-go "$STAGING_DIR/web_server.py" "$INSTALL_DIR/web_server.py"

env_tmp="$ENV_FILE.new.$$"
if [[ -f "$ENV_FILE" ]]; then
  grep -vE '^(DEPLOY_GO_BIND_ADDR|DEPLOY_GO_DATABASE_URL|DEPLOY_GO_ALLOWED_ORIGIN|DEPLOY_GO_ALLOWED_ORIGINS|DEPLOY_GO_COOKIE_SECURE|DEPLOY_GO_MASTER_KEY_VERSION|DEPLOY_GO_MASTER_KEY|DEPLOY_GO_MASTER_KEY_FILE|DEPLOY_GO_TERMINAL_SIGNING_KEY_FILE|DEPLOY_GO_PUBLIC_BASE_URL|DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED|DEPLOY_GO_ARTIFACTS_ROOT|DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES|DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES|DEPLOY_GO_ARTIFACT_MAX_FILES|DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES|DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS|DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS|RUST_LOG)=' \
    "$ENV_FILE" >"$env_tmp" || true
else
  : >"$env_tmp"
fi
{
  echo "DEPLOY_GO_BIND_ADDR=$API_BIND:$API_PORT"
  echo "DEPLOY_GO_DATABASE_URL=sqlite://$DATA_DIR/deploy-go.db"
  echo "DEPLOY_GO_ALLOWED_ORIGIN=$ALLOWED_ORIGIN"
  echo "DEPLOY_GO_COOKIE_SECURE=$COOKIE_SECURE"
  echo "DEPLOY_GO_MASTER_KEY_VERSION=$MASTER_KEY_VERSION"
  echo "DEPLOY_GO_MASTER_KEY_FILE=$MASTER_KEY_FILE"
  echo "DEPLOY_GO_TERMINAL_SIGNING_KEY_FILE=$TERMINAL_SIGNING_KEY_FILE"
  if [[ -n "$PUBLIC_BASE_URL" ]]; then
    echo "DEPLOY_GO_PUBLIC_BASE_URL=$PUBLIC_BASE_URL"
  fi
  echo "DEPLOY_GO_CROSS_NODE_ARTIFACTS_ENABLED=$ARTIFACTS_ENABLED"
  echo "DEPLOY_GO_ARTIFACTS_ROOT=$ARTIFACTS_ROOT"
  echo "DEPLOY_GO_ARTIFACT_MAX_FILE_BYTES=$ARTIFACT_MAX_FILE_BYTES"
  echo "DEPLOY_GO_ARTIFACT_MAX_TOTAL_BYTES=$ARTIFACT_MAX_TOTAL_BYTES"
  echo "DEPLOY_GO_ARTIFACT_MAX_FILES=$ARTIFACT_MAX_FILES"
  echo "DEPLOY_GO_ARTIFACT_MAX_CHUNK_BYTES=$ARTIFACT_MAX_CHUNK_BYTES"
  echo "DEPLOY_GO_ARTIFACT_UPLOAD_TTL_SECONDS=$ARTIFACT_UPLOAD_TTL_SECONDS"
  echo "DEPLOY_GO_ARTIFACT_RETENTION_TTL_SECONDS=$ARTIFACT_RETENTION_TTL_SECONDS"
  echo "RUST_LOG=info"
} >>"$env_tmp"
chmod 0600 "$env_tmp"
chown root:root "$env_tmp"
mv -f "$env_tmp" "$ENV_FILE"

api_unit_tmp="$(mktemp /etc/systemd/system/.deploy-go-api.service.XXXXXX)"
cat >"$api_unit_tmp" <<EOF
[Unit]
Description=Deploy Go API
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=deploy-go
Group=deploy-go
WorkingDirectory=$API_DIR
EnvironmentFile=$ENV_FILE
ExecStart=$API_DIR/deploy-go-api
Restart=on-failure
RestartSec=3
LimitNOFILE=65536
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadOnlyPaths=$MASTER_KEY_FILE
ReadOnlyPaths=$TERMINAL_SIGNING_KEY_FILE
StateDirectory=deploy-go
StateDirectoryMode=0750
ReadWritePaths=$DATA_DIR
RestrictSUIDSGID=true
LockPersonality=true
RestrictRealtime=true

[Install]
WantedBy=multi-user.target
EOF
chmod 0644 "$api_unit_tmp"
chown root:root "$api_unit_tmp"
mv -f -- "$api_unit_tmp" /etc/systemd/system/deploy-go-api.service
api_unit_tmp=""

web_unit_tmp="$(mktemp /etc/systemd/system/.deploy-go-web.service.XXXXXX)"
cat >"$web_unit_tmp" <<EOF
[Unit]
Description=Deploy Go Web
After=network-online.target deploy-go-api.service
Wants=network-online.target

[Service]
Type=simple
User=deploy-go
Group=deploy-go
WorkingDirectory=$WEB_DIR
ExecStart=$PYTHON_BIN $INSTALL_DIR/web_server.py --root $WEB_DIR --api http://127.0.0.1:$API_PORT --bind $WEB_BIND --port $WEB_PORT
Restart=on-failure
RestartSec=3
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true

[Install]
WantedBy=multi-user.target
EOF
chmod 0644 "$web_unit_tmp"
chown root:root "$web_unit_tmp"
mv -f -- "$web_unit_tmp" /etc/systemd/system/deploy-go-web.service
web_unit_tmp=""

if [[ -n "$AGENT_VERSION" ]]; then
  install_agent_release
fi

systemd_state_touched="1"
systemctl daemon-reload
systemctl enable deploy-go-api deploy-go-web >/dev/null
services_touched="1"
systemctl restart deploy-go-api
systemctl restart deploy-go-web

for service in deploy-go-api deploy-go-web; do
  systemctl is-active --quiet "$service" || die "systemd 服务未运行：$service"
done

wait_for_url "http://127.0.0.1:$API_PORT/healthz"
wait_for_url "http://127.0.0.1:$API_PORT/readyz"
wait_for_url "http://127.0.0.1:$WEB_PORT/"
wait_for_url "http://127.0.0.1:$WEB_PORT/api/v1/openapi.json"

deployment_committed="1"
rm -rf -- "$rollback_dir"
rollback_dir=""

printf '部署完成：API http://127.0.0.1:%s，Web http://127.0.0.1:%s\n' \
  "$API_PORT" "$WEB_PORT"
