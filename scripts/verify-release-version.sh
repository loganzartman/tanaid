#!/bin/bash
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 <version>" >&2
  exit 1
fi

tag_version="$1"
cargo_version=$(awk -F'"' '/^version/{print $2; exit}' Cargo.toml)
npm_version=$(node -p "require('./packages/tanaid/package.json').version")

echo "tag=$tag_version cargo=$cargo_version npm=$npm_version"

status=0
if [ "$tag_version" != "$cargo_version" ]; then
  echo "::error::Cargo workspace version ($cargo_version) does not match tag ($tag_version)"
  status=1
fi
if [ "$tag_version" != "$npm_version" ]; then
  echo "::error::packages/tanaid version ($npm_version) does not match tag ($tag_version)"
  status=1
fi
exit $status
