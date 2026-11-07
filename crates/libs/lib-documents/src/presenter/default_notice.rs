//! What Other, the Default Document Type, gets in place of the Remove Dialog: a notice saying why
//! it has no remove action. Confirm and Cancel both only close it; nothing is written.

use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{DialogFrame, Hint};

use crate::msg;
use crate::protocols::DocumentTypesOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultNoticeEvent {
    /// A Dialog command or typed key: `select` and `submit` close it, and nothing else does
    /// anything.
    Input(DialogInput),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultNoticeViewModel {
    pub frame: DialogFrame,
    pub text: String,
    /// The status line's legend while the Dialog is open.
    pub hints: Vec<Hint>,
}

#[derive(Debug)]
pub struct DefaultNoticePresenter {
    keys: DialogKeys,
}

impl DefaultNoticePresenter {
    /// `keys` are the Dialog keymap's chords, which its hints show.
    pub fn new(keys: DialogKeys) -> Self {
        Self { keys }
    }
}

impl Presenter for DefaultNoticePresenter {
    type Event = DefaultNoticeEvent;
    type ViewModel = DefaultNoticeViewModel;
    type Router<'r> = dyn DialogRouter<DocumentTypesOutcome> + 'r;

    fn handle(&mut self, event: DefaultNoticeEvent, router: &mut Self::Router<'_>) {
        match event {
            DefaultNoticeEvent::Input(DialogInput::Select | DialogInput::Submit)
            | DefaultNoticeEvent::Confirm
            | DefaultNoticeEvent::Cancel => router.close(None),
            DefaultNoticeEvent::Input(_) => {}
        }
    }

    fn view_model(&self) -> DefaultNoticeViewModel {
        DefaultNoticeViewModel {
            frame: DialogFrame {
                title: msg::documents_types_dialog_default_title(),
                submit: msg::documents_types_dialog_close(),
                can_confirm: true,
                destructive: false,
                hints: Vec::new(),
            },
            text: msg::documents_types_default_notice(),
            hints: vec![
                self.keys.hint(BACK, lib_locale::msg::dialog_hint_cancel()),
                self.keys
                    .hint(SELECT, lib_locale::msg::dialog_hint_confirm()),
            ],
        }
    }
}

impl DialogPresenter for DefaultNoticePresenter {
    type Outcome = DocumentTypesOutcome;

    fn input_event(input: DialogInput) -> DefaultNoticeEvent {
        DefaultNoticeEvent::Input(input)
    }

    fn confirm_event() -> DefaultNoticeEvent {
        DefaultNoticeEvent::Confirm
    }

    fn cancel_event() -> DefaultNoticeEvent {
        DefaultNoticeEvent::Cancel
    }

    /// Close is always on offer.
    fn can_confirm(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::locale;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};

    #[test]
    fn the_notice_says_why_and_confirm_or_cancel_only_close_it() {
        locale();
        let mut presenter = DefaultNoticePresenter::new(DialogKeys::default());
        let mut router = RecordingRouter::default();
        let view = presenter.view_model();

        assert_eq!(view.frame.title, "Other can't be removed");
        assert_eq!(view.frame.submit, "Close");
        assert!(view.frame.can_confirm);
        assert!(view.text.starts_with("Other is the default type"));
        presenter.handle(
            DefaultNoticeEvent::Input(DialogInput::Char('x')),
            &mut router,
        );
        presenter.handle(
            DefaultNoticeEvent::Input(DialogInput::NextField),
            &mut router,
        );
        assert_eq!(router.take(), [], "only select and submit close it");
        presenter.handle(DefaultNoticeEvent::Confirm, &mut router);
        presenter.handle(DefaultNoticeEvent::Cancel, &mut router);
        presenter.handle(DefaultNoticeEvent::Input(DialogInput::Select), &mut router);
        presenter.handle(DefaultNoticeEvent::Input(DialogInput::Submit), &mut router);
        assert_eq!(router.take(), vec![RouterCall::CloseDialog(None); 4]);
    }
}
