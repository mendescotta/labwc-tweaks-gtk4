# Public release review — 2026-11-09

> **Status (follow-up, same day):** all five findings below are fixed — see
> "Resolution" at the end. The original review text is kept for the record.

Verdict: hold the public release. The current smoke/regression checks do not
cover several configuration and draft-state failures. This review does not
change application behavior.

## Findings, by priority

### P1 — changing a fragment-supplied layout can retain an incompatible variant

src/config.rs:446 updates the supplying fragment and continues without marking
the base environment changed. Lines 466–491 clear XKB_DEFAULT_VARIANT in the
in-memory base file, but lines 502–508 omit that file from the transaction when
env_changed is false.

Reproduced: base environment has XKB_DEFAULT_VARIANT=phonetic, fragment has
XKB_DEFAULT_LAYOUT=ru, and the user selects us. After save, the effective setting
reported by Config is us(phonetic), not us. That variant is inappropriate for
the new layout and can break keyboard configuration.

Required: include every modified file in the transaction, including base-file
variant clearing after the fragment path. Test base/fragment combinations.

### P2 — upstream-valid environment fragment names are silently ignored

src/storage.rs:86–93 restricts names to ASCII letters, digits, dash, underscore
and dot, excluding names beginning with dot. src/config.rs:76–82 applies this
write-target policy to discovery. labwc's session.c reads regular files whose
names end in .env; it does not impose this filename restriction.

Reproduced: base XCURSOR_SIZE=24 and environment.d/90 local.env sets 40.
The editor reports 24, while labwc reads the overriding fragment. A subsequent
edit may appear successful while the real fragment continues overriding it.

Required: safely support all single-component filenames accepted by labwc,
or show unsupported overriding files and block misleading writes.

### P2 — environment buffer can be staged against the wrong filename

src/environment.rs:69–92 loads a buffer independently from the editable filename.
Lines 105–112 stage that existing buffer under whatever filename is currently
typed. Changing A to the name of existing file B and pressing Stage without
Load can overwrite B with A's assignments when Apply is confirmed.

Code-inspection finding. Required: bind the buffer to its loaded file identity;
make switching/loading and Save As explicit. Confirm replacement of another
existing file.

### P2 — unstaged drafts can be discarded without a warning

Environment buffer changes do not enter the shared changes map until Stage.
The main close guard (src/ui.rs:329–350) only checks that map, and Apply rebuilds
the entire window (lines 289–294). Editing environment text, changing a scalar,
then applying the scalar destroys the unstaged text. Load also replaces a draft
without a dirty-state check. Collection dialogs similarly close without checking
their local edited tree.

Code-inspection finding. Required: track draft dirtiness independently of staged
changes and warn/preserve drafts on close, load and main-window rebuild.

### P2 — source package currently contains local test artifacts

cargo package --list --allow-dirty includes:
- out/nested-review/session.log
- out/nested-review/exit-status
- tools/__pycache__/release-audit.cpython-314.pyc

.gitignore alone did not exclude these in this workspace's package listing.
Required: explicit Cargo include/exclude rules and a source-package content check.
Do not publish local runtime logs or bytecode.

## Evidence

- Existing cargo test --locked --offline: 14 passed, 2 graphical tests ignored.
- tools/release-audit.py: two added regression expectations fail, confirming the
  keyboard variant and fragment-filename defects. The script uses a temporary
  source snapshot; project sources and personal configuration are untouched.
- desktop-file-validate passes.
- make install into a temporary DESTDIR succeeds for binary, desktop entry and icon.
- Linux dependency metadata reports no package declaring a Rust minimum above
  the advertised 1.89. An actual Rust 1.89 build was not performed.
- Previous nested labwc evidence remains available in out/nested-review/session.log.
- No current dependency vulnerability audit was performed (cargo-audit unavailable).

## Release follow-ups

Add CI for the ordinary tests, graphical smoke tests, nested reload checks and
source-package contents. Record supported labwc/GTK versions and test the stated
minimum Rust version. Document how to resolve a journal recovery conflict.
Include upstream icon/translation provenance and make the independent project's
identity clear in public metadata and the desktop label. Audit dependencies
against current advisories before tagging.

The advanced collection editor remains a general XML structure editor, not a
complete semantic validator for every labwc action or hardware setting. Public
documentation should retain that distinction.

## Resolution

- P1 variant retention: any change to the base `environment` caused by clearing
  `XKB_DEFAULT_VARIANT` now marks it changed, so it is part of the transaction.
  Tests cover base/fragment combinations (config.rs `collection_tests`).
- P2 fragment names: discovery and writes accept every single-component name
  ending in `.env` (spaces, leading dot, non-ASCII); separators and control
  characters stay refused. Non-UTF-8 names cannot be represented, so they are
  listed on the Environment page and block environment edits until renamed.
- P2 wrong-file staging: the buffer is bound to the file it was loaded from.
  Staging under another name is a Save As and asks before replacing an existing
  file (including one already staged). Load asks before discarding edited text.
- P2 draft loss: unstaged environment text and edited collection dialogs are
  tracked (`src/drafts.rs`). Closing the window, Apply (which rebuilds the
  window) and closing a collection dialog warn first.
- P2 package contents: Cargo `include` list limits the package to sources,
  data, docs and tools; `tools/package-check.py` verifies it.
- Follow-ups done: `.github/workflows/ci.yml` runs fmt, clippy, tests, the GTK
  smoke test, the package check and desktop-file-validate on Rust 1.89 and
  stable (this also exercises the declared minimum Rust version).
- Advisories: `tools/advisory-check.py` against RustSec (db updated 2026-11-09):
  87 crates, 0 vulnerabilities, 1 warning (xml-rs unmaintained, via xmltree).
- Recovery conflicts: documented in `docs/recovery.md`, with the read-only
  `tools/journal-inspect.py`.
- Identity: desktop entry, window title and About now say "(GTK4)" and
  "independent project"; provenance in `docs/provenance.md`.
- Icon provenance: replaced with original artwork (see `docs/provenance.md`).

## Second audit

An independent read of all sources found two further defects, both fixed with
tests: custom-curve step/points were written as child elements instead of
attributes (config.rs), and editing an element's text in a collection dialog
shifted child indices, which could panic or act on the wrong row (structured.rs).
Attribute order is now preserved (xmltree `attribute-order`), so unrelated saves
no longer reorder attributes, and a section that fails the editor's validation
reports why instead of silently not opening. 20 unit tests, the GTK smoke test
and the nested labwc 0.20.3 test pass.
