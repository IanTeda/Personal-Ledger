//! The Clear logs Dialog's Presenter (issue #502): a destructive confirm with nothing to fill in,
//! since the capture is emptied for good. `Enter` or Confirm clears it through [`TracingLog`] and
//! raises the Toast; `Esc` or Cancel keeps it. The Client hosts it as a module Dialog.

use std::rc::Rc;

use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::Hint;

use crate::msg;
use crate::protocols::TracingLog;

/// What a confirmed Clear logs did, for the page that opened it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearLogsOutcome {
    /// The capture was emptied, so the page empties its mirror.
    Cleared,
}

/// What the Clear logs Dialog takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearLogsEvent {
    /// A Dialog command or typed key: `Enter` confirms, and the Dialog has no field to type into.
    Input(DialogInput),
    /// The confirm button.
    Confirm,
    Cancel,
}

/// The Dialog as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearLogsViewModel {
    pub title: String,
    pub warning: String,
    /// The confirm button's label.
    pub confirm: String,
    /// The status line's legend while the Dialog is open.
    pub hints: Vec<Hint>,
}

/// The Clear logs Dialog's Presenter.
pub struct ClearLogsPresenter {
    log: Rc<dyn TracingLog>,
    keys: DialogKeys,
}

impl ClearLogsPresenter {
    /// The Dialog over `log`. `keys` are the Dialog keymap's chords, which its hints show.
    pub fn new(log: Rc<dyn TracingLog>, keys: DialogKeys) -> Self {
        Self { log, keys }
    }

    /// Empties the capture, says so and closes. Nothing can refuse it.
    fn clear(&mut self, router: &mut dyn DialogRouter<ClearLogsOutcome>) {
        self.log.clear();
        router.notify(Notice::Toast(
            ToastKind::Info,
            msg::settings_tracing_toast_cleared(),
        ));
        router.close(Some(ClearLogsOutcome::Cleared));
    }
}

impl Presenter for ClearLogsPresenter {
    type Event = ClearLogsEvent;
    type ViewModel = ClearLogsViewModel;
    type Router<'r> = dyn DialogRouter<ClearLogsOutcome> + 'r;

    /// `Enter` confirms; every other input is swallowed. `Esc`, the Shell's `back`, keeps the
    /// capture as it was.
    fn handle(&mut self, event: ClearLogsEvent, router: &mut Self::Router<'_>) {
        match event {
            ClearLogsEvent::Input(DialogInput::Select | DialogInput::Submit)
            | ClearLogsEvent::Confirm => self.clear(router),
            ClearLogsEvent::Input(_) => {}
            ClearLogsEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> ClearLogsViewModel {
        ClearLogsViewModel {
            title: msg::settings_tracing_clear_title(),
            warning: msg::settings_tracing_clear_warning(),
            confirm: msg::settings_tracing_clear(),
            // A plain confirm has at most the one confirm field, so no `tab`.
            hints: vec![
                self.keys.hint(BACK, lib_locale::msg::dialog_hint_cancel()),
                self.keys
                    .hint(SELECT, lib_locale::msg::dialog_hint_confirm()),
            ],
        }
    }
}

impl DialogPresenter for ClearLogsPresenter {
    type Outcome = ClearLogsOutcome;

    fn input_event(input: DialogInput) -> ClearLogsEvent {
        ClearLogsEvent::Input(input)
    }

    fn confirm_event() -> ClearLogsEvent {
        ClearLogsEvent::Confirm
    }

    fn cancel_event() -> ClearLogsEvent {
        ClearLogsEvent::Cancel
    }

    fn can_confirm(&self) -> bool {
        true
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
        (
            ClearLogsPresenter::new(log.clone(), DialogKeys::default()),
            log,
        )
    }

    #[test]
    fn enter_clears_the_capture_raises_an_info_toast_and_closes() {
        let (mut presenter, log) = presenter();
        let mut router = Recording::default();

        presenter.handle(ClearLogsEvent::Input(DialogInput::Select), &mut router);

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

        presenter.handle(ClearLogsEvent::Cancel, &mut router);

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
            presenter.handle(ClearLogsEvent::Input(input), &mut router);
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
