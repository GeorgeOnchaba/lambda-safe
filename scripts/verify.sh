#!/usr/bin/env sh
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_dir"

command -v cargo >/dev/null 2>&1 || {
  echo "error: cargo is required" >&2
  exit 70
}

if [ ! -f Cargo.lock ]; then
  echo "error: Cargo.lock is required for reproducible release verification" >&2
  exit 70
fi

cargo fmt --all --check
cargo fmt --manifest-path fuzz/Cargo.toml --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --release --locked
"$repo_dir/scripts/smoke-test.sh" "$repo_dir/target/release/lambda-safe" finite
"$repo_dir/scripts/smoke-test.sh" "$repo_dir/target/release/lambda-safe" z3

echo "all release checks completed"
