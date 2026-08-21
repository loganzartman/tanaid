#!/usr/bin/env python3
"""Fail if Cargo.lock pins a crates.io version that is too freshly published.

A malicious release is most dangerous in the hours after it is published, before
anyone has looked at it: `arrayref` 0.3.10 shipped build-time malware and was
taken down the same day. Waiting a week before adopting a version means someone
else finds it first.

Cargo can enforce this itself (RFC 3923, `registry.global-min-publish-age` in
.cargo/config.toml), but only on cargo versions that implement it. This checks
the same rule against the lockfile, so it holds on the pinned toolchain and in
CI, and it covers transitive dependencies — which is how `arrayref` arrived.

Publish times come from the sparse index, the same source cargo resolves from.
A version the index doesn't know is reported too: that is what a crate looks
like after crates.io removes it.
"""

import argparse
import json
import re
import sys
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timedelta, timezone

INDEX_URL = "https://index.crates.io"
CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
PACKAGE_RE = re.compile(r"^\[\[package\]\]$", re.MULTILINE)
FIELD_RE = re.compile(r'^(name|version|source)\s*=\s*"([^"]*)"$', re.MULTILINE)


def parse_lockfile(path):
    """Yields (name, version) for every crates.io package in a Cargo.lock."""
    text = path.read_text()
    for block in PACKAGE_RE.split(text)[1:]:
        fields = dict(FIELD_RE.findall(block))
        # path and git dependencies aren't published, so they have no publish time
        if fields.get("source") != CRATES_IO_SOURCE:
            continue
        if "name" in fields and "version" in fields:
            yield fields["name"], fields["version"]


def index_path(name):
    """The sparse index lays crates out by name length; see the cargo book."""
    name = name.lower()
    if len(name) == 1:
        return f"1/{name}"
    if len(name) == 2:
        return f"2/{name}"
    if len(name) == 3:
        return f"3/{name[0]}/{name}"
    return f"{name[:2]}/{name[2:4]}/{name}"


def published_at(name, version):
    """The publish time of one version, or None if the index has no such version."""
    url = f"{INDEX_URL}/{index_path(name)}"
    try:
        with urllib.request.urlopen(url, timeout=30) as response:
            body = response.read().decode()
    except urllib.error.HTTPError as err:
        if err.code == 404:
            return None
        raise

    for line in body.splitlines():
        if not line.strip():
            continue
        entry = json.loads(line)
        if entry.get("vers") == version:
            pubtime = entry.get("pubtime")
            if not pubtime:
                # published before the index carried timestamps, so older than
                # any cooldown worth enforcing
                return datetime.min.replace(tzinfo=timezone.utc)
            # fromisoformat only learned to read a trailing "Z" in Python 3.11
            return datetime.fromisoformat(pubtime.replace("Z", "+00:00"))
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--min-age-days",
        type=int,
        default=7,
        help="how long a version must have been published (default: 7)",
    )
    parser.add_argument(
        "--lockfile",
        default="Cargo.lock",
        help="path to the lockfile to check (default: Cargo.lock)",
    )
    args = parser.parse_args()

    from pathlib import Path

    lockfile = Path(args.lockfile)
    if not lockfile.is_file():
        sys.exit(f"no lockfile at {lockfile}")

    packages = sorted(set(parse_lockfile(lockfile)))
    cutoff = datetime.now(timezone.utc) - timedelta(days=args.min_age_days)

    def check(package):
        name, version = package
        return package, published_at(name, version)

    too_fresh, unknown = [], []
    with ThreadPoolExecutor(max_workers=16) as pool:
        for (name, version), pubtime in pool.map(check, packages):
            if pubtime is None:
                unknown.append((name, version))
            elif pubtime > cutoff:
                too_fresh.append((name, version, pubtime))

    for name, version in unknown:
        print(f"NOT IN INDEX  {name} {version} — removed from crates.io, or renamed")
    for name, version, pubtime in sorted(too_fresh, key=lambda row: row[2], reverse=True):
        age = datetime.now(timezone.utc) - pubtime
        print(f"TOO FRESH     {name} {version} — published {age.days}d ago ({pubtime:%Y-%m-%d})")

    if too_fresh or unknown:
        print(
            f"\n{len(packages)} crates.io packages checked, "
            f"{len(too_fresh) + len(unknown)} rejected "
            f"(minimum age {args.min_age_days}d).\n"
            "Wait for the cooldown, pin an older version, or — if you have read the "
            "release and want it anyway —\n"
            "  CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow cargo update -p <crate>"
        )
        return 1

    print(f"{len(packages)} crates.io packages checked, all at least {args.min_age_days}d old.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
