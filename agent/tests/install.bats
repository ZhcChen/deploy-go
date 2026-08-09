#!/usr/bin/env bats

setup() {
  TEST_ROOT="$(mktemp -d)"
  export TEST_ROOT
  export DEPLOY_GO_AGENT_INSTALL_ROOT="$TEST_ROOT/root"
  export DEPLOY_GO_AGENT_ID="agent_001"
  export DEPLOY_GO_NODE_ID="node_001"
  export DEPLOY_GO_TERMINAL_CAPABILITY_PUBLIC_KEY="AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
  export DEPLOY_GO_AGENT_ENROLLMENT_TOKEN="enroll-secret-value"
  export DEPLOY_GO_AGENT_API_BASE_URL="https://control.example.test"
  export DEPLOY_GO_AGENT_CONTROL_URL="wss://control.example.test/api/v1/agent/connect"
  export DEPLOY_GO_AGENT_MANIFEST_URL="https://release.example.test/manifest.json"
  export DEPLOY_GO_AGENT_ARCHITECTURE="x86_64"
  export DEPLOY_GO_AGENT_OS="Linux"
  export DEPLOY_GO_AGENT_SYSTEMCTL="$TEST_ROOT/systemctl"

  printf 'new-agent-binary\n' >"$TEST_ROOT/agent"
  printf 'new-executor-binary\n' >"$TEST_ROOT/executor"
  ARTIFACT_SHA="$(sha256sum "$TEST_ROOT/agent" | awk '{print $1}')"
  EXECUTOR_ARTIFACT_SHA="$(sha256sum "$TEST_ROOT/executor" | awk '{print $1}')"
  export ARTIFACT_SHA
  export EXECUTOR_ARTIFACT_SHA
  write_manifest "$ARTIFACT_SHA" "$EXECUTOR_ARTIFACT_SHA"
  write_systemctl
  write_curl
  write_su
}

write_su() {
  cat >"$TEST_ROOT/su" <<'EOF'
#!/usr/bin/env bash
[[ "${FAIL_EXECUTOR_PROBE:-}" != "1" ]]
EOF
  chmod +x "$TEST_ROOT/su"
}

teardown() {
  rm -rf "$TEST_ROOT"
}

write_manifest() {
  AGENT_UNIT_SHA="$(sha256sum "$BATS_TEST_DIRNAME/../install/deploy-go-agent.service" | awk '{print $1}')"
  RUNNER_UNIT_SHA="$(sha256sum "$BATS_TEST_DIRNAME/../install/deploy-go-agent-runner.service" | awk '{print $1}')"
  EXECUTOR_UNIT_SHA="$(sha256sum "$BATS_TEST_DIRNAME/../install/deploy-go-agent-executor.service" | awk '{print $1}')"
  EXECUTOR_CONFIG_SHA="$(sha256sum "$BATS_TEST_DIRNAME/../install/executor.json.in" | awk '{print $1}')"
  jq -n \
    --arg agent_sha "$1" \
    --arg executor_sha "$2" \
    --arg agent_unit_sha "$AGENT_UNIT_SHA" \
    --arg runner_unit_sha "$RUNNER_UNIT_SHA" \
    --arg executor_unit_sha "$EXECUTOR_UNIT_SHA" \
    --arg executor_config_sha "$EXECUTOR_CONFIG_SHA" '{
    schema_version: 3,
    agent_version: "0.1.0",
    executor_version: "0.1.0",
    protocol: {minimum: 1, maximum: 6},
    systemd_units: {
      agent: {url: "https://release.example.test/deploy-go-agent.service", sha256: $agent_unit_sha},
      runner: {url: "https://release.example.test/deploy-go-agent-runner.service", sha256: $runner_unit_sha},
      executor: {url: "https://release.example.test/deploy-go-agent-executor.service", sha256: $executor_unit_sha}
    },
    executor_config: {url: "https://release.example.test/executor.json.in", sha256: $executor_config_sha},
    artifacts: [
      {component: "agent", os: "linux", architecture: "x86_64", url: "https://release.example.test/agent", sha256: $agent_sha},
      {component: "agent", os: "linux", architecture: "aarch64", url: "https://release.example.test/agent-arm64", sha256: $agent_sha},
      {component: "executor", os: "linux", architecture: "x86_64", url: "https://release.example.test/executor", sha256: $executor_sha},
      {component: "executor", os: "linux", architecture: "aarch64", url: "https://release.example.test/executor-arm64", sha256: $executor_sha}
    ]
  }' >"$TEST_ROOT/manifest.json"
}

write_systemctl() {
  cat >"$TEST_ROOT/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$TEST_ROOT/systemctl.calls"
service="${@: -1}"
if [[ "$1" == "stop" ]]; then
  rm -f "$TEST_ROOT/active.$service"
  exit 0
fi
if [[ "$1" == "is-active" ]]; then
  [[ "${FAIL_HEALTH:-}" != "1" || "$service" != "deploy-go-agent" ]]
  [[ "${FAIL_EXECUTOR_HEALTH:-}" != "1" || "$service" != "deploy-go-agent-executor" ]]
  [[ "${FAIL_RUNNER_HEALTH:-}" != "1" || "$service" != "deploy-go-agent-runner" ]]
  [[ -f "$TEST_ROOT/active.$service" ]]
  exit
fi
if [[ "$1" == "restart" ]]; then
  touch "$TEST_ROOT/active.$service"
fi
if [[ "$1" == "restart" && "$*" == *"deploy-go-agent-executor" && "${FAIL_EXECUTOR_HEALTH:-}" != "1" ]]; then
  mkdir -p "$DEPLOY_GO_AGENT_INSTALL_ROOT/run/deploy-go-agent"
  python3 - "$DEPLOY_GO_AGENT_INSTALL_ROOT/run/deploy-go-agent/executor.sock" <<'PY'
import socket
import sys
import os

try:
    os.unlink(sys.argv[1])
except FileNotFoundError:
    pass
sock = socket.socket(socket.AF_UNIX)
sock.bind(sys.argv[1])
sock.close()
os.chmod(sys.argv[1], 0o660)
PY
fi
if [[ "$1" == "restart" && "$*" == *"deploy-go-agent-runner" && "${FAIL_RUNNER_HEALTH:-}" != "1" ]]; then
  mkdir -p "$DEPLOY_GO_AGENT_INSTALL_ROOT/run/deploy-go-agent-runner"
  python3 - "$DEPLOY_GO_AGENT_INSTALL_ROOT/run/deploy-go-agent-runner/runner.sock" <<'PY'
import socket
import sys
import os

try:
    os.unlink(sys.argv[1])
except FileNotFoundError:
    pass
sock = socket.socket(socket.AF_UNIX)
sock.bind(sys.argv[1])
sock.close()
os.chmod(sys.argv[1], 0o660)
PY
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
  */executor | */executor-arm64) cp "$TEST_ROOT/executor" "$output" ;;
  */deploy-go-agent.service) cp "/code/agent/install/deploy-go-agent.service" "$output" ;;
  */deploy-go-agent-runner.service) cp "/code/agent/install/deploy-go-agent-runner.service" "$output" ;;
  */deploy-go-agent-executor.service) cp "/code/agent/install/deploy-go-agent-executor.service" "$output" ;;
  */executor.json.in) cp "/code/agent/install/executor.json.in" "$output" ;;
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
  [ -x "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent-executor" ]
  [ -f "$DEPLOY_GO_AGENT_INSTALL_ROOT/etc/systemd/system/deploy-go-agent-executor.service" ]
  [ -f "$DEPLOY_GO_AGENT_INSTALL_ROOT/etc/systemd/system/deploy-go-agent-runner.service" ]
  [ "$(jq -r .allowed_uid "$DEPLOY_GO_AGENT_INSTALL_ROOT/etc/deploy-go-agent/executor.json")" = "1001" ]
  [ "$(jq -r .allowed_gid "$DEPLOY_GO_AGENT_INSTALL_ROOT/etc/deploy-go-agent/executor.json")" = "1001" ]
  [ "$(jq -r .allowed_executable "$DEPLOY_GO_AGENT_INSTALL_ROOT/etc/deploy-go-agent/executor.json")" = "/usr/local/bin/deploy-go-agent" ]
  [ "$(jq -r .node_id "$DEPLOY_GO_AGENT_INSTALL_ROOT/etc/deploy-go-agent/executor.json")" = "$DEPLOY_GO_NODE_ID" ]
  [ "$(jq -r .agent_id "$DEPLOY_GO_AGENT_INSTALL_ROOT/etc/deploy-go-agent/executor.json")" = "$DEPLOY_GO_AGENT_ID" ]
  [ "$(jq -r .capability_public_key "$DEPLOY_GO_AGENT_INSTALL_ROOT/etc/deploy-go-agent/executor.json")" = "$DEPLOY_GO_TERMINAL_CAPABILITY_PUBLIC_KEY" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent-executor/used-capabilities")" = "700" ]
  [ "$(jq -r .agent_id "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json")" = "$DEPLOY_GO_AGENT_ID" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json")" = "600" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/tasks")" = "3770" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/apps")" = "2770" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/secrets")" = "2750" ]
  [ "$(jq -r .protocol_version "$TEST_ROOT/enroll.request")" = "6" ]
  grep -Fx 'is-active --quiet deploy-go-agent-executor' "$TEST_ROOT/systemctl.calls"
  grep -Fx 'is-active --quiet deploy-go-agent-runner' "$TEST_ROOT/systemctl.calls"
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
  grep -Fx 'new-executor-binary' "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent-executor"
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

  export DEPLOY_GO_AGENT_ID="agent_002"
  install_agent

  echo "$output"
  [ "$status" -ne 0 ]
  [[ "$output" == *"拒绝覆盖"* ]]
  [ "$(jq -r .agent_id "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json")" = "agent_001" ]
}

@test "checksum 不匹配时不安装" {
  write_manifest "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" "$EXECUTOR_ARTIFACT_SHA"

  install_agent

  echo "$output"
  [ "$status" -ne 0 ]
  [[ "$output" == *"校验失败"* ]]
  [ ! -e "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent" ]
  [ ! -e "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent-executor" ]
}

@test "Agent 与 executor 版本不一致时拒绝安装" {
  jq '.executor_version = "0.2.0"' "$TEST_ROOT/manifest.json" >"$TEST_ROOT/manifest.new"
  mv "$TEST_ROOT/manifest.new" "$TEST_ROOT/manifest.json"

  install_agent

  [ "$status" -ne 0 ]
  [[ "$output" == *"未成对"* ]]
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

@test "Agent 健康失败时恢复旧配对二进制" {
  mkdir -p "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin" "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent"
  printf 'old-agent-binary\n' >"$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
  printf 'old-executor-binary\n' >"$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent-executor"
  chmod 0755 \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent-executor"
  printf '{"agent_id":"agent_001","refresh_token":"rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr"}\n' \
    >"$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json"
  chmod 0700 "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent"
  chmod 0600 "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json"
  export FAIL_HEALTH=1

  install_agent

  [ "$status" -ne 0 ]
  [[ "$output" == *"已恢复上一配对版本"* ]]
  grep -Fx 'old-agent-binary' "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
  grep -Fx 'old-executor-binary' "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent-executor"
}

@test "executor 健康失败时恢复旧 Agent 且不声明新能力" {
  mkdir -p "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin" "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent"
  printf 'old-agent-binary\n' >"$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
  chmod 0755 "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
  printf '{"agent_id":"agent_001","refresh_token":"rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr"}\n' \
    >"$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json"
  chmod 0700 "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent"
  chmod 0600 "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json"
  export FAIL_EXECUTOR_HEALTH=1
  export DEPLOY_GO_AGENT_EXECUTOR_HEALTH_ATTEMPTS=1

  install_agent

  [ "$status" -ne 0 ]
  [[ "$output" == *"executor 健康检查失败"* ]]
  grep -Fx 'old-agent-binary' "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
  [ ! -e "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent-executor" ]
}

@test "runner 健康失败时恢复旧配对" {
  mkdir -p \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/tasks/task_old" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/apps/app_old" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/secrets/app_old"
  printf 'old-agent-binary\n' >"$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
  chmod 0755 "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
  printf '{"agent_id":"agent_001","refresh_token":"rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr"}\n' \
    >"$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json"
  chmod 0700 "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent"
  chmod 0600 "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json"
  printf '{}\n' >"$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/tasks/task_old/journal.json"
  printf 'run\n' >"$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/apps/app_old/deploy.sh"
  printf 'VALUE=old\n' >"$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/secrets/app_old/api.env"
  chmod 0700 \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/tasks" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/tasks/task_old" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/apps" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/apps/app_old" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/secrets" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/secrets/app_old"
  chmod 0600 \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/tasks/task_old/journal.json" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/apps/app_old/deploy.sh" \
    "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/secrets/app_old/api.env"
  export FAIL_RUNNER_HEALTH=1
  export DEPLOY_GO_AGENT_RUNNER_HEALTH_ATTEMPTS=1

  install_agent

  [ "$status" -ne 0 ]
  [[ "$output" == *"runner broker 健康检查失败"* ]]
  grep -Fx 'old-agent-binary' "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent"
  [ ! -e "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent-executor" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/tasks/task_old")" = "700" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/tasks/task_old/journal.json")" = "600" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/apps/app_old/deploy.sh")" = "600" ]
  [ "$(stat -c %a "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/secrets/app_old/api.env")" = "600" ]
}

@test "卸载先停止 Agent 再停止 runner 和 executor 并保留凭证" {
  install_agent
  [ "$status" -eq 0 ]

  run "$BATS_TEST_DIRNAME/../install/install.sh" --uninstall

  [ "$status" -eq 0 ]
  [ ! -e "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent" ]
  [ ! -e "$DEPLOY_GO_AGENT_INSTALL_ROOT/usr/local/bin/deploy-go-agent-executor" ]
  [ -f "$DEPLOY_GO_AGENT_INSTALL_ROOT/var/lib/deploy-go-agent/credentials.json" ]
  agent_stop_line="$(grep -n '^stop deploy-go-agent$' "$TEST_ROOT/systemctl.calls" | tail -n 1 | cut -d: -f1)"
  executor_stop_line="$(grep -n '^stop deploy-go-agent-executor$' "$TEST_ROOT/systemctl.calls" | tail -n 1 | cut -d: -f1)"
  runner_stop_line="$(grep -n '^stop deploy-go-agent-runner$' "$TEST_ROOT/systemctl.calls" | tail -n 1 | cut -d: -f1)"
  [ "$agent_stop_line" -lt "$runner_stop_line" ]
  [ "$runner_stop_line" -lt "$executor_stop_line" ]
}
