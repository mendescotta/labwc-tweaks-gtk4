# Qt settings parity

Baseline: labwc-tweaks 0cfcec57f464f7099a7bfadb9862c53697094e52.

| Page | Implemented |
| --- | --- |
| Appearance | Discovered Labwc/Openbox and icon themes, corner radius, shadows, tiled shadows, server/client decoration, maximized decoration |
| Behaviour | Placement, mouse focus and movement requirement, raise delay, gaps, snapping overlays/top maximize/client notification, resistance thresholds, resize controls, magnifier size/scale/filter |
| Mouse | Cursor discovery/size, natural scroll, handedness, pointer speed, acceleration profiles, three optional validated curves, tap/button mapping/drag/drag lock/three-finger drag, middle emulation, disable while typing, click and scroll methods, send-events mode, scroll factor |
| Keyboard | Installed XKB layouts and variants, editable ordered layout list, searchable chooser, repeat rate/delay, Num Lock, group-switch shortcut chooser preserving unrelated options |
| Touchscreen | Normal, left, right and inverted calibration matrices; unknown existing values retained |
| About | Application version, compositor build output including wlroots/features when supplied, session environment and active configuration directory |

The GTK interface uses native controls instead of reproducing Qt widget layout.
The ordered keyboard list is editable comma-separated text with a searchable
add chooser. This also preserves layouts absent from the installed catalog.

The complete scalar inventory is imported by tools/import-settings.py; the
round-trip test exercises every imported setting. Curve tests cover parsing,
attribute-based configuration, preservation and removal. The GTK smoke test
opens all 12 pages and confirms startup/close do not create configuration.

Upstream settings/page label translations are compiled into the application
without gettext or Qt dependencies. New diagnostics use English. The Qt
developer-only Template page is not a user-facing settings feature and is omitted.

Collections and Environment extend this baseline to all XML sections and
arbitrary environment assignments/fragments. A nested labwc 0.20.3 test exercises
Apply and configuration reload; see docs/safety-review.md for the evidence and
hardware-dependent limitations.
