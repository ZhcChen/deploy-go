#!/usr/bin/env bash
set -Eeuo pipefail

launcher_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
entrypoint="$launcher_dir/release-entry.sh"

state_root=${DEPLOY_GO_LAUNCHER_STATE_ROOT:-/var/lib/deploy-go-launcher/demo}
allowed_task_root=${DEPLOY_GO_LAUNCHER_ALLOWED_TASK_ROOT:-/var/lib/deploy-go-agent}
self_test=0
input_file=""
child_pid=""

die() {
  printf 'launcher: %s\n' "$1" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Usage:
  deploy-go-release-launcher --input <file> [--self-test]
EOF
}

canonicalize() {
  local path=$1
  if command -v realpath >/dev/null 2>&1; then
    realpath "$path"
    return
  fi
  python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$path"
}

is_under() {
  local path=$1
  local root=$2
  [[ "$path" == "$root" || "$path" == "$root/"* ]]
}

validate_staging_dir() {
  local staging=$1
  local resolved=""

  [[ "$staging" == /* && "$staging" != *".."* ]] || die "staging_dir 必须是绝对路径且不能包含 .."
  [[ -d "$staging" && ! -L "$staging" ]] || die "staging_dir 不存在或为符号链接"
  resolved=$(canonicalize "$staging")
  is_under "$resolved" "$(canonicalize "$allowed_task_root")" || die "staging_dir 不在允许的任务根内"
}

validate_input() {
  local input=$1
  local staging=""
  local app_id=""
  local operation=""
  local module=""
  local task_id=""
  local release_version=""

  [[ -f "$input" && -r "$input" ]] || die "输入文件不存在或不可读"
  command -v jq >/dev/null 2>&1 || die "缺少 jq"
  is_under "$(canonicalize "$input")" "$(canonicalize "$allowed_task_root")" || die "输入文件不在允许的任务根内"

  jq -e '
    type == "object" and
    (keys | sort) == ["app_id","module","operation","release_version","schema_version","staging_dir","task_id"] and
    .schema_version == 1 and
    (.app_id | type == "string") and
    (.module | type == "string") and
    (.operation | type == "string") and
    (.task_id | type == "string" and (length >= 1 and length <= 128) and test("^[A-Za-z0-9._-]+$")) and
    (.release_version | type == "string" and (length >= 1 and length <= 256) and test("^[A-Za-z0-9._-]+$")) and
    (.staging_dir | type == "string")
  ' "$input" >/dev/null || die "输入 JSON 格式或字段不合法"

  app_id=$(jq -er '.app_id' "$input")
  operation=$(jq -er '.operation' "$input")
  module=$(jq -er '.module' "$input")
  task_id=$(jq -er '.task_id' "$input")
  release_version=$(jq -er '.release_version' "$input")
  staging=$(jq -er '.staging_dir' "$input")

  [[ "$app_id" == "demo" ]] || die "未知应用: $app_id"
  [[ "$operation" == "release" ]] || die "未知操作: $operation"
  [[ "$module" == "demo" ]] || die "未知模块: $module"
  [[ "$task_id" =~ ^[A-Za-z0-9._-]+$ && ${#task_id} -ge 1 && ${#task_id} -le 128 ]] || die "task_id 不合法"
  [[ "$release_version" =~ ^[A-Za-z0-9._-]+$ && ${#release_version} -ge 1 && ${#release_version} -le 256 ]] || die "release_version 不合法"
  validate_staging_dir "$staging"
}

forward_signal() {
  if [[ -n "$child_pid" ]] && kill -0 "$child_pid" >/dev/null 2>&1; then
    kill -TERM "$child_pid" >/dev/null 2>&1 || true
  fi
  exit 143
}

run_release() {
  local input=$1
  local staging=""
  local task_id=""
  local release_version=""
  local module=""
  local status=0

  staging=$(jq -er '.staging_dir' "$input")
  task_id=$(jq -er '.task_id' "$input")
  release_version=$(jq -er '.release_version' "$input")
  module=$(jq -er '.module' "$input")

  mkdir -p "$state_root/audit"
  printf '%s app_id=demo task_id=%s module=%s release=%s staging=%s\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$task_id" "$module" "$release_version" "$staging" \
    >>"$state_root/audit/launcher.log"

  trap forward_signal TERM INT

  env -i \
    PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    HOME=/root \
    LANG=C.UTF-8 \
    DEPLOY_GO_DEMO_RELEASE_ROOT="$state_root" \
    DEPLOY_GO_LAUNCHER_SELF_TEST="$self_test" \
    DEPLOY_GO_LAUNCHER_TEST_SLEEP_SECONDS="${DEPLOY_GO_LAUNCHER_TEST_SLEEP_SECONDS:-0}" \
    "$entrypoint" "$staging" "$release_version" "$module" &
  child_pid=$!

  set +e
  wait "$child_pid"
  status=$?
  set -e
  child_pid=""
  trap - TERM INT
  return "$status"
}

main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --input)
        shift
        input_file=${1:-}
        ;;
      --self-test)
        self_test=1
        ;;
      --help|-h)
        usage
        exit 0
        ;;
      *)
        die "不支持的参数: $1"
        ;;
    esac
    shift || true
  done

  [[ -n "$input_file" ]] || die "缺少 --input"
  if [[ "$self_test" == "0" ]] && [[ "$(id -u)" != "0" ]]; then
    die "launcher 必须以 root 运行"
  fi

  validate_input "$input_file"
  run_release "$input_file"
}

main "$@"
