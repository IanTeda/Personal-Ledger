//! What Other, the Default Document Type, gets in place of the Remove Dialog: a notice saying why
//! it has no remove action. Confirm and Cancel both only close it; nothing is written.

use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::view::DialogFrame;

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
}

#[derive(Debug, Default)]
pub struct DefaultNoticePresenter;

impl DefaultNoticePresenter {
    pub fn handle(
        &mut self,
        event: DefaultNoticeEvent,
        router: &mut dyn DialogRouter<DocumentTypesOutcome>,
    ) {
        match event {
            DefaultNoticeEvent::Input(DialogInput::Select | DialogInput::Submit)
            | DefaultNoticeEvent::Confirm
            | DefaultNoticeEvent::Cancel => router.close(None),
            DefaultNoticeEvent::Input(_) => {}
        }
    }

    pub fn view_model(&self) -> DefaultNoticeViewModel {
        DefaultNoticeViewModel {
            frame: DialogFrame {
                title: msg::documents_types_dialog_default_title(),
                submit: msg::documents_types_dialog_close(),
                can_confirm: true,
                destructive: false,
                hints: Vec::new(),
            },
            text: msg::documents_types_default_notice(),
        }
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
        let mut presenter = DefaultNoticePresenter;
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
