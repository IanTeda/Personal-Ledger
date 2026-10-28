//! Settings › General (TUI-019), a wireframe until Stage 2 of the README's work plan: ledger
//! name, owner and financial-year start have no Preference columns yet, and the base unit isn't
//! read from the default-Unit Preference here yet, so every value is the card's worked example.
//!
//! `enter` on the base unit opens the base-unit guard (TUI-021); on any other field it opens the
//! in-place editor, which still draws its own worked example (`popup::settings::edit`).

use crossterm::event::{KeyCode, KeyEvent};
use lib_list_cursor::Step;
use ratatui::{Frame, layout::Rect};

use super::{FieldRow, render_fields};
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;

/// The base unit's row, the last of the four.
const BASE_UNIT_ROW: usize = 3;

/// General's state: the selected field.
#[derive(Default)]
pub(super) struct GeneralPage {
    selected: usize,
}

impl GeneralPage {
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        let step = match key.code {
            KeyCode::Char('j') | KeyCode::Down => Step::Next,
            KeyCode::Char('k') | KeyCode::Up => Step::Prev,
            KeyCode::Home => Step::First,
            KeyCode::Char('G') | KeyCode::End => Step::Last,
            KeyCode::Enter if self.selected == BASE_UNIT_ROW => {
                return Some(Action::OpenBaseUnitGuardPopup);
            }
            KeyCode::Enter => return Some(Action::OpenEditSettingPopup),
            _ => return None,
        };
        self.selected = lib_list_cursor::step(self.selected, fields().len(), step);
        Some(Action::NoOp)
    }

    /// `focused`, the selected field is reversed.
    pub(super) fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        render_fields(frame, area, &fields(), focused.then_some(self.selected), c);
    }
}

/// The four fields. wireframe: TUI-019's worked values, until Stage 2 reads the Preferences.
fn fields() -> [FieldRow; 4] {
    let row = |label: String, value: &str| FieldRow {
        glyph: None,
        label,
        value: value.to_string(),
        note: String::new(),
    };
    [
        row(msg::tui_settings_general_ledger_name(), "Teda household"),
        row(msg::tui_settings_general_owner(), "Ian Teda"),
        row(msg::tui_settings_general_financial_year(), "july"),
        row(
            msg::tui_settings_general_base_unit(),
            "aud — Australian Dollar",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;
    use crate::view::View;
    use crate::view::settings::{SettingsPage, SettingsView, tests::render_at};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn enter_opens_the_guard_on_the_base_unit_and_the_editor_elsewhere() {
        let mut page = GeneralPage::default();
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::OpenEditSettingPopup)
        );
        page.handle_key(key(KeyCode::Char('G')));
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::OpenBaseUnitGuardPopup)
        );
        page.handle_key(key(KeyCode::Char('j')));
        assert_eq!(page.selected, BASE_UNIT_ROW);
    }

    #[test]
    fn it_draws_its_four_fields() {
        crate::locale::init_for_tests();
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::General));
        let text = render_at(&view, 96, 28);
        for label in ["ledger name", "owner", "financial year starts", "base unit"] {
            assert!(text.contains(label), "{label} missing:\n{text}");
        }
    }
}
