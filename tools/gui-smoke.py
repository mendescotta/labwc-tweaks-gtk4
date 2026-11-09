#!/usr/bin/env python3
"""Run the GTK page smoke test in an isolated Xvfb display.
Usage: python3 tools/gui-smoke.py
Requires Xvfb and cargo. Does not touch the user's labwc configuration.
"""
import os
import pathlib
import select
import subprocess
import tempfile

root = pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="labwc-gtk-test-") as temporary:
    read_fd, write_fd = os.pipe()
    process = None
    try:
        process = subprocess.Popen(
            ["Xvfb", "-displayfd", str(write_fd), "-screen", "0", "1024x768x24", "-nolisten", "tcp"],
            pass_fds=(write_fd,), stdout=subprocess.DEVNULL,
        )
        os.close(write_fd)
        write_fd = None
        if not select.select([read_fd], [], [], 10)[0]:
            raise RuntimeError("Xvfb did not report a display within 10 seconds")
        display = os.read(read_fd, 64).decode().strip()
        if not display.isdigit():
            raise RuntimeError("Invalid display number from Xvfb")
        env = dict(os.environ, DISPLAY=":" + display, GDK_BACKEND="x11",
                   GSK_RENDERER="cairo", XDG_CONFIG_HOME=temporary)
        subprocess.run(
            ["cargo", "test", "--offline", "gtk_pages", "--", "--ignored", "--test-threads=1"],
            cwd=root, env=env, check=True, timeout=180,
        )
    finally:
        os.close(read_fd)
        if write_fd is not None:
            os.close(write_fd)
        if process is not None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
