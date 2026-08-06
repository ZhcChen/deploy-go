.DEFAULT_GOAL := help

PYTHON ?= python3
UI_PORT ?= 30102
ADMIN_PORT ?= 30101
ADMIN_API_PROXY_TARGET ?= http://127.0.0.1:30100
API_IMAGE ?= deploy-go-api:local
DOCKER_PLATFORM ?=
DEPLOY_GO_API_BASE_URL ?= http://127.0.0.1:30100
DEPLOY_GO_ALLOWED_ORIGIN ?= http://127.0.0.1:30101
DEPLOY_GO_ALLOWED_ORIGINS ?=
DEPLOY_GO_COOKIE_SECURE ?= false
DEVICE_ID ?=

.PHONY: help api-run api-migrate api-openapi api-openapi-check api-client-generate api-client-check credential-reencrypt api-test api-check api-image agent-check agent-install-check agent-manifest-check deploy-contract-demo-check privileged-launcher-check admin admin-check admin-test admin-build admin-test-e2e admin-app-get admin-app admin-app-check admin-app-test admin-app-build admin-app-test-integration client-sensitive-check ui ui-serve ui-check ui-test deploy-production deploy-production-check check

help: ## 显示可用命令
	@printf '%s\n' \
		'可用命令：' \
		'  make api-run   启动 Rust API（默认 http://127.0.0.1:30100）' \
		'  make api-migrate 执行 SQLite migration 后退出' \
		'  make api-openapi 生成 OpenAPI JSON 产物' \
		'  make api-openapi-check 检查 OpenAPI 产物是否最新' \
		'  make api-client-generate 生成 Web 与 Flutter API client' \
		'  make api-client-check 检查双端 API client 是否漂移' \
		'  make credential-reencrypt 离线重加密 legacy SSH 凭证' \
		'  make api-test  执行 API 测试' \
		'  make api-check 检查 Rust 格式、clippy 和测试' \
		'  make api-image 构建 API release Docker 镜像' \
		'  make agent-check 检查 Agent、协议、安装器与 manifest' \
		'  make agent-install-check 检查 Agent 安装器与 systemd unit' \
		'  make agent-manifest-check 检查 Agent release manifest 生成器' \
		'  make deploy-contract-demo-check 检查业务应用分支部署接入 Demo' \
		'  make privileged-launcher-check 检查受控发布 launcher 契约 Demo' \
		'  make agent-release-sync 历史手动同步脚本（GitHub Actions 已停用，部署不再使用）' \
		'  make agent-release-sync-check 检查同步脚本与本地 fixture 同步' \
		'  make admin     启动 Web 管理端开发服务器（默认 http://127.0.0.1:$(ADMIN_PORT)）' \
		'  make admin-check 检查 Web 管理端格式、类型、测试与构建' \
		'  make admin-test 执行 Web 管理端单元和组件测试' \
		'  make admin-build 构建 Web 管理端' \
		'  make admin-test-e2e 执行 Web 管理端浏览器 smoke' \
		'  make admin-app-get 安装 Flutter 管理端依赖' \
		'  make admin-app 启动 Flutter 管理端' \
		'  make admin-app-check 检查 Flutter 格式、analyze 与测试' \
		'  make admin-app-test 执行 Flutter 管理端测试' \
		'  make admin-app-build 构建 Flutter Android debug APK' \
		'  make admin-app-test-integration DEVICE_ID=<id> 执行设备 smoke' \
		'  make client-sensitive-check 扫描客户端源码与 fixture 的敏感模式' \
		'  make ui        启动 UI 设计源预览（http://127.0.0.1:$(UI_PORT)）' \
		'  make ui-serve  与 make ui 相同' \
		'  make ui-check  检查 UI 设计源语法与文件格式' \
		'  make ui-test   执行 UI Playwright 交互回归' \
		'  make deploy-production 部署正式环境（SSH alias: qfy-test）' \
		'  make deploy-production-check 检查正式环境部署脚本安全契约' \
		'  make check     执行全仓检查'

api-run: ## 启动 Rust API
	env $(if $(strip $(DEPLOY_GO_ALLOWED_ORIGINS)),-u DEPLOY_GO_ALLOWED_ORIGIN DEPLOY_GO_ALLOWED_ORIGINS='$(DEPLOY_GO_ALLOWED_ORIGINS)',-u DEPLOY_GO_ALLOWED_ORIGINS DEPLOY_GO_ALLOWED_ORIGIN=$(DEPLOY_GO_ALLOWED_ORIGIN)) \
	DEPLOY_GO_COOKIE_SECURE=$(DEPLOY_GO_COOKIE_SECURE) \
	cargo run -p deploy-go-api

api-migrate: ## 执行 SQLite migration
	cargo run -p deploy-go-api -- migrate

credential-reencrypt: ## 使用 current/previous 主密钥离线重加密 legacy SSH 凭证
	cargo run -p deploy-go-api -- credential-reencrypt

api-test: ## 执行 API 测试
	cargo test -p deploy-go-api

api-check: ## 检查 Rust API
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	$(MAKE) api-openapi-check
	@! grep -nE 'openssh-client|ssh-keyscan' api/docker/release/Dockerfile

api-image: ## 构建 API release Docker 镜像
	docker build \
		$(if $(DOCKER_PLATFORM),--platform $(DOCKER_PLATFORM)) \
		--tag $(API_IMAGE) \
		--file api/docker/release/Dockerfile \
		.

agent-install-check: ## 检查 Agent 安装器与 systemd unit
	bash -n agent/install/install.sh
	jq -e . agent/release/manifest.schema.json >/dev/null
	@if command -v bats >/dev/null 2>&1; then bats agent/tests/install.bats; else printf '%s\n' '提示：未安装 bats，仅执行安装器静态检查'; fi
	@! grep -nE '(access_token|refresh_token|enrollment_token)=' agent/install/deploy-go-agent.service
	@grep -Fx 'User=deploy-go-agent' agent/install/deploy-go-agent.service >/dev/null
	@grep -Fx 'NoNewPrivileges=true' agent/install/deploy-go-agent.service >/dev/null

agent-check: agent-install-check agent-manifest-check agent-release-sync-check ## 检查 Agent 与协议
	cargo test -p deploy-go-agent-protocol -p deploy-go-agent

agent-manifest-check: ## 检查 Agent release manifest 生成器
	bash -n agent/release/generate-manifest.sh
	bash agent/release/test-generate-manifest.sh
	jq -e . agent/release/manifest.schema.json >/dev/null

deploy-contract-demo-check: ## 检查业务应用分支部署接入 Demo
	bash -n examples/branch-deployment/scripts/prepare.sh
	bash -n examples/branch-deployment/scripts/release.sh
	bash -n examples/branch-deployment/test-contract.sh
	bash examples/branch-deployment/test-contract.sh
	jq -e . docs/standards/deploy-artifact-manifest.schema.json >/dev/null

privileged-launcher-check: ## 检查受控发布 launcher 契约 Demo
	bash -n examples/privileged-release-launcher/launcher.sh
	bash -n examples/privileged-release-launcher/release-entry.sh
	bash -n examples/privileged-release-launcher/test-contract.sh
	bash examples/privileged-release-launcher/test-contract.sh
	@grep -Fq 'deploy-go-agent ALL=(root) NOPASSWD: /usr/local/sbin/deploy-go-release-launcher --input /var/lib/deploy-go-agent/apps/*' examples/privileged-release-launcher/sudoers.example
	@! grep -Eq 'ALL=\(ALL\)( NOPASSWD:)? ALL|/usr/bin/sudo|/bin/bash|docker' examples/privileged-release-launcher/sudoers.example

agent-release-sync: ## 从 GitHub Release 同步 Agent 发布物到 API 发布目录
	bash scripts/sync-agent-release.sh

agent-release-sync-check: ## 检查 Agent release 同步脚本
	bash -n scripts/sync-agent-release.sh
	bash scripts/test-sync-agent-release.sh

ui: ui-serve ## 启动 UI 设计源预览

ui-serve: ## 使用 Python 静态服务器启动 UI 设计源
	@printf 'UI 预览地址：http://127.0.0.1:%s/#/entry\n' '$(UI_PORT)'
	$(PYTHON) ui/serve.py --port $(UI_PORT) --bind 127.0.0.1

ui-check: ## 检查 UI 设计源语法与文件格式
	node --check ui/assets/app.js
	node --check ui/assets/mock-data.js
	node --check ui/tests/ui-preview.spec.js
	PYTHONPYCACHEPREFIX=/tmp/deploy-go-pycache $(PYTHON) -m py_compile ui/serve.py
	@! git grep -nI -E '[[:blank:]]+$$' -- Makefile README.md docs ui
	git diff --check

ui-test: ## 执行 UI Playwright 交互回归
	npm run test:ui

admin: ## 启动 Web 管理端开发服务器
	VITE_API_PROXY_TARGET=$(ADMIN_API_PROXY_TARGET) npm run dev --workspace deploy-go-admin -- --port $(ADMIN_PORT)

admin-check: ## 检查 Web 管理端
	npm run admin:check

admin-test: ## 执行 Web 管理端单元和组件测试
	npm test --workspace deploy-go-admin

admin-build: ## 构建 Web 管理端
	npm run build --workspace deploy-go-admin

admin-test-e2e: ## 执行 Web 管理端浏览器 smoke
	npm run test:e2e --workspace deploy-go-admin

admin-app-get: ## 安装 Flutter 管理端依赖
	cd admin-app && flutter pub get

admin-app: ## 启动 Flutter 管理端
	cd admin-app && flutter run \
		--dart-define=DEPLOY_GO_API_BASE_URL=$(DEPLOY_GO_API_BASE_URL) \
		--dart-define=DEPLOY_GO_ALLOWED_ORIGIN=$(DEPLOY_GO_ALLOWED_ORIGIN)

deploy-production: ## 部署正式环境（systemd，Agent 由本机构建上传）
	bash deploy/production/deploy.sh

deploy-production-check: ## 检查正式环境部署脚本安全契约
	bash -n deploy/production/deploy.sh
	bash -n deploy/production/install.sh
	bash deploy/production/test-install-contract.sh

admin-app-check: ## 检查 Flutter 管理端
	cd admin-app && dart format --output=none --set-exit-if-changed \
		lib/main.dart lib/api/*.dart lib/app lib/features lib/routing lib/security lib/theme test integration_test
	cd admin-app && flutter analyze
	cd admin-app && flutter test

admin-app-test: ## 执行 Flutter 管理端测试
	cd admin-app && flutter test

admin-app-build: ## 构建 Flutter Android debug APK
	cd admin-app && flutter build apk --debug \
		--dart-define=DEPLOY_GO_API_BASE_URL=$(DEPLOY_GO_API_BASE_URL) \
		--dart-define=DEPLOY_GO_ALLOWED_ORIGIN=$(DEPLOY_GO_ALLOWED_ORIGIN)

admin-app-test-integration: ## 在指定设备执行 Flutter 集成 smoke
	@test -n "$(DEVICE_ID)" || { printf '%s\n' '请指定 DEVICE_ID，例如 make admin-app-test-integration DEVICE_ID=emulator-5554' >&2; exit 2; }
	cd admin-app && flutter test integration_test -d $(DEVICE_ID)

client-sensitive-check: ## 扫描客户端源码与 fixture 的敏感模式
	npm run client:sensitive:check

check: api-check agent-install-check agent-manifest-check agent-release-sync-check deploy-contract-demo-check privileged-launcher-check deploy-production-check ui-check api-client-check admin-check admin-app-check client-sensitive-check ## 执行全仓检查

api-openapi: ## 生成 OpenAPI JSON 产物
	cargo run -p deploy-go-api -- openapi

api-openapi-check: ## 检查 OpenAPI JSON 产物
	cargo run -p deploy-go-api -- openapi-check

api-client-generate: ## 根据 OpenAPI 生成双端 API client
	npm run api:client:generate

api-client-check: ## 检查双端 API client 是否漂移
	npm run api:client:check
