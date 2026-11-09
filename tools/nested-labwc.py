#!/usr/bin/env python3
"""Test the editor and configuration reload inside an isolated windowed labwc.
Usage: python3 tools/nested-labwc.py
Requires an X11 desktop, labwc and cargo. Logs stay in out/nested-review/.
The compositor receives only a temporary config, environment and runtime directory.
"""
import argparse
import os
import pathlib
import shlex
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]

def client(report):
    env = dict(os.environ, GDK_BACKEND="wayland", GSK_RENDERER="cairo")
    result = subprocess.run(
        ["cargo", "test", "--offline", "nested_live", "--", "--ignored", "--test-threads=1"],
        cwd=ROOT, env=env, timeout=120,
    )
    pathlib.Path(report).write_text(str(result.returncode))
    return result.returncode

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--client")
    args = parser.parse_args()
    if args.client:
        return client(args.client)
    output = ROOT / "out/nested-review"
    output.mkdir(parents=True, exist_ok=True)
    report = output / "exit-status"
    report.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory(prefix="labwc-nested-") as temporary:
        base = pathlib.Path(temporary)
        config = base / "config"
        runtime = base / "runtime"
        config.mkdir()
        runtime.mkdir(mode=0o700)
        (config / "rc.xml").write_text(
            "<labwc_config><keyboard><default/></keyboard><mouse><default/></mouse></labwc_config>"
        )
        (config / "environment").write_text("")
        (config / "menu.xml").write_text(
            '<openbox_menu><menu id="root-menu" label="Test"><item label="Test">'
            '<action name="None"/></item></menu></openbox_menu>'
        )
        env = dict(os.environ, WLR_BACKENDS="x11", WLR_RENDERER="pixman",
                   WLR_X11_OUTPUTS="1", XDG_RUNTIME_DIR=str(runtime),
                   LABWC_CONFIG_DIR=str(config), LABWC_TWEAKS_TEST_CONFIG=str(config),
                   XDG_CONFIG_HOME=str(base))
        for name in ["WAYLAND_DISPLAY", "LABWC_PID", "LABWC_VER"]:
            env.pop(name, None)
        session = shlex.join(["python3", str(pathlib.Path(__file__).resolve()),
                              "--client", str(report)])
        with (output / "session.log").open("w") as log:
            process = subprocess.Popen(
                ["labwc", "-V", "-C", str(config), "-t", "Labwc Tweaks GTK4 isolated test",
                 "-S", session], env=env, stdout=log, stderr=subprocess.STDOUT,
            )
            try:
                code = process.wait(timeout=150)
            except subprocess.TimeoutExpired:
                process.terminate()
                process.wait(timeout=10)
                raise
        if code or not report.exists() or report.read_text() != "0":
            raise SystemExit(f"Nested test failed; see {output / 'session.log'}")
        log_text = (output / "session.log").read_text()
        if log_text.count("read config file") < 2:
            raise SystemExit("Expected initial config load and reload; inspect session.log")
        if any("[ERROR]" in line and ("rcxml" in line or "config/" in line) for line in log_text.splitlines()):
            raise SystemExit("Compositor reported a configuration error; inspect session.log")
        print(f"Nested labwc test passed. Evidence: {output / 'session.log'}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
