# Resolving a save-recovery conflict

Every save writes `.labwc-tweaks-journal` (original and proposed bytes) in the
configuration directory before replacing files, and deletes it afterwards. If
the editor or machine stopped mid-save, the next start restores the originals.

If another program edited a file in the meantime, recovery refuses to overwrite
that edit and the editor reports:

    Recovery paused: <file> was changed by another program; journal retained at <path>

Nothing has been lost: the file on disk is untouched and `<file>.bak` holds the
content from before the interrupted save.

1. Close other editors that touch the configuration. Find the directory in
   About, or the error text.
2. Run `python3 tools/journal-inspect.py DIR` (read-only). Each file is reported
   as matching the journal's OLD content, its NEW content, or CHANGED, with a
   diff for changed files.
3. Decide per changed file: keep the on-disk version (usually right when you
   edited it by hand), or restore `<file>.bak` (`cp DIR/<file>.bak DIR/<file>`).
   Files reported OLD or NEW are consistent; check NEW ones are what you want.
4. Delete `DIR/.labwc-tweaks-journal` and restart the editor. A journal that is
   left in place is replayed on every start.

An "Unrecognized save journal" or "Invalid recovery path" error means the
journal was damaged or edited; apply step 3 by hand and delete it.
