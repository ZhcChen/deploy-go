#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

docker run --rm --privileged --cgroupns=private \
  -e DEPLOY_GO_RUN_CGROUP_V2_TEST=1 \
  -e CARGO_TARGET_DIR=/tmp/deploy-go-target \
  -e RUSTUP_TOOLCHAIN=1.94.1 \
  -e 'RUSTFLAGS=-D warnings' \
  -v "$HOME/.cargo/registry:/usr/local/cargo/registry:ro" \
  -v deploy-go-cgroup-target:/tmp/deploy-go-target \
  -v "$repo_root:/workspace" \
  -w /workspace \
  rust:1.94-bookworm \
  cargo test --offline -p deploy-go-agent-executor --test cgroup_v2_lifecycle -- --nocapture
