#!/bin/sh
# Build the zero-crate binary on first use, then run it: scripts/run.sh gather 7 | scripts/run.sh tokens 7
dir=$(cd "$(dirname "$0")/.." && pwd)
bin="$dir/target/release/wdygd"
if [ ! -x "$bin" ]; then
  command -v cargo >/dev/null 2>&1 || { echo "Rust is not installed. Get it at https://rustup.rs" >&2; exit 1; }
  cargo build --release --quiet --manifest-path "$dir/Cargo.toml" || exit 1
fi
exec "$bin" "$@"
