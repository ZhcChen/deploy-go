.DEFAULT_GOAL := help

PYTHON ?= python3
UI_PORT ?= 8050

.PHONY: help api-run api-migrate credential-reencrypt api-test api-check ui ui-serve ui-check check

help: ## 显示可用命令
	@printf '%s\n' \
		'可用命令：' \
		'  make api-run   启动 Rust API（默认 http://127.0.0.1:8080）' \
		'  make api-migrate 执行 SQLite migration 后退出' \
		'  make credential-reencrypt 离线重加密 SSH 凭证' \
		'  make api-test  执行 API 测试' \
		'  make api-check 检查 Rust 格式、clippy 和测试' \
		'  make ui        启动 UI 设计源预览（http://127.0.0.1:$(UI_PORT)）' \
		'  make ui-serve  与 make ui 相同' \
		'  make ui-check  检查 UI 设计源语法与文件格式' \
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

check: api-check ui-check ## 执行全仓检查
