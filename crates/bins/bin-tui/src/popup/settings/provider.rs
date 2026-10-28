//! Settings › AI Connections' Add / Edit model provider overlay (TUI-034), drawn like Sync
//! Server's Edit (TUI-035): the provider, its model and its API key, a test, and Save, which stays
//! dim until a test passes.
//!
//! A wireframe: the Client can't call a provider yet, so the test runs on a stub, [`check`], and
//! Save only flashes "not yet built". The API key is a [`SecretString`], drawn masked, never
//! logged, and never shown again: editing a provider asks for the key anew.

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use secrecy::{ExposeSecret as _, SecretString};

use super::overlay::{Field, Key, Outcome, Overlay, Tested, Value, stepped};
use crate::colours::Colours;
use crate::msg;
use crate::units::view::dialog::edited;

/// wireframe: the providers the overlay offers, until the Client knows them.
pub const PROVIDERS: [&str; 3] = ["Claude", "OpenAI", "Ollama (local)"];

/// The provider that runs on this device and needs no key.
const LOCAL: usize = 2;

/// wireframe: keys the stub accepts start with this.
const KEY_PREFIX: &str = "sk-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Provider,
    Model,
    Key,
}

impl Focus {
    const ALL: [Self; 3] = [Self::Provider, Self::Model, Self::Key];

    fn step(self, delta: isize) -> Self {
        let index = Self::ALL
            .iter()
            .position(|focus| *focus == self)
            .unwrap_or(0);
        let len = Self::ALL.len();
        Self::ALL[(index + len).wrapping_add_signed(delta) % len]
    }
}

pub struct ProviderPopup {
    adding: bool,
    /// The selected one of [`PROVIDERS`].
    provider: usize,
    model: String,
    key: SecretString,
    focus: Focus,
    tested: Tested,
}

impl ProviderPopup {
    /// The overlay on `provider` and `model`, adding a provider or editing one, with the key to
    /// type again.
    pub fn new(provider: &str, model: &str, adding: bool) -> Self {
        Self {
            adding,
            provider: PROVIDERS.iter().position(|p| *p == provider).unwrap_or(0),
            model: model.to_string(),
            key: SecretString::from(String::new()),
            focus: Focus::Provider,
            tested: Tested::Untested,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        let changed = match Key::of(key) {
            Key::Cancel => return Outcome::Cancel,
            Key::Save if self.tested.passed() => return Outcome::Save,
            Key::Test => {
                self.tested = self.check();
                false
            }
            Key::Next => {
                self.focus = self.focus.step(1);
                false
            }
            Key::Prev => {
                self.focus = self.focus.step(-1);
                false
            }
            Key::Step(delta) if self.focus == Focus::Provider => {
                let was = self.provider;
                self.provider = stepped(self.provider, PROVIDERS.len(), delta);
                self.provider != was
            }
            Key::Edit(key) => self.edit(key),
            Key::Step(_) | Key::Save => false,
        };
        if changed {
            self.tested = Tested::Untested;
        }
        Outcome::Stay
    }

    /// Types `key` into the focused text field; whether it changed.
    fn edit(&mut self, key: KeyEvent) -> bool {
        match self.focus {
            Focus::Provider => false,
            Focus::Model => edited(&self.model, key)
                .map(|model| self.model = model)
                .is_some(),
            Focus::Key => edited(self.key.expose_secret(), key)
                .map(|api_key| self.key = api_key.into())
                .is_some(),
        }
    }

    /// wireframe: the stub test. It checks what the Client can check alone, a model and, away
    /// from a local provider, a key, then accepts keys that start with [`KEY_PREFIX`].
    fn check(&self) -> Tested {
        let provider = PROVIDERS[self.provider];
        let model = self.model.trim();
        let key = self.key.expose_secret().trim();
        let failed = |title, detail| Tested::Failed { title, detail };
        if model.is_empty() {
            return failed(
                msg::tui_settings_provider_failed_model(),
                msg::tui_settings_provider_failed_model_detail(),
            );
        }
        if self.provider != LOCAL {
            if key.is_empty() {
                return failed(
                    msg::tui_settings_provider_failed_key(),
                    msg::tui_settings_provider_failed_key_detail(),
                );
            }
            if !key.starts_with(KEY_PREFIX) {
                return failed(
                    msg::tui_settings_provider_failed_rejected(),
                    msg::tui_settings_provider_failed_rejected_detail(provider),
                );
            }
        }
        Tested::Passed {
            title: msg::tui_settings_provider_passed(provider),
            detail: msg::tui_settings_provider_passed_detail(model),
        }
    }

    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        Overlay {
            title: if self.adding {
                msg::tui_settings_provider_add_title()
            } else {
                msg::tui_settings_provider_edit_title()
            },
            tag: None,
            fields: vec![
                Field {
                    label: msg::tui_settings_provider_provider(),
                    value: Value::Choice {
                        options: PROVIDERS.iter().map(ToString::to_string).collect(),
                        selected: self.provider,
                    },
                    note: None,
                },
                Field {
                    label: msg::tui_settings_provider_model(),
                    value: Value::Text(self.model.clone()),
                    note: None,
                },
                Field {
                    label: msg::tui_settings_provider_key(),
                    value: Value::Secret(self.key.expose_secret().chars().count()),
                    note: Some(msg::tui_settings_provider_key_note()),
                },
            ],
            focus: Focus::ALL
                .iter()
                .position(|focus| *focus == self.focus)
                .unwrap_or(0),
            tested: &self.tested,
            save_note: msg::tui_settings_provider_save_note(),
            save_label: msg::tui_settings_overlay_hint_save(),
        }
        .render(frame, area, c);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;

    fn press(popup: &mut ProviderPopup, code: KeyCode, modifiers: KeyModifiers) -> Outcome {
        popup.handle_key(KeyEvent::new(code, modifiers))
    }

    fn type_text(popup: &mut ProviderPopup, text: &str) {
        for ch in text.chars() {
            press(popup, KeyCode::Char(ch), KeyModifiers::NONE);
        }
    }

    fn test(popup: &mut ProviderPopup) {
        press(popup, KeyCode::Char('t'), KeyModifiers::CONTROL);
    }

    #[test]
    fn save_stays_locked_until_a_test_passes_and_any_change_locks_it_again() {
        crate::locale::init_for_tests();
        let mut popup = ProviderPopup::new("Claude", "claude-sonnet-5-5", false);
        press(&mut popup, KeyCode::Tab, KeyModifiers::NONE);
        press(&mut popup, KeyCode::Tab, KeyModifiers::NONE);
        type_text(&mut popup, "sk-secret");
        assert_eq!(
            press(&mut popup, KeyCode::Enter, KeyModifiers::NONE),
            Outcome::Stay
        );
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
        let mut popup = ProviderPopup::new("OpenAI", "", true);
        test(&mut popup);
        assert!(matches!(&popup.tested, Tested::Failed { title, .. } if title.contains("model")));

        let mut popup = ProviderPopup::new("OpenAI", "gpt-5", true);
        test(&mut popup);
        assert!(matches!(&popup.tested, Tested::Failed { title, .. } if title.contains("api key")));

        press(&mut popup, KeyCode::Tab, KeyModifiers::NONE);
        press(&mut popup, KeyCode::Tab, KeyModifiers::NONE);
        type_text(&mut popup, "hunter2");
        test(&mut popup);
        let Tested::Failed { title, detail } = &popup.tested else {
            panic!("the test passed: {:?}", popup.tested);
        };
        assert_eq!(title, "refused · key rejected");
        assert!(
            detail.starts_with("OpenAI didn't accept the key"),
            "{detail}"
        );
    }

    #[test]
    fn a_local_provider_passes_without_a_key() {
        crate::locale::init_for_tests();
        let mut popup = ProviderPopup::new("Ollama (local)", "llama3.1:8b", false);
        test(&mut popup);
        assert!(popup.tested.passed(), "{:?}", popup.tested);
    }
}
