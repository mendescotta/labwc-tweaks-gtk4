// Recoverable two-file commits; no additional runtime dependencies.
use std::{fs, io::Write, path::Path};
use xmltree::{Element, XMLNode};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const JOURNAL: &str = ".labwc-tweaks-journal";
pub fn lock(dir: &Path) -> Result<fs::File> {
    fs::create_dir_all(dir)?;
    let path = dir.join(".labwc-tweaks-lock");
    regular(&path)?;
    let f = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    f.try_lock()
        .map_err(|e| format!("Another editor is saving this configuration: {e}"))?;
    Ok(f)
}
pub fn regular(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if !m.is_file() => Err(format!(
            "{} must be a regular file, not a symbolic link or directory",
            path.display()
        )
        .into()),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
pub fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)?;
    let result = (|| {
        if let Ok(m) = fs::symlink_metadata(path) {
            if m.is_file() {
                f.set_permissions(m.permissions())?;
            }
        }
        f.write_all(data)?;
        f.sync_all()?;
        fs::rename(&temp, path)?;
        fs::File::open(path.parent().ok_or("Missing parent")?)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
}
fn decode(s: &str) -> Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return Err("Invalid recovery data".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(Into::into))
        .collect()
}
fn journal(dir: &Path, root: &Element) -> Result<()> {
    let mut b = Vec::new();
    root.write(&mut b)?;
    atomic_write(&dir.join(JOURNAL), &b)
}
/// A single-component fragment file name that labwc reads: any name ending in
/// `.env`. Path separators, NUL and control characters are refused so a name
/// can never escape environment.d or corrupt the recovery journal.
pub fn valid_fragment(n: &str) -> bool {
    n.len() > ".env".len()
        && n.ends_with(".env")
        && n != "."
        && n != ".."
        && !n.chars().any(|c| c == '/' || c.is_control())
}
pub fn valid_target(name: &str) -> bool {
    ["rc.xml", "environment"].contains(&name)
        || name
            .strip_prefix("environment.d/")
            .is_some_and(valid_fragment)
}
/// Lists environment.d regular files labwc would read. Returns the usable
/// `environment.d/NAME` keys and the names this editor cannot represent
/// (non-UTF-8), which may still override values.
pub fn fragments(dir: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let mut usable = Vec::new();
    let mut skipped = Vec::new();
    let files = match fs::read_dir(dir.join("environment.d")) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((usable, skipped)),
        Err(e) => return Err(e.into()),
    };
    for file in files {
        let file = file?;
        if !file.path().is_file() {
            continue;
        }
        let raw = file.file_name();
        match raw.to_str() {
            Some(n) if valid_fragment(n) => usable.push(format!("environment.d/{n}")),
            _ => {
                if raw.to_string_lossy().ends_with(".env") {
                    skipped.push(raw.to_string_lossy().into_owned());
                }
            }
        }
    }
    usable.sort();
    skipped.sort();
    Ok((usable, skipped))
}
pub fn recover(dir: &Path) -> Result<()> {
    let path = dir.join(JOURNAL);
    regular(&path)?;
    let Some(bytes) = read(&path)? else {
        return Ok(());
    };
    let root = Element::parse(bytes.as_slice())?;
    if root.name != "transaction" {
        return Err("Unrecognized save journal".into());
    }
    if root.attributes.get("state").map(String::as_str) != Some("committed") {
        let mut entries = Vec::new();
        for e in root.children.iter().filter_map(XMLNode::as_element) {
            let name = e
                .attributes
                .get("name")
                .ok_or("Missing recovery file name")?;
            if !valid_target(name) {
                return Err("Invalid recovery path".into());
            }
            let old = e.attributes.get("old").map(|s| decode(s)).transpose()?;
            let new = decode(e.attributes.get("new").ok_or("Missing new data")?)?;
            if name.starts_with("environment.d/")
                && fs::symlink_metadata(dir.join("environment.d"))
                    .is_ok_and(|m| !m.is_dir() || m.file_type().is_symlink())
            {
                return Err("Recovery paused: environment.d is not a real directory".into());
            }
            let target = dir.join(name);
            regular(&target)?;
            let current = read(&target)?;
            if current != old && current.as_deref() != Some(new.as_slice()) {
                return Err(format!("Recovery paused: {name} was changed by another program; journal retained at {}",path.display()).into());
            }
            entries.push((target, old));
        }
        for (target, old) in entries {
            if let Some(b) = old {
                atomic_write(&target, &b)?;
            } else if target.exists() {
                fs::remove_file(target)?;
            }
        }
    }
    fs::remove_file(path)?;
    fs::File::open(dir)?.sync_all()?;
    Ok(())
}

pub type Entry = (String, Option<Vec<u8>>, Vec<u8>);
pub fn commit(dir: &Path, entries: &[Entry]) -> Result<()> {
    for (name, old, _) in entries {
        if !valid_target(name) {
            return Err("Invalid save path".into());
        }
        let path = dir.join(name);
        if name.starts_with("environment.d/") {
            let parent = dir.join("environment.d");
            if fs::symlink_metadata(&parent)
                .is_ok_and(|m| !m.is_dir() || m.file_type().is_symlink())
            {
                return Err("environment.d must be a real directory".into());
            }
            fs::create_dir_all(parent)?;
        }
        regular(&path)?;
        if read(&path)? != *old {
            return Err(format!("{name} changed externally; reopen before saving").into());
        }
    }
    let mut root = Element::new("transaction");
    for (name, old, new) in entries {
        if let Some(old) = old {
            atomic_write(&dir.join(format!("{name}.bak")), old)?;
        }
        let mut e = Element::new("file");
        e.attributes.insert("name".into(), name.clone());
        e.attributes.insert("new".into(), hex(new));
        if let Some(old) = old {
            e.attributes.insert("old".into(), hex(old));
        }
        root.children.push(XMLNode::Element(e));
    }
    journal(dir, &root)?;
    let result = (|| -> Result<()> {
        for (name, old, new) in entries {
            let path = dir.join(name);
            regular(&path)?;
            if read(&path)? != *old {
                return Err(format!("{name} changed during saving").into());
            }
            atomic_write(&path, new)?;
        }
        root.attributes.insert("state".into(), "committed".into());
        journal(dir, &root)?;
        recover(dir)?;
        Ok(())
    })();
    if let Err(e) = result {
        return match recover(dir) {
            Ok(()) => Err(format!("Save failed; original files restored: {e}").into()),
            Err(recovery) => Err(format!("Save failed: {e}. {recovery}").into()),
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_commit_rolls_back_both_files() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("rc.xml"), b"new").unwrap();
        fs::write(d.path().join("environment"), b"old-env").unwrap();
        let mut root = Element::new("transaction");
        for (name, old, new) in [
            ("rc.xml", "old", "new"),
            ("environment", "old-env", "new-env"),
        ] {
            let mut e = Element::new("file");
            for (k, v) in [
                ("name", name.into()),
                ("old", hex(old.as_bytes())),
                ("new", hex(new.as_bytes())),
            ] {
                e.attributes.insert(k.into(), v);
            }
            root.children.push(XMLNode::Element(e));
        }
        journal(d.path(), &root).unwrap();
        recover(d.path()).unwrap();
        assert_eq!(fs::read(d.path().join("rc.xml")).unwrap(), b"old");
        assert_eq!(fs::read(d.path().join("environment")).unwrap(), b"old-env");
        assert!(!d.path().join(JOURNAL).exists());
    }
    #[test]
    fn recovery_preserves_external_edits() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("rc.xml"), b"external").unwrap();
        let mut root = Element::new("transaction");
        let mut e = Element::new("file");
        for (k, v) in [
            ("name", "rc.xml".into()),
            ("old", hex(b"old")),
            ("new", hex(b"new")),
        ] {
            e.attributes.insert(k.into(), v);
        }
        root.children.push(XMLNode::Element(e));
        journal(d.path(), &root).unwrap();
        assert!(recover(d.path()).is_err());
        assert_eq!(fs::read(d.path().join("rc.xml")).unwrap(), b"external");
        assert!(d.path().join(JOURNAL).exists());
    }
    #[test]
    fn editor_lock_is_exclusive() {
        let d = tempfile::tempdir().unwrap();
        let first = lock(d.path()).unwrap();
        assert!(lock(d.path()).is_err());
        drop(first);
        assert!(lock(d.path()).is_ok());
    }
}
