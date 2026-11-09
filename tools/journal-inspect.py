#!/usr/bin/env python3
"""Show a labwc-tweaks save journal against the files on disk (read-only).
Usage: python3 tools/journal-inspect.py [CONFIG_DIR]   (default ~/.config/labwc)
Prints, per file: whether it matches the journal's old or new content, so you
can decide how to resolve a "Recovery paused" conflict (see docs/recovery.md).
Never modifies anything.
"""
import difflib, pathlib, sys, xml.etree.ElementTree as ET

d = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "~/.config/labwc").expanduser()
j = d / ".labwc-tweaks-journal"
if not j.is_file():
    sys.exit(f"no journal in {d}")
root = ET.parse(j).getroot()
print(f"journal state: {root.get('state', 'in progress (not committed)')}")
for e in root:
    name = e.get("name")
    old = bytes.fromhex(e.get("old")) if e.get("old") is not None else None
    new = bytes.fromhex(e.get("new", ""))
    p = d / name
    cur = p.read_bytes() if p.is_file() else None
    status = "matches OLD" if cur == old else "matches NEW" if cur == new else "CHANGED BY ANOTHER PROGRAM"
    print(f"\n== {name}: {status}")
    if status.startswith("CHANGED"):
        a = (old or b"").decode(errors="replace").splitlines()
        b = (cur or b"").decode(errors="replace").splitlines()
        print("\n".join(difflib.unified_diff(a, b, "journal-old", "on-disk", lineterm="")))
