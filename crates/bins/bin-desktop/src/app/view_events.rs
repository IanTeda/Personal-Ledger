//! The `ViewEvent` channel into `Shell`: `Shell` subscribes to each View Entity and handles
//! every event in `handle_view_event`, the one place a View's request turns into `Shell` state
//! (ADR-0032, ADR-0037). As the root Router, it turns a Toast or a status a View not yet a module
//! raised into one input (`ShellInput::Notice`), so it reaches the frame as a module's Notice
//! does; the other requests are handled as before. An `impl Shell` block, not a module of the
//! same name.

use gpui::{Context, Entity, EventEmitter, Subscription};
use lib_shell::ShellInput;
use lib_toast::Notice;

use super::Shell;
use crate::view::event::{Reveal, ViewEvent};

impl Shell {
    /// Subscribes `Shell` to a View Entity's events. The subscription is kept on `Shell`, since
    /// dropping it would silently stop the View's events.
    pub(super) fn subscribe_view<V: EventEmitter<ViewEvent>>(
        &mut self,
        view: &Entity<V>,
        cx: &mut Context<'_, Self>,
    ) {
        let subscription = Self::view_event_subscription(view, cx);
        self.view_subscriptions.push(subscription);
    }

    /// The subscription that hands a View's events to [`Self::handle_view_event`], for a View
    /// built before `Shell` itself is.
    pub(super) fn view_event_subscription<V: EventEmitter<ViewEvent>>(
        view: &Entity<V>,
        cx: &mut Context<'_, Self>,
    ) -> Subscription {
        cx.subscribe(view, |shell, _view, event: &ViewEvent, cx| {
            shell.handle_view_event(event.clone(), cx);
            // A View's request changes what `Shell` draws, and nothing else schedules a frame.
            cx.notify();
        })
    }

    /// Applies one View request to `Shell`'s own state, using the same helpers the keyboard and
    /// the palette use, so a View's request and a keypress leave the same state behind.
    pub(super) fn handle_view_event(&mut self, event: ViewEvent, cx: &mut Context<'_, Self>) {
        match event {
            ViewEvent::RaiseToast { kind, text } => {
                self.send_shell(ShellInput::Notice(Notice::Toast(kind, text)), cx);
            }
            // The legacy arm: the twelve Dialogs not yet on a module's Presenter (#820).
            ViewEvent::OpenDialog(dialog) => self.open_dialog(*dialog),
            ViewEvent::OpenHostedDialog(open) => self.open_hosted_dialog(*open),
            ViewEvent::HostDialog(dialog) => self.host_dialog(dialog, cx),
            ViewEvent::Navigate(noun) => {
                let noun_before = self.nav.noun();
                self.nav.set_noun(noun);
                if self.nav.noun() != noun_before {
                    self.reset_view_scroll(cx);
                }
            }
            ViewEvent::SetStatus(Some(text)) => {
                self.send_shell(ShellInput::Notice(Notice::Status(text)), cx);
            }
            // A Notice carries text only, so clearing stays the View's request.
            ViewEvent::SetStatus(None) => self.chrome.status_message = None,
            ViewEvent::Reveal(reveal) => self.reveal(reveal, cx),
            ViewEvent::EnterMode(mode) => {
                self.nav.enter_mode(mode.input_mode());
                self.view_mode = Some(mode);
            }
            // Only the View's own mode: a mode entered since (a Dialog) is left alone.
            ViewEvent::ExitMode => {
                if self.view_owns_mode() {
                    self.nav.exit_mode();
                }
                self.view_mode = None;
            }
            ViewEvent::RunCommand(name) => self.handle_empty_state_command_click(name, cx),
            ViewEvent::SettingsPageChanged(page) => self.nav.record_settings_page(page),
            ViewEvent::DisplayChanged(display) => self.push_display_preferences(display, cx),
        }
    }

    /// A cross-View jump: makes the record's destination active, then asks its View to show it.
    pub(super) fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        self.nav.set_noun(reveal.noun());
        self.reset_view_scroll(cx);
        self.active_view_entity(cx).reveal(reveal, cx);
    }

    /// Whether the current mode is one the active View entered itself, so its keys are the View's.
    pub(super) fn view_owns_mode(&self) -> bool {
        self.view_mode
            .is_some_and(|mode| mode.input_mode() == self.nav.mode())
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
        let storage = crate::ledger_store::LedgerSetup::stub(today);
        let shell = cx.new(|cx| {
            Shell::with_clock(
                NavState::new(),
                cx.focus_handle(),
                lib_core::Clock::fixed(today),
                crate::units::UnitsSetup::stub(),
                crate::bills::BillsSetup::stub(&storage),
                storage,
                cx,
            )
        });
        // A Toast or status arrives as one input, which drains into the Shell through the door.
        cx.update(|cx| {
            crate::input::install(
                std::rc::Rc::new(lib_database::InMemoryCommit),
                shell.downgrade(),
                cx,
            );
        });
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
            cx.emit(ViewEvent::SetStatus(Some("Exported".to_string())))
        });
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| {
            assert_eq!(shell.status_message(), Some("Exported"));
        });

        view.update(cx, |_, cx| cx.emit(ViewEvent::SetStatus(None)));
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| assert_eq!(shell.status_message(), None));
    }

    #[gpui::test]
    fn a_reveal_makes_its_destination_active_and_shows_the_record(cx: &mut TestAppContext) {
        crate::locale::init_for_tests();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 9).expect("a valid date");
        let storage = crate::ledger_store::LedgerSetup::stub(today);
        let shell = cx.new(|cx| {
            Shell::with_clock(
                NavState::new(),
                cx.focus_handle(),
                lib_core::Clock::fixed(today),
                crate::units::UnitsSetup::stub(),
                crate::bills::BillsSetup::stub(&storage),
                storage,
                cx,
            )
        });
        let view = cx.new(|_| StandInView);
        shell.update(cx, |shell, cx| shell.subscribe_view(&view, cx));
        let (id, last_plan) = shell.read_with(cx, |shell, cx| {
            lib_bills::planner_order(shell.bill_plans(cx))
                .last()
                .map(|plan| (plan.id, plan.name.clone()))
                .expect("the seed has Bill Plans")
        });

        view.update(cx, |_, cx| cx.emit(ViewEvent::Reveal(Reveal::BillPlan(id))));
        cx.run_until_parked();
        shell.read_with(cx, |shell, cx| {
            assert_eq!(shell.nav.noun(), Noun::Bills);
            let page = shell.bills_snapshot(cx);
            assert_eq!(page.tab, "Planner");
            assert_eq!(page.plans[page.selected], last_plan);
        });
    }
}
