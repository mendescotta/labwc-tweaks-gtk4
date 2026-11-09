#!/usr/bin/env python3
"""Check Cargo.lock against a local RustSec advisory-db checkout (offline cargo-audit stand-in).
Usage: python3 tools/advisory-check.py [ADVISORY_DB]   (default ~/.cargo/advisory-db)
Update the database first with: git -C ~/.cargo/advisory-db pull --ff-only
Exit 1 on a vulnerability; informational advisories (unmaintained/unsound) are
reported as warnings. Does not execute project or advisory code.
"""
import pathlib, re, subprocess, sys, tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
DB = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "~/.cargo/advisory-db").expanduser()

def ver(s):
    core = re.split(r"[-+]", s, maxsplit=1)[0]
    return tuple(int(x) for x in (core.split(".") + ["0", "0"])[:3])

def satisfies(v, req):
    v = ver(v)
    for part in (p.strip() for p in req.split(",")):
        m = re.fullmatch(r"(>=|<=|>|<|=|\^|~)?\s*(\d+(?:\.\d+){0,2}.*)", part)
        if not m:
            raise ValueError(f"unparsed requirement {req!r}")
        op, r = m.group(1) or "^", ver(m.group(2))
        if op == "^":
            # ^0.x.y pins the leftmost non-zero component
            n = next((i for i, c in enumerate(r) if c), 2)
            ok = v >= r and v[: n + 1] == r[: n + 1]
        elif op == "~":
            ok = v >= r and v[:2] == r[:2]
        else:
            ok = {">=": v >= r, "<=": v <= r, ">": v > r, "<": v < r, "=": v == r}[op]
        if not ok:
            return False
    return True

def main():
    if not (DB / "crates").is_dir():
        sys.exit(f"advisory database not found at {DB}")
    head = subprocess.run(["git", "-C", str(DB), "log", "-1", "--format=%cs"], capture_output=True, text=True).stdout.strip()
    packages = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
    failures = warnings = 0
    for pkg in packages:
        for f in (DB / "crates" / pkg["name"]).glob("RUSTSEC-*.md") if (DB / "crates" / pkg["name"]).is_dir() else []:
            toml = re.search(r"```toml\n(.*?)\n```", f.read_text(), re.S)
            adv = tomllib.loads(toml.group(1))
            patched = adv.get("versions", {}).get("patched", []) + adv.get("versions", {}).get("unaffected", [])
            try:
                fixed = any(satisfies(pkg["version"], r) for r in patched)
            except ValueError as e:
                print(f"WARN cannot evaluate {f.stem} for {pkg['name']}: {e}")
                warnings += 1
                continue
            if fixed:
                continue
            info = adv["advisory"].get("informational")
            print(f"{'WARN' if info else 'VULN'} {f.stem} {pkg['name']} {pkg['version']}: {info or 'vulnerability'} ({adv['advisory'].get('url', '')})")
            warnings, failures = warnings + bool(info), failures + (not info)
    print(f"checked {len(packages)} crates against advisory-db {head}: {failures} vulnerabilities, {warnings} warnings")
    return 1 if failures else 0

if __name__ == "__main__":
    raise SystemExit(main())
