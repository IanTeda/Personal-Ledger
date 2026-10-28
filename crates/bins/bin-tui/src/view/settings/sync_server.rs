//! Settings › Sync Server (TUI-035): the server's address, status and last sync, `s` sync now and
//! `e` the Edit overlay (`popup::settings::sync_server`), which the Settings View draws over the
//! page and which takes every key while open.
//!
//! A wireframe: the Client can't call the Sync Server yet, so the facts are fixed values, `s` and
//! the overlay's Save flash the Shell's "not yet built", and the overlay tests on a stub.

use chrono::NaiveDate;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
};

use super::keys::{self, Binding, Press};
use super::{moment, render_fact, render_keys};
use crate::colours::Colours;
use crate::msg;
use crate::popup::settings::overlay::Outcome;
use crate::popup::settings::sync_server::SyncServerPopup;
use crate::view::Action;

/// wireframe: the server this device syncs with.
const SERVER_URL: &str = "https://sync.ledger.localhost";

/// wireframe: when it last synced.
const LAST_SYNC: Option<NaiveDate> = NaiveDate::from_ymd_opt(2026, 9, 14);

/// What Sync Server's keys do, which `:sync now|edit` names (#898).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncCommand {
    Now,
    Edit,
}

/// The page's keys and its hint bar, one table (#898).
const KEYS: &[Binding<SyncCommand>] = &[
    Binding {
        keys: &[Press::char('s')],
        label: "s",
        hint: msg::tui_settings_hint_sync_now,
        command: SyncCommand::Now,
    },
    Binding {
        keys: &[Press::char('e')],
        label: "e",
        hint: msg::tui_settings_hint_edit,
        command: SyncCommand::Edit,
    },
];

#[derive(Default)]
pub(super) struct SyncServerPage {
    /// `Some` while the Edit overlay is open.
    overlay: Option<SyncServerPopup>,
}

impl SyncServerPage {
    pub(super) fn overlay_open(&self) -> bool {
        self.overlay.is_some()
    }

    /// Closes the overlay, discarding what was typed; `false` when it wasn't open.
    pub(super) fn close_overlay(&mut self) -> bool {
        self.overlay.take().is_some()
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(overlay) = &mut self.overlay {
            return Some(match overlay.handle_key(key) {
                Outcome::Stay => Action::NoOp,
                Outcome::Cancel => {
                    self.overlay = None;
                    Action::NoOp
                }
                Outcome::Save => {
                    self.overlay = None;
                    Action::FlashNotYetBuilt("sync-server save")
                }
            });
        }
        Some(self.run(keys::lookup(KEYS, key)?))
    }

    /// What a key, or `:sync now|edit`, does: the one path both take.
    pub(super) fn run(&mut self, command: SyncCommand) -> Action {
        match command {
            SyncCommand::Now => Action::FlashNotYetBuilt("sync"),
            SyncCommand::Edit => {
                self.overlay = Some(SyncServerPopup::new(SERVER_URL));
                Action::NoOp
            }
        }
    }

    pub(super) fn hints() -> Vec<(String, String)> {
        keys::hints(KEYS)
    }

    pub(super) fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let [url, status, last, _, keys] = Layout::vertical([Constraint::Length(1); 5]).areas(area);
        let host = SERVER_URL
            .split_once("://")
            .map_or(SERVER_URL, |(_, host)| host);
        render_fact(
            frame,
            url,
            &msg::tui_settings_sync_url(),
            Line::from(host),
            c,
        );
        render_fact(
            frame,
            status,
            &msg::tui_settings_sync_status(),
            Line::from(vec![
                Span::styled("● ", c.accent()),
                Span::raw(msg::tui_settings_sync_connected()),
            ]),
            c,
        );
        render_fact(
            frame,
            last,
            &msg::tui_settings_sync_last_sync(),
            Line::from(moment(LAST_SYNC, 14, 22)),
            c,
        );
        render_keys(
            frame,
            keys,
            &[
                ("s", msg::tui_settings_hint_sync_now()),
                ("e", msg::tui_settings_hint_edit()),
            ],
        );
    }

    pub(super) fn render_overlay(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        if let Some(overlay) = &self.overlay {
            overlay.render(frame, area, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;
    use crate::view::View;
    use crate::view::settings::tests::render_at;
    use crate::view::settings::{SettingsPage, SettingsView};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn sync_page() -> SettingsView {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::SyncServer));
        view
    }

    fn type_text(view: &mut SettingsView, text: &str) {
        for ch in text.chars() {
            view.handle_key(key(KeyCode::Char(ch)));
        }
    }

    /// The Edit overlay on `url` with `token` typed and tested: TUI-035's state.
    fn failed_test(token: &str) -> SettingsView {
        let mut view = sync_page();
        view.handle_key(key(KeyCode::Char('e')));
        for _ in SERVER_URL.chars() {
            view.handle_key(key(KeyCode::Backspace));
        }
        type_text(&mut view, "https://sync.ledger.home:8443");
        view.handle_key(key(KeyCode::Tab));
        type_text(&mut view, token);
        view.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
        view
    }

    #[test]
    fn the_page_draws_the_server_its_status_and_last_sync() {
        crate::locale::init_for_tests();
        let text = render_at(&sync_page(), 96, 30);
        for needle in [
            "Settings › Sync server",
            "this device · synced · every 30 seconds",
            "server url",
            "sync.ledger.localhost",
            "● connected",
            "last sync",
            "sep 14, 2026 · 2:22",
            "s sync now",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
    }

    #[test]
    fn at_96_by_30_the_overlay_draws_the_failed_test_its_cause_and_a_masked_token() {
        crate::locale::init_for_tests();
        let view = failed_test("hunter2secret");
        assert!(view.editing(), "the overlay shows INSERT");
        let text = render_at(&view, 96, 30);
        for needle in [
            "edit sync server",
            ":sync edit",
            "https://sync.ledger.home:8443",
            "•••••••••••••",
            "■ couldn't connect · connection refused",
            "sync.ledger.home:8443 didn't answer.",
            "Save unlocks after a passing test.",
            "test failed",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
        assert!(!text.contains("hunter2secret"), "the token shows:\n{text}");
    }

    #[test]
    fn enter_does_nothing_until_a_test_passes_then_saves_with_not_yet_built() {
        crate::locale::init_for_tests();
        let mut view = failed_test("secret");
        assert_eq!(view.handle_key(key(KeyCode::Enter)), Some(Action::NoOp));
        assert!(view.popup_open(), "a locked Save keeps the overlay open");

        view.handle_key(key(KeyCode::BackTab));
        for _ in "https://sync.ledger.home:8443".chars() {
            view.handle_key(key(KeyCode::Backspace));
        }
        type_text(&mut view, SERVER_URL);
        view.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::FlashNotYetBuilt("sync-server save"))
        );
        assert!(!view.popup_open());
    }

    #[test]
    fn esc_closes_the_overlay_before_it_leaves_the_page() {
        crate::locale::init_for_tests();
        let mut view = failed_test("secret");
        view.handle_key(key(KeyCode::Esc));
        assert!(!view.popup_open());
        assert!(render_at(&view, 96, 30).contains("Settings › Sync server"));

        view.handle_key(key(KeyCode::Char('e')));
        assert!(view.back(), "the back key closes the overlay");
        assert!(!view.popup_open());
        assert!(view.back(), "then leaves the page for the index");
        assert!(!view.back(), "and the index leaves Settings");
    }

    #[test]
    fn s_flashes_not_yet_built() {
        let mut view = sync_page();
        assert_eq!(
            view.handle_key(key(KeyCode::Char('s'))),
            Some(Action::FlashNotYetBuilt("sync"))
        );
    }

    #[test]
    fn the_hint_bar_is_the_keys_the_page_takes() {
        keys::tests::assert_the_hints_are_the_keys(KEYS, SyncServerPage::hints(), |key| {
            SyncServerPage::default().handle_key(key)
        });
    }
}
