//! Settings › About (TUI-039): the project's facts, the same as the Desktop's About page
//! (`bin-desktop/src/help.rs`). The version is `CARGO_PKG_VERSION`; the release date has no
//! build-time source, so it stays a static value. The links are rows: `j/k` move, `enter` opens
//! one in the browser and `y` copies it, both through `Shell`'s [`crate::link::Outside`]. The
//! copyright is pinned to the bottom.

use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent};
use lib_locale::format::upper;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use super::keys::{self, Binding, Press};
use super::list;
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;

const AUTHOR: &str = "Ian Teda";

/// wireframe: the release date, until a build stamps one.
const RELEASED: Option<NaiveDate> = NaiveDate::from_ymd_opt(2026, 9, 8);

/// A link row: its label's Message, the text shown and the URL `enter` opens and `y` copies.
struct Link {
    label: fn() -> String,
    text: fn() -> String,
    url: &'static str,
}

/// The links, top to bottom, the Desktop's (`bin-desktop/src/help.rs`) and its licence's.
const LINKS: [Link; 4] = [
    Link {
        label: msg::tui_settings_about_repository,
        text: || "github.com/IanTeda/personal-ledger".into(),
        url: "https://github.com/IanTeda/Personal-Ledger",
    },
    Link {
        label: msg::tui_settings_about_documentation,
        text: || "ianteda.github.io/personal-ledger".into(),
        url: "https://ianteda.github.io/personal-ledger",
    },
    Link {
        label: msg::tui_settings_about_issues,
        text: || "github.com/IanTeda/personal-ledger/issues".into(),
        url: "https://github.com/IanTeda/Personal-Ledger/issues",
    },
    Link {
        label: msg::tui_settings_about_license,
        text: msg::tui_settings_about_license_text,
        url: "https://github.com/IanTeda/Personal-Ledger/blob/main/LICENSE",
    },
];

/// The fact labels' column.
const LABEL_WIDTH: u16 = 18;

#[derive(Debug, Clone, Copy)]
enum Command {
    Row,
    Open,
    Copy,
}

/// The page's keys and its hint bar, one table (#898).
const KEYS: &[Binding<Command>] = &[
    Binding {
        keys: keys::ROW_KEYS,
        label: "j/k",
        hint: msg::tui_settings_hint_link,
        command: Command::Row,
    },
    Binding {
        keys: &[Press::key(KeyCode::Enter)],
        label: "enter",
        hint: msg::tui_settings_hint_open_browser,
        command: Command::Open,
    },
    Binding {
        keys: &[Press::char('y')],
        label: "y",
        hint: msg::tui_settings_hint_copy_link,
        command: Command::Copy,
    },
];

#[derive(Default)]
pub(super) struct AboutPage {
    /// The selected link in [`LINKS`].
    selected: usize,
}

impl AboutPage {
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        let url = LINKS[self.selected].url;
        Some(match keys::lookup(KEYS, key)? {
            Command::Row => {
                let step = list::motion(key)?;
                self.selected = lib_list_cursor::step(self.selected, LINKS.len(), step);
                Action::NoOp
            }
            Command::Open => Action::OpenLink(url),
            Command::Copy => Action::CopyText(url.to_string()),
        })
    }

    pub(super) fn hints() -> Vec<(String, String)> {
        keys::hints(KEYS)
    }

    pub(super) fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let mut constraints = vec![Constraint::Length(1); 6 + LINKS.len()];
        constraints.extend([Constraint::Min(0), Constraint::Length(1)]);
        let rows = Layout::vertical(constraints).split(area);
        let bold = Style::default().add_modifier(Modifier::BOLD);

        frame.render_widget(
            Paragraph::new(Span::styled(
                msg::tui_settings_about_description(),
                c.muted(),
            )),
            rows[0],
        );
        let mut version = vec![Span::styled(
            msg::tui_settings_about_version(env!("CARGO_PKG_VERSION")),
            bold,
        )];
        if let Some(date) = RELEASED {
            version.push(Span::styled(
                format!(" ({})", crate::format::date(date)),
                c.muted(),
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(version)), rows[2]);
        frame.render_widget(
            Paragraph::new(Span::styled(
                msg::tui_settings_about_built_with(),
                c.muted(),
            )),
            rows[3],
        );
        fact(
            frame,
            rows[5],
            &msg::tui_settings_about_author(),
            Span::raw(AUTHOR),
            c,
        );
        for (index, link) in LINKS.iter().enumerate() {
            let row = rows[6 + index];
            let selected = focused && index == self.selected;
            if selected {
                frame.render_widget(Block::new().style(c.selection()), row);
            }
            let style = if selected {
                c.accent_selection()
            } else {
                c.accent()
            };
            fact(
                frame,
                row,
                &(link.label)(),
                Span::styled((link.text)(), style),
                c,
            );
        }
        frame.render_widget(
            Paragraph::new(Span::styled(
                msg::tui_settings_about_copyright(AUTHOR),
                c.muted(),
            )),
            rows[rows.len() - 1],
        );
    }
}

/// A fact row: the label upper case and dim, then its value.
fn fact(frame: &mut Frame<'_>, area: Rect, label: &str, value: Span<'_>, c: &Colours) {
    let [label_area, value_area] =
        Layout::horizontal([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(Span::styled(upper(label), c.muted())),
        label_area,
    );
    frame.render_widget(Paragraph::new(value), value_area);
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

    fn about_page() -> SettingsView {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::About));
        view
    }

    #[test]
    fn the_page_draws_the_version_facts_and_links_with_the_copyright_at_the_bottom() {
        crate::locale::init_for_tests();
        let text = render_at(&about_page(), 96, 30);
        for needle in [
            "Settings › About",
            "version info",
            "A local-first personal ledger",
            &format!("Personal Ledger v{}", env!("CARGO_PKG_VERSION")),
            "Built with Rust + ratatui + SQLite",
            "AUTHOR",
            "Ian Teda",
            "REPOSITORY",
            "github.com/IanTeda/personal-ledger",
            "DOCUMENTATION",
            "REPORT AN ISSUE",
            "LICENSE",
            "Distributed under the GPL-3.0 License",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
        // The body keeps a blank row under it, so the copyright is the frame's last but one.
        let last = text.lines().rev().nth(1).unwrap_or_default();
        assert!(last.contains("© 2025–2026 Ian Teda"), "{text}");
    }

    #[test]
    fn enter_opens_the_selected_link_and_y_copies_it() {
        let mut view = about_page();
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::OpenLink(
                "https://github.com/IanTeda/Personal-Ledger"
            ))
        );
        view.handle_key(key(KeyCode::Char('j')));
        view.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            view.handle_key(key(KeyCode::Char('y'))),
            Some(Action::CopyText(
                "https://github.com/IanTeda/Personal-Ledger/issues".into()
            ))
        );
        view.handle_key(key(KeyCode::Char('G')));
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::OpenLink(
                "https://github.com/IanTeda/Personal-Ledger/blob/main/LICENSE"
            ))
        );
    }

    #[test]
    fn the_hint_bar_is_the_keys_the_page_takes() {
        keys::tests::assert_the_hints_are_the_keys(KEYS, AboutPage::hints(), |key| {
            AboutPage::default().handle_key(key)
        });
    }
}
