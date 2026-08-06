#!/usr/bin/env bats

setup() {
  TEST_ROOT="$(mktemp -d)"
  export TEST_ROOT
  export DEPLOY_GO_AGENT_INSTALL_ROOT="$TEST_ROOT/root"
  export DEPLOY_GO_AGENT_ID="agent-001"
  export DEPLOY_GO_AGENT_ENROLLMENT_TOKEN="enroll-secret-value"
  export DEPLOY_GO_AGENT_API_BASE_URL="https://control.example.test"
  export DEPLOY_GO_AGENT_CONTROL_URL="wss://control.example.test/api/v1/agent/connect"
  export DEPLOY_GO_AGENT_MANIFEST_URL="https://release.example.test/manifest.json"
  export DEPLOY_GO_AGENT_ARCHITECTURE="x86_64"
  export DEPLOY_GO_AGENT_OS="Linux"
  export DEPLOY_GO_AGENT_SYSTEMCTL="$TEST_ROOT/systemctl"

  printf 'new-agent-binary\n' >"$TEST_ROOT/agent"
  ARTIFACT_SHA="$(sha256sum "$TEST_ROOT/agent" | awk '{print $1}')"
  export ARTIFACT_SHA
  write_manifest "$ARTIFACT_SHA"
  write_systemctl
  write_curl
}

teardown() {
  rm -rf "$TEST_ROOT"
}

write_manifest() {
  UNIT_SHA="$(sha256sum "$BATS_TEST_DIRNAME/../install/deploy-go-agent.service" | awk '{print $1}')"
  jq -n --arg sha "$1" --arg unit_sha "$UNIT_SHA" '{
    schema_version: 1,
    agent_version: "0.1.0",
    protocol: {minimum: 1, maximum: 4},
    systemd_unit: {url: "https://release.example.test/deploy-go-agent.service", sha256: $unit_sha},
    artifacts: [
      {os: "linux", architecture: "x86_64", url: "https://release.example.test/agent", sha256: $sha},
      {os: "linux", architecture: "aarch64", url: "https://release.example.test/agent-arm64", sha256: $sha}
    ]
  }' >"$TEST_ROOT/manifest.json"
}

write_systemctl() {
  cat >"$TEST_ROOT/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$TEST_ROOT/systemctl.calls"
if [[ "${FAIL_HEALTH:-}" == "1" && "$1" == "is-active" ]]; then
  exit 1
fi
EOF
  chmod +x "$TEST_ROOT/systemctl"
}

write_curl() {
  cat >"$TEST_ROOT/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
output=""
url=""
while (($#)); do
  case "$1" in
    --output) output="$2"; shift 2 ;;
    --header | --data-binary | --proto) shift 2 ;;
    --fail | --silent | --show-error | --location | --tlsv1.2) shift ;;
    *) url="$1"; shift ;;
  esac
done
case "$url" in
  */manifest.json) cp "$TEST_ROOT/manifest.json" "$output" ;;
  */agent | */agent-arm64) cp "$TEST_ROOT/agent" "$output" ;;
  */deploy-go-agent.service) cp "/code/agent/install/deploy-go-agent.service" "$output" ;;
  */api/v1/agent/enroll)
    request="$(cat)"
    printf '%s' "$request" >"$TEST_ROOT/enroll.request"
    jq -n --arg id "$(jq -r .agent_id <<<"$request")" \
      '{agent_id: $id, access_token: "ignored", access_expires_at: "ignored", refresh_token: "rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr", refresh_expires_at: "ignored"}' >"$output"
    ;;
  *) exit 22 ;;
esac
EOF
  chmod +x "$TEST_ROOT/curl"
  export PATH="$TEST_ROOT:$PATH"
}

install_agent() {
  run "$BATS_TEST_DIRNAME/../install/install.sh"
}

@test "首次安装注册身份并启动服务" {
  install_agent

  echo "$output"
  [ "$status" -eq 0 ]
  [ -x "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent" ]
  [ "$(jq -r .agent_id "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json")" = "$DEPLOY_GO_AGENT_ID" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json")" = "600" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/apps")" = "700" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/secrets")" = "700" ]
  [ "$(jq -r .protocol_version "$TEST_ROOT/enroll.request")" = "4" ]
  grep -Fx 'is-active --quiet deploy-go-agent' "$TEST_ROOT/systemctl.calls"
  [[ "$output" != *"$DEPLOY_GO_AGENT_ENROLLMENT_TOKEN"* ]]
  ! grep -R "$DEPLOY_GO_AGENT_ENROLLMENT_TOKEN" "$DEPLOY_GO_AGENT_INSTALL_ROOT"
}

@test "同一 Agent 重跑保留凭证且不再次注册" {
  install_agent
  echo "$output"
  [ "$status" -eq 0 ]
  cp "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json" "$TEST_ROOT/credentials.before"
  rm "$TEST_ROOT/enroll.request"

  unset DEPLOY_GO_AGENT_ENROLLMENT_TOKEN
  install_agent

  [ "$status" -eq 0 ]
  cmp "$TEST_ROOT/credentials.before" "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json"
  [ ! -e "$TEST_ROOT/enroll.request" ]
  grep -Fx 'new-agent-binary' "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
}

@test "撤销后可用新 token 重新绑定同一 Agent" {
  install_agent
  [ "$status" -eq 0 ]
  export DEPLOY_GO_AGENT_REBIND=1
  export DEPLOY_GO_AGENT_ENROLLMENT_TOKEN="replacement-enrollment-token"

  install_agent

  echo "$output"
  [ "$status" -eq 0 ]
  [ "$(jq -r .enrollment_token "$TEST_ROOT/enroll.request")" = "$DEPLOY_GO_AGENT_ENROLLMENT_TOKEN" ]
  [[ "$output" != *"$DEPLOY_GO_AGENT_ENROLLMENT_TOKEN"* ]]
}

@test "不同 Agent ID 拒绝覆盖现有身份" {
  install_agent
  echo "$output"
  [ "$status" -eq 0 ]

  export DEPLOY_GO_AGENT_ID="agent-002"
  install_agent

  echo "$output"
  [ "$status" -ne 0 ]
  [[ "$output" == *"拒绝覆盖"* ]]
  [ "$(jq -r .agent_id "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json")" = "agent-001" ]
}

@test "checksum 不匹配时不安装" {
  write_manifest "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

  install_agent

  echo "$output"
  [ "$status" -ne 0 ]
  [[ "$output" == *"校验失败"* ]]
  [ ! -e "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent" ]
}

@test "不支持的架构在下载前失败" {
  export DEPLOY_GO_AGENT_ARCHITECTURE="riscv64"

  install_agent

  echo "$output"
  [ "$status" -ne 0 ]
  [[ "$output" == *"不支持的架构"* ]]
  [ ! -e "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent" ]
}

@test "服务健康失败时恢复旧二进制" {
  mkdir -p "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin" "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent"
  printf 'old-agent-binary\n' >"$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
  printf '{"agent_id":"agent-001","refresh_token":"rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr"}\n' \
    >"$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json"
  chmod 0700 "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent"
  chmod 0600 "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json"
  export FAIL_HEALTH=1

  install_agent

  [ "$status" -ne 0 ]
  [[ "$output" == *"已恢复上一版本"* ]]
  grep -Fx 'old-agent-binary' "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
}
