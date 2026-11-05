//! The Clear logs Dialog's Presenter (issue #502): a destructive confirm with nothing to fill in,
//! since the capture is emptied for good. `Enter` or Confirm clears it through [`TracingLog`] and
//! raises the Toast; `Esc` or Cancel keeps it. The Client hosts it as a module Dialog.

use std::rc::Rc;

use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogRouter};

use crate::msg;
use crate::protocols::TracingLog;

/// What a confirmed Clear logs did, for the page that opened it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearLogsOutcome {
    /// The capture was emptied, so the page empties its mirror.
    Cleared,
}

/// The Dialog as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearLogsViewModel {
    pub title: String,
    pub warning: String,
    /// The confirm button's label.
    pub confirm: String,
}

/// The Clear logs Dialog's Presenter.
pub struct ClearLogsPresenter {
    log: Rc<dyn TracingLog>,
}

impl ClearLogsPresenter {
    pub fn new(log: Rc<dyn TracingLog>) -> Self {
        Self { log }
    }

    pub fn view_model(&self) -> ClearLogsViewModel {
        ClearLogsViewModel {
            title: msg::settings_tracing_clear_title(),
            warning: msg::settings_tracing_clear_warning(),
            confirm: msg::settings_tracing_clear(),
        }
    }

    /// `Enter` confirms; the Dialog has no field, so every other input is swallowed.
    pub fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<ClearLogsOutcome>) {
        if matches!(input, DialogInput::Select | DialogInput::Submit) {
            self.confirm(router);
        }
    }

    /// Empties the capture, says so and closes. Nothing can refuse it.
    pub fn confirm(&mut self, router: &mut dyn DialogRouter<ClearLogsOutcome>) {
        self.log.clear();
        router.notify(Notice::Toast(
            ToastKind::Info,
            msg::settings_tracing_toast_cleared(),
        ));
        router.close(Some(ClearLogsOutcome::Cleared));
    }

    /// `Esc`, the Shell's `back`: the capture stays as it was.
    pub fn cancel(&mut self, router: &mut dyn DialogRouter<ClearLogsOutcome>) {
        router.close(None);
    }
}

#[cfg(test)]
mod tests {
    use lib_toast::Notify;

    use super::*;
    use crate::fake::InMemoryTracingLog;
    use crate::log::LogLevel;

    #[derive(Default)]
    struct Recording {
        closed: Vec<Option<ClearLogsOutcome>>,
        notices: Vec<Notice>,
    }

    impl Notify for Recording {
        fn notify(&mut self, notice: Notice) {
            self.notices.push(notice);
        }
    }

    impl DialogRouter<ClearLogsOutcome> for Recording {
        fn close(&mut self, outcome: Option<ClearLogsOutcome>) {
            self.closed.push(outcome);
        }
    }

    fn presenter() -> (ClearLogsPresenter, Rc<InMemoryTracingLog>) {
        let log = Rc::new(InMemoryTracingLog::new(8));
        log.push(LogLevel::Info, "bin_desktop::shell", "hello");
        (ClearLogsPresenter::new(log.clone()), log)
    }

    #[test]
    fn enter_clears_the_capture_raises_an_info_toast_and_closes() {
        let (mut presenter, log) = presenter();
        let mut router = Recording::default();

        presenter.input(DialogInput::Select, &mut router);

        assert!(log.since(0).is_empty());
        assert_eq!(router.closed, [Some(ClearLogsOutcome::Cleared)]);
        assert_eq!(
            router.notices,
            [Notice::Toast(ToastKind::Info, "Logs cleared".to_string())]
        );
    }

    #[test]
    fn cancel_keeps_the_capture() {
        let (mut presenter, log) = presenter();
        let mut router = Recording::default();

        presenter.cancel(&mut router);

        assert_eq!(log.since(0).len(), 1);
        assert_eq!(router.closed, [None]);
        assert!(router.notices.is_empty());
    }

    #[test]
    fn typing_and_tab_are_swallowed() {
        let (mut presenter, log) = presenter();
        let mut router = Recording::default();

        for input in [
            DialogInput::Char('x'),
            DialogInput::NextField,
            DialogInput::Backspace,
        ] {
            presenter.input(input, &mut router);
        }

        assert!(router.closed.is_empty());
        assert_eq!(log.since(0).len(), 1);
    }

    #[test]
    fn the_view_model_words_the_destructive_confirm() {
        let (presenter, _) = presenter();
        let view = presenter.view_model();
        assert_eq!(view.title, "Clear logs");
        assert_eq!(view.confirm, "Clear logs");
        assert!(view.warning.starts_with("Remove every captured log entry"));
    }
}
