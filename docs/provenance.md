# Provenance and notices

This is an independent project. It is not affiliated with or endorsed by the
labwc project or the authors of labwc-tweaks.

- **Code:** Rust, GPL-2.0-only. Setting names, defaults and ranges were derived
  from labwc-tweaks (Qt) at commit 0cfcec57f464f7099a7bfadb9862c53697094e52,
  also GPL-2.0-only, via `tools/import-settings.py`.
- **Translations:** `src/localization.rs` is generated from the translation
  catalogs of that commit (maintained upstream through Weblate by its
  translators). They carry the upstream GPL-2.0-only license. Only page and
  setting labels are translated; new editor text is English.
- **Icon:** `data/org.labwc.TweaksGtk4.svg` is original artwork created for
  this project (not derived from the labwc logo or upstream icons), released
  under GPL-2.0-only.
- **Dependencies:** gtk4 (MIT), xmltree (MIT), tempfile (tests only, MIT/Apache-2.0).
  `tools/advisory-check.py` checks them against RustSec; xml-rs (via xmltree)
  is currently flagged *unmaintained* (RUSTSEC-2022-0048), with no vulnerability.
