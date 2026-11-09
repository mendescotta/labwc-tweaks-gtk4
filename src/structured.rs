// Structured access to every XML collection, including nested actions.
use gtk::prelude::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use xmltree::{Element, XMLNode};
pub const SECTIONS: &[(&str, &str)] = &[
    ("Keyboard bindings", "keyboard"),
    ("Mouse bindings", "mouse"),
    ("Window rules", "windowRules"),
    ("Snap regions", "regions"),
    ("Workspace names", "desktops"),
    ("Fonts by role", "theme"),
    ("Input device categories", "libinput"),
    ("Touch and output mappings", "touch"),
    ("Tablet mappings", "tablet"),
    ("Tablet tools", "tabletTool"),
    ("Output margins", "margin"),
    ("Switcher fields", "windowSwitcher"),
    ("Privileged interfaces", "privilegedInterfaces"),
    ("Core and command hooks", "core"),
    ("Focus", "focus"),
    ("Placement", "placement"),
    ("Resistance", "resistance"),
    ("Snapping", "snapping"),
    ("Resize", "resize"),
    ("Menus", "menu"),
    ("Magnifier", "magnifier"),
];
pub fn valid_name(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}
pub fn parse(section: &str, text: &str) -> Result<Element, String> {
    if !SECTIONS.iter().any(|(_, s)| *s == section) {
        return Err("Unknown collection".into());
    }
    let root = Element::parse(text.as_bytes()).map_err(|e| e.to_string())?;
    if root.name != "section" || !root.attributes.is_empty() {
        return Err("Invalid collection container".into());
    }
    for child in root.children.iter().filter_map(XMLNode::as_element) {
        if child.name != section {
            return Err("Unexpected collection element".into());
        }
        validate(child)?;
    }
    Ok(root)
}
fn validate(e: &Element) -> Result<(), String> {
    if !valid_name(&e.name) || e.attributes.keys().any(|s| !valid_name(s)) {
        return Err("Invalid element or attribute name".into());
    }
    let required: &[&str] = match e.name.as_str() {
        "keybind" => &["key"],
        "context" => &["name"],
        "action" => &["name"],
        "region" => &["name", "x", "y", "width", "height"],
        "map" => &["button", "to"],
        _ => &[],
    };
    for key in required {
        if e.attributes.get(*key).is_none_or(|v| v.trim().is_empty()) {
            return Err(format!("{} requires {key}", e.name));
        }
    }
    if e.name == "region" {
        for k in ["x", "y", "width", "height"] {
            let n = e.attributes[k]
                .strip_suffix('%')
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|n| n.is_finite() && (0.0..=100.0).contains(n));
            if n.is_none() || (["width", "height"].contains(&k) && n == Some(0.0)) {
                return Err(
                    "Region geometry requires percentages from 0 to 100, with positive size".into(),
                );
            }
        }
    }

    if e.name == "action"
        && e.attributes
            .get("name")
            .is_some_and(|n| n.eq_ignore_ascii_case("Execute"))
    {
        let command = e
            .attributes
            .get("command")
            .cloned()
            .or_else(|| {
                e.get_child("command")
                    .and_then(|c| c.get_text())
                    .map(|v| v.into_owned())
            })
            .unwrap_or_default();
        if command.trim().is_empty() {
            return Err("Execute action requires a command".into());
        }
    }
    if e.name == "mousebind"
        && !e.attributes.contains_key("button")
        && !e.attributes.contains_key("direction")
    {
        return Err("Mouse binding requires a button or scroll direction".into());
    }
    if e.name == "font" {
        if let Some(size) = e.attributes.get("size") {
            if !size.parse::<f64>().is_ok_and(|v| v.is_finite() && v > 0.) {
                return Err("Font size must be positive".into());
            }
        }
    }
    if e.name == "device" {
        for c in e.children.iter().filter_map(XMLNode::as_element) {
            let value = c.get_text().unwrap_or_default();
            let range = match c.name.as_str() {
                "pointerSpeed" => Some((-1., 1.)),
                "rotationAngle" => Some((0., 359.)),
                "scrollFactor" => Some((0., 10000.)),
                _ => None,
            };
            if let Some((min, max)) = range {
                if !value
                    .parse::<f64>()
                    .is_ok_and(|n| n.is_finite() && (min..=max).contains(&n))
                {
                    return Err(format!("Invalid {} for input device", c.name));
                }
            }
        }
    }
    for child in e.children.iter().filter_map(XMLNode::as_element) {
        validate(child)?;
    }
    Ok(())
}
pub fn serialize(e: &Element) -> String {
    let mut b = Vec::new();
    e.write_with_config(&mut b, xmltree::EmitterConfig::new().perform_indent(true))
        .expect("Vec write");
    String::from_utf8(b).expect("XML is UTF-8")
}
fn at<'a>(e: &'a mut Element, path: &[usize]) -> &'a mut Element {
    if let Some((i, rest)) = path.split_first() {
        at(e.children[*i].as_mut_element().unwrap(), rest)
    } else {
        e
    }
}
#[derive(Clone)]
struct Editor {
    tree: Rc<RefCell<Element>>,
    list: gtk::ListBox,
    form: gtk::Box,
    paths: Rc<RefCell<Vec<Vec<usize>>>>,
}
impl Editor {
    fn selected(&self) -> Vec<usize> {
        self.list
            .selected_row()
            .and_then(|r| self.paths.borrow().get(r.index() as usize).cloned())
            .unwrap_or_default()
    }
    fn refresh(&self) {
        while let Some(w) = self.list.first_child() {
            self.list.remove(&w);
        }
        let mut rows = Vec::new();
        fn visit(e: &Element, path: Vec<usize>, rows: &mut Vec<(Vec<usize>, String)>) {
            let desc = e
                .attributes
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ");
            rows.push((
                path.clone(),
                format!("{}{} {}", "  ".repeat(path.len()), e.name, desc),
            ));
            for (i, c) in e.children.iter().enumerate() {
                if let Some(c) = c.as_element() {
                    let mut p = path.clone();
                    p.push(i);
                    visit(c, p, rows);
                }
            }
        }
        visit(&self.tree.borrow(), vec![], &mut rows);
        *self.paths.borrow_mut() = rows.iter().map(|(p, _)| p.clone()).collect();
        for (_, label) in rows {
            let l = gtk::Label::new(Some(&label));
            l.set_xalign(0.);
            l.set_ellipsize(gtk::pango::EllipsizeMode::End);
            self.list.append(&l);
        }
    }
    fn fields(&self) {
        while let Some(w) = self.form.first_child() {
            self.form.remove(&w);
        }
        if self.list.selected_row().is_none() {
            return;
        }
        let path = self.selected();
        let e = at(&mut self.tree.borrow_mut(), &path).clone();
        let title = gtk::Label::new(Some(&e.name));
        self.form.append(&title);
        for (key, value) in &e.attributes {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            row.append(&gtk::Label::new(Some(key)));
            let entry = gtk::Entry::new();
            entry.set_text(value);
            entry.set_hexpand(true);
            row.append(&entry);
            let tree = self.tree.clone();
            let p = path.clone();
            let k = key.clone();
            entry.connect_changed(move |v| {
                at(&mut tree.borrow_mut(), &p)
                    .attributes
                    .insert(k.clone(), v.text().into());
            });
            let remove = gtk::Button::with_label("Remove");
            row.append(&remove);
            let this = self.clone();
            let k = key.clone();
            let p = path.clone();
            remove.connect_clicked(move |_| {
                at(&mut this.tree.borrow_mut(), &p)
                    .attributes
                    .shift_remove(&k);
                this.fields();
            });
            self.form.append(&row);
        }
        let text = gtk::Entry::new();
        text.set_placeholder_text(Some("Element text"));
        text.set_text(&e.get_text().unwrap_or_default());
        self.form.append(&text);
        let tree = self.tree.clone();
        let this = self.clone();
        let p = path.clone();
        text.connect_changed(move |entry| {
            let mut tree = tree.borrow_mut();
            let e = at(&mut tree, &p);
            // Replace the first text node in place so element child indices
            // (used by the row paths) never shift.
            let first = e
                .children
                .iter()
                .position(|c| matches!(c, XMLNode::Text(_) | XMLNode::CData(_)));
            let mut seen = false;
            e.children.retain(|c| {
                let is_text = matches!(c, XMLNode::Text(_) | XMLNode::CData(_));
                let keep = !is_text || !seen;
                seen |= is_text;
                keep
            });
            let new = entry.text().to_string();
            match (first, new.is_empty()) {
                (Some(i), true) => {
                    e.children.remove(i);
                    drop(tree);
                    this.refresh();
                }
                (Some(i), false) => e.children[i] = XMLNode::Text(new),
                (None, false) => {
                    e.children.push(XMLNode::Text(new));
                }
                (None, true) => {}
            }
        });
        let name = gtk::Entry::new();
        name.set_placeholder_text(Some("New attribute name"));
        self.form.append(&name);
        let value = gtk::Entry::new();
        value.set_placeholder_text(Some("New attribute value"));
        self.form.append(&value);
        let add = gtk::Button::with_label("Add attribute");
        let this = self.clone();
        let p = path.clone();
        add.connect_clicked(move |_| {
            if valid_name(&name.text()) {
                at(&mut this.tree.borrow_mut(), &p)
                    .attributes
                    .insert(name.text().into(), value.text().into());
                this.fields();
            }
        });
        self.form.append(&add);
        let child = gtk::Entry::new();
        child.set_placeholder_text(Some("Child element: keybind, action, region, font, name…"));
        let suggestions = match e.name.as_str() {
            "section" => "Add a record using the button below the tree",
            "keyboard" => "keybind, default, repeatRate, repeatDelay",
            "keybind" | "mousebind" | "windowRule" => "action",
            "mouse" => "context, default, doubleClickTime",
            "context" => "mousebind",
            "windowRules" => "windowRule",
            "regions" => "region",
            "desktops" => "names, initial, popupTime, prefix",
            "names" => "name",
            "theme" => "font, titlebar, name, cornerRadius",
            "font" => "name, size, slant, weight",
            "libinput" => "device",
            "device" => "naturalScroll, pointerSpeed, accelProfile, tap, scrollFactor",
            "tablet" => "area, map",
            "windowSwitcher" => "osd, fields",
            "fields" => "field",
            _ => "Enter a child setting name from labwc-config(5)",
        };
        child.set_placeholder_text(Some(suggestions));
        self.form.append(&child);
        let add = gtk::Button::with_label("Add child");
        let this = self.clone();
        let p = path;
        add.connect_clicked(move |_| {
            let name = child.text();
            if !valid_name(&name) {
                return;
            }
            at(&mut this.tree.borrow_mut(), &p)
                .children
                .push(XMLNode::Element(template(&name)));
            this.refresh();
        });
        self.form.append(&add);
    }
}
fn template(name: &str) -> Element {
    let mut e = Element::new(name);
    let attrs: &[(&str, &str)] = match name {
        "keybind" => &[("key", "W-F1")],
        "action" => &[("name", "Execute"), ("command", "")],
        "context" => &[("name", "Root")],
        "mousebind" => &[("button", "Right"), ("action", "Press")],
        "windowRule" => &[("identifier", "app.id")],
        "region" => &[
            ("name", "region"),
            ("x", "0%"),
            ("y", "0%"),
            ("width", "50%"),
            ("height", "100%"),
        ],
        "font" => &[("place", "ActiveWindow")],
        "device" => &[("category", "touchpad")],
        "touch" => &[
            ("deviceName", ""),
            ("mapToOutput", ""),
            ("mouseEmulation", "no"),
        ],
        "tablet" => &[("mapToOutput", ""), ("rotate", "0")],
        "map" => &[("button", "Stylus"), ("to", "Right")],
        _ => &[],
    };
    for (k, v) in attrs {
        e.attributes.insert((*k).into(), (*v).into());
    }
    e
}

fn add_record(tree: &mut Element, section: &str) {
    let child = match section {
        "keyboard" => Some("keybind"),
        "mouse" => Some("context"),
        "windowRules" => Some("windowRule"),
        "regions" => Some("region"),
        "theme" => Some("font"),
        "libinput" => Some("device"),
        "tablet" => Some("map"),
        "windowSwitcher" => Some("fields"),
        "desktops" => Some("names"),
        _ => None,
    };
    if let Some(child) = child {
        let index = tree
            .children
            .iter()
            .position(|c| c.as_element().is_some_and(|e| e.name == section))
            .unwrap_or_else(|| {
                tree.children.push(XMLNode::Element(template(section)));
                tree.children.len() - 1
            });
        let container = tree.children[index].as_mut_element().unwrap();
        if section == "desktops" {
            let index = container
                .children
                .iter()
                .position(|c| c.as_element().is_some_and(|e| e.name == "names"))
                .unwrap_or_else(|| {
                    container
                        .children
                        .push(XMLNode::Element(Element::new("names")));
                    container.children.len() - 1
                });
            let mut name = Element::new("name");
            name.children.push(XMLNode::Text("New workspace".into()));
            container.children[index]
                .as_mut_element()
                .unwrap()
                .children
                .push(XMLNode::Element(name));
        } else {
            container.children.push(XMLNode::Element(template(child)));
        }
    } else {
        tree.children.push(XMLNode::Element(template(section)));
    }
}
pub fn page(
    parent: &gtk::ApplicationWindow,
    config: Rc<RefCell<crate::config::Config>>,
    changes: Rc<RefCell<BTreeMap<String, String>>>,
    apply: &gtk::Button,
    drafts: &crate::drafts::Drafts,
) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let help=gtk::Label::new(Some("Edit collections and advanced options. Select a node to edit text and attributes; add, remove or reorder nested elements. Stage changes here, then Apply in the main window. Command hooks and Execute actions run when labwc invokes them."));
    help.set_wrap(true);
    page.append(&help);
    for (title, section) in SECTIONS {
        let button = gtk::Button::with_label(title);
        page.append(&button);
        let parent = parent.clone();
        let cfg = config.clone();
        let edits = changes.clone();
        let apply = apply.clone();
        let drafts = drafts.clone();
        button.connect_clicked(move |_| {
            let key = format!("#section/{section}");
            let source = edits
                .borrow()
                .get(&key)
                .cloned()
                .unwrap_or_else(|| cfg.borrow().collection(section));
            let tree = match parse(section, &source) {
                Ok(t) => t,
                Err(e) => {
                    let d = gtk::MessageDialog::builder()
                        .transient_for(&parent)
                        .modal(true)
                        .text("This section cannot be opened in the editor")
                        .secondary_text(format!(
                            "The existing configuration fails validation: {e}. Fix it in rc.xml by hand."
                        ))
                        .buttons(gtk::ButtonsType::Close)
                        .build();
                    d.connect_response(|d, _| d.close());
                    d.present();
                    return;
                }
            };
            let dialog = gtk::Window::builder()
                .transient_for(&parent)
                .modal(true)
                .title(*title)
                .default_width(960)
                .default_height(680)
                .build();
            let outer = gtk::Box::new(gtk::Orientation::Vertical, 8);
            let body = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let list = gtk::ListBox::new();
            let form = gtk::Box::new(gtk::Orientation::Vertical, 6);
            let editor = Editor {
                tree: Rc::new(RefCell::new(tree)),
                list: list.clone(),
                form: form.clone(),
                paths: Rc::new(RefCell::new(Vec::new())),
            };
            editor.refresh();
            // Unstaged edits are anything that differs from what was opened.
            let baseline = serialize(&editor.tree.borrow());
            let open = Rc::new(std::cell::Cell::new(true));
            let is_dirty = {
                let (tree, baseline, open) = (editor.tree.clone(), baseline, open.clone());
                Rc::new(move || open.get() && serialize(&tree.borrow()) != baseline)
            };
            {
                let is_dirty = is_dirty.clone();
                drafts.register(move || is_dirty());
            }
            {
                let (is_dirty, open) = (is_dirty.clone(), open.clone());
                dialog.connect_close_request(move |w| {
                    if !open.get() || !is_dirty() {
                        open.set(false);
                        return gtk::glib::Propagation::Proceed;
                    }
                    let (w, open) = (w.clone(), open.clone());
                    crate::drafts::confirm(
                        &w.clone(),
                        "Discard unstaged edits in this collection?",
                        "Use Stage collection to keep them.",
                        move || {
                            open.set(false);
                            w.close();
                        },
                    );
                    gtk::glib::Propagation::Stop
                });
            }
            let ed = editor.clone();
            list.connect_row_selected(move |_, _| ed.fields());
            body.append(
                &gtk::ScrolledWindow::builder()
                    .hexpand(true)
                    .vexpand(true)
                    .child(&list)
                    .build(),
            );
            body.append(
                &gtk::ScrolledWindow::builder()
                    .hexpand(true)
                    .child(&form)
                    .build(),
            );
            outer.append(&body);
            let tools = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            for (label, delta) in [("Remove selected", 0), ("Move up", -1), ("Move down", 1)] {
                let b = gtk::Button::with_label(label);
                let ed = editor.clone();
                b.connect_clicked(move |_| {
                    let mut path = ed.selected();
                    let Some(i) = path.pop() else {
                        return;
                    };
                    let mut tree = ed.tree.borrow_mut();
                    let parent = at(&mut tree, &path);
                    if delta == 0 {
                        parent.children.remove(i);
                    } else {
                        let j = i as isize + delta;
                        if j >= 0 && (j as usize) < parent.children.len() {
                            parent.children.swap(i, j as usize);
                        }
                    }
                    drop(tree);
                    ed.refresh();
                });
                tools.append(&b);
            }
            let add = gtk::Button::with_label("Add record");
            let ed = editor.clone();
            add.connect_clicked(move |_| {
                add_record(&mut ed.tree.borrow_mut(), section);
                ed.refresh();
            });
            tools.append(&add);
            let status = gtk::Label::new(None);
            status.set_wrap(true);
            outer.append(&status);
            let stage = gtk::Button::with_label("Stage collection");
            let ed = editor;
            let edits = edits.clone();
            let cfg = cfg.clone();
            let apply = apply.clone();
            let d = dialog.clone();
            let staged_open = open;
            stage.connect_clicked(move |_| {
                let value = serialize(&ed.tree.borrow());
                let mut staged = edits.borrow().clone();
                if value == cfg.borrow().collection(section) {
                    staged.remove(&key);
                } else {
                    staged.insert(key.clone(), value);
                }
                match cfg.borrow().validate(&staged) {
                    Ok(()) => {
                        *edits.borrow_mut() = staged;
                        apply.set_sensitive(!edits.borrow().is_empty());
                        staged_open.set(false);
                        d.close();
                    }
                    Err(e) => status.set_text(&e.to_string()),
                }
            });
            tools.append(&stage);
            outer.append(&tools);
            dialog.set_child(Some(&outer));
            dialog.present();
        });
    }
    page.append(&gtk::LinkButton::with_label(
        "https://labwc.github.io/labwc-config.5.html",
        "Configuration reference",
    ));
    page.append(&gtk::LinkButton::with_label(
        "https://labwc.github.io/labwc-actions.5.html",
        "Available actions and arguments",
    ));
    page
}
