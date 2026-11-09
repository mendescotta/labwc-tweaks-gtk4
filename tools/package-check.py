#!/usr/bin/env python3
"""Fail if the source package contains local artifacts (logs, bytecode, out/, target/).
Usage: python3 tools/package-check.py   (run from anywhere; uses cargo package --list)
"""
import pathlib, re, subprocess, sys

root = pathlib.Path(__file__).resolve().parents[1]
listing = subprocess.run(
    ["cargo", "package", "--list", "--allow-dirty", "--locked", "--offline"],
    cwd=root, capture_output=True, text=True,
)
if listing.returncode:
    sys.exit(listing.stderr)
files = listing.stdout.split()
bad = [f for f in files if re.search(r"(^|/)(out|target|__pycache__)/|\.(pyc|log)$|exit-status$", f)]
for required in ("Cargo.toml", "Cargo.lock", "LICENSE", "src/main.rs", "data/org.labwc.TweaksGtk4.desktop"):
    if required not in files:
        bad.append(f"missing: {required}")
if bad:
    sys.exit("Unexpected package contents:\n  " + "\n  ".join(bad))
print(f"package ok ({len(files)} files)")
