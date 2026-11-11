#!/usr/bin/env bash
# 并行运行 make check 的各个检查组。
#
# 为什么不用 make -j：本机是 GNU Make 3.81，没有 --output-sync，并行输出会交错；
# 且 Cargo 步骤必须同组串行（共用 target 锁，并行只会互相等待）。
# 同一组内按顺序执行，组与组之间互不依赖、可写目录互不相交，因此并行。
# 每组输出缓冲到临时文件，完成后整体打印，结尾给出耗时与失败汇总。
#
# 用法：run-checks.sh [--serial]   --serial 顺序运行同一批组（调试用，输出直接流式打印）

set -u
cd "$(dirname "$0")/../.."

MODE=parallel
case "${1:-}" in
  "") ;;
  --serial) MODE=serial ;;
  *) printf '未知参数：%s（只支持 --serial）\n' "$1" >&2; exit 2 ;;
esac

# 组名 | 该组按顺序执行的 make 目标。
# rust 组含全部会调用 cargo 的检查；api-check 已覆盖整个 workspace 的 nextest 与 doctest，
# 因此这里不再重复 deployer、container-template 与对外 API 的 cargo test。
GROUPS_SPEC=(
  "rust|api-check api-external-openapi-check deployer-check-contract"
  "contracts|agent-install-check agent-manifest-check deploy-contract-demo-check privileged-launcher-check app-template-static external-deploy-compose-check migration-git-guard-self-test deploy-production-check dev-remote-check"
  "docs|spec-kit-verify docs-check whitespace-check client-sensitive-check"
  "web|admin-check"
  "client|api-client-check"
)

TMP="$(mktemp -d "${TMPDIR:-/tmp}/deploy-go-checks.XXXXXX")"
PIDS=()
cleanup() {
  local pid
  for pid in ${PIDS[@]+"${PIDS[@]}"}; do kill "$pid" 2>/dev/null || true; done
  rm -rf "$TMP"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

group_name() { printf '%s' "${1%%|*}"; }
group_targets() { printf '%s' "${1#*|}"; }

run_group() {
  # 在子 shell 内顺序执行一个组的目标，遇到第一个失败即停止（与 make 的 fail-fast 一致）。
  local name="$1" targets="$2" target start rc=0
  start="$(date +%s)"
  for target in $targets; do
    printf '$ make %s\n' "$target"
    make "$target" || { rc=$?; break; }
  done
  printf '%s\n' "$(( $(date +%s) - start ))" >"$TMP/$name.dur"
  printf '%s\n' "$rc" >"$TMP/$name.rc"
  return "$rc"
}

START_ALL="$(date +%s)"
FAILED=()
SUMMARY=()

if [[ "$MODE" == serial ]]; then
  for spec in "${GROUPS_SPEC[@]}"; do
    name="$(group_name "$spec")"
    printf '\n===== [%s] =====\n' "$name"
    run_group "$name" "$(group_targets "$spec")" || true
  done
else
  for spec in "${GROUPS_SPEC[@]}"; do
    name="$(group_name "$spec")"
    ( run_group "$name" "$(group_targets "$spec")" >"$TMP/$name.log" 2>&1 ) &
    PIDS+=("$!")
  done
  printf '已并行启动 %d 个检查组：' "${#GROUPS_SPEC[@]}"
  for spec in "${GROUPS_SPEC[@]}"; do printf ' %s' "$(group_name "$spec")"; done
  printf '\n'
  # 按完成顺序打印每组的完整输出（bash 3.2 没有 wait -n，用轮询）。
  PRINTED=" "
  while :; do
    pending=0
    for spec in "${GROUPS_SPEC[@]}"; do
      name="$(group_name "$spec")"
      case "$PRINTED" in *" $name "*) continue ;; esac
      if [[ -f "$TMP/$name.rc" ]]; then
        printf '\n===== [%s] %ss，%s =====\n' "$name" "$(cat "$TMP/$name.dur")" \
          "$([[ "$(cat "$TMP/$name.rc")" == 0 ]] && echo 通过 || echo 失败)"
        cat "$TMP/$name.log"
        PRINTED="$PRINTED$name "
      else
        pending=$((pending + 1))
      fi
    done
    [[ "$pending" -gt 0 ]] || break
    sleep 1
  done
  wait
fi

printf '\n===== 汇总（总墙钟 %ss，%s）=====\n' "$(( $(date +%s) - START_ALL ))" "$MODE"
for spec in "${GROUPS_SPEC[@]}"; do
  name="$(group_name "$spec")"
  rc="$(cat "$TMP/$name.rc" 2>/dev/null || echo '?')"
  printf '  %-10s %5ss  %s\n' "$name" "$(cat "$TMP/$name.dur" 2>/dev/null || echo '?')" "$([[ "$rc" == 0 ]] && echo 通过 || echo "失败（退出码 $rc）")"
  [[ "$rc" == 0 ]] || FAILED+=("$name")
done
if [[ "${#FAILED[@]}" -gt 0 ]]; then
  printf '检查失败的组：%s\n' "${FAILED[*]}" >&2
  exit 1
fi
printf '全部检查通过\n'
