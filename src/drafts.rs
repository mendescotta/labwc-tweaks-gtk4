// Tracks editor drafts that are not yet staged, so closing, loading or
// rebuilding the window can warn instead of silently discarding them.
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};
type Check = Rc<dyn Fn() -> bool>;
#[derive(Clone, Default)]
pub struct Drafts(Rc<RefCell<Vec<Check>>>);
impl Drafts {
    /// Registers a check that returns true while a draft has unstaged edits.
    pub fn register(&self, dirty: impl Fn() -> bool + 'static) {
        self.0.borrow_mut().push(Rc::new(dirty));
    }
    pub fn any(&self) -> bool {
        let checks: Vec<_> = self.0.borrow().clone();
        checks.iter().any(|f| f())
    }
}
/// Asks a Yes/No question (default No) and runs `on_yes` after confirmation.
pub fn confirm(
    parent: &impl IsA<gtk::Window>,
    text: &str,
    detail: &str,
    on_yes: impl Fn() + 'static,
) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .text(text)
        .secondary_text(detail)
        .buttons(gtk::ButtonsType::YesNo)
        .build();
    dialog.set_default_response(gtk::ResponseType::No);
    dialog.connect_response(move |d, r| {
        d.close();
        if r == gtk::ResponseType::Yes {
            on_yes();
        }
    });
    dialog.present();
}
