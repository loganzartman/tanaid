"""Render hyperfine 2.x JSON as a table with instruction counts and timing uncertainty."""

import json
import random
import sys
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


def instructions(result):
    metric = result["summary"].get("instructions")
    # Do not compare partial measurements or unavailable counters reported as zero.
    if metric and metric["count"] == len(result["measurements"]) and metric["mean"] > 0:
        return metric
    return None


def report(results):
    base = results[0]
    base_times = [m["time_wall_clock"]["value"] for m in base["measurements"]]
    base_instructions = instructions(base)
    print("| Command | Wall time (ms) | Time change (95% CI) | Instructions (M) | Instruction change | Runs |")
    print("|:---|---:|---:|---:|---:|---:|")
    for result in results:
        timing = result["summary"]["time_wall_clock"]
        time_change = instruction_change = "—"
        counter = instructions(result)
        counts = "unavailable"
        if counter:
            counts = f'{counter["mean"] / 1e6:.2f} ± {counter["stddev"] / 1e6:.2f}'
        if result is not base:
            times = [m["time_wall_clock"]["value"] for m in result["measurements"]]
            low, high = change_interval(base_times, times)
            delta = 100 * (timing["mean"] / base["summary"]["time_wall_clock"]["mean"] - 1)
            time_change = f"{delta:+.2f}% [{low:+.2f}%, {high:+.2f}%]"
            if counter and base_instructions:
                instruction_change = f'{100 * (counter["mean"] / base_instructions["mean"] - 1):+.2f}%'
        print(f'| `{result.get("name", result["command"])}` '
              f'| {timing["mean"] * 1000:.1f} ± {timing["stddev"] * 1000:.1f} '
              f'| {time_change} | {counts} | {instruction_change} | {timing["count"]} |')


if __name__ == "__main__":
    with open(sys.argv[1]) as source:
        report(json.load(source)["results"])
