//! What Other, the Default Document Type, gets in place of the Remove Dialog: a notice saying why
//! it has no remove action. Confirm and Cancel both only close it; nothing is written.

use lib_viper::view::DialogFrame;

use crate::msg;
use crate::protocols::DocumentTypesRouter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultNoticeEvent {
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
    pub fn handle(&mut self, _event: DefaultNoticeEvent, router: &mut dyn DocumentTypesRouter) {
        router.close_dialog(None);
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
        presenter.handle(DefaultNoticeEvent::Confirm, &mut router);
        presenter.handle(DefaultNoticeEvent::Cancel, &mut router);
        assert_eq!(
            router.take(),
            [RouterCall::CloseDialog(None), RouterCall::CloseDialog(None)]
        );
    }
}
