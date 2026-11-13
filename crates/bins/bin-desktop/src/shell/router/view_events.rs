//! The `ViewEvent` channel into `Root`: `Root` follows each View through `PageHandle::subscribe`
//! with [`Root::page_signals`], and handles every event in `handle_view_event`, the one place a View's request turns into `Root` state
//! (ADR-0032, ADR-0037). As the root Router, it turns a Toast or a status a View not yet a module
//! raised into one input (`ShellInput::Notice`), so it reaches the frame as a module's Notice
//! does; the other requests are handled as before. An `impl Root` block, not a module of the
//! same name.

use gpui::Context;
use lib_shell::ShellInput;
use lib_toast::Notice;

use super::Root;
use crate::shell::view::page_handle::PageSignals;
use crate::view::event::{Reveal, ViewEvent};
use lib_shell::{NavInput, ShellCommand};

use crate::shell::view::window_size::WindowSize;

impl Root {
    /// Where a View's signals reach `Root`: its events to [`Self::handle_view_event`], and its
    /// changes repainting `Root`, whose status line reads the View's state. Whoever built the View
    /// keeps the subscriptions: dropping them silently stops the View's events. A row click is the
    /// View's own, as `Root` already gives the View zone focus on any click in it.
    pub(super) fn page_signals(cx: &Context<'_, Self>) -> PageSignals {
        PageSignals::to(
            cx,
            |shell, event, cx| {
                shell.handle_view_event(event.clone(), cx);
                // A View's request changes what `Root` draws, and nothing else schedules a frame.
                cx.notify();
            },
            |_, _| {},
        )
    }

    /// Applies one View request to `Root`'s own state, using the same helpers the keyboard and
    /// the palette use, so a View's request and a keypress leave the same state behind.
    pub(super) fn handle_view_event(&mut self, event: ViewEvent, cx: &mut Context<'_, Self>) {
        match event {
            ViewEvent::RaiseToast { kind, text } => {
                self.send_shell(ShellInput::Notice(Notice::Toast(kind, text)), cx);
            }
            ViewEvent::HostDialog(dialog) => self.host_dialog(dialog, cx),
            ViewEvent::Navigate(noun) => {
                let noun_before = self.nav().noun();
                self.navigate(NavInput::Show(noun));
                if self.nav().noun() != noun_before {
                    self.reset_view_scroll(cx);
                }
            }
            ViewEvent::SetStatus(Some(text)) => {
                self.send_shell(ShellInput::Notice(Notice::Status(text)), cx);
            }
            // A Notice carries text only, so clearing stays the View's request.
            ViewEvent::SetStatus(None) => self.presenter.clear_status(),
            ViewEvent::Reveal(reveal) => self.reveal(reveal, cx),
            ViewEvent::EnterMode(mode) => {
                self.navigate(NavInput::EnterViewMode(mode.input_mode()));
            }
            // Only the View's own mode: a mode entered since (a Dialog) is left alone.
            ViewEvent::ExitMode => {
                if self.view_owns_mode() {
                    self.navigate(NavInput::ExitMode);
                }
            }
            ViewEvent::DisplayChanged(display) => self.push_display_preferences(display, cx),
            ViewEvent::ToastsChanged(on) => self.push_toasts_on(on),
            ViewEvent::TestToasts => self.test_toasts(),
            ViewEvent::PickDataFolder => {
                self.with_router(WindowSize::Large, cx, |presenter, router| {
                    presenter.command(ShellCommand::ChangeDataFolder, router)
                });
            }
        }
    }

    /// A cross-View jump: makes the record's destination active, then asks its View to show it.
    pub(super) fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        self.navigate(NavInput::Show(reveal.noun()));
        self.reset_view_scroll(cx);
        if let Some(page) = self.active_page(cx) {
            page.reveal(reveal, cx);
        }
    }

    /// Whether the current mode is one the active View entered itself, so its keys are the View's.
    pub(super) fn view_owns_mode(&self) -> bool {
        self.nav().view_owns_mode()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, EventEmitter, TestAppContext};
    use lib_toast::ToastKind;

    use super::*;
    use crate::navigation::nav::{NavState, Noun};

    /// A stand-in for a View Entity: it emits whatever the test hands it and holds no `Root`.
    struct StandInView;

    impl EventEmitter<ViewEvent> for StandInView {}

    #[gpui::test]
    fn each_view_event_reaches_shell(cx: &mut TestAppContext) {
        crate::locale::init_for_tests();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 9).expect("a valid date");
        let shell = cx.new(|cx| {
            crate::input::install(cx.weak_entity(), cx);
            let mut shell = Root::with_clock(
                NavState::new(),
                cx.focus_handle(),
                lib_core::Clock::fixed(today),
                std::rc::Rc::new(lib_settings::fake::InMemoryDeviceConfiguration::default()),
                &lib_config::KeyBindingConfig::default(),
                cx,
            );
            let (stores, _) = lib_database::Stores::fake(lib_database::FakeStores::sample(today));
            shell.open_ledger(
                crate::shell::router::LedgerFile::in_memory(stores.share()),
                cx,
            );
            shell
        });
        let view = cx.new(|_| StandInView);
        // A Toast or status arrives as one input, which drains into the Shell through the door.
        let _subscriptions = shell.update(cx, |_, cx| Root::page_signals(cx).follow(&view, cx));

        view.update(cx, |_, cx| {
            cx.emit(ViewEvent::RaiseToast {
                kind: ToastKind::Success,
                text: "Saved".to_string(),
            });
        });
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| {
            let toasts = shell.presenter.toasts().visible();
            assert_eq!(toasts.len(), 1);
            assert_eq!(toasts[0].text(), "Saved");
        });

        view.update(cx, |_, cx| cx.emit(ViewEvent::Navigate(Noun::Bills)));
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| assert_eq!(shell.nav().noun(), Noun::Bills));

        view.update(cx, |_, cx| {
            cx.emit(ViewEvent::SetStatus(Some("Exported".to_string())))
        });
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| {
            assert_eq!(shell.status_message().as_deref(), Some("Exported"));
        });

        view.update(cx, |_, cx| cx.emit(ViewEvent::SetStatus(None)));
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| assert_eq!(shell.status_message(), None));
    }

    #[gpui::test]
    fn a_reveal_makes_its_destination_active_and_shows_the_record(cx: &mut TestAppContext) {
        crate::locale::init_for_tests();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 9).expect("a valid date");
        let shell = cx.new(|cx| {
            crate::input::install(cx.weak_entity(), cx);
            let mut shell = Root::with_clock(
                NavState::new(),
                cx.focus_handle(),
                lib_core::Clock::fixed(today),
                std::rc::Rc::new(lib_settings::fake::InMemoryDeviceConfiguration::default()),
                &lib_config::KeyBindingConfig::default(),
                cx,
            );
            let (stores, _) = lib_database::Stores::fake(lib_database::FakeStores::sample(today));
            shell.open_ledger(
                crate::shell::router::LedgerFile::in_memory(stores.share()),
                cx,
            );
            shell
        });
        let view = cx.new(|_| StandInView);
        // A Toast or status arrives as one input, which drains into the Shell through the door.
        let _subscriptions = shell.update(cx, |_, cx| Root::page_signals(cx).follow(&view, cx));
        // The sample seed's plans, which `FakeStores::sample` holds.
        let (id, last_plan) =
            lib_bills::planner_order(&lib_database::seed::sample_bills(today).plans)
                .last()
                .map(|plan| (plan.id, plan.name.clone()))
                .expect("the seed has Bill Plans");

        view.update(cx, |_, cx| cx.emit(ViewEvent::Reveal(Reveal::BillPlan(id))));
        cx.run_until_parked();
        shell.read_with(cx, |shell, cx| {
            assert_eq!(shell.nav().noun(), Noun::Bills);
            let page = shell.bills_snapshot(cx);
            assert_eq!(page.tab, "Planner");
            assert_eq!(page.plans[page.selected], last_plan);
        });
    }
}
