#!/usr/bin/env bash
set -Eeuo pipefail

readonly GUARD_SOURCE_REL='scripts/test/migration-git-guard.sh'
readonly GUARD_INSTALL_DIR_REL='deploy-go-tools/migration-git-guard/v1'
WATCHED_DIRS=(
  'api/migrations'
)
DISABLED_DIRS=()

ROOT_DIR=''
TMP_DIR=''
WATCHED_CHANGE=0
ADDED_VERSIONS=()

cleanup() {
  if [[ -n "$TMP_DIR" && -d "$TMP_DIR" ]]; then
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT INT TERM

die() {
  echo "migration Git guard: $*" >&2
  exit 1
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    die "missing required command: $1"
  fi
}

usage() {
  cat >&2 <<'USAGE'
usage: migration-git-guard.sh --staged | --worktree [base-ref] | --setup | --verify
USAGE
  exit 2
}

set_root_dir() {
  ROOT_DIR=$(git rev-parse --show-toplevel 2>/dev/null) || die 'must run inside a Git worktree'
  cd "$ROOT_DIR"
  git rev-parse --verify HEAD >/dev/null 2>&1 || die 'requires a committed HEAD baseline'
}

absolute_git_path() {
  local relative=$1 path
  path=$(git rev-parse --git-path "$relative") || die "cannot resolve Git path: $relative"
  if [[ "$path" == /* || "$path" =~ ^[A-Za-z]:/ ]]; then
    printf '%s\n' "$path"
  else
    printf '%s/%s\n' "$ROOT_DIR" "$path"
  fi
}

guard_install_dir_path() {
  absolute_git_path "$GUARD_INSTALL_DIR_REL"
}

is_under_dir() {
  local path=$1 dir=$2
  [[ "$path" == "$dir" || "$path" == "$dir/"* ]]
}

is_watched_path() {
  local path=$1 dir
  for dir in "${WATCHED_DIRS[@]}" "${DISABLED_DIRS[@]}"; do
    if is_under_dir "$path" "$dir"; then
      return 0
    fi
  done
  return 1
}

valid_directory_for_path() {
  local path=$1 dir
  for dir in "${WATCHED_DIRS[@]}"; do
    if is_under_dir "$path" "$dir"; then
      printf '%s\n' "$dir"
      return 0
    fi
  done
  return 1
}

is_disabled_path() {
  local path=$1 dir
  for dir in "${DISABLED_DIRS[@]}"; do
    if is_under_dir "$path" "$dir"; then
      return 0
    fi
  done
  return 1
}

baseline_version() {
  local dir=$1 path name version maximum=''
  while IFS= read -r path; do
    name=${path##*/}
    if [[ "$name" =~ ^([0-9]{4})_[a-z0-9]+(_[a-z0-9]+)*\.sql$ ]]; then
      version=${BASH_REMATCH[1]}
      if [[ -z "$maximum" || $((10#$version)) -gt $((10#$maximum)) ]]; then
        maximum=$version
      fi
    fi
  done < <(git ls-tree -r --name-only HEAD -- "$dir")
  printf '%s\n' "$maximum"
}

record_version() {
  local dir=$1 version=$2 existing existing_dir existing_version baseline
  baseline=$(baseline_version "$dir")
  if [[ -n "$baseline" && $((10#$version)) -le $((10#$baseline)) ]]; then
    die "new migration version ${version} must be greater than HEAD baseline ${baseline} in ${dir}"
  fi
  for existing in "${ADDED_VERSIONS[@]:-}"; do
    existing_dir=${existing%%:*}
    existing_version=${existing#*:}
    if [[ "$existing_dir" == "$dir" && "$existing_version" == "$version" ]]; then
      die "duplicate new migration version ${version} in ${dir}"
    fi
  done
  ADDED_VERSIONS+=("${dir}:${version}")
}

# Strip comments and quoted strings so DROP detection only sees executable SQL.
contains_forbidden_destructive_sql() {
  awk '
    BEGIN {
      in_block = 0
      in_single = 0
      in_double = 0
      sql = ""
    }
    function append_code(line,    i, c, n, out, line_comment) {
      out = ""
      n = length(line)
      line_comment = 0
      for (i = 1; i <= n; i++) {
        c = substr(line, i, 1)
        if (line_comment) break
        if (in_block) {
          if (c == "*" && substr(line, i + 1, 1) == "/") {
            in_block = 0
            i++
          }
          continue
        }
        if (in_single) {
          if (c == "\047") {
            if (substr(line, i + 1, 1) == "\047") i++
            else in_single = 0
          }
          continue
        }
        if (in_double) {
          if (c == "\"") {
            if (substr(line, i + 1, 1) == "\"") i++
            else in_double = 0
          }
          continue
        }
        if (c == "-" && substr(line, i + 1, 1) == "-") {
          line_comment = 1
          break
        }
        if (c == "/" && substr(line, i + 1, 1) == "*") {
          in_block = 1
          i++
          continue
        }
        if (c == "\047") { in_single = 1; continue }
        if (c == "\"") { in_double = 1; continue }
        out = out c
      }
      return out
    }
    {
      sql = sql " " append_code($0)
    }
    END {
      gsub(/[[:space:]]+/, " ", sql)
      if (sql ~ /DROP[[:space:]]+TABLE([[:space:]]|$)/ || sql ~ /DROP[[:space:]]+COLUMN([[:space:]]|$)/) exit 0
      exit 1
    }
  '
}

validate_new_migration_sql() {
  local path=$1 source=$2
  if [[ "$source" == staged ]]; then
    if git show ":$path" | contains_forbidden_destructive_sql; then
      die "new migration must not DROP TABLE or DROP COLUMN; deprecate the object instead: $path"
    fi
  elif contains_forbidden_destructive_sql <"$path"; then
    die "new migration must not DROP TABLE or DROP COLUMN; deprecate the object instead: $path"
  fi
}

validate_new_path() {
  local path=$1 mode=$2 source=$3 dir name version suffix
  if is_disabled_path "$path"; then
    die "disabled API migration directory must stay empty: $path"
  fi
  dir=$(valid_directory_for_path "$path") || die "unexpected watched migration path: $path"
  suffix=${path#"$dir/"}
  [[ "$suffix" != */* && -n "$suffix" ]] || die "migration must be a direct file in ${dir}: $path"
  name=${path##*/}
  if [[ ! "$name" =~ ^([0-9]{4})_[a-z0-9]+(_[a-z0-9]+)*\.sql$ ]]; then
    die "invalid migration filename: $path"
  fi
  version=${BASH_REMATCH[1]}
  if [[ "$source" == staged ]]; then
    [[ "$mode" == '100644' ]] || die "new migration must use Git index mode 100644: $path (got ${mode})"
    git cat-file -e ":$path" 2>/dev/null || die "missing staged migration content: $path"
  else
    [[ -f "$path" && ! -L "$path" ]] || die "new migration must be a regular file: $path"
  fi
  record_version "$dir" "$version"
  validate_new_migration_sql "$path" "$source"
}

validate_change() {
  local source=$1 status=$2 old_path=$3 new_path=$4 path mode
  case "$status" in
    A)
      path=$new_path
      if ! is_watched_path "$path"; then
        return
      fi
      WATCHED_CHANGE=1
      if [[ "$source" == staged ]]; then
        local entry entry_count=0
        while IFS= read -r -d '' entry; do
          entry_count=$((entry_count + 1))
          mode=${entry%% *}
        done < <(git ls-files --stage -z -- "$path")
        [[ "$entry_count" -eq 1 ]] || die "new migration must have exactly one index entry: $path"
      else
        mode='100644'
      fi
      validate_new_path "$path" "$mode" "$source"
      ;;
    R*|C*)
      if is_watched_path "$old_path" || is_watched_path "$new_path"; then
        WATCHED_CHANGE=1
        die "rename and copy changes are forbidden for migrations: ${old_path} -> ${new_path}"
      fi
      ;;
    *)
      path=$new_path
      if is_watched_path "$path"; then
        WATCHED_CHANGE=1
        die "only direct new SQL migrations are allowed; rejected ${status}: ${path}"
      fi
      ;;
  esac
}

validate_staged_metadata() {
  local status old_path new_path
  WATCHED_CHANGE=0
  ADDED_VERSIONS=()
  while IFS= read -r -d '' status; do
    case "$status" in
      R*|C*)
        IFS= read -r -d '' old_path || die "malformed staged ${status} entry"
        IFS= read -r -d '' new_path || die "malformed staged ${status} entry"
        validate_change staged "$status" "$old_path" "$new_path"
        ;;
      *)
        IFS= read -r -d '' new_path || die "malformed staged ${status} entry"
        validate_change staged "$status" '' "$new_path"
        ;;
    esac
  done < <(git diff --cached --name-status --find-renames --find-copies-harder -z HEAD --)
}

validate_worktree_metadata() {
  local base_ref=$1 status old_path new_path path
  WATCHED_CHANGE=0
  ADDED_VERSIONS=()
  git rev-parse --verify "$base_ref" >/dev/null 2>&1 || die "invalid worktree baseline: $base_ref"
  while IFS= read -r -d '' status; do
    case "$status" in
      R*|C*)
        IFS= read -r -d '' old_path || die "malformed worktree ${status} entry"
        IFS= read -r -d '' new_path || die "malformed worktree ${status} entry"
        validate_change worktree "$status" "$old_path" "$new_path"
        ;;
      *)
        IFS= read -r -d '' new_path || die "malformed worktree ${status} entry"
        validate_change worktree "$status" '' "$new_path"
        ;;
    esac
  done < <(git diff --name-status --find-renames --find-copies-harder -z "$base_ref" --)
  while IFS= read -r -d '' path; do
    validate_change worktree A '' "$path"
  done < <(git ls-files --others --exclude-standard -z -- "${WATCHED_DIRS[@]}" "${DISABLED_DIRS[@]}")
}

run_staged() {
  set_root_dir
  validate_staged_metadata
  [[ "$WATCHED_CHANGE" -eq 1 ]] || exit 0
  echo 'migration Git staged guard passed.'
}

run_worktree() {
  local base_ref=${1:-HEAD}
  set_root_dir
  validate_worktree_metadata "$base_ref"
  [[ "$WATCHED_CHANGE" -eq 1 ]] || exit 0
  echo 'migration Git worktree guard passed.'
}

verify_hook_source() {
  local hook='.githooks/pre-commit' mode attr
  [[ -f "$hook" ]] || die "missing $hook"
  git cat-file -e "HEAD:${hook}" 2>/dev/null || die "$hook must be committed before setup"
  git diff --quiet HEAD -- "$hook" || die "$hook differs from committed source"
  mode=$(git ls-files --stage -- "$hook" | awk 'NR == 1 {print $1}')
  [[ "$mode" == '100755' ]] || die "$hook must have Git index mode 100755"
  attr=$(git check-attr eol -- "$hook")
  [[ "$attr" == *'eol: lf'* ]] || die "$hook must have LF eol attribute"
  if git show "HEAD:${hook}" | grep -q $'\r'; then
    die "$hook must not contain CRLF"
  fi
  bash -n "$hook"
}

verify_guard_sources_clean() {
  local source
  for source in "$GUARD_SOURCE_REL"; do
    git cat-file -e "HEAD:${source}" 2>/dev/null || die "$source must be committed before setup"
    git diff --quiet HEAD -- "$source" || die "$source differs from committed source"
  done
}

install_guard_scripts() {
  local install_dir parent_dir temp_dir
  install_dir=$(guard_install_dir_path)
  parent_dir=$(dirname "$install_dir")
  mkdir -p "$parent_dir"
  temp_dir=$(mktemp -d "$parent_dir/.migration-git-guard.XXXXXX") || die 'cannot create local guard install directory'
  if ! git show "HEAD:${GUARD_SOURCE_REL}" >"$temp_dir/migration-git-guard.sh"; then
    rm -rf "$temp_dir"
    die 'cannot materialize committed migration guard script'
  fi
  chmod 700 "$temp_dir/migration-git-guard.sh"
  printf '%s\n' "$(git rev-parse "HEAD:${GUARD_SOURCE_REL}")" >"$temp_dir/blobs"
  rm -rf "$install_dir"
  mv "$temp_dir" "$install_dir"
}

verify_installed_guard_scripts() {
  local install_dir expected actual
  install_dir=$(guard_install_dir_path)
  [[ -f "$install_dir/migration-git-guard.sh" ]] || die 'missing installed migration guard; run make setup-git-hooks or bash scripts/test/migration-git-guard.sh --setup'
  expected=$(git rev-parse "HEAD:${GUARD_SOURCE_REL}")
  actual=$(git hash-object "$install_dir/migration-git-guard.sh")
  [[ "$actual" == "$expected" ]] || die 'installed migration guard does not match committed source; run make setup-git-hooks or bash scripts/test/migration-git-guard.sh --setup'
  bash -n "$install_dir/migration-git-guard.sh"
}

setup_hooks() {
  set_root_dir
  require_cmd mktemp
  require_cmd git
  verify_hook_source
  verify_guard_sources_clean
  install_guard_scripts
  git config --local core.hooksPath .githooks
  verify_hooks
  echo 'local Git hooks configured.'
}

verify_hooks() {
  local hooks_path
  set_root_dir
  hooks_path=$(git config --local --get core.hooksPath 2>/dev/null || true)
  [[ "$hooks_path" == '.githooks' ]] || die 'core.hooksPath must be .githooks; run make setup-git-hooks or bash scripts/test/migration-git-guard.sh --setup'
  verify_hook_source
  verify_guard_sources_clean
  verify_installed_guard_scripts
  echo 'local Git hook verification passed.'
}

main() {
  [[ $# -ge 1 ]] || usage
  case "$1" in
    --staged)
      [[ $# -eq 1 ]] || usage
      run_staged
      ;;
    --worktree)
      [[ $# -le 2 ]] || usage
      run_worktree "${2:-HEAD}"
      ;;
    --setup)
      [[ $# -eq 1 ]] || usage
      setup_hooks
      ;;
    --verify)
      [[ $# -eq 1 ]] || usage
      verify_hooks
      ;;
    *)
      usage
      ;;
  esac
}

main "$@"
