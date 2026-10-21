#!/bin/bash
# Benchmarks the tanaid CLI built from the working tree against a baseline commit and
# tclsh, running each bench/*.tcl script with hyperfine. Results go to target/bench/.
#
# usage: scripts/bench.sh [base-ref]
#   base-ref defaults to main; the baseline is its merge-base with HEAD.
# requires: hyperfine 2.x, tclsh
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

if ! hyperfine --version | grep -q '^hyperfine 2\.'; then
  echo "error: requires hyperfine 2.x (found: $(hyperfine --version))" >&2
  exit 1
fi

base=$(git merge-base HEAD "${1:-main}")
out=target/bench
mkdir -p "$out"

# Build the baseline in a temporary worktree with its own target dir. Sharing target/
# isn't safe: workspace crates hash the same at any path, so cargo can treat one
# tree's build as fresh for the other.
worktree=$(mktemp -d)
trap 'git worktree remove --force "$worktree" 2> /dev/null || rm -rf "$worktree"' EXIT
git worktree add --quiet --detach "$worktree" "$base"
(cd "$worktree" && cargo build --locked --release -p tanaid-cli --target-dir "$root/$out/base")
cargo build --locked --release -p tanaid-cli

base_bin=$out/base/release/tanaid
head_bin=target/release/tanaid

# Instruction counts are steadier than wall time, but need hardware counter access.
metrics=time_wall_clock:ms,memory_peak_resident:MiB
if hyperfine --runs 1 --metrics instructions true > /dev/null 2>&1; then
  metrics+=,instructions:M
fi

summary=$out/summary.md
cat > "$summary" << EOF
## Benchmarks

Compares this build against \`$(git rev-parse --short "$base")\` (base) and tclsh $(echo 'puts [info patchlevel]' | tclsh). Lower is better.
EOF
details=$(mktemp)

for script in bench/*.tcl; do
  name=$(basename "$script" .tcl)

  # tanaid also prints the script's result, which is an empty line after a final puts.
  if ! diff <(tclsh "$script" && echo) <("$head_bin" "$script"); then
    echo "error: $script output differs between tanaid and tclsh" >&2
    exit 1
  fi

  # hyperfine reports changes relative to the first command.
  hyperfine --warmup 1 --metrics "$metrics" \
    --export-markdown "$out/$name.md" --export-json "$out/$name.json" \
    -n base -n this -n tclsh \
    "$base_bin $script" "$head_bin $script" "tclsh $script" \
    | tee "$out/$name.txt"

  printf '\n### %s\n\n' "$name" >> "$summary"
  cat "$out/$name.md" >> "$summary"
  printf '\n%s\n' "$name" >> "$details"
  cat "$out/$name.txt" >> "$details"
done

{
  printf '\n<details><summary>All metrics</summary>\n\n```\n'
  cat "$details"
  printf '```\n\n</details>\n'
} >> "$summary"
rm "$details"

echo "Summary written to $out/summary.md"
