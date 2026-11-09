//! Accept all strong matches' count-first confirm: how many Unfiled Documents it would file, asked
//! before any is. A confirm hands the page [`DocumentsOutcome::AcceptAll`], which files them as
//! one batch, since the page holds what the batch keeps in view and leaves last.

use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{DialogFrame, Hint, count};

use crate::msg;
use crate::protocols::DocumentsOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptAllEvent {
    /// A Dialog keymap command or a typed key: `select` and `submit` confirm, and every other
    /// key does nothing.
    Input(DialogInput),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptAllViewModel {
    pub frame: DialogFrame,
    pub body: String,
    pub note: String,
    /// The status line's hints while the Dialog is open.
    pub hints: Vec<Hint>,
}

pub struct AcceptAllPresenter {
    /// The Dialog keymap's chords, which its hints show.
    keys: DialogKeys,
    /// The Strong matches when the page asked.
    strong: usize,
}

impl AcceptAllPresenter {
    /// The confirm for `strong` matches. `keys` are the Dialog keymap's chords, which its hints
    /// show.
    pub fn new(strong: usize, keys: DialogKeys) -> Self {
        Self { keys, strong }
    }

    /// The status line's hints, their chords the Dialog keys': cancel and confirm.
    fn hints(&self) -> Vec<Hint> {
        let key = |command| self.keys.chord(command).to_string();
        vec![
            Hint::new(key(BACK), msg::documents_dialog_hint_cancel()),
            Hint::new(key(SELECT), msg::documents_dialog_hint_confirm()),
        ]
    }
}

impl Presenter for AcceptAllPresenter {
    type Event = AcceptAllEvent;
    type ViewModel = AcceptAllViewModel;
    type Router<'r> = dyn DialogRouter<DocumentsOutcome> + 'r;

    fn handle(&mut self, event: AcceptAllEvent, router: &mut Self::Router<'_>) {
        router.close(match event {
            AcceptAllEvent::Input(DialogInput::Select | DialogInput::Submit)
            | AcceptAllEvent::Confirm => Some(DocumentsOutcome::AcceptAll),
            AcceptAllEvent::Input(_) => return,
            AcceptAllEvent::Cancel => None,
        });
    }

    fn view_model(&self) -> AcceptAllViewModel {
        AcceptAllViewModel {
            hints: self.hints(),
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
}

impl DialogPresenter for AcceptAllPresenter {
    type Outcome = DocumentsOutcome;

    fn input_event(input: DialogInput) -> AcceptAllEvent {
        AcceptAllEvent::Input(input)
    }

    fn confirm_event() -> AcceptAllEvent {
        AcceptAllEvent::Confirm
    }

    fn cancel_event() -> AcceptAllEvent {
        AcceptAllEvent::Cancel
    }

    fn can_confirm(&self) -> bool {
        true
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
        let view = AcceptAllPresenter::new(2, DialogKeys::default()).view_model();
        assert_eq!(view.body, "File 2 documents with strong suggestions?");
        assert_eq!(view.frame.submit, "File 2");
        assert_eq!(
            AcceptAllPresenter::new(1, DialogKeys::default())
                .view_model()
                .body,
            "File 1 document with a strong suggestion?"
        );
    }

    #[test]
    fn confirm_hands_the_page_the_batch_and_cancel_nothing() {
        let mut router = RecordingDocumentsRouter::default();
        let mut presenter = AcceptAllPresenter::new(2, DialogKeys::default());
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

    #[test]
    fn select_and_submit_confirm_and_every_other_key_is_swallowed() {
        let mut router = RecordingDocumentsRouter::default();
        let mut presenter = AcceptAllPresenter::new(2, DialogKeys::default());
        for input in [
            DialogInput::Char('x'),
            DialogInput::Backspace,
            DialogInput::NextField,
            DialogInput::SelectNext,
        ] {
            presenter.handle(AcceptAllEvent::Input(input), &mut router);
        }
        assert!(router.take().is_empty());

        presenter.handle(AcceptAllEvent::Input(DialogInput::Select), &mut router);
        presenter.handle(AcceptAllEvent::Input(DialogInput::Submit), &mut router);
        assert_eq!(
            router.take(),
            [
                DocumentsCall::CloseDialog(Some(DocumentsOutcome::AcceptAll)),
                DocumentsCall::CloseDialog(Some(DocumentsOutcome::AcceptAll)),
            ]
        );
    }
}
