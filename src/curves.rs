use crate::config::Config;
use gtk::prelude::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
pub const NAMES: [&str; 3] = ["motionCurve", "scrollCurve", "fallbackCurve"];
pub fn validate(step: &str, points: &str) -> bool {
    let number = |v: &str| {
        v.trim()
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
    };
    let values: Vec<_> = points.split_whitespace().collect();
    number(step).is_some_and(|v| v > 0.0)
        && (2..=32).contains(&values.len())
        && values
            .into_iter()
            .all(|v| number(v).is_some_and(|n| n >= 0.0))
}
pub fn add(
    grid: &gtk::Grid,
    start: i32,
    config: Rc<RefCell<Config>>,
    changes: Rc<RefCell<BTreeMap<String, String>>>,
    apply: &gtk::Button,
) {
    for (index, name) in NAMES.iter().enumerate() {
        let path = format!("libinput/device/{name}");
        let define = gtk::CheckButton::with_label(&format!("Define {name}"));
        define
            .set_active(config.borrow().get(&format!("{path}/defined")).as_deref() == Some("yes"));
        let step = gtk::Entry::new();
        step.set_placeholder_text(Some("Step > 0"));
        step.set_text(
            &config
                .borrow()
                .get(&format!("{path}/step"))
                .unwrap_or("1".into()),
        );
        let points = gtk::Entry::new();
        points.set_placeholder_text(Some("2–32 non-negative points"));
        points.set_text(
            &config
                .borrow()
                .get(&format!("{path}/points"))
                .unwrap_or("0 1".into()),
        );
        let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
        row.append(&define);
        row.append(&step);
        row.append(&points);
        grid.attach(&row, 0, start + index as i32, 2, 1);
        step.set_sensitive(define.is_active());
        points.set_sensitive(define.is_active());
        let d = define.clone();
        let s = step.clone();
        let p = points.clone();
        let cfg = config.clone();
        let edits = changes.clone();
        let button = apply.clone();
        let update: Rc<dyn Fn()> = Rc::new(move || {
            s.set_sensitive(d.is_active());
            p.set_sensitive(d.is_active());
            let mut changes = edits.borrow_mut();
            let initial_defined =
                cfg.borrow().get(&format!("{path}/defined")).as_deref() == Some("yes");
            let values = [
                (
                    "defined",
                    if d.is_active() {
                        "yes".into()
                    } else {
                        "no".into()
                    },
                ),
                ("step", s.text().to_string()),
                ("points", p.text().to_string()),
            ];
            for (field, value) in values {
                let key = format!("{path}/{field}");
                let same = cfg.borrow().get(&key).unwrap_or_else(|| {
                    match field {
                        "defined" => "no",
                        "step" => "1",
                        _ => "0 1",
                    }
                    .into()
                }) == value;
                if same || (field != "defined" && !d.is_active()) {
                    changes.remove(&key);
                } else {
                    changes.insert(key, value);
                }
            }
            if d.is_active() && !initial_defined {
                changes.insert(format!("{path}/step"), s.text().into());
                changes.insert(format!("{path}/points"), p.text().into());
            }
            let validation = cfg.borrow().validate(&changes);
            button.set_tooltip_text(validation.as_ref().err().map(|e| e.to_string()).as_deref());
            button.set_sensitive(!changes.is_empty() && validation.is_ok());
        });
        let u = update.clone();
        define.connect_toggled(move |_| u());
        let u = update.clone();
        step.connect_changed(move |_| u());
        points.connect_changed(move |_| update());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numbers() {
        assert!(validate("0,5", "0 1 2"));
        assert!(!validate("0", "0 1"));
        assert!(!validate("1", "NaN 1"));
        assert!(!validate("1", "1"));
        assert!(!validate("inf", "0 1"));
        assert!(!validate("1", "-1 2"));
    }
}
