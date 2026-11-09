# Labwc Tweaks GTK4

An independent project, not affiliated with or endorsed by labwc. See
[provenance and notices](docs/provenance.md).

A lean native GTK4 configuration editor for the labwc compositor, written in
Rust with separate configuration and UI modules. GPL-2.0-only.

## Build and run

Requires Rust 1.89 or newer, pkg-config and GTK4 development files.

    cargo build --release
    cargo run --release
    cargo test

For an isolated configuration:

    LABWC_CONFIG_DIR=/tmp/labwc-preview cargo run

Runtime Rust dependencies are gtk4 and xmltree (xml-rs). GLib and GIO are
accessed through gtk4 reexports. No libadwaita, async runtime, serialization
framework, shell scripts or external settings helper is required.
tempfile is used only by tests.

## Supported versions

Tested with labwc 0.20.3 / wlroots 0.20.2 and GTK 4.24.1 on Rust stable; CI also builds
on the declared minimum, Rust 1.89. Other labwc versions work as far as their
configuration format matches; settings a compositor does not know are ignored by it.

## Behavior

Appearance, focus/snapping, mouse/touchpad and keyboard settings are staged
until Apply. Closing with unsaved changes asks before discarding them.
Files are checked for external edits before saving. Each modified file gets a
.bak backup and is replaced atomically. A lock and durable recovery journal
coordinate all changed files and restore interrupted writes. XML formatting can change; unrelated elements, attributes and
comments are retained. Existing custom acceleration curves remain intact.

Uses LABWC_CONFIG_DIR, otherwise XDG_CONFIG_HOME/labwc or ~/.config/labwc.
When rc.xml is absent, the first system file in XDG_CONFIG_DIRS is inherited.
An invalid file is never silently replaced. Input settings target the default
device category; Collections also edits category/named device profiles. Environment changes
may require logging out and back in. Applying invokes labwc -r asynchronously;
reload failures are reported independently from save failures.

Install target/release/labwc-tweaks-gtk4 in your executable search path and
data/org.labwc.TweaksGtk4.desktop in your applications directory.

The settings covered by the upstream Qt release are implemented; see
[the parity matrix](docs/parity.md) and [issue/PR triage](docs/upstream-review.md)
for the exact baseline, included fixes and differences.

Reusable maintenance tools are saved in tools/:
- import-settings.py UPSTREAM_CHECKOUT regenerates the scalar inventory and
  available upstream label translations. Review generated changes, then run
  cargo fmt and cargo test.
- advisory-check.py checks Cargo.lock against a local RustSec database.
- journal-inspect.py explains a retained save journal; see [recovery](docs/recovery.md).
- package-check.py verifies the source package has no logs or bytecode.
- release-audit.py reproduces the release-review regressions in a snapshot.
- gui-smoke.py visits all 12 GTK pages on an isolated Xvfb display.
- nested-labwc.py tests collection dialogs, Apply, configuration saves and
  reconfiguration in an isolated windowed labwc. Logs stay in out/nested-review/.

Use -C DIRECTORY or --config-dir DIRECTORY to select a configuration directory.
For packaging, run make followed by make install DESTDIR=/path/to/package PREFIX=/usr.
The release profile uses size optimization, thin LTO and stripped symbols.

The editor exposes 89 scalar settings, structured editing for all 21 XML
sections, and an environment-file editor including environment.d fragments.
Collections supports adding/removing/reordering nested records and editing their
attributes and text. Changes are staged before Apply. Apply existing scalar edits
before staging the same section in Collections.

If a save is interrupted and recovery reports a conflict, see
[recovery](docs/recovery.md). Release history is in [CHANGELOG.md](CHANGELOG.md).

See [the configuration safety review](docs/safety-review.md) for coverage,
verification evidence, recovery behavior and filesystem/hardware limits.
