//! The `ViewEvent` channel into `Shell`: `Shell` subscribes to each View Entity and handles
//! every event in `handle_view_event`, the one place a View's request turns into `Shell` state
//! (ADR-0032). An `impl Shell` block, not a module of the same name.

use gpui::{Context, Entity, EventEmitter};

use super::Shell;
use crate::view::event::ViewEvent;

impl Shell {
    /// Subscribes `Shell` to a View Entity's events. The subscription is kept on `Shell`, since
    /// dropping it would silently stop the View's events.
    pub(super) fn subscribe_view<V: EventEmitter<ViewEvent>>(
        &mut self,
        view: &Entity<V>,
        cx: &mut Context<'_, Self>,
    ) {
        let subscription = cx.subscribe(view, |shell, _view, event: &ViewEvent, cx| {
            shell.handle_view_event(event.clone());
            // A View's request changes what `Shell` draws, and nothing else schedules a frame.
            cx.notify();
        });
        self.view_subscriptions.push(subscription);
    }

    /// Applies one View request to `Shell`'s own state, using the same helpers the keyboard and
    /// the palette use, so a View's request and a keypress leave the same state behind.
    pub(super) fn handle_view_event(&mut self, event: ViewEvent) {
        match event {
            ViewEvent::RaiseToast { kind, text } => self.raise_toast(kind, text),
            ViewEvent::OpenDialog(dialog) => self.open_dialog(*dialog),
            ViewEvent::Navigate(noun) => {
                let noun_before = self.nav.noun();
                self.nav.set_noun(noun);
                if self.nav.noun() != noun_before {
                    self.reset_view_scroll();
                }
            }
            ViewEvent::SetStatus(text) => self.chrome.status_message = Some(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, EventEmitter, TestAppContext};
    use lib_toast::ToastKind;

    use super::*;
    use crate::{
        chrome::dialog_host::{OpenDialog, ToastHistoryDialog},
        navigation::nav::{InputMode, NavState, Noun},
    };

    /// A stand-in for a View Entity: it emits whatever the test hands it and holds no `Shell`.
    struct StandInView;

    impl EventEmitter<ViewEvent> for StandInView {}

    #[gpui::test]
    fn each_view_event_reaches_shell(cx: &mut TestAppContext) {
        crate::locale::init_for_tests();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 9).expect("a valid date");
        let shell = cx.new(|cx| Shell::with_today(NavState::new(), cx.focus_handle(), today));
        let view = cx.new(|_| StandInView);
        shell.update(cx, |shell, cx| shell.subscribe_view(&view, cx));

        view.update(cx, |_, cx| {
            cx.emit(ViewEvent::RaiseToast {
                kind: ToastKind::Success,
                text: "Saved".to_string(),
            });
        });
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| {
            let toasts = shell.chrome.toasts.visible();
            assert_eq!(toasts.len(), 1);
            assert_eq!(toasts[0].text(), "Saved");
        });

        view.update(cx, |_, cx| {
            cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::ToastHistory(
                ToastHistoryDialog,
            ))));
        });
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| {
            assert!(shell.chrome.dialog.is_some());
            assert_eq!(shell.nav.mode(), InputMode::Dialog);
        });

        view.update(cx, |_, cx| cx.emit(ViewEvent::Navigate(Noun::Bills)));
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| assert_eq!(shell.nav.noun(), Noun::Bills));

        view.update(cx, |_, cx| {
            cx.emit(ViewEvent::SetStatus("Exported".to_string()))
        });
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| {
            assert_eq!(shell.status_message(), Some("Exported"));
        });
    }
}
