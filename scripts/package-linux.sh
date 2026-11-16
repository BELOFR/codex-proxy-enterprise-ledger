#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -m)" != "x86_64" || "$(uname -s)" != "Linux" ]]; then
  echo '此脚本需要 Linux x86_64，请在 Windows 使用 package.ps1' >&2
  exit 1
fi
export RUST_MIN_STACK=16777216
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/backend/target-linux}"
cargo fmt --manifest-path backend/Cargo.toml --check
cargo test --manifest-path backend/Cargo.toml --locked
cargo clippy --manifest-path backend/Cargo.toml --all-targets --all-features --locked -- -D warnings
cargo build --manifest-path backend/Cargo.toml --release --locked
sdk_rev=0f5bf03e13a43b34c43b9e3a6dee935ad43f6d2b
if [[ -z "${PLUGIN_CLI:-}" ]]; then
  cargo install --locked --git https://github.com/zyycn/codex-proxy-rs.git --rev "$sdk_rev" codex-proxy-plugin-cli --root "$PWD/.tools/linux-cli"
  PLUGIN_CLI="$PWD/.tools/linux-cli/bin/cpr-plugin"
fi
"$PLUGIN_CLI" package --manifest plugin.json --binary "$CARGO_TARGET_DIR/release/enterprise-ledger" --target x86_64-unknown-linux-gnu --resource-map web=frontend/dist --output-dir dist
