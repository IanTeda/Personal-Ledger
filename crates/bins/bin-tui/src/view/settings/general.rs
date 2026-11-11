//! Settings › General (TUI-058), the first this device page: On start, Start view, Toasts, the
//! read-only Locale and ledger folder, then the THIS DEVICE box. Everything here belongs to this
//! device and works with no Ledger open (ADR-0051).
//!
//! On start and Start view open in place as a choice and Toasts flips on `enter` or `space`; all are
//! Configuration (ADR-0053) `Shell` writes to the user's `personal-ledger.conf` as they change, and
//! a key an environment variable or a higher file sets is noted `set by …` on its row. The Locale and
//! `data` are Configuration read at start-up, so `enter` on either says where to change it. THIS
//! DEVICE shows the Ledger file On start would reopen, which is the Open Ledger's, and the config
//! file Display's rows write.

use crossterm::event::{KeyCode, KeyEvent};
use lib_list_cursor::Step;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};

use super::field::{ON_STARTS, SettingField, on_off, on_start_label};
use super::keys::{self, Binding, Press};
use super::{Context, FIELD_LABEL_WIDTH, FieldRow, Open, list, render_form};
use crate::colours::Colours;
use crate::msg;
use crate::popup::settings::edit::{Draft, Segment};
use crate::start_view;
use crate::view::Action;

/// The fields, in the page's order.
const FIELDS: [SettingField; 5] = [
    SettingField::OnStart,
    SettingField::StartView,
    SettingField::Toasts,
    SettingField::Locale,
    SettingField::LedgerFolder,
];

#[derive(Debug, Clone, Copy)]
enum Command {
    Row,
    Open,
    Toggle,
    Test,
}

/// The page's keys and its hint bar, one table (#898).
const KEYS: &[Binding<Command>] = &[
    Binding {
        keys: keys::ROW_KEYS,
        label: "j/k",
        hint: msg::tui_settings_hint_field,
        command: Command::Row,
    },
    Binding {
        keys: &[Press::key(KeyCode::Enter)],
        label: "enter",
        hint: msg::tui_settings_hint_edit,
        command: Command::Open,
    },
    Binding {
        keys: &[Press::char(' ')],
        label: "space",
        hint: msg::tui_settings_hint_toggle,
        command: Command::Toggle,
    },
    Binding {
        keys: &[Press::char('t')],
        label: "t",
        hint: msg::tui_settings_hint_test,
        command: Command::Test,
    },
];

/// General's state: the selected field, and the Toasts Preference as `Shell` last reported it.
pub(super) struct GeneralPage {
    selected: usize,
    toasts_on: bool,
}

impl GeneralPage {
    pub(super) fn new() -> Self {
        Self {
            selected: 0,
            toasts_on: true,
        }
    }

    pub(super) fn set_toasts_on(&mut self, on: bool) {
        self.toasts_on = on;
    }

    /// `enter` opens the selected field in place; on Toasts it, like `space`, flips the checkbox
    /// at once. `space` on any other row does nothing.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        let field = FIELDS[self.selected];
        let flipped = (field == SettingField::Toasts).then_some(!self.toasts_on);
        Some(match keys::lookup(KEYS, key)? {
            Command::Row => {
                self.step(list::motion(key)?);
                Action::NoOp
            }
            Command::Open | Command::Toggle if let Some(on) = flipped => {
                Action::SetSetting(field, Some(on_off(on).to_string()))
            }
            Command::Open => Action::SetSetting(field, None),
            Command::Test if field == SettingField::Toasts => Action::TestToasts,
            Command::Toggle | Command::Test => Action::NoOp,
        })
    }

    /// The whole table, which the hint-bar test compares against the keys.
    #[cfg(test)]
    pub(super) fn hints() -> Vec<(String, String)> {
        keys::hints(KEYS)
    }

    /// The hint bar for the selected field: `t test` only on Toasts.
    pub(super) fn hints_for_selected(&self) -> Vec<(String, String)> {
        let on_toasts = FIELDS[self.selected] == SettingField::Toasts;
        keys::hints(KEYS)
            .into_iter()
            .filter(|(label, _)| on_toasts || label != "t")
            .collect()
    }

    pub(super) fn step(&mut self, step: Step) {
        self.selected = lib_list_cursor::step(self.selected, FIELDS.len(), step);
    }

    /// Selects `field`'s row, if it is on this page.
    pub(super) fn select(&mut self, field: SettingField) {
        if let Some(index) = FIELDS.iter().position(|row| *row == field) {
            self.selected = index;
        }
    }

    /// What opening `field` in place starts from: its draft and the old value's label. `None` for
    /// a read-only field.
    pub(super) fn draft(
        &self,
        field: SettingField,
        context: &Context<'_>,
    ) -> Option<(Draft, String)> {
        match field {
            SettingField::OnStart => {
                let on_start = context.values.configuration.on_start;
                let segments = ON_STARTS
                    .iter()
                    .map(|(value, on_start)| Segment {
                        value: (*value).to_string(),
                        label: on_start_label(*on_start),
                    })
                    .collect();
                let chosen = ON_STARTS
                    .iter()
                    .position(|(_, choice)| *choice == on_start)
                    .unwrap_or_default();
                Some((Draft::Choice { segments, chosen }, on_start_label(on_start)))
            }
            SettingField::StartView => {
                let key = context.values.configuration.start_view.key().to_string();
                let segments = start_view::CHOICES
                    .iter()
                    .map(|choice| Segment {
                        value: (*choice).to_string(),
                        label: start_view::label(choice),
                    })
                    .collect();
                // A noun from the file the TUI doesn't have opens on the Dashboard's choice.
                let chosen = start_view::CHOICES
                    .iter()
                    .position(|choice| *choice == key)
                    .unwrap_or_default();
                Some((Draft::Choice { segments, chosen }, start_view::label(&key)))
            }
            // `:set toasts` without a value opens the checkbox in place.
            SettingField::Toasts => {
                Some((Draft::Check(self.toasts_on), toasts_label(self.toasts_on)))
            }
            _ => None,
        }
    }

    /// `focused`, the selected field is reversed; an `editor` open on one of its fields draws in
    /// that row's place. THIS DEVICE sits under the fields, the note at the bottom.
    pub(super) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        focused: bool,
        context: &Context<'_>,
        c: &Colours,
    ) {
        let rows: Vec<FieldRow> = FIELDS
            .iter()
            .map(|field| self.row(*field, context))
            .collect();
        let open = context.editor.map(|editor| Open {
            editor,
            sub_rows: vec![(msg::tui_settings_edit_was(), editor.was().to_string())],
        });
        let rest = render_form(
            frame,
            area,
            &rows,
            focused.then_some(self.selected),
            open,
            c,
        );
        let [_, this_device, _, note] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .areas(rest);
        render_this_device(frame, this_device, context, c);
        frame.render_widget(
            Paragraph::new(Span::styled(msg::tui_settings_general_note(), c.muted()))
                .wrap(Wrap { trim: true }),
            note,
        );
    }

    /// One field's row at rest.
    fn row(&self, field: SettingField, context: &Context<'_>) -> FieldRow {
        let shadowed = |field| context.values.configuration.shadowed_note(field);
        let (value, note) = match field {
            SettingField::OnStart => (
                on_start_label(context.values.configuration.on_start),
                shadowed(field).unwrap_or_else(msg::tui_settings_general_on_start_note),
            ),
            SettingField::StartView => (
                start_view::label(context.values.configuration.start_view.key()),
                shadowed(field).unwrap_or_else(msg::tui_settings_general_start_view_note),
            ),
            SettingField::Toasts => (
                format!(
                    "{}  {}",
                    toasts_label(self.toasts_on),
                    msg::tui_settings_general_test_chip()
                ),
                shadowed(field).unwrap_or_else(lib_locale::msg::toast_setting_note),
            ),
            SettingField::Locale => {
                // The Locale startup resolved and where it came from, never a fixed value.
                crate::locale::describe(&crate::locale::info())
            }
            _ => (
                context.values.ledger_folder.display().to_string(),
                msg::tui_settings_general_ledger_folder_note(),
            ),
        };
        FieldRow { field, value, note }
    }
}

fn toasts_label(on: bool) -> String {
    if on {
        lib_locale::msg::toast_setting_on()
    } else {
        lib_locale::msg::toast_setting_off()
    }
}

/// THIS DEVICE: the Ledger file On start would reopen and the config file in use, each a dim
/// label and its path, clipped from the left so the file's name stays in view.
fn render_this_device(frame: &mut Frame<'_>, area: Rect, context: &Context<'_>, c: &Colours) {
    let none = msg::tui_settings_general_none;
    let facts = [
        (
            msg::tui_settings_general_last_ledger(),
            context
                .values
                .last_ledger
                .as_ref()
                .map_or_else(none, |path| path.display().to_string()),
        ),
        (
            msg::tui_settings_general_config_file(),
            context
                .values
                .configuration
                .path
                .as_ref()
                .map_or_else(none, |path| path.display().to_string()),
        ),
    ];
    let block = Block::bordered()
        .border_style(c.muted())
        .title(Span::styled(
            msg::tui_settings_general_this_device(),
            c.muted(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([Constraint::Length(1); 2]).split(inner);
    for ((label, path), row) in facts.into_iter().zip(rows.iter()) {
        let [label_area, path_area] =
            Layout::horizontal([Constraint::Length(FIELD_LABEL_WIDTH), Constraint::Min(0)])
                .areas(*row);
        frame.render_widget(Paragraph::new(Span::styled(label, c.muted())), label_area);
        frame.render_widget(
            Paragraph::new(Line::from(clip_left(&path, usize::from(path_area.width)))),
            path_area,
        );
    }
}

/// `text` cut to `width` cells from the left, starting `…` when it doesn't fit, so a path keeps
/// its file name.
fn clip_left(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let kept: String = text.chars().skip(count - (width - 1)).collect();
    format!("…{kept}")
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
    fn enter_opens_on_start_and_start_view_and_flips_toasts_and_the_read_only_rows_go_to_the_shell()
    {
        let mut page = GeneralPage::new();
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::SetSetting(SettingField::OnStart, None))
        );
        assert_eq!(page.handle_key(key(KeyCode::Char(' '))), Some(Action::NoOp));
        page.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::SetSetting(SettingField::StartView, None))
        );
        page.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::SetSetting(SettingField::Toasts, Some("off".into())))
        );
        page.set_toasts_on(false);
        assert_eq!(
            page.handle_key(key(KeyCode::Char(' '))),
            Some(Action::SetSetting(SettingField::Toasts, Some("on".into())))
        );
        for field in [SettingField::Locale, SettingField::LedgerFolder] {
            page.select(field);
            assert_eq!(
                page.handle_key(key(KeyCode::Enter)),
                Some(Action::SetSetting(field, None)),
                "the Shell refuses the read-only {field:?} with its reason"
            );
        }
    }

    #[test]
    fn it_draws_its_five_fields_and_this_device() {
        crate::locale::init_for_tests();
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::General));
        let text = render_at(&view, 96, 30);
        for needle in [
            "Settings \u{203a} General",
            "this device \u{b7} never synced",
            "on start",
            "reopen last ledger",
            "start view",
            "dashboard",
            "Toasts",
            "[ test ]",
            "locale",
            "en-US",
            "ledger folder",
            "last ledger",
            "config file",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
    }

    #[test]
    fn a_long_path_keeps_its_file_name() {
        assert_eq!(clip_left("/a/b/household.pldb", 30), "/a/b/household.pldb");
        assert_eq!(clip_left("/a/b/household.pldb", 10), "…hold.pldb");
    }

    #[test]
    fn the_hint_bar_is_the_keys_the_page_takes() {
        keys::tests::assert_the_hints_are_the_keys(KEYS, GeneralPage::hints(), |key| {
            GeneralPage::new().handle_key(key)
        });
    }
}
