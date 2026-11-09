#!/usr/bin/env python3
"""Reproduce release-review regressions in a temporary source snapshot.
Usage: python3 tools/release-audit.py
Expected to fail until the reviewed defects are fixed. Does not edit project
sources or user config. Reuses target/ to avoid rebuilding GTK dependencies.
"""
import os
import pathlib
import shutil
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
TESTS = r'''
#[cfg(test)]
mod release_review {
 use super::*;
 #[test]
 fn changing_fragment_layout_clears_base_variant() {
 let d=tempfile::tempdir().unwrap();
 fs::write(d.path().join("rc.xml"),"<labwc_config/>").unwrap();
 fs::write(d.path().join("environment"),"XKB_DEFAULT_VARIANT=phonetic\n").unwrap();
 fs::create_dir(d.path().join("environment.d")).unwrap();
 fs::write(d.path().join("environment.d/90-layout.env"),"XKB_DEFAULT_LAYOUT=ru\n").unwrap();
 let mut c=Config::load(d.path().into()).unwrap();
 c.save(&BTreeMap::from([("XKB_DEFAULT_LAYOUT".into(),"us".into())])).unwrap();
 assert_eq!(c.get("XKB_DEFAULT_LAYOUT").as_deref(),Some("us"),"old variant must not survive a new layout");
 }
 #[test]
 fn upstream_accepted_fragment_filename_is_loaded() {
 let d=tempfile::tempdir().unwrap();
 fs::write(d.path().join("rc.xml"),"<labwc_config/>").unwrap();
 fs::write(d.path().join("environment"),"XCURSOR_SIZE=24\n").unwrap();
 fs::create_dir(d.path().join("environment.d")).unwrap();
 fs::write(d.path().join("environment.d/90 local.env"),"XCURSOR_SIZE=40\n").unwrap();
 let c=Config::load(d.path().into()).unwrap();
 assert_eq!(c.get("XCURSOR_SIZE").as_deref(),Some("40"),"labwc reads all regular *.env files");
 }
}
'''

def main():
    with tempfile.TemporaryDirectory(prefix="labwc-release-review-") as temporary:
        source = pathlib.Path(temporary)
        for name in ["Cargo.toml", "Cargo.lock"]:
            shutil.copy2(ROOT / name, source / name)
        shutil.copytree(ROOT / "src", source / "src")
        with (source / "src/config.rs").open("a") as stream:
            stream.write(TESTS)
        env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / "target"))
        result = subprocess.run(
            ["cargo", "test", "--locked", "--offline", "--manifest-path",
             str(source / "Cargo.toml"), "release_review", "--", "--test-threads=1"],
            env=env, timeout=180,
        )
        return result.returncode

if __name__ == "__main__":
    raise SystemExit(main())
