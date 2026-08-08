#!/bin/bash
set -euo pipefail

rm -rf crates/tanaid-wasm/pkg

wasm-pack build crates/tanaid-wasm \
  --target bundler \
  --out-dir pkg/bundler \
  --no-pack \
  --release
