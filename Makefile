.DEFAULT_GOAL := help

PYTHON ?= python3
UI_PORT ?= 8050
API_IMAGE ?= deploy-go-api:local
DOCKER_PLATFORM ?=

.PHONY: help api-run api-migrate api-openapi api-openapi-check api-client-generate api-client-check credential-reencrypt api-test api-check api-image admin admin-check admin-build ui ui-serve ui-check ui-test check

help: ## 显示可用命令
	@printf '%s\n' \
		'可用命令：' \
		'  make api-run   启动 Rust API（默认 http://127.0.0.1:8080）' \
		'  make api-migrate 执行 SQLite migration 后退出' \
		'  make api-openapi 生成 OpenAPI JSON 产物' \
		'  make api-openapi-check 检查 OpenAPI 产物是否最新' \
		'  make api-client-generate 生成 Web 与 Flutter API client' \
		'  make api-client-check 检查双端 API client 是否漂移' \
		'  make credential-reencrypt 离线重加密 SSH 凭证' \
		'  make api-test  执行 API 测试' \
		'  make api-check 检查 Rust 格式、clippy 和测试' \
		'  make api-image 构建 API release Docker 镜像' \
		'  make admin     启动 Web 管理端开发服务器' \
		'  make admin-check 检查 Web 管理端格式、类型、测试与构建' \
		'  make admin-build 构建 Web 管理端' \
		'  make ui        启动 UI 设计源预览（http://127.0.0.1:$(UI_PORT)）' \
		'  make ui-serve  与 make ui 相同' \
		'  make ui-check  检查 UI 设计源语法与文件格式' \
		'  make ui-test   执行 UI Playwright 交互回归' \
		'  make check     执行全仓检查'

api-run: ## 启动 Rust API
	cargo run -p deploy-go-api

api-migrate: ## 执行 SQLite migration
	cargo run -p deploy-go-api -- migrate

credential-reencrypt: ## 使用 current/previous 主密钥离线重加密 SSH 凭证
	cargo run -p deploy-go-api -- credential-reencrypt

api-test: ## 执行 API 测试
	cargo test -p deploy-go-api

api-check: ## 检查 Rust API
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	$(MAKE) api-openapi-check

api-image: ## 构建 API release Docker 镜像
	docker build \
		$(if $(DOCKER_PLATFORM),--platform $(DOCKER_PLATFORM)) \
		--tag $(API_IMAGE) \
		--file api/docker/release/Dockerfile \
		.

ui: ui-serve ## 启动 UI 设计源预览

ui-serve: ## 使用 Python 静态服务器启动 UI 设计源
	@printf 'UI 预览地址：http://127.0.0.1:%s/#/entry\n' '$(UI_PORT)'
	$(PYTHON) ui/serve.py --port $(UI_PORT) --bind 127.0.0.1

ui-check: ## 检查 UI 设计源语法与文件格式
	node --check ui/assets/app.js
	node --check ui/assets/mock-data.js
	node --check ui/tests/ui-preview.spec.js
	PYTHONPYCACHEPREFIX=/tmp/deploy-go-pycache $(PYTHON) -m py_compile ui/serve.py
	@! rg -n '[[:blank:]]+$$' Makefile README.md docs ui
	git diff --check

ui-test: ## 执行 UI Playwright 交互回归
	npm run test:ui

admin: ## 启动 Web 管理端开发服务器
	npm run admin:dev

admin-check: ## 检查 Web 管理端
	npm run admin:check

admin-build: ## 构建 Web 管理端
	npm run build --workspace deploy-go-admin

check: api-check ui-check api-client-check admin-check ## 执行全仓检查

api-openapi: ## 生成 OpenAPI JSON 产物
	cargo run -p deploy-go-api -- openapi

api-openapi-check: ## 检查 OpenAPI JSON 产物
	cargo run -p deploy-go-api -- openapi-check

api-client-generate: ## 根据 OpenAPI 生成双端 API client
	npm run api:client:generate

api-client-check: ## 检查双端 API client 是否漂移
	npm run api:client:check
