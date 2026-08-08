#!/usr/bin/bash
set -euo pipefail

wasm-pack build crates/tanaid-wasm \
  --target nodejs \
  --out-dir pkg/node \
  --release

wasm-pack build crates/tanaid-wasm \
  --target web \
  --out-dir pkg/web \
  --release
