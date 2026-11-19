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
use crate::shell::view::module_page::PageCue;
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
            ViewEvent::DataFolderChanged(folder) => {
                self.presenter
                    .set_explorer_start(Some(folder), dirs::home_dir());
            }
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
            page.cue(PageCue::Reveal(reveal), cx);
        }
    }

    /// Whether the current mode is one the active View entered itself, so its keys are the View's.
    pub(super) fn view_owns_mode(&self) -> bool {
        self.nav().view_owns_mode()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, Entity, EventEmitter, TestAppContext};
    use lib_toast::ToastKind;

    use super::*;
    use crate::navigation::nav::{NavState, Noun};
    use crate::shell::view::page_handle::{PageBinding, test_support::hosted_pages};
    use lib_viper::keymap::Palette;

    /// The palette-listed Command names of `keys`, sorted, so two lists compare by content.
    fn listed_names(keys: &[PageBinding]) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = keys
            .iter()
            .filter_map(|binding| match binding.palette {
                Palette::Listed { name, .. } => Some(name),
                _ => None,
            })
            .collect();
        names.sort_unstable();
        names
    }

    #[gpui::test]
    fn every_hosted_page_with_listed_commands_is_in_the_palette_fixture(cx: &mut TestAppContext) {
        crate::locale::init_for_tests();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 9).expect("a valid date");
        let shell = sample_root(today, cx);
        let fixture = hosted_pages(&lib_config::KeyBindingConfig::default());
        let hosted = shell.read_with(cx, |shell, cx| shell.hosted_page_keys(cx));
        for (view, keys) in hosted {
            let names = listed_names(&keys);
            if names.is_empty() {
                continue;
            }
            let Some((_, fixture_keys)) = fixture.iter().find(|(fixture, _)| *fixture == view)
            else {
                panic!(
                    "{view:?} lists palette Commands {names:?}, but test_support::hosted_pages \
                     does not host it, so the palette tests would not see them"
                );
            };
            assert_eq!(
                listed_names(fixture_keys),
                names,
                "test_support::hosted_pages lists different palette Commands for {view:?}"
            );
        }
    }

    /// A `Root` on the in-memory fakes with the Sample Ledger open, so both tests start from one setup.
    fn sample_root(today: chrono::NaiveDate, cx: &mut TestAppContext) -> Entity<Root> {
        cx.new(|cx| {
            crate::input::install(cx.weak_entity(), cx);
            let mut shell = Root::with_clock(
                NavState::new(),
                cx.focus_handle(),
                lib_core::Clock::fixed(today),
                std::rc::Rc::new(lib_settings::fake::InMemoryDeviceConfiguration::default()),
                &lib_config::KeyBindingConfig::default(),
                std::rc::Rc::new(std::cell::RefCell::new(
                    lib_ledger_creation::fake::InMemoryLedgerCreation::default(),
                )),
                std::rc::Rc::new(lib_shell::fake::FakeLedgerFolders::default()),
                cx,
            );
            let (stores, _) = lib_database::Stores::fake(lib_database::FakeStores::sample(today));
            shell.open_ledger(
                crate::shell::router::LedgerFile::in_memory(stores.share()),
                cx,
            );
            shell
        })
    }

    /// A stand-in for a View Entity: it emits whatever the test hands it and holds no `Root`.
    struct StandInView;

    impl EventEmitter<ViewEvent> for StandInView {}

    #[gpui::test]
    fn each_view_event_reaches_shell(cx: &mut TestAppContext) {
        crate::locale::init_for_tests();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 9).expect("a valid date");
        let shell = sample_root(today, cx);
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
        let shell = sample_root(today, cx);
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
