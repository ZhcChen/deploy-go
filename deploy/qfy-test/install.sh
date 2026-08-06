#!/usr/bin/env bash

set -euo pipefail

die() {
  printf '部署失败：%s\n' "$1" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "缺少命令：$1"
}

[[ "$(id -u)" -eq 0 ]] || die "install.sh 必须以 root 运行"

API_PORT="${DEPLOY_GO_API_PORT:-30100}"
API_BIND="${DEPLOY_GO_API_BIND:-0.0.0.0}"
WEB_PORT="${DEPLOY_GO_WEB_PORT:-30101}"
WEB_BIND="${DEPLOY_GO_WEB_BIND:-0.0.0.0}"
ALLOWED_ORIGIN="${DEPLOY_GO_ALLOWED_ORIGIN:-}"
COOKIE_SECURE="${DEPLOY_GO_COOKIE_SECURE:-false}"
MASTER_KEY_VERSION="${DEPLOY_GO_MASTER_KEY_VERSION:-1}"
PUBLIC_BASE_URL="${DEPLOY_GO_PUBLIC_BASE_URL:-}"
AGENT_VERSION="${DEPLOY_GO_AGENT_VERSION:-}"
GITHUB_REPOSITORY="${DEPLOY_GO_GITHUB_REPOSITORY:-ZhcChen/deploy-go}"
STAGING_DIR="${DEPLOY_GO_STAGING_DIR:-/opt/deploy-go/.staging}"
INSTALL_DIR="${DEPLOY_GO_INSTALL_DIR:-/opt/deploy-go}"
DATA_DIR="${DEPLOY_GO_DATA_DIR:-/var/lib/deploy-go}"
API_DIR="$INSTALL_DIR/api"
WEB_DIR="$INSTALL_DIR/web"
ENV_FILE="/etc/deploy-go/api.env"
MASTER_KEY_FILE="/etc/deploy-go/master.key"
SYNC_SCRIPT="$INSTALL_DIR/sync-agent-release.sh"

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
fi

[[ -f "$STAGING_DIR/deploy-go-api" ]] || die "缺少 API 二进制：$STAGING_DIR/deploy-go-api"
[[ -f "$STAGING_DIR/web/index.html" ]] || die "缺少 Web 构建产物：$STAGING_DIR/web/index.html"
[[ -f "$STAGING_DIR/web_server.py" ]] || die "缺少 Web 服务脚本：$STAGING_DIR/web_server.py"

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

install -d -m 0750 -o deploy-go -g deploy-go "$INSTALL_DIR" "$API_DIR" "$WEB_DIR"
install -d -m 0750 -o deploy-go -g deploy-go "$DATA_DIR" "$DATA_DIR/agent-releases"
chown -R deploy-go:deploy-go "$DATA_DIR"

install -m 0755 "$STAGING_DIR/deploy-go-api" "$API_DIR/deploy-go-api.new"
chown deploy-go:deploy-go "$API_DIR/deploy-go-api.new"
mv -f "$API_DIR/deploy-go-api.new" "$API_DIR/deploy-go-api"

rm -rf "$WEB_DIR.new" "$WEB_DIR.old"
cp -a "$STAGING_DIR/web" "$WEB_DIR.new"
chown -R deploy-go:deploy-go "$WEB_DIR.new"
if [[ -d "$WEB_DIR" ]]; then
  mv "$WEB_DIR" "$WEB_DIR.old"
fi
mv "$WEB_DIR.new" "$WEB_DIR"
rm -rf "$WEB_DIR.old"

install -m 0755 "$STAGING_DIR/web_server.py" "$INSTALL_DIR/web_server.py"
chown root:deploy-go "$INSTALL_DIR/web_server.py"
if [[ -f "$STAGING_DIR/sync-agent-release.sh" ]]; then
  install -m 0755 "$STAGING_DIR/sync-agent-release.sh" "$SYNC_SCRIPT"
  chown root:root "$SYNC_SCRIPT"
fi

install -d -m 0700 /etc/deploy-go
if [[ ! -s "$MASTER_KEY_FILE" ]]; then
  umask 077
  openssl rand -base64 32 >"$MASTER_KEY_FILE"
  echo "已生成主密钥文件：$MASTER_KEY_FILE"
fi
chown root:deploy-go "$MASTER_KEY_FILE"
chmod 0640 "$MASTER_KEY_FILE"

env_tmp="$ENV_FILE.new.$$"
trap 'rm -f "$env_tmp"' EXIT
if [[ -f "$ENV_FILE" ]]; then
  grep -vE '^(DEPLOY_GO_BIND_ADDR|DEPLOY_GO_DATABASE_URL|DEPLOY_GO_ALLOWED_ORIGIN|DEPLOY_GO_ALLOWED_ORIGINS|DEPLOY_GO_COOKIE_SECURE|DEPLOY_GO_MASTER_KEY_VERSION|DEPLOY_GO_MASTER_KEY|DEPLOY_GO_MASTER_KEY_FILE|DEPLOY_GO_PUBLIC_BASE_URL|RUST_LOG)=' \
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
  if [[ -n "$PUBLIC_BASE_URL" ]]; then
    echo "DEPLOY_GO_PUBLIC_BASE_URL=$PUBLIC_BASE_URL"
  fi
  echo "RUST_LOG=info"
} >>"$env_tmp"
chmod 0600 "$env_tmp"
chown root:root "$env_tmp"
mv -f "$env_tmp" "$ENV_FILE"

cat >/etc/systemd/system/deploy-go-api.service <<EOF
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
StateDirectory=deploy-go
ReadWritePaths=$DATA_DIR
RestrictSUIDSGID=true
LockPersonality=true
RestrictRealtime=true

[Install]
WantedBy=multi-user.target
EOF

cat >/etc/systemd/system/deploy-go-web.service <<EOF
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

if [[ -n "$AGENT_VERSION" ]]; then
  [[ -x "$SYNC_SCRIPT" ]] || die "已启用 Agent 同步，但缺少同步脚本"
  "$SYNC_SCRIPT" --version "$AGENT_VERSION" --repository "$GITHUB_REPOSITORY"
  chown -R deploy-go:deploy-go "$DATA_DIR/agent-releases"
fi

systemctl daemon-reload
systemctl enable deploy-go-api deploy-go-web >/dev/null
systemctl restart deploy-go-api
systemctl restart deploy-go-web

for service in deploy-go-api deploy-go-web; do
  systemctl is-active --quiet "$service" || die "systemd 服务未运行：$service"
done

curl --fail --silent "http://127.0.0.1:$API_PORT/healthz" >/dev/null
curl --fail --silent "http://127.0.0.1:$API_PORT/readyz" >/dev/null
curl --fail --silent "http://127.0.0.1:$WEB_PORT/" >/dev/null
curl --fail --silent "http://127.0.0.1:$WEB_PORT/api/v1/openapi.json" >/dev/null

printf '部署完成：API http://127.0.0.1:%s，Web http://127.0.0.1:%s\n' \
  "$API_PORT" "$WEB_PORT"
