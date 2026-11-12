use std::{collections::BTreeMap, fs, path::PathBuf};
use xmltree::{Element, XMLNode};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub struct Config {
    pub dir: PathBuf,
    originals: [Option<Vec<u8>>; 2],
    root: Element,
    env: String,
    pub env_files: BTreeMap<String, String>,
    /// environment.d files labwc reads but this editor cannot represent.
    pub skipped: Vec<String>,
}
fn read(path: &std::path::Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn matches(e: &Element, name: &str) -> bool {
    e.name.eq_ignore_ascii_case(name)
        && (name != "device"
            || (e.attributes.get("category").is_none_or(|v| v == "default")
                && !e.attributes.contains_key("name")))
}
fn node<'a>(root: &'a mut Element, parts: &[&str]) -> &'a mut Element {
    if parts.is_empty() {
        return root;
    }
    let i = root
        .children
        .iter()
        .position(|n| n.as_element().is_some_and(|e| matches(e, parts[0])))
        .unwrap_or_else(|| {
            let mut e = Element::new(parts[0]);
            if parts[0] == "device" {
                e.attributes.insert("category".into(), "default".into());
            }
            root.children.push(XMLNode::Element(e));
            root.children.len() - 1
        });
    node(root.children[i].as_mut_element().unwrap(), &parts[1..])
}
impl Config {
    pub fn load(dir: PathBuf) -> Result<Self> {
        let _lock = if dir.is_dir() {
            Some(crate::storage::lock(&dir)?)
        } else {
            None
        };
        crate::storage::recover(&dir)?;
        Self::load_inner(dir)
    }
    fn load_inner(dir: PathBuf) -> Result<Self> {
        let originals = [read(&dir.join("rc.xml"))?, read(&dir.join("environment"))?];
        let root = match &originals[0] {
            Some(b) => Element::parse(b.as_slice())?,
            None => {
                let dirs = std::env::var_os("XDG_CONFIG_DIRS").unwrap_or("/etc/xdg".into());
                let mut fallback = None;
                for base in std::env::split_paths(&dirs) {
                    if !base.is_absolute() {
                        continue;
                    }
                    if let Some(bytes) = read(&base.join("labwc/rc.xml"))? {
                        fallback = Some(Element::parse(bytes.as_slice())?);
                        break;
                    }
                }
                fallback.unwrap_or_else(|| Element::new("labwc_config"))
            }
        };
        if root.name != "labwc_config" {
            return Err("Expected labwc_config XML root".into());
        }
        let env = String::from_utf8(originals[1].clone().unwrap_or_default())?;
        let mut env_files = BTreeMap::new();
        let (names, skipped) = crate::storage::fragments(&dir)?;
        for key in names {
            let text = fs::read_to_string(dir.join(&key))?;
            env_files.insert(key, text);
        }
        Ok(Self {
            dir,
            originals,
            root,
            env,
            env_files,
            skipped,
        })
    }
    pub fn get(&self, key: &str) -> Option<String> {
        if !key.contains('/') {
            let combined = std::iter::once(self.env.as_str())
                .chain(self.env_files.values().map(String::as_str))
                .collect::<Vec<_>>()
                .join(
                    "
",
                );
            let value = combined
                .lines()
                .filter_map(|l| l.split_once('='))
                .rfind(|(k, _)| k.trim() == key)
                .map(|(_, v)| v.trim().to_owned())
                .or_else(|| std::env::var(key).ok());
            if key == "XKB_DEFAULT_LAYOUT" {
                return value.map(|v| {
                    merge_variants(&v, &self.get("XKB_DEFAULT_VARIANT").unwrap_or_default())
                });
            }
            return value;
        }
        if let Some((parent, attribute)) = key.rsplit_once("/@") {
            return self.find(parent)?.attributes.get(attribute).cloned();
        }
        if let Some(parent) = key.strip_suffix("/defined") {
            return Some(
                if self.find(parent).is_some() {
                    "yes"
                } else {
                    "no"
                }
                .into(),
            );
        }
        if let Some((parent, field)) = key.rsplit_once('/') {
            if let Some(e) = self.find(parent) {
                if let Some(value) = e
                    .attributes
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(field))
                    .map(|(_, v)| v.clone())
                {
                    return Some(value);
                }
            }
        }
        let mut e = &self.root;
        for p in key.split('/') {
            e = e
                .children
                .iter()
                .filter_map(XMLNode::as_element)
                .find(|e| matches(e, p))?;
        }
        e.get_text().map(|s| s.into_owned())
    }
    fn find(&self, key: &str) -> Option<&Element> {
        let mut e = &self.root;
        for p in key.split('/') {
            e = e
                .children
                .iter()
                .filter_map(XMLNode::as_element)
                .find(|e| matches(e, p))?;
        }
        Some(e)
    }
    pub fn environment(&self) -> &str {
        &self.env
    }
    pub fn collection(&self, section: &str) -> String {
        let mut wrapper = Element::new("section");
        wrapper.children = self
            .root
            .children
            .iter()
            .filter(|c| c.as_element().is_some_and(|e| e.name == section))
            .cloned()
            .collect();
        crate::structured::serialize(&wrapper)
    }
    pub fn validate(&self, changes: &BTreeMap<String, String>) -> Result<()> {
        for (key, value) in changes {
            if let Some(file) = key.strip_prefix("#env/") {
                if file != "environment"
                    && !(file.starts_with("environment.d/") && crate::storage::valid_target(file))
                {
                    return Err("Invalid environment file name".into());
                }
                crate::environment::validate(value)?;
                if changes
                    .keys()
                    .any(|k| !k.contains('/') && !k.starts_with('#'))
                {
                    return Err(
                        "Apply or discard keyboard/cursor changes before staging environment files"
                            .into(),
                    );
                }
                continue;
            }
            if let Some(section) = key.strip_prefix("#section/") {
                crate::structured::parse(section, value)?;
                if changes
                    .keys()
                    .any(|k| k.starts_with(&format!("{section}/")))
                {
                    return Err(format!("Apply or discard scalar changes in {section} before staging its collection").into());
                }
                continue;
            }

            let known = crate::schema::pages()
                .into_iter()
                .flat_map(|(_, f)| f)
                .any(|(_, k, _, _)| k == key);
            let curve = crate::curves::NAMES.iter().any(|name| {
                ["defined", "step", "points"]
                    .iter()
                    .any(|field| key == &format!("libinput/device/{name}/{field}"))
            });
            if !known && !curve {
                return Err(format!("Unsupported setting: {key}").into());
            }
            if value.contains(['\n', '\r', '\0']) {
                return Err("Values cannot contain newlines or NUL".into());
            }
            if key == "XKB_DEFAULT_LAYOUT" && value.trim().is_empty() {
                return Err("Keyboard layout cannot be empty".into());
            }
            if key == "theme/titlebar/layout" {
                let parts: Vec<_> = value.split(':').collect();
                let mut seen = std::collections::BTreeSet::new();
                if parts.len() != 2
                    || parts
                        .iter()
                        .flat_map(|s| s.split(','))
                        .filter(|s| !s.is_empty())
                        .any(|s| {
                            !["icon", "menu", "iconify", "max", "close", "shade", "desk"]
                                .contains(&s)
                                || !seen.insert(s)
                        })
                {
                    return Err(
                        "Titlebar layout needs one colon and unique valid button names".into(),
                    );
                }
            }
        }
        for (_, fields) in crate::schema::pages() {
            for (label, key, _, kind) in fields {
                if let Some(v) = changes.get(key) {
                    if kind == "bool" && !["yes", "no"].contains(&v.as_str()) {
                        return Err(format!("Invalid boolean for {label}").into());
                    }
                    let options = crate::catalog::options(key);
                    if kind == "text"
                        && !options.is_empty()
                        && ![
                            "theme/name",
                            "theme/icon",
                            "XCURSOR_THEME",
                            "libinput/device/calibrationMatrix",
                        ]
                        .contains(&key)
                        && !options.contains(v)
                    {
                        return Err(format!("Invalid choice for {label}").into());
                    }
                    if key == "libinput/device/calibrationMatrix"
                        && !v.is_empty()
                        && (v.split_whitespace().count() != 6
                            || !v
                                .split_whitespace()
                                .all(|s| s.parse::<f64>().is_ok_and(|n| n.is_finite())))
                    {
                        return Err("Calibration matrix requires six finite numbers".into());
                    }
                    if kind == "integer" && v.parse::<i32>().is_err() {
                        return Err(format!("{label} requires an integer").into());
                    }
                    if kind == "number" || kind == "integer" {
                        let (min, max, _) = crate::ranges::bounds(key);
                        if !v
                            .parse::<f64>()
                            .is_ok_and(|n| n.is_finite() && (min..=max).contains(&n))
                        {
                            return Err(format!("{label} must be between {min} and {max}").into());
                        }
                    }
                }
            }
        }
        let effective = |key: &str| changes.get(key).cloned().or_else(|| self.get(key));
        if effective("libinput/device/accelProfile").as_deref() == Some("custom") {
            for name in crate::curves::NAMES {
                let p = format!("libinput/device/{name}");
                if effective(&format!("{p}/defined")).as_deref() == Some("yes")
                    && !crate::curves::validate(
                        &effective(&format!("{p}/step")).unwrap_or_default(),
                        &effective(&format!("{p}/points")).unwrap_or_default(),
                    )
                {
                    return Err(format!(
                        "{name}: use a positive Step and 2–32 finite, non-negative Points"
                    )
                    .into());
                }
            }
        }
        Ok(())
    }
    pub fn save(&mut self, changes: &BTreeMap<String, String>) -> Result<()> {
        let _lock = crate::storage::lock(&self.dir)?;
        crate::storage::recover(&self.dir)?;
        for (i, name) in ["rc.xml", "environment"].iter().enumerate() {
            if read(&self.dir.join(name))? != self.originals[i] {
                return Err(format!(
                    "{name} changed externally. Reopen the application before saving."
                )
                .into());
            }
        }
        for name in ["rc.xml", "environment"] {
            if fs::symlink_metadata(self.dir.join(name)).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(format!("{name} is a symbolic link. Select its actual configuration directory before editing.").into());
            }
        }
        let (current_files, skipped) = crate::storage::fragments(&self.dir)?;
        if current_files.iter().ne(self.env_files.keys()) || skipped != self.skipped {
            return Err("Environment fragments changed externally; reopen before saving".into());
        }
        for (name, text) in &self.env_files {
            if read(&self.dir.join(name))?.as_deref() != Some(text.as_bytes()) {
                return Err(format!("{name} changed externally; reopen before saving").into());
            }
        }
        self.validate(changes)?;
        if !self.skipped.is_empty()
            && changes
                .keys()
                .any(|k| !k.contains('/') || k.starts_with("#env/"))
        {
            return Err(format!(
                "Cannot edit environment settings: unsupported file name(s) in environment.d would override them: {}",
                self.skipped.join(", ")
            )
            .into());
        }
        let effective = |key: &str| changes.get(key).cloned().or_else(|| self.get(key));
        let mut root = self.root.clone();
        let mut env = self.env.clone();
        let mut env_files = self.env_files.clone();
        let mut xml_changed = false;
        let mut env_changed = false;
        for (k, v) in changes {
            if let Some(file) = k.strip_prefix("#env/") {
                if file == "environment" {
                    env = v.clone();
                    env_changed = true;
                } else {
                    env_files.insert(file.into(), v.clone());
                }
                continue;
            }
            if let Some(section) = k.strip_prefix("#section/") {
                let wrapper = crate::structured::parse(section, v)?;
                let index = root
                    .children
                    .iter()
                    .position(|c| c.as_element().is_some_and(|e| e.name == section))
                    .unwrap_or(root.children.len());
                root.children
                    .retain(|c| c.as_element().is_none_or(|e| e.name != section));
                for (offset, c) in wrapper
                    .children
                    .into_iter()
                    .filter(|c| c.as_element().is_some())
                    .enumerate()
                {
                    root.children.insert(index + offset, c);
                }
                xml_changed = true;
                continue;
            }

            if k == "XKB_DEFAULT_LAYOUT" && v.trim().is_empty() {
                return Err(
                    "Keyboard layout cannot be empty (use a valid XKB layout, such as us)".into(),
                );
            }
            if k == "XCURSOR_SIZE" && !v.parse::<u32>().is_ok_and(|n| (16..=60).contains(&n)) {
                return Err("Cursor size must be an integer between 16 and 60".into());
            }
            if (k.ends_with("repeatRate")
                || k.ends_with("repeatDelay")
                || k.ends_with("cornerRadius")
                || k == "core/gap")
                && v.parse::<u32>().is_err()
            {
                return Err(format!("{k} must be a non-negative integer").into());
            }
            if v.contains(['\n', '\r', '\0']) {
                return Err("Values cannot contain newlines or NUL".into());
            }
            if k.contains('/') {
                if let Some((parent, attribute)) = k.rsplit_once("/@") {
                    node(&mut root, &parent.split('/').collect::<Vec<_>>())
                        .attributes
                        .insert(attribute.into(), v.clone());
                    xml_changed = true;
                    continue;
                }
                if k.ends_with("/pointerSpeed")
                    && effective("libinput/device/accelProfile").as_deref() == Some("custom")
                {
                    continue;
                }
                if k.contains("Curve/") {
                    if effective("libinput/device/accelProfile").as_deref() != Some("custom") {
                        continue;
                    }
                    let (parent, field) = k.rsplit_once('/').unwrap();
                    if field == "defined" {
                        if v == "no" {
                            let (container, name) = parent.rsplit_once('/').unwrap();
                            let e = node(&mut root, &container.split('/').collect::<Vec<_>>());
                            e.children
                                .retain(|n| !n.as_element().is_some_and(|e| matches(e, name)));
                            xml_changed = true;
                        }
                        continue;
                    }
                    if effective(&format!("{parent}/defined")).as_deref() == Some("no") {
                        continue;
                    }
                    let e = node(&mut root, &parent.split('/').collect::<Vec<_>>());
                    let key = e
                        .attributes
                        .keys()
                        .find(|name| name.eq_ignore_ascii_case(field))
                        .cloned()
                        .unwrap_or_else(|| field.to_owned());
                    e.attributes.insert(key, v.clone());
                    xml_changed = true;
                    continue;
                }
                xml_changed = true;
                if let Some((parent, field)) = k.rsplit_once('/') {
                    let e = node(&mut root, &parent.split('/').collect::<Vec<_>>());
                    e.attributes
                        .retain(|name, _| !name.eq_ignore_ascii_case(field));
                }
                let e = node(&mut root, &k.split('/').collect::<Vec<_>>());
                e.children
                    .retain(|n| !matches!(n, XMLNode::Text(_) | XMLNode::CData(_)));
                e.children.push(XMLNode::Text(v.clone()));
            } else {
                if let Some((file, text)) = env_files.iter_mut().rev().find(|(_, text)| {
                    text.lines().any(|line| {
                        line.split_once('=')
                            .is_some_and(|(name, _)| name.trim() == k)
                    })
                }) {
                    let _ = file;
                    *text = crate::environment::set(text, k, v);
                    continue;
                }
                env_changed = true;
                let mut lines: Vec<String> = env
                    .lines()
                    .filter(|l| l.split_once('=').is_none_or(|(name, _)| name.trim() != k))
                    .map(str::to_owned)
                    .collect();
                lines.push(format!("{k}={v}"));
                env = lines.join("\n") + "\n";
            }
        }
        if changes.contains_key("XKB_DEFAULT_LAYOUT") {
            for text in env_files.values_mut() {
                if text.lines().any(|l| {
                    l.split_once('=')
                        .is_some_and(|(k, _)| k.trim() == "XKB_DEFAULT_VARIANT")
                }) {
                    *text = crate::environment::set(text, "XKB_DEFAULT_VARIANT", "");
                }
            }

            let mut lines: Vec<&str> = env
                .lines()
                .filter(|l| {
                    l.split_once('=')
                        .is_none_or(|(k, _)| k.trim() != "XKB_DEFAULT_VARIANT")
                })
                .collect();
            lines.push("XKB_DEFAULT_VARIANT=");
            let cleared = lines.join("\n") + "\n";
            if cleared != env {
                env = cleared;
                env_changed = true;
            }
        }
        let mut bytes = Vec::new();
        root.write_with_config(
            &mut bytes,
            xmltree::EmitterConfig::new().perform_indent(true),
        )?;
        let mut entries = Vec::new();
        if xml_changed {
            entries.push(("rc.xml".into(), self.originals[0].clone(), bytes));
        }
        if env_changed {
            entries.push((
                "environment".into(),
                self.originals[1].clone(),
                env.into_bytes(),
            ));
        }
        for (file, text) in env_files {
            if self.env_files.get(&file) != Some(&text) {
                let original = self.env_files.get(&file).map(|s| s.as_bytes().to_vec());
                entries.push((file, original, text.into_bytes()));
            }
        }
        crate::storage::commit(&self.dir, &entries)?;
        *self = Self::load_inner(self.dir.clone())?;
        Ok(())
    }
}
fn merge_variants(layout: &str, variants: &str) -> String {
    let variants: Vec<_> = variants.split(',').collect();
    layout
        .split(',')
        .enumerate()
        .map(|(i, l)| {
            let variant = variants.get(i).copied().unwrap_or("");
            if l.contains('(') || variant.is_empty() {
                l.to_owned()
            } else {
                format!("{l}({variant})")
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_rules_and_specific_devices() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("rc.xml"),"<labwc_config><!--keep--><libinput><device category=\"touchpad\"><tap>yes</tap></device></libinput><keyboard><keybind key=\"A-x\"/></keyboard></labwc_config>").unwrap();
        let mut c = Config::load(d.path().into()).unwrap();
        c.save(&BTreeMap::from([(
            "libinput/device/tap".into(),
            "no".into(),
        )]))
        .unwrap();
        let s = fs::read_to_string(d.path().join("rc.xml")).unwrap();
        assert!(s.contains("keep"));
        assert!(s.contains("<!--"));
        assert!(s.contains("keybind"));
        assert!(s.contains("touchpad"));
        assert_eq!(c.get("libinput/device/tap").as_deref(), Some("no"));
    }
    #[test]
    fn refuses_conflicts_and_invalid_xml() {
        let d = tempfile::tempdir().unwrap();
        let mut c = Config::load(d.path().into()).unwrap();
        fs::write(d.path().join("environment"), "OTHER=value").unwrap();
        assert!(c.save(&BTreeMap::new()).is_err());
        fs::write(d.path().join("rc.xml"), "<broken").unwrap();
        assert!(Config::load(d.path().into()).is_err());
    }

    #[test]
    fn curves_preserve_attributes_and_remove_explicitly() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("rc.xml"),r#"<labwc_config><libinput><device><accelProfile>custom</accelProfile><motionCurve step="0.5" points="0 1 2"/><pointerSpeed>0.75</pointerSpeed></device></libinput></labwc_config>"#).unwrap();
        let mut c = Config::load(d.path().into()).unwrap();
        assert_eq!(
            c.get("libinput/device/motionCurve/step").as_deref(),
            Some("0.5")
        );
        c.save(&BTreeMap::from([("theme/name".into(), "A & B".into())]))
            .unwrap();
        assert!(c
            .find("libinput/device/motionCurve")
            .unwrap()
            .attributes
            .contains_key("step"));
        c.save(&BTreeMap::from([(
            "libinput/device/motionCurve/step".into(),
            "1".into(),
        )]))
        .unwrap();
        assert_eq!(
            c.get("libinput/device/motionCurve/step").as_deref(),
            Some("1")
        );
        let xml = fs::read_to_string(d.path().join("rc.xml")).unwrap();
        assert!(
            xml.contains(r#"step="1""#) && xml.contains(r#"points="0 1 2""#),
            "{xml}"
        );
        assert!(
            !xml.contains("<step>") && !xml.contains("<points>"),
            "{xml}"
        );
        assert_eq!(
            c.get("libinput/device/motionCurve/points").as_deref(),
            Some("0 1 2")
        );
        c.save(&BTreeMap::from([(
            "libinput/device/motionCurve/defined".into(),
            "no".into(),
        )]))
        .unwrap();
        assert!(c.find("libinput/device/motionCurve").is_none());
    }
    #[test]
    fn invalid_values_do_not_write_and_environment_is_preserved() {
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("environment"),
            "# comment
OTHER=a=b
XKB_DEFAULT_LAYOUT=us
",
        )
        .unwrap();
        let mut c = Config::load(d.path().into()).unwrap();
        assert!(c
            .save(&BTreeMap::from([("XKB_DEFAULT_LAYOUT".into(), "".into())]))
            .is_err());
        assert!(c
            .save(&BTreeMap::from([("XCURSOR_SIZE".into(), "0".into())]))
            .is_err());
        c.save(&BTreeMap::from([(
            "XKB_DEFAULT_LAYOUT".into(),
            "us,ee".into(),
        )]))
        .unwrap();
        let text = fs::read_to_string(d.path().join("environment")).unwrap();
        assert!(text.contains("# comment"));
        assert!(text.contains("OTHER=a=b"));
        assert_eq!(c.get("XKB_DEFAULT_LAYOUT").as_deref(), Some("us,ee"));
    }
    #[test]
    fn scalar_inventory_roundtrips() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("rc.xml"), "<labwc_config/>").unwrap();
        let mut c = Config::load(d.path().into()).unwrap();
        let mut edits = BTreeMap::new();
        for (_, fields) in crate::schema::pages() {
            for (_, key, default, _) in fields {
                edits.insert(
                    key.to_string(),
                    if key == "XKB_DEFAULT_LAYOUT" {
                        "us"
                    } else {
                        default
                    }
                    .to_string(),
                );
            }
        }
        c.save(&edits).unwrap();
        for (key, value) in edits {
            assert_eq!(c.get(&key).unwrap_or_default(), value, "{key}");
        }
    }
    #[test]
    fn variants_keep_alignment() {
        assert_eq!(
            merge_variants("us(dvorak),ee,ru", ",,phonetic"),
            "us(dvorak),ee,ru(phonetic)"
        );
    }

    #[test]
    fn attributes_and_backups_are_safe() {
        use std::os::unix::fs::symlink;
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("rc.xml"),
            r#"<labwc_config><desktops number="2"/></labwc_config>"#,
        )
        .unwrap();
        let victim = d.path().join("untouched");
        fs::write(&victim, "keep").unwrap();
        symlink(&victim, d.path().join("rc.xml.bak")).unwrap();
        let mut c = Config::load(d.path().into()).unwrap();
        c.save(&BTreeMap::from([("desktops/@number".into(), "4".into())]))
            .unwrap();
        assert_eq!(fs::read_to_string(&victim).unwrap(), "keep");
        assert_eq!(c.get("desktops/@number").as_deref(), Some("4"));
        assert!(!fs::read_to_string(d.path().join("rc.xml"))
            .unwrap()
            .contains("<number>"));
        assert!(c
            .save(&BTreeMap::from([(
                "theme/titlebar/layout".into(),
                "close:close".into()
            )]))
            .is_err());
        assert!(c
            .save(&BTreeMap::from([("../anything".into(), "bad".into())]))
            .is_err());
    }
}

#[cfg(test)]
mod collection_tests {
    use super::*;
    #[test]
    fn collections_preserve_nested_actions_and_siblings() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("rc.xml"),r#"<labwc_config><theme><name>Original</name></theme><keyboard><default/><keybind key="A-x"><action name="If"><query title="*"/><then><action name="Raise"/></then></action></keybind></keyboard><touch deviceName="one"/><touch deviceName="two"/></labwc_config>"#).unwrap();
        let mut cfg = Config::load(d.path().into()).unwrap();
        let mut section =
            crate::structured::parse("keyboard", &cfg.collection("keyboard")).unwrap();
        let keyboard = section.children[0].as_mut_element().unwrap();
        let mut bind = Element::new("keybind");
        bind.attributes.insert("key".into(), "W-F1".into());
        keyboard.children.push(XMLNode::Element(bind));
        cfg.save(&BTreeMap::from([(
            "#section/keyboard".into(),
            crate::structured::serialize(&section),
        )]))
        .unwrap();
        let xml = fs::read_to_string(d.path().join("rc.xml")).unwrap();
        assert!(xml.contains("query"));
        assert!(xml.contains("Raise"));
        assert!(xml.contains("Original"));
        assert_eq!(xml.matches("<touch ").count(), 2);
        assert!(cfg
            .validate(&BTreeMap::from([
                ("#section/theme".into(), cfg.collection("theme")),
                ("theme/name".into(), "Conflict".into())
            ]))
            .is_err());
    }
    #[test]
    fn environment_fragments_override_and_save_to_correct_file() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir(d.path().join("environment.d")).unwrap();
        fs::write(d.path().join("environment"), "XCURSOR_SIZE=24\n").unwrap();
        fs::write(
            d.path().join("environment.d/90-local.env"),
            "XCURSOR_SIZE=32\nOTHER=a=b\n",
        )
        .unwrap();
        let mut c = Config::load(d.path().into()).unwrap();
        assert_eq!(c.get("XCURSOR_SIZE").as_deref(), Some("32"));
        c.save(&BTreeMap::from([("XCURSOR_SIZE".into(), "40".into())]))
            .unwrap();
        assert_eq!(c.get("XCURSOR_SIZE").as_deref(), Some("40"));
        assert!(
            fs::read_to_string(d.path().join("environment.d/90-local.env"))
                .unwrap()
                .contains("OTHER=a=b")
        );
        assert!(fs::read_to_string(d.path().join("environment"))
            .unwrap()
            .contains("24"));
        c.save(&BTreeMap::from([(
            "#env/environment.d/95-test.env".into(),
            "CUSTOM=value\n".into(),
        )]))
        .unwrap();
        assert_eq!(c.get("CUSTOM").as_deref(), Some("value"));
    }

    #[test]
    fn environment_editor_cannot_replace_xml() {
        let d = tempfile::tempdir().unwrap();
        let cfg = Config::load(d.path().into()).unwrap();
        assert!(cfg
            .validate(&BTreeMap::from([(
                "#env/rc.xml".into(),
                "VALUE=text".into()
            )]))
            .is_err());
        assert!(cfg
            .validate(&BTreeMap::from([(
                "#env/environment.d/../rc.xml".into(),
                "VALUE=text".into()
            )]))
            .is_err());
    }

    fn fixture(env: &str, fragment: Option<(&str, &str)>) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("rc.xml"), "<labwc_config/>").unwrap();
        fs::write(d.path().join("environment"), env).unwrap();
        if let Some((name, text)) = fragment {
            fs::create_dir(d.path().join("environment.d")).unwrap();
            fs::write(d.path().join("environment.d").join(name), text).unwrap();
        }
        d
    }
    #[test]
    fn fragment_layout_change_clears_base_variant() {
        let d = fixture(
            "XKB_DEFAULT_VARIANT=phonetic\n",
            Some(("90-layout.env", "XKB_DEFAULT_LAYOUT=ru\n")),
        );
        let mut c = Config::load(d.path().into()).unwrap();
        assert_eq!(c.get("XKB_DEFAULT_LAYOUT").as_deref(), Some("ru(phonetic)"));
        c.save(&BTreeMap::from([(
            "XKB_DEFAULT_LAYOUT".into(),
            "us".into(),
        )]))
        .unwrap();
        assert_eq!(c.get("XKB_DEFAULT_LAYOUT").as_deref(), Some("us"));
        let base = fs::read_to_string(d.path().join("environment")).unwrap();
        assert!(!base.contains("phonetic"), "{base:?}");
    }
    #[test]
    fn fragment_and_base_variant_combinations() {
        for (base, frag) in [
            ("", "XKB_DEFAULT_LAYOUT=ru\nXKB_DEFAULT_VARIANT=phonetic\n"),
            ("XKB_DEFAULT_LAYOUT=ru\n", "XKB_DEFAULT_VARIANT=phonetic\n"),
            ("XKB_DEFAULT_VARIANT=phonetic\n", "OTHER=1\n"),
            (
                "XKB_DEFAULT_LAYOUT=ru\nXKB_DEFAULT_VARIANT=phonetic\n",
                "OTHER=1\n",
            ),
        ] {
            let d = fixture(base, Some(("a.env", frag)));
            let mut c = Config::load(d.path().into()).unwrap();
            c.save(&BTreeMap::from([(
                "XKB_DEFAULT_LAYOUT".into(),
                "us".into(),
            )]))
            .unwrap();
            assert_eq!(
                c.get("XKB_DEFAULT_LAYOUT").as_deref(),
                Some("us"),
                "{base:?} {frag:?}"
            );
            let again = Config::load(d.path().into()).unwrap();
            assert_eq!(again.get("XKB_DEFAULT_LAYOUT").as_deref(), Some("us"));
        }
    }
    #[test]
    fn upstream_fragment_names_are_loaded_and_edited() {
        for name in ["90 local.env", ".hidden.env", "ü.env", "a,b.env"] {
            let d = fixture("XCURSOR_SIZE=24\n", Some((name, "XCURSOR_SIZE=40\n")));
            let mut c = Config::load(d.path().into()).unwrap();
            assert_eq!(c.get("XCURSOR_SIZE").as_deref(), Some("40"), "{name}");
            c.save(&BTreeMap::from([("XCURSOR_SIZE".into(), "32".into())]))
                .unwrap();
            assert_eq!(c.get("XCURSOR_SIZE").as_deref(), Some("32"), "{name}");
            let text = fs::read_to_string(d.path().join("environment.d").join(name)).unwrap();
            assert!(text.contains("XCURSOR_SIZE=32"));
        }
    }
    #[test]
    fn unrepresentable_fragment_blocks_environment_writes() {
        use std::os::unix::ffi::OsStrExt;
        let d = fixture("XCURSOR_SIZE=24\n", Some(("ok.env", "A=1\n")));
        let bad = std::ffi::OsStr::from_bytes(b"bad\xff.env");
        fs::write(
            d.path().join("environment.d").join(bad),
            "XCURSOR_SIZE=40\n",
        )
        .unwrap();
        let mut c = Config::load(d.path().into()).unwrap();
        assert_eq!(c.skipped.len(), 1);
        assert!(c
            .save(&BTreeMap::from([("XCURSOR_SIZE".into(), "32".into())]))
            .is_err());
        c.save(&BTreeMap::from([("core/gap".into(), "4".into())]))
            .unwrap();
    }
    #[test]
    fn fragment_names_cannot_escape() {
        for bad in [
            "environment.d/../rc.xml",
            "environment.d/a/b.env",
            "environment.d/.env",
            "environment.d/x\n.env",
        ] {
            assert!(!crate::storage::valid_target(bad), "{bad}");
        }
        assert!(crate::storage::valid_target("environment.d/90 local.env"));
    }
    #[test]
    fn attribute_order_is_stable_across_saves() {
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("rc.xml"),
            r#"<labwc_config><keyboard><keybind key="A-x" zeta="1" alpha="2"/></keyboard></labwc_config>"#,
        )
        .unwrap();
        let mut c = Config::load(d.path().into()).unwrap();
        let section = c.collection("keyboard");
        assert!(section.find("zeta").unwrap() < section.find("alpha").unwrap());
        c.save(&BTreeMap::from([("core/gap".into(), "2".into())]))
            .unwrap();
        assert_eq!(c.collection("keyboard"), section);
    }
}
