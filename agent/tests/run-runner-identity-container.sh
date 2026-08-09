#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
docker_arch="$(docker version --format '{{.Server.Arch}}')"
target_volume="deploy-go-runner-target-${docker_arch}-1_94_1"

docker run --rm --pull=never \
  -e DEPLOY_GO_RUNNER_IDENTITY_TEST=1 \
  -e CARGO_TARGET_DIR=/tmp/deploy-go-runner-target \
  -e RUSTUP_TOOLCHAIN=1.94.1 \
  -e 'RUSTFLAGS=-D warnings' \
  -v "$HOME/.cargo/registry:/usr/local/cargo/registry:ro" \
  -v "$target_volume:/tmp/deploy-go-runner-target" \
  -v "$repo_root:/workspace" \
  -w /workspace \
  rust:1.94-bookworm \
  cargo test --offline -p deploy-go-agent --test runner_identity_linux -- --nocapture
