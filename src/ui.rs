use crate::config::Config;
use crate::localization::translate as tr;
use crate::schema::pages;
use gtk::prelude::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
pub fn build(app: &gtk::Application) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Labwc Tweaks (GTK4)")
        .default_width(820)
        .default_height(620)
        .build();
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = gtk::HeaderBar::new();
    let apply = gtk::Button::with_label(tr("Apply"));
    apply.add_css_class("suggested-action");
    apply.set_widget_name("apply-settings");
    apply.set_sensitive(false);
    header.pack_end(&apply);
    window.set_titlebar(Some(&header));
    let status = gtk::Label::new(None);
    status.set_wrap(true);
    status.set_margin_top(12);
    status.set_margin_bottom(12);
    let dir = crate::paths::config_dir();
    let config = match Config::load(dir) {
        Ok(c) => Rc::new(RefCell::new(c)),
        Err(e) => {
            status.set_text(&format!("Cannot load configuration: {e}"));
            outer.append(&status);
            window.set_child(Some(&outer));
            window.present();
            return window;
        }
    };
    let changes = Rc::new(RefCell::new(BTreeMap::<String, String>::new()));
    let stack = gtk::Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    let sidebar = gtk::StackSidebar::new();
    sidebar.set_stack(&stack);
    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    body.append(&sidebar);
    body.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    body.append(&stack);
    outer.append(&body);
    outer.append(&status);
    let mut curve_rows = Vec::new();
    let mut controls = BTreeMap::<String, gtk::Widget>::new();
    for (title, fields) in pages() {
        let grid = gtk::Grid::builder()
            .row_spacing(16)
            .column_spacing(28)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();
        let count = fields.len() as i32;
        for (i, (label, key, default, kind)) in fields.into_iter().enumerate() {
            let initial = config.borrow().get(key).unwrap_or(default.into());
            let l = gtk::Label::new(Some(tr(label)));
            l.set_halign(gtk::Align::Start);
            l.set_wrap(true);
            l.set_max_width_chars(38);
            grid.attach(&l, 0, i as i32, 1, 1);
            let changes = changes.clone();
            let apply = apply.clone();
            let options = crate::catalog::options(key);
            let field_key = key;
            let key = key.to_owned();
            let cfg = config.clone();
            let fallback = default.to_owned();
            let changed = move |v: String| {
                let mut c = changes.borrow_mut();
                if v == cfg.borrow().get(&key).unwrap_or(fallback.clone()) {
                    c.remove(&key);
                } else {
                    c.insert(key.clone(), v);
                }
                let validation = cfg.borrow().validate(&c);
                apply.set_tooltip_text(validation.as_ref().err().map(|e| e.to_string()).as_deref());
                apply.set_sensitive(!c.is_empty() && validation.is_ok());
            };

            if kind == "bool" {
                let w = gtk::Switch::new();
                w.set_halign(gtk::Align::End);
                w.set_active(matches!(
                    initial.trim().to_ascii_lowercase().as_str(),
                    "yes" | "true" | "on" | "1"
                ));
                w.connect_active_notify(move |w| {
                    changed(if w.is_active() { "yes" } else { "no" }.into())
                });
                grid.attach(&w, 1, i as i32, 1, 1);
            } else if kind == "integer" || kind == "number" {
                let (min, max, step) = crate::ranges::bounds(field_key);
                let existing = initial.parse::<f64>().unwrap_or(min);
                let w = gtk::SpinButton::with_range(min.min(existing), max.max(existing), step);
                w.set_digits(if kind == "integer" { 0 } else { 2 });
                w.set_value(existing);
                w.connect_value_changed(move |w| changed(w.value().to_string()));
                grid.attach(&w, 1, i as i32, 1, 1);
            } else if !options.is_empty() {
                let mut options = options;
                if !options.contains(&initial) {
                    options.push(initial.clone());
                }
                let refs: Vec<&str> = options.iter().map(String::as_str).collect();
                let w = gtk::DropDown::from_strings(&refs);
                w.set_selected(options.iter().position(|v| v == &initial).unwrap_or(0) as u32);
                w.set_hexpand(true);
                w.connect_selected_notify(move |w| {
                    if let Some(v) = options.get(w.selected() as usize) {
                        changed(v.clone());
                    }
                });
                grid.attach(&w, 1, i as i32, 1, 1);
            } else {
                let w = gtk::Entry::new();
                w.set_text(&initial);
                w.set_hexpand(true);
                w.connect_changed(move |w| changed(w.text().to_string()));
                if field_key == "XKB_DEFAULT_LAYOUT" || field_key == "XKB_DEFAULT_OPTIONS" {
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                    row.append(&w);
                    let add = gtk::Button::with_label("Choose…");
                    row.append(&add);
                    let entry = w.clone();
                    let parent = window.clone();
                    add.connect_clicked(move |_| {
                        choose_xkb(&parent, &entry, field_key == "XKB_DEFAULT_LAYOUT")
                    });
                    grid.attach(&row, 1, i as i32, 1, 1);
                } else {
                    grid.attach(&w, 1, i as i32, 1, 1);
                }
            }
            if let Some(w) = grid.child_at(1, i as i32) {
                w.set_widget_name(field_key);
                controls.insert(field_key.into(), w);
            }
        }
        if title == "Mouse" {
            crate::curves::add(&grid, count, config.clone(), changes.clone(), &apply);
            for row in count..count + 3 {
                if let Some(w) = grid.child_at(0, row) {
                    curve_rows.push(w);
                }
            }
        }
        let scroll = gtk::ScrolledWindow::builder().child(&grid).build();
        stack.add_titled(&scroll, Some(title), tr(title));
    }

    let drafts = crate::drafts::Drafts::default();
    let advanced =
        crate::structured::page(&window, config.clone(), changes.clone(), &apply, &drafts);
    stack.add_titled(
        &gtk::ScrolledWindow::builder().child(&advanced).build(),
        Some("Collections"),
        "Collections",
    );

    let environment =
        crate::environment::page(&window, config.clone(), changes.clone(), &apply, &drafts);
    stack.add_titled(&environment, Some("Environment"), "Environment");
    for (source, targets) in [
        ("theme/dropShadows", vec!["theme/dropShadowsOnTiled"]),
        (
            "focus/followMouse",
            vec![
                "focus/followMouseRequiresMovement",
                "focus/raiseOnFocus",
                "focus/raiseOnFocusDelay",
            ],
        ),
    ] {
        if let Some(w) = controls
            .get(source)
            .and_then(|w| w.clone().downcast::<gtk::Switch>().ok())
        {
            let dependents: Vec<_> = targets
                .iter()
                .filter_map(|k| controls.get(*k).cloned())
                .collect();
            for target in &dependents {
                target.set_sensitive(w.is_active());
            }
            w.connect_active_notify(move |w| {
                for target in &dependents {
                    target.set_sensitive(w.is_active());
                }
            });
        }
    }
    if let (Some(profile), Some(speed)) = (
        controls
            .get("libinput/device/accelProfile")
            .and_then(|w| w.clone().downcast::<gtk::DropDown>().ok()),
        controls.get("libinput/device/pointerSpeed"),
    ) {
        let speed = speed.clone();
        let update = move |w: &gtk::DropDown| {
            let custom = w
                .selected_item()
                .and_then(|o| o.downcast::<gtk::StringObject>().ok())
                .is_some_and(|o| o.string() == "custom");
            speed.set_sensitive(!custom);
            for row in &curve_rows {
                row.set_sensitive(custom);
            }
        };
        update(&profile);
        profile.connect_selected_notify(update);
    }
    let about = gtk::Box::new(gtk::Orientation::Vertical, 16);
    for text in [
        "Labwc Tweaks GTK4",
        env!("CARGO_PKG_VERSION"),
        "Native GTK4 desktop settings • GPL-2.0-only",
        "An independent project, not affiliated with or endorsed by labwc.
Based on labwc-tweaks and labwc-tweaks-gtk.",
    ] {
        let l = gtk::Label::new(Some(text));
        l.set_wrap(true);
        about.append(&l);
    }
    let link = gtk::LinkButton::with_label(
        "https://github.com/labwc/labwc-tweaks",
        "Upstream labwc-tweaks",
    );
    about.append(&link);
    let location = gtk::Label::new(Some(&config.borrow().dir.display().to_string()));
    location.set_selectable(true);
    location.set_wrap(true);
    about.append(&location);

    for name in [
        "LABWC_VER",
        "LABWC_PID",
        "XDG_CURRENT_DESKTOP",
        "WAYLAND_DISPLAY",
    ] {
        let l = gtk::Label::new(Some(&format!(
            "{name}: {}",
            std::env::var(name).unwrap_or("Unknown".into())
        )));
        l.set_selectable(true);
        about.append(&l);
    }
    let features = gtk::Label::new(Some("Reading compositor build information…"));
    features.set_wrap(true);
    features.set_selectable(true);
    about.append(&features);
    gtk::glib::MainContext::default().spawn_local(async move {
        match gtk::gio::Subprocess::newv(
            &[std::ffi::OsStr::new("labwc"), std::ffi::OsStr::new("-v")],
            gtk::gio::SubprocessFlags::STDOUT_PIPE | gtk::gio::SubprocessFlags::STDERR_PIPE,
        ) {
            Ok(p) => match p.communicate_utf8_future(None).await {
                Ok((out, err)) => features.set_text(&format!(
                    "Compositor build: {} {}",
                    out.unwrap_or_default(),
                    err.unwrap_or_default()
                )),
                Err(e) => features.set_text(&format!("Build information unavailable: {e}")),
            },
            Err(_) => {
                features.set_text("Compositor build information unavailable (labwc not installed)")
            }
        }
    });
    stack.add_titled(&about, Some("About"), tr("About"));
    status.set_text(&format!(
        "{} • Changes are saved when you apply.",
        config.borrow().dir.display()
    ));
    let force_close = Rc::new(std::cell::Cell::new(false));
    let c = changes.clone();
    let cfg = config.clone();
    let s = status.clone();
    let commit: Rc<dyn Fn()> = Rc::new({
        let parent = window.clone();
        let app = app.clone();
        let selected = stack.clone();
        let button = apply.clone();
        let force_close = force_close.clone();
        move || {
            let edits = c.borrow().clone();
            let saved = cfg.borrow_mut().save(&edits);
            match saved {
                Ok(()) => {
                    c.borrow_mut().clear();
                    button.set_sensitive(false);
                    let page = selected.visible_child_name();
                    let refreshed = build(&app);
                    force_close.set(true);
                    parent.close();
                    if let Some(page) = page {
                        select_page(&refreshed, &page);
                    }
                    let s = refreshed
                        .child()
                        .and_then(|w| w.last_child())
                        .and_then(|w| w.downcast::<gtk::Label>().ok())
                        .unwrap_or_else(|| s.clone());
                    s.set_text("Saved. Reloading labwc…");
                    gtk::glib::MainContext::default().spawn_local(async move {
 match gtk::gio::Subprocess::newv(&[std::ffi::OsStr::new("labwc"),std::ffi::OsStr::new("-r")],gtk::gio::SubprocessFlags::STDERR_SILENCE) {
 Ok(p)=>match p.wait_check_future().await {Ok(_)=>s.set_text("Saved and reloaded. Environment changes may require signing in again."),Err(e)=>s.set_text(&format!("Saved; labwc reload failed: {e}"))},
 Err(e)=>s.set_text(&format!("Saved; could not reload labwc: {e}"))
 }
 });
                }
                Err(e) => s.set_text(&format!("Save failed: {e}")),
            }
        }
    });
    let parent = window.clone();
    let staged = changes.clone();
    let pending = drafts.clone();
    apply.connect_clicked(move |_|{
        if pending.any() {
            let commit = commit.clone();
            let staged = staged.clone();
            let parent2 = parent.clone();
            crate::drafts::confirm(&parent, "Unstaged edits will be lost", "Applying rebuilds the window. Environment text that is not staged would be discarded. Apply anyway?", move || run_apply(&parent2, &staged, &commit));
            return;
        }
        run_apply(&parent, &staged, &commit);
    });
    let dirty = changes;
    let pending = drafts;
    window.connect_close_request(move |w| {
        if force_close.get() || (dirty.borrow().is_empty() && !pending.any()) {
            return gtk::glib::Propagation::Proceed;
        }
        let dialog = gtk::MessageDialog::builder()
            .transient_for(w)
            .modal(true)
            .text("Discard unsaved changes?")
            .buttons(gtk::ButtonsType::YesNo)
            .build();
        let w = w.clone();
        let dirty = dirty.clone();
        let force_close = force_close.clone();
        dialog.connect_response(move |d, r| {
            d.close();
            if r == gtk::ResponseType::Yes {
                force_close.set(true);
                dirty.borrow_mut().clear();
                w.close();
            }
        });
        dialog.present();
        gtk::glib::Propagation::Stop
    });
    window.set_child(Some(&outer));
    window.present();
    window
}
fn run_apply(
    parent: &gtk::ApplicationWindow,
    staged: &Rc<RefCell<BTreeMap<String, String>>>,
    commit: &Rc<dyn Fn()>,
) {
    if staged
        .borrow()
        .get("libinput/device/sendEventsMode")
        .is_some_and(|v| v == "no")
        || staged.borrow().keys().any(|k| {
            k.starts_with("#section/")
                || k.starts_with("#env/")
                || k == "core/autoEnableOutputs"
                || k == "core/promptCommand"
                || k == "core/errorCommand"
        })
    {
        let commit = commit.clone();
        crate::drafts::confirm(parent, "Apply advanced configuration changes?", "Bindings and command hooks can execute programs; device/output settings can disable interaction. Confirm the staged changes before saving. Backups and interrupted-save recovery are available.", move || commit());
    } else {
        commit();
    }
}
fn select_page(window: &gtk::ApplicationWindow, name: &str) {
    fn visit(w: &gtk::Widget, name: &str) {
        if let Ok(stack) = w.clone().downcast::<gtk::Stack>() {
            stack.set_visible_child_name(name);
            return;
        }
        let mut child = w.first_child();
        while let Some(c) = child {
            visit(&c, name);
            child = c.next_sibling();
        }
    }
    visit(window.upcast_ref(), name);
}
fn choose_xkb(parent: &gtk::ApplicationWindow, entry: &gtk::Entry, layouts: bool) {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(if layouts {
            "Add keyboard layout"
        } else {
            "Choose layout switch shortcut"
        })
        .default_width(600)
        .default_height(480)
        .build();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let search = gtk::SearchEntry::new();
    column.append(&search);
    let list = gtk::ListBox::new();
    let mut data = crate::catalog::xkb(if layouts { "layout" } else { "option" });
    if layouts {
        data.extend(crate::catalog::xkb("variant"));
    }
    for (code, description) in &data {
        let label = gtk::Label::new(Some(&format!("{description} [{code}]")));
        label.set_wrap(true);
        label.set_xalign(0.0);
        label.set_margin_top(6);
        label.set_margin_bottom(6);
        list.append(&label);
    }
    let query = Rc::new(RefCell::new(String::new()));
    let q = query.clone();
    let filter_data = data.clone();
    list.set_filter_func(move |row| {
        filter_data.get(row.index() as usize).is_some_and(|(c, d)| {
            format!("{c} {d}")
                .to_lowercase()
                .contains(q.borrow().as_str())
        })
    });
    let l = list.clone();
    search.connect_search_changed(move |s| {
        *query.borrow_mut() = s.text().to_lowercase();
        l.invalidate_filter();
    });
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    column.append(&scroll);
    let e = entry.clone();
    let d = dialog.clone();
    list.connect_row_activated(move |_, row| {
        if let Some((code, _)) = data.get(row.index() as usize) {
            let text = e.text();
            let mut values: Vec<&str> = text.split(',').filter(|s| !s.is_empty()).collect();
            if !layouts {
                values.retain(|s| !s.starts_with("grp:"));
            }
            if !values.contains(&code.as_str()) {
                values.push(code);
            }
            e.set_text(&values.join(","));
            d.close();
        }
    });
    dialog.set_child(Some(&column));
    dialog.present();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a graphical display; run through tools/gui-smoke.py"]
    fn gtk_pages() {
        gtk::init().unwrap();
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("LABWC_CONFIG_DIR", temp.path());
        let app = gtk::Application::builder()
            .application_id("org.labwc.TweaksGtk4.Test")
            .build();
        app.register(gtk::gio::Cancellable::NONE).unwrap();
        build(&app);
        let w = app.active_window().unwrap();
        fn find(widget: &gtk::Widget) -> Option<gtk::Stack> {
            if let Ok(stack) = widget.clone().downcast::<gtk::Stack>() {
                return Some(stack);
            }
            let mut child = widget.first_child();
            while let Some(c) = child {
                if let Some(s) = find(&c) {
                    return Some(s);
                }
                child = c.next_sibling();
            }
            None
        }
        let stack = find(w.upcast_ref()).unwrap();
        assert_eq!(
            stack.pages().n_items(),
            (crate::schema::pages().len() + 3) as u32
        );
        for page in crate::schema::pages()
            .into_iter()
            .map(|(name, _)| name)
            .chain(["Collections", "Environment", "About"])
        {
            stack.set_visible_child_name(page);
            assert_eq!(stack.visible_child_name().as_deref(), Some(page));
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
        }
        w.close();
        assert!(!temp.path().join("rc.xml").exists());
    }

    #[test]
    #[ignore = "requires isolated nested labwc; tools/nested-labwc.py"]
    fn nested_live() {
        let dir = std::path::PathBuf::from(
            std::env::var_os("LABWC_TWEAKS_TEST_CONFIG").expect("isolated config"),
        );
        assert_eq!(
            std::env::var_os("LABWC_CONFIG_DIR").unwrap(),
            dir.as_os_str()
        );
        assert!(std::env::var_os("LABWC_PID").is_some());
        gtk::init().unwrap();
        let app = gtk::Application::builder()
            .application_id("org.labwc.TweaksGtk4.NestedTest")
            .build();
        app.register(gtk::gio::Cancellable::NONE).unwrap();
        let mut window = build(&app);
        fn pump() {
            let context = gtk::glib::MainContext::default();
            for _ in 0..20 {
                while context.pending() {
                    context.iteration(false);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        pump();
        for page in crate::schema::pages().into_iter().map(|(n, _)| n).chain([
            "Collections",
            "Environment",
            "About",
        ]) {
            select_page(&window, page);
            pump();
        }
        fn buttons(widget: &gtk::Widget, out: &mut Vec<gtk::Button>) {
            if let Ok(b) = widget.clone().downcast::<gtk::Button>() {
                out.push(b);
            }
            let mut child = widget.first_child();
            while let Some(c) = child {
                buttons(&c, out);
                child = c.next_sibling();
            }
        }
        select_page(&window, "Collections");
        let mut found = Vec::new();
        buttons(window.upcast_ref(), &mut found);
        for (title, _) in crate::structured::SECTIONS {
            found
                .iter()
                .find(|b| b.label().as_deref() == Some(title))
                .unwrap()
                .emit_clicked();
            pump();
            for w in gtk::Window::list_toplevels()
                .into_iter()
                .filter_map(|w| w.downcast::<gtk::Window>().ok())
            {
                if w.title().as_deref() == Some(title) {
                    w.close();
                }
            }
        }
        fn named(w: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
            if w.widget_name() == name {
                return Some(w.clone());
            }
            let mut child = w.first_child();
            while let Some(c) = child {
                if let Some(found) = named(&c, name) {
                    return Some(found);
                }
                child = c.next_sibling();
            }
            None
        }

        found
            .iter()
            .find(|b| b.label().as_deref() == Some("Snap regions"))
            .unwrap()
            .emit_clicked();
        pump();
        let dialog = gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Window>().ok())
            .find(|w| w.title().as_deref() == Some("Snap regions"))
            .unwrap();
        let mut dialog_buttons = Vec::new();
        buttons(dialog.upcast_ref(), &mut dialog_buttons);
        dialog_buttons
            .iter()
            .find(|b| b.label().as_deref() == Some("Add record"))
            .unwrap()
            .emit_clicked();
        dialog_buttons
            .iter()
            .find(|b| b.label().as_deref() == Some("Stage collection"))
            .unwrap()
            .emit_clicked();
        pump();
        named(window.upcast_ref(), "core/gap")
            .unwrap()
            .downcast::<gtk::SpinButton>()
            .unwrap()
            .set_value(8.);
        named(window.upcast_ref(), "apply-settings")
            .unwrap()
            .downcast::<gtk::Button>()
            .unwrap()
            .emit_clicked();
        pump();
        for w in gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::MessageDialog>().ok())
        {
            w.response(gtk::ResponseType::Yes);
        }
        pump();
        window = app
            .active_window()
            .unwrap()
            .downcast::<gtk::ApplicationWindow>()
            .unwrap();
        assert_eq!(
            crate::config::Config::load(dir.clone())
                .unwrap()
                .get("core/gap")
                .as_deref(),
            Some("8")
        );
        let mut cfg = crate::config::Config::load(dir.clone()).unwrap();
        let mut edits = BTreeMap::new();
        for (section, xml) in [
            (
                "regions",
                r#"<section><regions><region name="test" x="0%" y="0%" width="50%" height="100%"/></regions></section>"#,
            ),
            (
                "windowRules",
                r#"<section><windowRules><windowRule identifier="org.labwc.TweaksGtk4.NestedTest" skipTaskbar="no"><action name="Raise"/></windowRule></windowRules></section>"#,
            ),
            (
                "theme",
                r#"<section><theme><font place="ActiveWindow" name="sans" size="11"/></theme></section>"#,
            ),
            (
                "libinput",
                r#"<section><libinput><device category="touchpad"><naturalScroll>yes</naturalScroll></device><device category="non-touch"><leftHanded>no</leftHanded></device></libinput></section>"#,
            ),
            (
                "keyboard",
                r#"<section><keyboard><default/><keybind key="W-F12"><action name="Execute" command="true"/></keybind></keyboard></section>"#,
            ),
            (
                "mouse",
                r#"<section><mouse><default/><context name="Root"><mousebind button="Right" action="Press"><action name="ShowMenu" menu="root-menu"/></mousebind></context></mouse></section>"#,
            ),
            (
                "desktops",
                r#"<section><desktops number="2"><names><name>One</name><name>Two</name></names></desktops></section>"#,
            ),
            (
                "tablet",
                r#"<section><tablet rotate="90"><map button="Stylus" to="Right"/></tablet></section>"#,
            ),
        ] {
            edits.insert(format!("#section/{section}"), xml.into());
        }
        cfg.save(&edits).unwrap();
        assert!(std::process::Command::new("labwc")
            .arg("-r")
            .status()
            .unwrap()
            .success());
        pump();
        assert!(
            std::path::Path::new(&format!("/proc/{}", std::env::var("LABWC_PID").unwrap()))
                .exists()
        );
        window.close();
        pump();
        assert!(!dir.join(".labwc-tweaks-journal").exists());
    }
}
