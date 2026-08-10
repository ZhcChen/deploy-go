#!/usr/bin/env bash

set -euo pipefail

agent_unit="agent/install/deploy-go-agent.service"
runner_unit="agent/install/deploy-go-agent-runner.service"
executor_unit="agent/install/deploy-go-agent-executor.service"
config_template="agent/install/executor.json.in"
install_script="agent/install/install.sh"

grep -F "sudo -u deploy-go-agent %s status" "$install_script" >/dev/null
grep -F "sudo -u deploy-go-agent %s doctor" "$install_script" >/dev/null
if grep -E '状态命令：.*(TOKEN|token|credential)|诊断命令：.*(TOKEN|token|credential)' "$install_script" >/dev/null; then
  printf '安装完成诊断提示不得包含凭证\n' >&2
  exit 1
fi

grep -Fx 'User=deploy-go-agent' "$agent_unit" >/dev/null
grep -Fx 'Group=deploy-go-agent' "$agent_unit" >/dev/null
grep -Fx 'SupplementaryGroups=deploy-go-runner' "$agent_unit" >/dev/null
grep -Fx 'After=network-online.target deploy-go-agent-executor.service deploy-go-agent-runner.service' "$agent_unit" >/dev/null
grep -Fx 'Wants=network-online.target deploy-go-agent-executor.service deploy-go-agent-runner.service' "$agent_unit" >/dev/null
grep -Fx 'NoNewPrivileges=true' "$agent_unit" >/dev/null
grep -Fx 'UMask=0007' "$agent_unit" >/dev/null

grep -Fx 'User=root' "$runner_unit" >/dev/null
grep -Fx 'NoNewPrivileges=true' "$runner_unit" >/dev/null
grep -Fx 'ExecStart=/usr/local/bin/deploy-go-agent runner-service' "$runner_unit" >/dev/null
grep -Fx 'RuntimeDirectoryMode=0755' "$runner_unit" >/dev/null

grep -Fx 'User=root' "$executor_unit" >/dev/null
grep -Fx 'Before=deploy-go-agent.service' "$executor_unit" >/dev/null
grep -Fx 'Delegate=yes' "$executor_unit" >/dev/null
grep -Fx 'KillMode=control-group' "$executor_unit" >/dev/null
grep -Fx 'InaccessiblePaths=/var/lib/deploy-go-agent/credentials.json' "$executor_unit" >/dev/null
grep -Fx 'InaccessiblePaths=/etc/deploy-go-agent/config' "$executor_unit" >/dev/null
if grep -Eq '^(RestrictAddressFamilies|IPAddressDeny|PrivateDevices|PrivateTmp|ProtectClock|ProtectKernelTunables|ProtectKernelModules|ProtectKernelLogs|ProtectControlGroups|ProtectHostname|RestrictSUIDSGID|LockPersonality|RestrictRealtime|SystemCallArchitectures|UMask)=' "$executor_unit"; then
  printf 'executor unit 不得限制完整 root 登录终端能力\n' >&2
  exit 1
fi

if grep -Eq '(access_token|refresh_token|enrollment_token)=' "$agent_unit" "$executor_unit"; then
  printf 'systemd unit 不得包含 Agent 凭证\n' >&2
  exit 1
fi
if [[ -e agent/install/deploy-go-agent.socket ]]; then
  printf 'executor 当前自行创建 Unix Socket，不得同时启用 systemd socket activation\n' >&2
  exit 1
fi

rendered_config="$(mktemp)"
verify_dir="$(mktemp -d)"
cleanup() {
  rm -f -- "$rendered_config"
  rm -rf -- "$verify_dir"
}
trap cleanup EXIT
sed \
  -e 's/@DEPLOY_GO_AGENT_UID@/1001/' \
  -e 's/@DEPLOY_GO_AGENT_GID@/1001/' \
  -e 's#@DEPLOY_GO_ROOT_SHELL@#/bin/sh#' \
  -e 's#@DEPLOY_GO_ROOT_HOME@#/root#' \
  "$config_template" >"$rendered_config"
jq -e \
  '.allowed_uid == 1001 and .allowed_gid == 1001 and .shell == "/bin/sh" and .home == "/root"' \
  "$rendered_config" >/dev/null

if command -v systemd-analyze >/dev/null 2>&1; then
  sed 's#/usr/local/bin/deploy-go-agent-executor#/bin/true#' \
    "$executor_unit" >"$verify_dir/deploy-go-agent-executor.service"
  sed 's#/usr/local/bin/deploy-go-agent#/bin/true#' \
    "$agent_unit" >"$verify_dir/deploy-go-agent.service"
  sed 's#/usr/local/bin/deploy-go-agent runner-service#/bin/true runner-service#' \
    "$runner_unit" >"$verify_dir/deploy-go-agent-runner.service"
  systemd-analyze verify \
    "$verify_dir/deploy-go-agent-executor.service" \
    "$verify_dir/deploy-go-agent-runner.service" \
    "$verify_dir/deploy-go-agent.service"
else
  printf '提示：当前系统无 systemd-analyze，已完成 unit 静态安全契约检查\n'
fi

printf 'Agent/executor systemd 安全契约检查通过\n'
