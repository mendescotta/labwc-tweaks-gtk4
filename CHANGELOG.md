# Changelog

## 0.1.0

First public release.

- 89 scalar settings across Appearance, Behaviour, Mouse, Keyboard and
  Touchscreen, plus structured editing of all 21 labwc XML sections
  (Collections) and an environment / environment.d editor.
- Staged edits with Apply; atomic multi-file saves with a lock, durable
  recovery journal and `.bak` backups; external-edit detection.
- Warnings before discarding unstaged environment text, collection edits or
  staged changes; explicit Save As when staging an environment file under a
  different name.
- Fixed: changing a fragment-supplied keyboard layout kept the base-file
  variant; environment.d fragments with names labwc reads (spaces, leading
  dot, non-ASCII) were ignored.
- Original application icon.
