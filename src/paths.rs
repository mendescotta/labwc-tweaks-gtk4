pub fn config_dir() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Some(dir) = std::env::var_os("LABWC_CONFIG_DIR").filter(|s| !s.is_empty()) {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| gtk::glib::home_dir().join(".config"));
    let xfce = base.join("xfce4/labwc");
    if std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .any(|s| s.eq_ignore_ascii_case("xfce"))
        && xfce.join("rc.xml").is_file()
    {
        return xfce;
    }
    base.join("labwc")
}
