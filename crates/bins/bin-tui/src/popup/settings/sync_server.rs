//! Settings › Sync Server's Edit overlay (TUI-035): the server URL and its access token, a test,
//! and Save, which stays dim until a test passes, so a bad address can't silently break sync.
//!
//! A wireframe: the Client can't call the Sync Server yet, so the test runs on a stub,
//! [`check`], and Save only flashes "not yet built". The token is a [`SecretString`], drawn
//! masked and never logged.

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use secrecy::{ExposeSecret as _, SecretString};

use super::overlay::{Field, Key, Outcome, Overlay, Tested, Value};
use crate::colours::Colours;
use crate::msg;
use crate::units::view::dialog::edited;

/// wireframe: the one server the stub answers for, until the Client can reach a real one.
const ANSWERING_HOST: &str = "sync.ledger.localhost";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Url,
    Token,
}

pub struct SyncServerPopup {
    url: String,
    token: SecretString,
    focus: Focus,
    tested: Tested,
}

impl SyncServerPopup {
    /// The overlay on the server's `url`, with the token to type again.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            token: SecretString::from(String::new()),
            focus: Focus::Url,
            tested: Tested::Untested,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        match Key::of(key) {
            Key::Cancel => return Outcome::Cancel,
            Key::Save if self.tested.passed() => return Outcome::Save,
            Key::Test => self.tested = check(&self.url, &self.token),
            Key::Next | Key::Prev => {
                self.focus = match self.focus {
                    Focus::Url => Focus::Token,
                    Focus::Token => Focus::Url,
                }
            }
            Key::Edit(key) => self.edit(key),
            Key::Save | Key::Step(_) => {}
        }
        Outcome::Stay
    }

    /// Types `key` into the focused field; a change needs a new test before Save.
    fn edit(&mut self, key: KeyEvent) {
        let changed = match self.focus {
            Focus::Url => edited(&self.url, key).map(|url| self.url = url),
            Focus::Token => {
                edited(self.token.expose_secret(), key).map(|token| self.token = token.into())
            }
        };
        if changed.is_some() {
            self.tested = Tested::Untested;
        }
    }

    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        Overlay {
            title: msg::tui_settings_sync_edit_title(),
            tag: Some(":sync edit"),
            fields: vec![
                Field {
                    label: msg::tui_settings_sync_url(),
                    value: Value::Text(self.url.clone()),
                    note: Some(msg::tui_settings_sync_url_note()),
                },
                Field {
                    label: msg::tui_settings_sync_token(),
                    value: Value::Secret(self.token.expose_secret().chars().count()),
                    note: Some(msg::tui_settings_sync_token_note()),
                },
            ],
            focus: match self.focus {
                Focus::Url => 0,
                Focus::Token => 1,
            },
            tested: &self.tested,
            save_note: msg::tui_settings_sync_save_note(),
            save_label: msg::tui_settings_overlay_hint_save(),
        }
        .render(frame, area, c);
    }
}

/// The host and port of `url`, without its scheme or path.
fn host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split('/').next().unwrap_or_default()
}

/// wireframe: the stub test. It checks what the Client can check alone, `https://` and a token,
/// then answers as [`ANSWERING_HOST`] would and refuses everywhere else.
fn check(url: &str, token: &SecretString) -> Tested {
    let url = url.trim();
    let failed = |title, detail| Tested::Failed { title, detail };
    if !url.starts_with("https://") || host(url).is_empty() {
        return failed(
            msg::tui_settings_sync_failed_https(),
            msg::tui_settings_sync_failed_https_detail(),
        );
    }
    if token.expose_secret().trim().is_empty() {
        return failed(
            msg::tui_settings_sync_failed_token(),
            msg::tui_settings_sync_failed_token_detail(),
        );
    }
    let host = host(url);
    if host != ANSWERING_HOST {
        return failed(
            msg::tui_settings_sync_failed_refused(),
            msg::tui_settings_sync_failed_refused_detail(host),
        );
    }
    Tested::Passed {
        title: msg::tui_settings_sync_passed(host),
        detail: msg::tui_settings_sync_passed_detail(),
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;

    fn press(popup: &mut SyncServerPopup, code: KeyCode, modifiers: KeyModifiers) -> Outcome {
        popup.handle_key(KeyEvent::new(code, modifiers))
    }

    fn type_text(popup: &mut SyncServerPopup, text: &str) {
        for ch in text.chars() {
            press(popup, KeyCode::Char(ch), KeyModifiers::NONE);
        }
    }

    fn test(popup: &mut SyncServerPopup) {
        press(popup, KeyCode::Char('t'), KeyModifiers::CONTROL);
    }

    #[test]
    fn host_drops_the_scheme_and_path() {
        assert_eq!(
            host("https://sync.ledger.home:8443/api"),
            "sync.ledger.home:8443"
        );
        assert_eq!(host("sync.ledger.home"), "sync.ledger.home");
    }

    #[test]
    fn save_stays_locked_until_a_test_passes_and_any_change_locks_it_again() {
        crate::locale::init_for_tests();
        let mut popup = SyncServerPopup::new("https://sync.ledger.localhost");
        assert_eq!(
            press(&mut popup, KeyCode::Enter, KeyModifiers::NONE),
            Outcome::Stay
        );
        press(&mut popup, KeyCode::Tab, KeyModifiers::NONE);
        type_text(&mut popup, "secret");
        test(&mut popup);
        assert!(popup.tested.passed(), "{:?}", popup.tested);
        type_text(&mut popup, "x");
        assert_eq!(popup.tested, Tested::Untested);
        test(&mut popup);
        assert_eq!(
            press(&mut popup, KeyCode::Enter, KeyModifiers::NONE),
            Outcome::Save
        );
    }

    #[test]
    fn a_failed_test_names_its_cause() {
        crate::locale::init_for_tests();
        let mut popup = SyncServerPopup::new("http://sync.ledger.home");
        test(&mut popup);
        assert!(matches!(&popup.tested, Tested::Failed { title, .. } if title.contains("https")));

        let mut popup = SyncServerPopup::new("https://sync.ledger.home:8443");
        test(&mut popup);
        assert!(
            matches!(&popup.tested, Tested::Failed { title, .. } if title.contains("access token"))
        );

        press(&mut popup, KeyCode::Tab, KeyModifiers::NONE);
        type_text(&mut popup, "secret");
        test(&mut popup);
        let Tested::Failed { title, detail } = &popup.tested else {
            panic!("the test passed: {:?}", popup.tested);
        };
        assert_eq!(title, "couldn't connect · connection refused");
        assert!(
            detail.starts_with("sync.ledger.home:8443 didn't answer"),
            "{detail}"
        );
    }
}
