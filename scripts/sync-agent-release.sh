#!/usr/bin/env bash

set -euo pipefail

github_repository="${DEPLOY_GO_GITHUB_REPOSITORY:-ZhcChen/deploy-go}"
release_dir="/var/lib/deploy-go/agent-releases"
version="${DEPLOY_GO_AGENT_VERSION:-}"
base_url="${DEPLOY_GO_AGENT_RELEASE_BASE_URL:-}"
protocol_version="$(sed -n 's/^pub const PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' agent-protocol/src/lib.rs 2>/dev/null | head -n 1)"
allow_http=0

while (($#)); do
  case "$1" in
    --release-dir)
      # 仅供测试隔离使用；生产目录固定为 /var/lib/deploy-go/agent-releases
      release_dir="${2:-}"
      shift 2
      ;;
    --version)
      version="${2:-}"
      shift 2
      ;;
    --repository)
      github_repository="${2:-}"
      shift 2
      ;;
    --base-url)
      base_url="${2:-}"
      shift 2
      ;;
    --allow-http)
      allow_http=1
      shift
      ;;
    *)
      printf '未知参数：%s\n' "$1" >&2
      exit 2
      ;;
  esac
done

die() {
  printf '同步失败：%s\n' "$1" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "缺少命令：$1"
}

download() {
  local url="$1"
  local output="$2"
  if [[ "$allow_http" == "1" ]]; then
    curl --fail --silent --show-error --location --retry 3 "$url" --output "$output"
  else
    curl --fail --silent --show-error --location --retry 3 \
      --proto '=https' --tlsv1.2 "$url" --output "$output"
  fi
}

sha256_file() {
  sha256sum "$1" | awk '{print $1}'
}

manifest_version_matches() {
  local manifest="$1"
  jq -e \
    --arg version "$version" \
    --argjson protocol "$protocol_version" \
    '.schema_version == 1 and .agent_version == $version and .protocol.minimum <= $protocol and .protocol.maximum >= $protocol' \
    "$manifest" >/dev/null
}

verify_sha256() {
  local file="$1"
  local expected="$2"
  [[ "$(sha256_file "$file")" == "$expected" ]] || die "$(basename "$file") 校验失败"
}

require_command curl
require_command jq
require_command sha256sum
[[ "$protocol_version" =~ ^[1-9][0-9]*$ ]] || die "无法读取当前 Agent 协议版本"
[[ -n "$release_dir" ]] || die "缺少 Agent 发布目录"
[[ "$release_dir" = /* ]] || die "Agent 发布目录必须是绝对路径"

if [[ -z "$version" ]]; then
  api_version=""
  agent_version=""
  [[ -f api/Cargo.toml ]] && api_version="$(sed -n 's/^version = "\(.*\)"/\1/p' api/Cargo.toml | head -n 1)"
  [[ -f agent/Cargo.toml ]] && agent_version="$(sed -n 's/^version = "\(.*\)"/\1/p' agent/Cargo.toml | head -n 1)"
  if [[ -n "$api_version" && -n "$agent_version" && "$api_version" != "$agent_version" ]]; then
    die "API 与 Agent 版本不一致：$api_version != $agent_version"
  fi
  version="${api_version:-$agent_version}"
fi
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]] ||
  die "Agent 版本无效：${version:-未设置}"
[[ "$github_repository" =~ ^[^/]+/[^/]+$ ]] ||
  die "GitHub 仓库格式无效：$github_repository"

if [[ -z "$base_url" ]]; then
  base_url="https://github.com/${github_repository}/releases/download/v${version}"
fi

staging_dir="${release_dir}/.deploy-go-agent-${version}.tmp.$$"
target_dir="${release_dir}/${version}"
backup_dir="${release_dir}/.deploy-go-agent-${version}.old.$$"
rm -rf "$staging_dir" "$backup_dir"
mkdir -p "$staging_dir"
trap 'rm -rf "$staging_dir" "$backup_dir"' EXIT

manifest_file="$staging_dir/deploy-go-agent-manifest.json"
x86_file="$staging_dir/deploy-go-agent-linux-x86_64"
arm_file="$staging_dir/deploy-go-agent-linux-aarch64"
unit_file="$staging_dir/deploy-go-agent.service"

download "$base_url/deploy-go-agent-manifest.json" "$manifest_file"
manifest_version_matches "$manifest_file" || die "manifest 版本与目标版本不一致"

download "$base_url/deploy-go-agent-linux-x86_64" "$x86_file"
download "$base_url/deploy-go-agent-linux-aarch64" "$arm_file"
download "$base_url/deploy-go-agent.service" "$unit_file"

verify_sha256 \
  "$x86_file" \
  "$(jq -er --arg arch x86_64 '.artifacts[] | select(.architecture == $arch) | .sha256' "$manifest_file")"
verify_sha256 \
  "$arm_file" \
  "$(jq -er --arg arch aarch64 '.artifacts[] | select(.architecture == $arch) | .sha256' "$manifest_file")"
verify_sha256 "$unit_file" "$(jq -er '.systemd_unit.sha256' "$manifest_file")"

grep -Fx 'User=deploy-go-agent' "$unit_file" >/dev/null ||
  die "systemd unit 缺少专用用户"
grep -Fx 'NoNewPrivileges=true' "$unit_file" >/dev/null ||
  die "systemd unit 缺少 NoNewPrivileges"

chmod 0755 "$x86_file" "$arm_file"
chmod 0644 "$manifest_file" "$unit_file"

mkdir -p "$release_dir"
if [[ -e "$target_dir" ]]; then
  mv "$target_dir" "$backup_dir"
fi
if ! mv "$staging_dir" "$target_dir"; then
  [[ -e "$backup_dir" ]] && mv "$backup_dir" "$target_dir"
  exit 1
fi
rm -rf "$backup_dir"

printf 'Agent %s 已同步到 %s\n' "$version" "$target_dir"
