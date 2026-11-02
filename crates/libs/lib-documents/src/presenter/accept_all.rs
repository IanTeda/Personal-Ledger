//! Accept all strong matches' count-first confirm: how many Unfiled Documents it would file, asked
//! before any is. A confirm hands the page [`DocumentsOutcome::AcceptAll`], which files them as
//! one batch, since the page holds what the batch keeps in view and leaves last.

use lib_viper::keymap::{BACK, Keymap, SELECT};
use lib_viper::view::{DialogFrame, Hint};

use super::count;
use crate::msg;
use crate::protocols::{DocumentsOutcome, DocumentsRouter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptAllEvent {
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptAllViewModel {
    pub frame: DialogFrame,
    pub body: String,
    pub note: String,
}

pub struct AcceptAllPresenter {
    /// The Strong matches when the page asked.
    strong: usize,
}

impl AcceptAllPresenter {
    pub fn new(strong: usize) -> Self {
        Self { strong }
    }

    pub fn handle(&mut self, event: AcceptAllEvent, router: &mut dyn DocumentsRouter) {
        router.close_dialog(match event {
            AcceptAllEvent::Confirm => Some(DocumentsOutcome::AcceptAll),
            AcceptAllEvent::Cancel => None,
        });
    }

    pub fn view_model(&self) -> AcceptAllViewModel {
        AcceptAllViewModel {
            frame: DialogFrame {
                title: msg::documents_dialog_accept_all_title(),
                submit: msg::documents_dialog_accept_all_submit(&self.strong.to_string()),
                can_confirm: true,
                destructive: false,
                hints: Vec::new(),
            },
            body: msg::documents_dialog_accept_all_body(count(self.strong)),
            note: msg::documents_dialog_accept_all_note(),
        }
    }

    /// The status line's legend: cancel and confirm.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        vec![
            Hint::new(key(BACK), msg::documents_dialog_hint_cancel()),
            Hint::new(key(SELECT), msg::documents_dialog_hint_confirm()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::locale;
    use super::*;
    use crate::fake::{DocumentsCall, RecordingDocumentsRouter};

    #[test]
    fn it_asks_with_the_count() {
        locale();
        let view = AcceptAllPresenter::new(2).view_model();
        assert_eq!(view.body, "File 2 documents with strong suggestions?");
        assert_eq!(view.frame.submit, "File 2");
        assert_eq!(
            AcceptAllPresenter::new(1).view_model().body,
            "File 1 document with a strong suggestion?"
        );
    }

    #[test]
    fn confirm_hands_the_page_the_batch_and_cancel_nothing() {
        let mut router = RecordingDocumentsRouter::default();
        let mut presenter = AcceptAllPresenter::new(2);
        presenter.handle(AcceptAllEvent::Confirm, &mut router);
        presenter.handle(AcceptAllEvent::Cancel, &mut router);
        assert_eq!(
            router.take(),
            [
                DocumentsCall::CloseDialog(Some(DocumentsOutcome::AcceptAll)),
                DocumentsCall::CloseDialog(None),
            ]
        );
    }
}
