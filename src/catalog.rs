use std::{collections::BTreeSet, fs, path::PathBuf};
pub fn options(key: &str) -> Vec<String> {
    let fixed: &[&str] = match key {
        "core/adaptiveSync" => &["no", "yes", "fullscreen"],
        "core/allowTearing" => &["no", "yes", "fullscreen", "fullscreenForced"],
        "windowSwitcher/@order" => &["focus", "age"],
        "windowSwitcher/osd/@style" => &["classic", "thumbnail"],
        "windowSwitcher/osd/@output" => &["all", "focused", "cursor"],
        "keyboard/@layoutScope" => &["global", "window"],
        "snapping/overlay/placement" => &["above", "below"],
        "libinput/device/dragLock" => &["no", "yes", "timeout"],
        "libinput/device/threeFingerDrag" => &["no", "yes", "3", "4"],
        "libinput/device/clickfingerButtonMap" => &["lrm", "lmr"],

        "core/decoration" => &["server", "client"],
        "theme/maximizedDecoration" => &["titlebar", "none"],
        "placement/policy" => &["automatic", "cascade", "center", "cursor"],
        "snapping/notifyClient" => &["always", "region", "edge", "never"],
        "resize/popupShow" => &["never", "always", "nonpixel"],
        "libinput/device/accelProfile" => &["flat", "adaptive", "custom"],
        "libinput/device/tapButtonMap" => &["lrm", "lmr"],
        "libinput/device/clickMethod" => &["none", "buttonAreas", "clickFinger"],
        "libinput/device/scrollMethod" => &["twoFinger", "edge", "onbutton", "none"],
        "libinput/device/sendEventsMode" => &["yes", "no", "disabledOnExternalMouse"],
        "libinput/device/calibrationMatrix" => &[
            "",
            "1 0 0 0 1 0",
            "0 -1 1 1 0 0",
            "0 1 0 -1 0 1",
            "-1 0 1 0 -1 1",
        ],
        _ => &[],
    };
    if !fixed.is_empty() {
        return fixed.iter().map(|v| (*v).into()).collect();
    }
    if matches!(key, "theme/name" | "theme/icon" | "XCURSOR_THEME") {
        return themes(key);
    }
    Vec::new()
}
fn themes(key: &str) -> Vec<String> {
    let home = gtk::glib::home_dir();
    let sub = if key == "theme/name" {
        "themes"
    } else {
        "icons"
    };
    let mut paths = vec![
        home.join(format!(".{sub}")),
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or(home.join(".local/share"))
            .join(sub),
    ];
    paths.extend(
        std::env::split_paths(
            &std::env::var_os("XDG_DATA_DIRS").unwrap_or("/usr/local/share:/usr/share".into()),
        )
        .filter(|p| p.is_absolute())
        .map(|p| p.join(sub)),
    );
    let mut names = BTreeSet::from([String::new(), "Adwaita".into()]);
    for path in paths {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                let valid = match key {
                    "theme/name" => {
                        p.join("labwc/themerc").is_file() || p.join("openbox-3/themerc").is_file()
                    }
                    "XCURSOR_THEME" => p.join("cursors").is_dir(),
                    _ => {
                        p.join("index.theme").is_file()
                            && fs::read_dir(&p).is_ok_and(|es| {
                                es.flatten()
                                    .any(|e| e.path().is_dir() && e.file_name() != "cursors")
                            })
                    }
                };
                if valid {
                    names.insert(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
    }
    names.into_iter().collect()
}
/// Installed XKB data, including variants and layout-switch options.
pub fn xkb(section: &str) -> Vec<(String, String)> {
    let text = fs::read_to_string("/usr/share/X11/xkb/rules/evdev.lst").unwrap_or_default();
    let mut active = "";
    let mut result = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(s) = line.strip_prefix("! ") {
            active = s;
            continue;
        }
        if active != section {
            continue;
        }
        if let Some((code, desc)) = line.split_once(char::is_whitespace) {
            let desc = desc.trim();
            if section == "variant" {
                if let Some((layout, description)) = desc.split_once(':') {
                    result.push((
                        format!("{}({code})", layout.trim()),
                        description.trim().into(),
                    ));
                }
            } else if section != "option" || code.starts_with("grp:") {
                result.push((code.into(), desc.into()));
            }
        }
    }
    result
}
