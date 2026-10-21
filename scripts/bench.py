#!/usr/bin/env python3
"""Compare the working tree, its merge-base with base-ref, and tclsh.

Requires hyperfine 2.x, Valgrind, tclsh, and Cargo. Results go to target/bench/.
On headless Linux, run under xvfb-run -a.
"""

import argparse
import json
import os
import random
import shlex
import subprocess
import sys
import tempfile
from contextlib import redirect_stdout
from pathlib import Path
from statistics import fmean, quantiles


def change_interval(base, current):
    """Percentile bootstrap CI for the relative change in independent sample means."""
    rng = random.Random(0)
    changes = [
        100 * (fmean(rng.choices(current, k=len(current)))
               / fmean(rng.choices(base, k=len(base))) - 1)
        for _ in range(10000)
    ]
    bounds = quantiles(changes, n=40, method="inclusive")
    return bounds[0], bounds[-1]


def report(results, counts):
    base = results[0]
    base_times = [m["time_wall_clock"]["value"] for m in base["measurements"]]
    print("| Command | Wall time (ms) | Time change (95% CI) | Cachegrind instructions (M) | Instruction change | Runs |")
    print("|:---|---:|---:|---:|---:|---:|")
    for result, count in zip(results, counts, strict=True):
        timing = result["summary"]["time_wall_clock"]
        time_change = instruction_change = "—"
        if result is not base:
            times = [m["time_wall_clock"]["value"] for m in result["measurements"]]
            low, high = change_interval(base_times, times)
            delta = 100 * (timing["mean"] / base["summary"]["time_wall_clock"]["mean"] - 1)
            time_change = f"{delta:+.2f}% [{low:+.2f}%, {high:+.2f}%]"
            instruction_change = f'{100 * (count / counts[0] - 1):+.2f}%'
        print(f'| `{result.get("name", result["command"])}` '
              f'| {timing["mean"] * 1000:.1f} ± {timing["stddev"] * 1000:.1f} '
              f'| {time_change} | {count / 1e6:.2f} | {instruction_change} | {timing["count"]} |')


def run(*args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def output(*args, **kwargs):
    return run(*args, stdout=subprocess.PIPE, **kwargs).stdout.rstrip("\n")


def count_instructions(binary, script, destination):
    run("valgrind", "--tool=cachegrind", "--quiet", "--cache-sim=no", "--branch-sim=no",
        f"--cachegrind-out-file={destination}", binary, str(script), stdout=subprocess.DEVNULL)
    lines = destination.read_text().splitlines()
    events = next(line.split()[1:] for line in lines if line.startswith("events:"))
    totals = next(line.split()[1:] for line in lines if line.startswith("summary:"))
    count = int(totals[events.index("Ir")])
    if count <= 0:
        raise ValueError(f"No instructions recorded in {destination}")
    return count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base_ref", nargs="?", default="main")
    args = parser.parse_args()
    os.chdir(output("git", "rev-parse", "--show-toplevel"))
    root = Path.cwd()
    base = output("git", "merge-base", "HEAD", args.base_ref)
    out = Path("target/bench")
    out.mkdir(parents=True, exist_ok=True)

    # Separate target dirs keep Cargo from reusing one tree's build for the other.
    with tempfile.TemporaryDirectory() as worktree:
        run("git", "worktree", "add", "--quiet", "--detach", worktree, base)
        try:
            run("cargo", "build", "--locked", "--release", "-p", "tanaid-cli",
                "--target-dir", str(root / out / "base"), cwd=worktree)
        finally:
            run("git", "worktree", "remove", "--force", worktree)
    run("cargo", "build", "--locked", "--release", "-p", "tanaid-cli",
        "--target-dir", str(out / "head"))

    binaries = [str(out / "base/release/tanaid"), str(out / "head/release/tanaid"), "tclsh"]
    tcl_version = output("tclsh", input="puts [info patchlevel]\n")
    short_base = output("git", "rev-parse", "--short", base)
    summary_path = out / "summary.md"
    with summary_path.open("w") as summary:
        summary.write(f"""## Benchmarks

Compares this build against `{short_base}` (base) and tclsh {tcl_version}. Lower is better.

Values are mean ± standard deviation. Time changes include a 95% bootstrap confidence interval:
an interval containing zero does not establish a difference. Intervals describe run-to-run variation,
not machine drift between hyperfine's sequential command batches. Small changes on shared runners
need confirmation. Instructions come from one separate Cachegrind run per command, including startup.
These are instrumented guest-instruction counts, not hardware counters or elapsed time.
""")
        for script in sorted(Path("bench").glob("*.tcl")):
            print(script.stem, flush=True)
            expected = output("tclsh", str(script))
            for binary in binaries[:2]:
                # Trim the empty result line tanaid prints after the final puts.
                actual = output(binary, str(script))
                if actual != expected:
                    raise ValueError(f"{script} prints {actual!r} on {binary} but {expected!r} on tclsh")

            results_path = out / f"{script.stem}.json"
            run("hyperfine", "--warmup", "5", "--min-runs", "30",
                "--export-json", str(results_path), "-n", "base", "-n", "this", "-n", "tclsh",
                *(shlex.join([binary, str(script)]) for binary in binaries))
            counts = [
                count_instructions(binary, script, out / f"{script.stem}.{label}.cachegrind")
                for label, binary in zip(("base", "this", "tclsh"), binaries)
            ]
            summary.write(f"\n### {script.stem}\n\n")
            with results_path.open() as source, redirect_stdout(summary):
                report(json.load(source)["results"], counts)
    print(f"Summary written to {summary_path}")


if __name__ == "__main__":
    try:
        main()
    except (subprocess.CalledProcessError, OSError, ValueError) as error:
        sys.exit(f"error: {error}")
