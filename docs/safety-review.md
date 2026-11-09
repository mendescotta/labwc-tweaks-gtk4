# Configuration review — 2026-11-09

Reference: labwc commit 8517392472d2e804d979aae73db4256ea2c8f439,
docs/labwc-config.5.scd. Live test: labwc 0.20.3 / wlroots 0.20.2.

## Coverage

The scalar pages expose 89 settings. Collections provides structured access to
all 21 XML configuration sections in the reference: keyboard and mouse bindings,
nested actions, window rules, regions, workspace names, fonts by role, default/
category/named libinput profiles, touch/output mappings, tablets and tools,
output margins, switcher fields, privileged interfaces and the general settings
sections. Select a record to edit its attributes and text; add/remove/reorder
records and children. Unknown existing attributes and nested actions are kept.

Environment edits the main environment file and arbitrary environment.d/*.env
fragments. Fragment values override the base in filename order. Cursor and XKB
controls update the fragment supplying the effective value when applicable.
Changes to the fragment set or contents during editing cause a save conflict.
Automatic output enablement, command hooks and arbitrary assignments are exposed.
Advanced changes require confirmation. Commands are never executed by the editor;
labwc can execute configured hooks and actions after loading the configuration.

The active configuration uses LABWC_CONFIG_DIR first. In an XFCE session with
an existing xfce4/labwc/rc.xml, that directory is detected before the default
labwc directory. -C/--config-dir remains an explicit override. Separate editor
processes are allowed, so opening an isolated directory cannot activate a window
that edits a different configuration.

## Save protections

- Existing XML/config symlinks are rejected before mutation.
- Backup replacement uses rename and does not write through a .bak symlink.
- An OS file lock serializes loads, saves and recovery across editor processes.
- A durable journal contains original and proposed bytes before replacement.
- Failed or interrupted saves restore originals on recovery. If another program
  changed a file, recovery preserves that edit, retains the journal, and reports
  the conflict instead of overwriting it.
- The journal is marked committed after every target file is replaced and synced.
- Environment fragments participate in the same recovery operation.
- Known scalar values, choices, matrices and titlebar layouts are validated.
  Collections also validate required binding/action/region attributes, Execute
  commands, region percentages, font sizes and known device numeric values.
- Conflicting scalar/collection edits are rejected rather than silently merged.
- Controls are rebuilt after saving to reflect collection/environment changes.

## Verification and practical limits

Fourteen regression tests cover XML preservation, collection edits, scalar
round trips, environment precedence, prohibited paths, malformed values,
symlinks, concurrent editor locks, interrupted saves and external edits during
recovery. An isolated GTK smoke test visits all 12 pages.

tools/nested-labwc.py starts a windowed X11 labwc with temporary configuration and
runtime directories. Its Wayland client opens every collection editor, stages a
region through the UI, confirms Apply, verifies a scalar edit and refreshed
window, then saves representative bindings, nested rule actions, regions,
fonts, workspace names, separate input profiles and tablet mapping. Compositor
logs confirm initial reads and reconfiguration without configuration errors.
The X11 backend emits an unrelated startup ChangeProperty/Atom error; tests pass.

No personal configuration is modified by these tests. Logs are kept in
out/nested-review/session.log.

A filesystem cannot atomically replace several independent paths for unrelated
readers. The journal provides rollback and crash recovery; it does not make the
files appear simultaneously to another reader. An external program that ignores
the editor's lock can still race the final rename. Conflict checks and retained
backups reduce that risk, but cannot eliminate it.

A nested X11 backend cannot test DRM/HDR, adaptive sync, tearing, physical input
calibration or tablet hardware. These settings remain dependent on compositor
version, backend and hardware. Primary selection and XWayland persistence may
require restart; environment changes may require signing in again. Collection
validation is not a complete semantic validator for every labwc action or
hardware-dependent setting.
