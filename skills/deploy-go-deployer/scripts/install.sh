#!/usr/bin/env bash
set -euo pipefail

skill_source_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
install_dir="${DEPLOY_GO_SKILL_INSTALL_DIR:-${CODEX_HOME:-$HOME/.codex}/skills/deploy-go-deployer}"
version="${DEPLOY_GO_DEPLOYER_VERSION:-0.3.1}"
api_base="${DEPLOY_GO_API_BASE_URL:-https://deploy.quanxinfu.com}"
binary_name="deploy-go-deployer"

fail() {
  printf '安装失败：%s\n' "$1" >&2
  exit 1
}

for required in SKILL.md agents/openai.yaml references/commands.md references/workflows.md references/errors.md; do
  [[ -f "$skill_source_dir/$required" ]] || fail "Skill 源缺少 $required"
done

os="$(uname -s)"
arch_raw="$(uname -m)"
case "$os" in
  Darwin) os_name="darwin" ;;
  Linux) os_name="linux" ;;
  *) fail "不支持的操作系统：$os" ;;
esac
case "$arch_raw" in
  x86_64|amd64|AMD64) arch="x86_64" ;;
  arm64|aarch64|ARM64) arch="aarch64" ;;
  *) fail "不支持的 CPU 架构：$arch_raw" ;;
esac

temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/deploy-go-skill.XXXXXX")"
cleanup() {
  rm -rf -- "$temp_dir"
}
trap cleanup EXIT
staging_dir="$temp_dir/deploy-go-deployer"
mkdir -p "$staging_dir/agents" "$staging_dir/references" "$staging_dir/scripts"
cp "$skill_source_dir/SKILL.md" "$staging_dir/SKILL.md"
cp "$skill_source_dir/agents/openai.yaml" "$staging_dir/agents/openai.yaml"
cp "$skill_source_dir/references/"*.md "$staging_dir/references/"
cp "$skill_source_dir/scripts/install.sh" "$staging_dir/scripts/install.sh"

if [[ -n "${DEPLOY_GO_DEPLOYER_BINARY:-}" ]]; then
  [[ -f "$DEPLOY_GO_DEPLOYER_BINARY" ]] || fail "DEPLOY_GO_DEPLOYER_BINARY 不存在：$DEPLOY_GO_DEPLOYER_BINARY"
  cp "$DEPLOY_GO_DEPLOYER_BINARY" "$staging_dir/scripts/$binary_name"
elif [[ -f "$skill_source_dir/../../Cargo.toml" && -f "$skill_source_dir/../../deploy-go-deployer/Cargo.toml" ]] && command -v cargo >/dev/null 2>&1; then
  repo_root="$(cd "$skill_source_dir/../.." && pwd)"
  (cd "$repo_root" && cargo build -p deploy-go-deployer --release) || fail "构建 $binary_name 失败"
  cp "$repo_root/target/release/$binary_name" "$staging_dir/scripts/$binary_name"
elif [[ "$os_name" == "linux" ]]; then
  command -v curl >/dev/null 2>&1 || fail "缺少命令：curl"
  download_version="${version//./_}"
  manifest_url="$api_base/api/v1/deployer/download/$download_version/manifest.json"
  binary_url="$api_base/api/v1/deployer/download/$download_version/deployer/$arch"
  curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    "$manifest_url" -o "$temp_dir/manifest.json" || fail "下载 deployer manifest 失败"
  expected_sha="$(
    tr -d '[:space:]' < "$temp_dir/manifest.json" |
      sed -n "s/.*\"architecture\":\"$arch\"[^}]*\"sha256\":\"\([0-9a-f]\{64\}\)\".*/\1/p"
  )"
  [[ "$expected_sha" =~ ^[0-9a-f]{64}$ ]] || fail "manifest 中缺少 $arch 的 SHA-256"
  curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    "$binary_url" -o "$staging_dir/scripts/$binary_name" || fail "下载 $binary_name 失败"
  if command -v shasum >/dev/null 2>&1; then
    actual_sha="$(shasum -a 256 "$staging_dir/scripts/$binary_name" | awk '{print $1}')"
  elif command -v sha256sum >/dev/null 2>&1; then
    actual_sha="$(sha256sum "$staging_dir/scripts/$binary_name" | awk '{print $1}')"
  else
    fail "缺少 shasum 或 sha256sum"
  fi
  [[ "$actual_sha" == "$expected_sha" ]] || fail "$binary_name SHA-256 校验失败"
else
  fail "macOS 未发布官方二进制，请在 deploy-go 仓库 checkout 中运行本脚本以从源码构建"
fi

chmod +x "$staging_dir/scripts/$binary_name"
"$staging_dir/scripts/$binary_name" --version >/dev/null || fail "$binary_name 自检失败"

install_parent="$(dirname "$install_dir")"
mkdir -p "$install_parent"
backup_dir="${install_dir}.backup.$$"
if [[ -d "$install_dir" ]]; then
  mv "$install_dir" "$backup_dir"
fi
if ! mv "$staging_dir" "$install_dir"; then
  if [[ -d "$backup_dir" ]]; then
    mv "$backup_dir" "$install_dir" || true
  fi
  fail "替换 Skill 安装目录失败"
fi
rm -rf -- "$backup_dir"

printf '已安装 deploy-go-deployer Skill 到 %s\n' "$install_dir"
printf '请配置 DEPLOY_GO_API_KEY 后再使用；需要自定义服务时再配置 DEPLOY_GO_API_BASE_URL。\n'
