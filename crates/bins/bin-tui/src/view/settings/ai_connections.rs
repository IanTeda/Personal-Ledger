//! Settings › AI Connections (TUI-034): two blocks, each a list with its own add key. Model
//! providers (PROVIDER · MODEL · KEY · STATUS) over a note that keys stay in the OS keychain, then
//! Assistant access, the MCP server's address, access and approval over the assistants connected
//! to it. Everything here stays on this Client and never syncs. Status is a glyph plus a word,
//! never colour alone, and Assistants are read only: Read & write is drawn dim as "not yet
//! offered" (ADR-0034). `tab` moves between the blocks.
//!
//! A wireframe: there is no `lib-mcp` server and no provider client yet, so both lists are
//! [`PROVIDERS`] and [`ASSISTANTS`], and every key but the movement flashes the Shell's "not yet
//! built". `n` and `e` on a provider open the Add / Edit model provider overlay
//! (`popup::settings::provider`), which the Settings View draws over the page and which takes
//! every key while open; its API key is masked.

use chrono::NaiveDate;
use chrono::NaiveTime;
use crossterm::event::{KeyCode, KeyEvent};
use lib_locale::format::format_time_minutes;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::keys::{self, Binding, Press};
use super::list::{self, Cell, Column, Row, Table, Tone};
use crate::colours::Colours;
use crate::msg;
use crate::popup::settings::overlay::Outcome;
use crate::popup::settings::provider::ProviderPopup;
use crate::view::Action;

/// Widths of the PROVIDER, MODEL and KEY columns; STATUS takes the rest.
const PROVIDER_WIDTH: u16 = 24;
const MODEL_WIDTH: u16 = 20;
const KEY_WIDTH: u16 = 18;
/// Widths of the ASSISTANT, ACCESS and LAST SEEN columns.
const ASSISTANT_WIDTH: u16 = 24;
const ACCESS_WIDTH: u16 = 12;
const SEEN_WIDTH: u16 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Connected,
    KeyRejected,
    Idle,
}

impl Status {
    /// A glyph and a word, so the state never rests on colour alone.
    fn cell(self) -> Cell {
        match self {
            Self::Connected => Cell::tone(msg::tui_settings_ai_connected(), Tone::Accent),
            Self::KeyRejected => Cell::tone(msg::tui_settings_ai_key_rejected(), Tone::Negative),
            Self::Idle => Cell::muted(msg::tui_settings_ai_idle()),
        }
    }
}

// wireframe: TUI-034's three providers. The key is where it lives, never the key.
struct FixtureProvider {
    name: &'static str,
    model: &'static str,
    key: &'static str,
    status: Status,
}

const PROVIDERS: [FixtureProvider; 3] = [
    FixtureProvider {
        name: "Claude",
        model: "claude-sonnet-5-5",
        key: "keychain",
        status: Status::Connected,
    },
    FixtureProvider {
        name: "Ollama (local)",
        model: "llama3.1:8b",
        key: "localhost:11434",
        status: Status::Connected,
    },
    FixtureProvider {
        name: "OpenAI",
        model: "gpt-5",
        key: "keychain",
        status: Status::KeyRejected,
    },
];

/// wireframe: the provider answers go to unless a conversation picks another.
const DEFAULT_PROVIDER: usize = 0;

/// wireframe: when an assistant was last seen.
#[derive(Clone, Copy)]
enum Seen {
    Today(NaiveTime),
    On(NaiveDate),
}

// wireframe: TUI-034's two connected assistants.
struct FixtureAssistant {
    name: &'static str,
    seen: Seen,
    status: Status,
}

const ASSISTANTS: [FixtureAssistant; 2] = [
    FixtureAssistant {
        name: "Claude Desktop",
        seen: match NaiveTime::from_hms_opt(14, 2, 0) {
            Some(time) => Seen::Today(time),
            None => unreachable!(),
        },
        status: Status::Connected,
    },
    FixtureAssistant {
        name: "Claude Code",
        seen: match NaiveDate::from_ymd_opt(2026, 9, 12) {
            Some(date) => Seen::On(date),
            None => unreachable!(),
        },
        status: Status::Idle,
    },
];

/// wireframe: where the MCP server listens, on this device only.
const SERVER_ADDRESS: &str = "127.0.0.1:7421";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    Providers,
    Assistants,
}

#[derive(Debug, Clone, Copy)]
enum Command {
    Row,
    Block,
    Edit,
    Test,
    New,
    Default,
    Remove,
}

/// The page's keys and its hint bar, one table (#898).
const KEYS: &[Binding<Command>] = &[
    Binding {
        keys: keys::ROW_KEYS,
        label: "j/k",
        hint: msg::tui_settings_hint_row,
        command: Command::Row,
    },
    Binding {
        keys: &[Press::key(KeyCode::Tab)],
        label: "tab",
        hint: msg::tui_settings_hint_ai_block,
        command: Command::Block,
    },
    Binding {
        keys: &[Press::char('e')],
        label: "e",
        hint: msg::tui_settings_hint_edit,
        command: Command::Edit,
    },
    Binding {
        keys: &[Press::char('t')],
        label: "t",
        hint: msg::tui_settings_hint_test,
        command: Command::Test,
    },
    Binding {
        keys: &[Press::char('n')],
        label: "n",
        hint: msg::tui_settings_hint_add,
        command: Command::New,
    },
    Binding {
        keys: &[Press::char('D')],
        label: "D",
        hint: msg::tui_settings_hint_make_default,
        command: Command::Default,
    },
    Binding {
        keys: &[Press::char('x')],
        label: "x",
        hint: msg::tui_settings_hint_remove_revoke,
        command: Command::Remove,
    },
];

pub(super) struct AiConnectionsPage {
    block: Block,
    provider: usize,
    assistant: usize,
    /// `Some` while the Add / Edit model provider overlay is open.
    overlay: Option<ProviderPopup>,
}

impl AiConnectionsPage {
    pub(super) fn new() -> Self {
        Self {
            block: Block::Providers,
            provider: 0,
            assistant: 0,
            overlay: None,
        }
    }

    pub(super) fn overlay_open(&self) -> bool {
        self.overlay.is_some()
    }

    /// Closes the overlay, discarding what was typed; `false` when it wasn't open.
    pub(super) fn close_overlay(&mut self) -> bool {
        self.overlay.take().is_some()
    }

    pub(super) fn render_overlay(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        if let Some(overlay) = &self.overlay {
            overlay.render(frame, area, c);
        }
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) -> Option<Action> {
        let overlay = self.overlay.as_mut()?;
        Some(match overlay.handle_key(key) {
            Outcome::Stay => Action::NoOp,
            Outcome::Cancel => {
                self.overlay = None;
                Action::NoOp
            }
            Outcome::Save => {
                self.overlay = None;
                Action::FlashNotYetBuilt("provider save")
            }
        })
    }
}

impl list::Page for AiConnectionsPage {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.overlay.is_some() {
            return self.handle_overlay_key(key);
        }
        let providers = self.block == Block::Providers;
        Some(match keys::lookup(KEYS, key)? {
            Command::Row => {
                let step = list::motion(key)?;
                match self.block {
                    Block::Providers => {
                        self.provider = lib_list_cursor::step(self.provider, PROVIDERS.len(), step);
                    }
                    Block::Assistants => {
                        self.assistant =
                            lib_list_cursor::step(self.assistant, ASSISTANTS.len(), step);
                    }
                }
                Action::NoOp
            }
            Command::Block => {
                self.block = if providers {
                    Block::Assistants
                } else {
                    Block::Providers
                };
                Action::NoOp
            }
            Command::New if providers => {
                self.overlay = Some(ProviderPopup::new(PROVIDERS[0].name, "", true));
                Action::NoOp
            }
            Command::Edit if providers => {
                let provider = &PROVIDERS[self.provider];
                self.overlay = Some(ProviderPopup::new(provider.name, provider.model, false));
                Action::NoOp
            }
            Command::New => Action::FlashNotYetBuilt("assistant add"),
            Command::Edit => Action::FlashNotYetBuilt("assistant edit"),
            Command::Test if providers => Action::FlashNotYetBuilt("provider test"),
            Command::Test => Action::FlashNotYetBuilt("assistant test"),
            Command::Default if providers => Action::FlashNotYetBuilt("provider default"),
            // An assistant has no default to make.
            Command::Default => Action::NoOp,
            Command::Remove if providers => Action::FlashNotYetBuilt("provider remove"),
            Command::Remove => Action::FlashNotYetBuilt("assistant revoke"),
        })
    }

    fn scope_note(&self) -> String {
        msg::tui_settings_scope_ai_connections_counts(
            i64::try_from(PROVIDERS.len()).unwrap_or(i64::MAX),
            i64::try_from(ASSISTANTS.len()).unwrap_or(i64::MAX),
        )
    }

    fn hints(&self) -> Vec<(String, String)> {
        keys::hints(KEYS)
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let providers = Table {
            heading: msg::tui_settings_ai_providers(),
            add: vec![("n".into(), msg::tui_settings_ai_add_provider())],
            columns: vec![
                Column::left(
                    msg::tui_settings_ai_provider(),
                    Constraint::Length(PROVIDER_WIDTH),
                ),
                Column::left(
                    msg::tui_settings_ai_model(),
                    Constraint::Length(MODEL_WIDTH),
                ),
                Column::left(msg::tui_settings_ai_key(), Constraint::Length(KEY_WIDTH)),
                Column::left(msg::tui_settings_ai_status(), Constraint::Min(0)),
            ],
            rows: PROVIDERS
                .iter()
                .enumerate()
                .map(|(index, provider)| {
                    let mut name = vec![(provider.name.to_string(), Tone::Plain)];
                    if index == DEFAULT_PROVIDER {
                        name.push((" ".into(), Tone::Plain));
                        name.push((msg::tui_settings_ai_default(), Tone::Chip(true)));
                    }
                    Row::Item {
                        cells: vec![
                            Cell(name),
                            Cell::plain(provider.model),
                            Cell::muted(provider.key),
                            provider.status.cell(),
                        ],
                        selected: index == self.provider,
                        dim: false,
                    }
                })
                .collect(),
        };
        let assistants = Table {
            heading: msg::tui_settings_ai_assistants(),
            add: vec![("n".into(), msg::tui_settings_ai_add_assistant())],
            columns: vec![
                Column::left(
                    msg::tui_settings_ai_assistant(),
                    Constraint::Length(ASSISTANT_WIDTH),
                ),
                Column::left(
                    msg::tui_settings_ai_access(),
                    Constraint::Length(ACCESS_WIDTH),
                ),
                Column::left(
                    msg::tui_settings_ai_last_seen(),
                    Constraint::Length(SEEN_WIDTH),
                ),
                Column::left(msg::tui_settings_ai_status(), Constraint::Min(0)),
            ],
            rows: ASSISTANTS
                .iter()
                .enumerate()
                .map(|(index, assistant)| Row::Item {
                    cells: vec![
                        Cell::plain(assistant.name),
                        Cell::plain(msg::tui_settings_ai_read_only()),
                        Cell::muted(match assistant.seen {
                            Seen::Today(time) => {
                                msg::tui_settings_ai_seen_today(&format_time_minutes(time))
                            }
                            Seen::On(date) => crate::format::date(date),
                        }),
                        assistant.status.cell(),
                    ],
                    selected: index == self.assistant,
                    dim: false,
                })
                .collect(),
        };
        // The MCP server's facts, drawn between the block's rule and its assistants.
        let lead = [
            (
                msg::tui_settings_ai_server(),
                Cell(vec![
                    (msg::tui_settings_ai_server_on(), Tone::Accent),
                    (format!(" · {SERVER_ADDRESS}"), Tone::Plain),
                ]),
            ),
            (
                msg::tui_settings_ai_access(),
                Cell(vec![
                    (msg::tui_settings_ai_read_only(), Tone::Chip(true)),
                    ("  ".into(), Tone::Plain),
                    (msg::tui_settings_ai_read_write_not_offered(), Tone::Muted),
                ]),
            ),
            (
                msg::tui_settings_ai_approval(),
                Cell::plain(msg::tui_settings_ai_approval_value()),
            ),
        ];
        let [
            providers_area,
            providers_note,
            _,
            assistants_area,
            _,
            server_note,
        ] = Layout::vertical([
            Constraint::Length(providers.height_with(0)),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(assistants.height_with(lead.len())),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);
        providers.render_with(
            &[],
            frame,
            providers_area,
            focused && self.block == Block::Providers,
            c,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                msg::tui_settings_ai_providers_note(),
                c.muted(),
            ))),
            providers_note,
        );
        assistants.render_with(
            &lead,
            frame,
            assistants_area,
            focused && self.block == Block::Assistants,
            c,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                msg::tui_settings_ai_server_note(),
                c.muted(),
            ))),
            server_note,
        );
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

    fn page() -> SettingsView {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::AiConnections));
        view
    }

    #[test]
    fn at_96_by_30_the_page_draws_tui_034_with_read_and_write_not_yet_offered() {
        crate::locale::init_for_tests();
        let text = render_at(&page(), 96, 30);
        for needle in [
            "Settings › AI connections",
            "this device · 3 providers · 2 assistants",
            "MODEL PROVIDERS",
            "n add provider",
            "Claude [default]",
            "claude-sonnet-5-5",
            "● connected",
            "■ key rejected",
            "Keys stay in the OS keychain and never sync.",
            "ASSISTANT ACCESS · MCP SERVER",
            "n add assistant",
            "● on · 127.0.0.1:7421",
            "[read only]",
            "read & write · not yet offered",
            "ask before each change",
            "Claude Desktop",
            "today 2:02",
            "○ idle",
            "The server listens on this device only.",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
    }

    #[test]
    fn tab_moves_between_the_blocks_and_j_k_move_in_the_focused_one() {
        let mut page = AiConnectionsPage::new();
        let mut handle = |code| list::Page::handle_key(&mut page, key(code));
        assert_eq!(handle(KeyCode::Char('j')), Some(Action::NoOp));
        assert_eq!(handle(KeyCode::Tab), Some(Action::NoOp));
        assert_eq!(handle(KeyCode::Char('j')), Some(Action::NoOp));
        assert_eq!(page.provider, 1);
        assert_eq!((page.block, page.assistant), (Block::Assistants, 1));
    }

    #[test]
    fn every_other_key_flashes_not_yet_built_by_block() {
        let mut page = AiConnectionsPage::new();
        let mut handle = |code| {
            let modifiers = if code == KeyCode::Char('D') {
                KeyModifiers::SHIFT
            } else {
                KeyModifiers::NONE
            };
            list::Page::handle_key(&mut page, KeyEvent::new(code, modifiers))
        };
        assert_eq!(
            handle(KeyCode::Char('t')),
            Some(Action::FlashNotYetBuilt("provider test"))
        );
        assert_eq!(
            handle(KeyCode::Char('D')),
            Some(Action::FlashNotYetBuilt("provider default"))
        );
        assert_eq!(
            handle(KeyCode::Char('x')),
            Some(Action::FlashNotYetBuilt("provider remove"))
        );
        handle(KeyCode::Tab);
        for (code, command) in [
            (KeyCode::Char('n'), "assistant add"),
            (KeyCode::Char('e'), "assistant edit"),
            (KeyCode::Char('t'), "assistant test"),
            (KeyCode::Char('x'), "assistant revoke"),
        ] {
            assert_eq!(handle(code), Some(Action::FlashNotYetBuilt(command)));
        }
    }

    #[test]
    fn e_opens_the_provider_overlay_whose_api_key_renders_masked_and_esc_closes_it_first() {
        crate::locale::init_for_tests();
        let mut view = page();
        view.handle_key(key(KeyCode::Char('e')));
        assert!(view.popup_open() && view.editing());
        // Tab to the API key, then type one.
        view.handle_key(key(KeyCode::Tab));
        view.handle_key(key(KeyCode::Tab));
        for ch in "sk-hunter2".chars() {
            view.handle_key(key(KeyCode::Char(ch)));
        }
        let text = render_at(&view, 96, 30);
        for needle in [
            "edit model provider",
            "claude-sonnet-5-5",
            "••••••••••",
            "never shown again",
            "untested",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
        assert!(!text.contains("sk-hunter2"), "the key shows:\n{text}");

        // A locked Save keeps it open; a passing test unlocks it and Save flashes.
        assert_eq!(view.handle_key(key(KeyCode::Enter)), Some(Action::NoOp));
        view.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::FlashNotYetBuilt("provider save"))
        );
        assert!(!view.popup_open());

        view.handle_key(key(KeyCode::Char('n')));
        view.handle_key(key(KeyCode::Esc));
        assert!(!view.popup_open());
        assert!(view.back(), "the page steps back to the index");
    }

    #[test]
    fn the_hint_bar_is_the_keys_the_page_takes() {
        use list::Page as _;

        keys::tests::assert_the_hints_are_the_keys(KEYS, AiConnectionsPage::new().hints(), |key| {
            AiConnectionsPage::new().handle_key(key)
        });
    }
}
