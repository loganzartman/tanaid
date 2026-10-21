#!/bin/bash
# Usage: scripts/bench.sh [base-ref=main] (requires hyperfine 2.x, tclsh, Python 3).
# Compares the working tree, its merge-base with base-ref, and tclsh.
# Results go to target/bench/. On headless Linux, run under xvfb-run -a.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

base=$(git merge-base HEAD "${1:-main}")
out=target/bench
mkdir -p "$out"

# Separate target dirs keep Cargo from reusing one tree's build for the other.
worktree=$(mktemp -d)
trap 'git worktree remove --force "$worktree"' EXIT
git worktree add --quiet --detach "$worktree" "$base"
(cd "$worktree" && cargo build --locked --release -p tanaid-cli --target-dir "$root/$out/base")
cargo build --locked --release -p tanaid-cli --target-dir target

base_bin=$out/base/release/tanaid
head_bin=target/release/tanaid

tcl_version=$(echo 'puts [info patchlevel]' | tclsh)
summary=$out/summary.md
cat > "$summary" << EOF
## Benchmarks

Compares this build against \`$(git rev-parse --short "$base")\` (base) and tclsh $tcl_version. Lower is better.

Values are mean ± standard deviation. Time changes include a 95% bootstrap confidence interval:
an interval containing zero does not establish a difference. Intervals describe run-to-run variation,
not machine drift between hyperfine's sequential command batches. Small changes on shared runners
need confirmation. Instruction counts are less sensitive to scheduling and CPU frequency, but do not measure elapsed time.
Unavailable counters are reported explicitly.
EOF

for script in bench/*.tcl; do
  name=$(basename "$script" .tcl)
  echo "$name"

  # $(...) trims the empty result line tanaid prints after the final puts.
  expected=$(tclsh "$script")
  for bin in "$base_bin" "$head_bin"; do
    actual=$("$bin" "$script")
    if [ "$actual" != "$expected" ]; then
      echo "error: $script prints '$actual' on $bin but '$expected' on tclsh" >&2
      exit 1
    fi
  done

  # JSON includes all available counters even when the console only shows time and memory.
  hyperfine --warmup 5 --min-runs 30 --export-json "$out/$name.json" \
    -n base -n this -n tclsh "$base_bin $script" "$head_bin $script" "tclsh $script"

  printf '\n### %s\n\n' "$name" >> "$summary"
  python3 scripts/bench-summary.py "$out/$name.json" >> "$summary"
done

echo "Summary written to $summary"
