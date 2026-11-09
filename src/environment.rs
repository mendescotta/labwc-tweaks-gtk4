use gtk::prelude::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
pub fn validate(text: &str) -> Result<(), String> {
    if text.contains('\0') {
        return Err("Environment cannot contain NUL".into());
    }
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, _)) = line.split_once('=') else {
            return Err(format!("Line {} needs NAME=value", i + 1));
        };
        if line
            .split_once('=')
            .is_some_and(|(_, value)| value.trim().len() > 1024)
        {
            return Err(format!(
                "Value on line {} exceeds labwc's 1024-byte limit",
                i + 1
            ));
        }
        let mut chars = name.trim().chars();
        if !chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(format!("Invalid variable on line {}", i + 1));
        }
    }
    Ok(())
}
pub fn set(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<_> = text
        .lines()
        .filter(|l| l.split_once('=').is_none_or(|(k, _)| k.trim() != key))
        .map(str::to_owned)
        .collect();
    lines.push(format!("{key}={value}"));
    lines.join("\n") + "\n"
}
pub fn page(
    window: &gtk::ApplicationWindow,
    config: Rc<RefCell<crate::config::Config>>,
    changes: Rc<RefCell<BTreeMap<String, String>>>,
    apply: &gtk::Button,
    drafts: &crate::drafts::Drafts,
) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let help=gtk::Label::new(Some("Environment assignments are read at session startup. Files in environment.d ending in .env override environment in filename order. Values are saved as text; this editor does not execute them."));
    help.set_wrap(true);
    page.append(&help);
    let mut existing = vec!["environment".to_owned()];
    existing.extend(config.borrow().env_files.keys().cloned());
    let files = gtk::Label::new(Some(&format!("Existing files: {}", existing.join(", "))));
    files.set_wrap(true);
    page.append(&files);
    if !config.borrow().skipped.is_empty() {
        let warn = gtk::Label::new(Some(&format!(
            "Unsupported file names in environment.d (they may override values, and environment edits are blocked until renamed): {}",
            config.borrow().skipped.join(", ")
        )));
        warn.set_wrap(true);
        page.append(&warn);
    }
    let file = gtk::Entry::new();
    file.set_placeholder_text(Some("environment or environment.d/90-local.env"));
    file.set_text("environment");
    page.append(&file);
    // The file the buffer was loaded from, and the text it held when loaded or
    // last staged. Staging under a different name is an explicit Save As.
    let loaded = Rc::new(RefCell::new("environment".to_owned()));
    let baseline = Rc::new(RefCell::new(config.borrow().environment().to_owned()));
    let text = gtk::TextView::new();
    text.set_monospace(true);
    text.buffer().set_text(&baseline.borrow());
    let read_buffer = {
        let buffer = text.buffer();
        move || {
            buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .to_string()
        }
    };
    {
        let (read, baseline) = (read_buffer.clone(), baseline.clone());
        drafts.register(move || read() != *baseline.borrow());
    }
    let status = gtk::Label::new(None);
    status.set_wrap(true);
    let current = {
        let (cfg, entries) = (config.clone(), changes.clone());
        move |name: &str| -> Option<String> {
            if let Some(v) = entries.borrow().get(&format!("#env/{name}")) {
                return Some(v.clone());
            }
            if name == "environment" {
                Some(cfg.borrow().environment().to_owned())
            } else {
                cfg.borrow().env_files.get(name).cloned()
            }
        }
    };
    let load = gtk::Button::with_label("Load file (new name starts an empty file)");
    {
        let (name, buffer, loaded, baseline) = (
            file.clone(),
            text.buffer(),
            loaded.clone(),
            baseline.clone(),
        );
        let (read, current, status, window) = (
            read_buffer.clone(),
            current.clone(),
            status.clone(),
            window.clone(),
        );
        load.connect_clicked(move |_| {
            let target = name.text().trim().to_owned();
            let do_load = {
                let (buffer, loaded, baseline, current, status) = (
                    buffer.clone(),
                    loaded.clone(),
                    baseline.clone(),
                    current.clone(),
                    status.clone(),
                );
                move || {
                    let value = current(&target).unwrap_or_default();
                    buffer.set_text(&value);
                    *baseline.borrow_mut() = value;
                    *loaded.borrow_mut() = target.clone();
                    status.set_text(&format!("Loaded {target}."));
                }
            };
            if read() != *baseline.borrow() {
                crate::drafts::confirm(
                    &window,
                    "Discard the unstaged environment text?",
                    "Loading another file replaces the text you have edited but not staged.",
                    do_load,
                );
            } else {
                do_load();
            }
        });
    }
    page.append(&load);
    page.append(
        &gtk::ScrolledWindow::builder()
            .vexpand(true)
            .child(&text)
            .build(),
    );
    page.append(&status);
    let stage = gtk::Button::with_label("Stage environment file");
    let apply = apply.clone();
    let window = window.clone();
    stage.connect_clicked(move |_| {
        let name = file.text().trim().to_owned();
        let value = read_buffer();
        let do_stage = {
            let (changes, config, apply, status) =
                (changes.clone(), config.clone(), apply.clone(), status.clone());
            let (loaded, baseline, name, value) =
                (loaded.clone(), baseline.clone(), name.clone(), value.clone());
            move || {
                let mut edits = changes.borrow().clone();
                edits.insert(format!("#env/{name}"), value.clone());
                match config.borrow().validate(&edits) {
                    Ok(()) => {
                        *changes.borrow_mut() = edits;
                        *loaded.borrow_mut() = name.clone();
                        *baseline.borrow_mut() = value.clone();
                        apply.set_sensitive(true);
                        status.set_text(&format!(
                            "Staged {name}. Use Apply to save; sign in again to use environment changes."
                        ));
                    }
                    Err(e) => status.set_text(&e.to_string()),
                }
            }
        };
        let replaces_other = *loaded.borrow() != name && current(&name).is_some();
        if replaces_other {
            crate::drafts::confirm(
                &window,
                &format!("Replace the existing file {name}?"),
                &format!(
                    "The text was loaded from {}. Staging it as {name} replaces that file's contents when applied.",
                    loaded.borrow()
                ),
                do_stage,
            );
        } else {
            do_stage();
        }
    });
    page.append(&stage);
    page
}
