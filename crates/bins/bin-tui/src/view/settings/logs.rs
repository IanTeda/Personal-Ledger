//! Settings › Logs (TUI-038): the recent `tracing` events `main` captures into a
//! [`LogBuffer`], read on each draw, newest first. The level filter and `C clear logs` share one
//! row over the log box, which takes the rest of the page and always draws its scrollbar.
//!
//! The filter only narrows what is drawn: the capture keeps recording at `debug` whatever it is
//! set to, so changing it is instant and retroactive, as on the Desktop's Tracing page. `C` asks
//! first, as the Desktop's Clear logs Dialog does; the confirm is the Settings View's own popup,
//! drawn over the page like the picker, so it needs no Dialog host of its own.
//!
//! A line is `[hh:mm:ss] subsystem: message key=value`: diagnostic output, so not a Message
//! (`docs/localisation-design.md`).

use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_toast::ToastKind;
use lib_tracing::{LogBuffer, LogEntry};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use tracing::Level;

use super::list;
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;

/// The most verbose level the log box shows. There is no `trace` or `off`: the capture never
/// records `trace`, and `off` would only ever show the empty state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Filter {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
}

impl Filter {
    const ALL: [Filter; 4] = [Self::Error, Self::Warn, Self::Info, Self::Debug];

    fn label(self) -> String {
        match self {
            Self::Error => msg::tui_settings_logs_level_error(),
            Self::Warn => msg::tui_settings_logs_level_warn(),
            Self::Info => msg::tui_settings_logs_level_info(),
            Self::Debug => msg::tui_settings_logs_level_debug(),
        }
    }

    /// Whether an entry at `level` shows under this filter.
    fn admits(self, level: Level) -> bool {
        let most_verbose = match self {
            Self::Error => Level::ERROR,
            Self::Warn => Level::WARN,
            Self::Info => Level::INFO,
            Self::Debug => Level::DEBUG,
        };
        // `tracing` orders levels by verbosity: `ERROR` is the least.
        level <= most_verbose
    }

    /// The filter one step along [`Self::ALL`], towards `debug` when `forward`, clamped at both ends.
    fn step(self, forward: bool) -> Self {
        let index = Self::ALL
            .iter()
            .position(|filter| *filter == self)
            .unwrap_or_default();
        let index = if forward {
            (index + 1).min(Self::ALL.len() - 1)
        } else {
            index.saturating_sub(1)
        };
        Self::ALL[index]
    }
}

/// The confirm's width, and its rows inside the border.
const CONFIRM_WIDTH: u16 = 64;
const CONFIRM_ROWS: u16 = 6;

pub(super) struct LogsPage {
    buffer: LogBuffer,
    filter: Filter,
    /// The selected line among those the filter shows, newest first, so `0` follows the newest.
    selected: usize,
    /// Whether the Clear logs confirm is open.
    confirm: bool,
    /// The Toasts this page raised, which `Shell` takes after each key.
    toasts: Vec<(ToastKind, String)>,
}

impl LogsPage {
    pub(super) fn new(buffer: LogBuffer) -> Self {
        Self {
            buffer,
            filter: Filter::default(),
            selected: 0,
            confirm: false,
            toasts: Vec::new(),
        }
    }

    /// The entries the filter shows, newest first.
    fn visible(&self) -> Vec<std::sync::Arc<LogEntry>> {
        self.buffer
            .snapshot()
            .into_iter()
            .rev()
            .filter(|entry| self.filter.admits(entry.level))
            .collect()
    }

    pub(super) fn confirm_open(&self) -> bool {
        self.confirm
    }

    /// Closes the confirm, keeping the logs; `false` when it wasn't open.
    pub(super) fn close_confirm(&mut self) -> bool {
        std::mem::take(&mut self.confirm)
    }

    pub(super) fn take_toasts(&mut self) -> Vec<(ToastKind, String)> {
        std::mem::take(&mut self.toasts)
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.confirm {
            match key.code {
                KeyCode::Enter => {
                    self.buffer.clear();
                    self.selected = 0;
                    self.confirm = false;
                    self.toasts
                        .push((ToastKind::Success, msg::tui_settings_logs_cleared()));
                }
                KeyCode::Esc => self.confirm = false,
                _ => {}
            }
            // The confirm takes every key, as a popup does.
            return Some(Action::NoOp);
        }
        if let Some(step) = list::motion(key) {
            self.selected = lib_list_cursor::step(self.selected, self.visible().len(), step);
            return Some(Action::NoOp);
        }
        if key.modifiers != KeyModifiers::NONE && key.modifiers != KeyModifiers::SHIFT {
            return None;
        }
        match key.code {
            KeyCode::Left | KeyCode::Right => {
                self.filter = self.filter.step(key.code == KeyCode::Right);
                self.selected = 0;
                Some(Action::NoOp)
            }
            KeyCode::Char('C') => {
                self.confirm = true;
                Some(Action::NoOp)
            }
            KeyCode::Char('y') => {
                let visible = self.visible();
                let entry = visible.get(self.selected)?;
                Some(Action::CopyText(line(entry)))
            }
            _ => None,
        }
    }

    pub(super) fn hints() -> Vec<(String, String)> {
        vec![
            ("←→".into(), msg::tui_settings_hint_level()),
            ("j/k".into(), msg::tui_settings_hint_scroll()),
            ("Home/G".into(), msg::tui_settings_hint_newest_oldest()),
            ("C".into(), msg::tui_settings_hint_clear()),
            ("y".into(), msg::tui_settings_hint_copy_line()),
        ]
    }

    pub(super) fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let [controls, _, log] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(area);
        self.render_controls(frame, controls, c);
        self.render_log(frame, log, focused, c);
    }

    /// `level  error  warn  info  debug` with the filter reversed, and `C clear logs` on the right.
    fn render_controls(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let mut spans = vec![
            Span::styled(msg::tui_settings_logs_level(), c.muted()),
            Span::raw("  "),
        ];
        for filter in Filter::ALL {
            let style = if filter == self.filter {
                c.selection()
            } else {
                c.muted()
            };
            spans.push(Span::styled(format!(" {} ", filter.label()), style));
            spans.push(Span::raw(" "));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        let clear = Line::from(vec![
            Span::styled("C", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(msg::tui_settings_logs_clear(), c.muted()),
        ]);
        frame.render_widget(Paragraph::new(clear).alignment(Alignment::Right), area);
    }

    /// The bordered log box, its lines scrolled to keep the selection in view, with the
    /// scrollbar in its rightmost inner column whether or not the lines overflow.
    fn render_log(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let block = Block::new().borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let [lines, bar] =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

        let visible = self.visible();
        let height = usize::from(lines.height);
        let selected = self.selected.min(visible.len().saturating_sub(1));
        let offset = if height > 0 && selected >= height {
            selected + 1 - height
        } else {
            0
        };
        let mut state = ScrollbarState::new(visible.len().saturating_sub(height).max(1))
            .position(offset)
            .viewport_content_length(height);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            bar,
            &mut state,
        );

        if visible.is_empty() {
            let hidden = self.buffer.snapshot().len();
            let text = if hidden == 0 {
                msg::tui_settings_logs_empty()
            } else {
                let count = i64::try_from(hidden).unwrap_or(i64::MAX);
                msg::tui_settings_logs_all_hidden(&self.filter.label(), count)
            };
            frame.render_widget(
                Paragraph::new(Span::styled(text, c.muted())).wrap(Wrap { trim: true }),
                lines,
            );
            return;
        }

        for (index, entry) in visible.iter().skip(offset).take(height).enumerate() {
            let row = Rect {
                y: lines.y + u16::try_from(index).unwrap_or(0),
                height: 1,
                ..lines
            };
            let is_selected = focused && offset + index == selected;
            if is_selected {
                frame.render_widget(Block::new().style(c.selection()), row);
            }
            let body_style = match (entry.level, is_selected) {
                (_, true) => Style::default(),
                (Level::ERROR, false) => c.negative(),
                (Level::WARN, false) => c.accent(),
                (Level::INFO, false) => Style::default(),
                (_, false) => c.muted(),
            };
            let time_style = if is_selected {
                Style::default()
            } else {
                c.muted()
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(timestamp(entry), time_style),
                    Span::raw(" "),
                    Span::styled(body(entry), body_style),
                ])),
                row,
            );
        }
    }

    /// The Clear logs confirm, centred in the top third of `area`.
    pub(super) fn render_confirm(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let width = CONFIRM_WIDTH.min(area.width);
        let height = (CONFIRM_ROWS + 2).min(area.height);
        let popup = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: (area.y + (area.height / 6).max(1)).min(area.y + area.height.saturating_sub(height)),
            width,
            height,
        };
        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let [title, rule, warning, keys] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(inner);
        let bold = Style::default().add_modifier(Modifier::BOLD);
        frame.render_widget(
            Paragraph::new(Span::styled(msg::tui_settings_logs_clear_title(), bold)),
            title,
        );
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rule);
        frame.render_widget(
            Paragraph::new(msg::tui_settings_logs_clear_warning()).wrap(Wrap { trim: true }),
            warning,
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("enter", bold),
                Span::styled(
                    format!(" {}", msg::tui_settings_logs_clear_confirm()),
                    c.negative(),
                ),
                Span::raw(" · "),
                Span::styled("esc", bold),
                Span::raw(format!(" {}", msg::tui_settings_logs_clear_cancel())),
            ])),
            keys,
        );
    }
}

/// `[hh:mm:ss]` in local time.
fn timestamp(entry: &LogEntry) -> String {
    DateTime::<Local>::from(entry.time)
        .format("[%H:%M:%S]")
        .to_string()
}

/// `subsystem: message key=value key=value`.
fn body(entry: &LogEntry) -> String {
    let mut text = format!("{}: {}", subsystem(&entry.target), entry.message);
    for (name, value) in &entry.fields {
        text.push_str(&format!(" {name}={value}"));
    }
    text
}

/// The whole line, as `y` copies it.
fn line(entry: &LogEntry) -> String {
    format!("{} {}", timestamp(entry), body(entry))
}

/// The short "subsystem" a target shows as, as the Desktop's Tracing page names it: for our own
/// crates the first module after the crate (`bin_tui::shell` is `shell`), or the crate without
/// its `bin_`/`lib_` prefix at its root; a dependency shows as its crate.
fn subsystem(target: &str) -> &str {
    let mut segments = target.split("::");
    let krate = segments.next().unwrap_or(target);
    let ours = krate
        .strip_prefix("bin_")
        .or_else(|| krate.strip_prefix("lib_"));
    match (ours, segments.next()) {
        (Some(_), Some(module)) => module,
        (Some(bare), None) => bare,
        (None, _) => krate,
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::time::SystemTime;

    use super::*;
    use crate::view::View;
    use crate::view::settings::tests::render_at;
    use crate::view::settings::{SettingsPage, SettingsView};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn push(buffer: &LogBuffer, level: Level, target: &'static str, message: &str) {
        buffer.push(LogEntry {
            seq: 0,
            time: SystemTime::now(),
            level,
            target: Cow::Borrowed(target),
            message: message.to_string(),
            fields: Vec::new(),
        });
    }

    fn logs_page(buffer: &LogBuffer) -> SettingsView {
        let mut view = SettingsView::with_logs(buffer.clone());
        view.update(&Action::OpenSettingsPage(SettingsPage::Logs));
        view
    }

    #[test]
    fn the_page_draws_the_filter_clear_and_the_log_newest_first() {
        crate::locale::init_for_tests();
        let buffer = LogBuffer::new(8);
        push(&buffer, Level::INFO, "bin_tui::shell", "opened the ledger");
        push(&buffer, Level::WARN, "lib_database::writer", "slow write");
        let text = render_at(&logs_page(&buffer), 96, 30);
        for needle in [
            "Settings › Logs",
            "this device · diagnostic · last 1,000 entries",
            "level",
            " error ",
            " debug ",
            "C clear logs",
            "] writer: slow write",
            "] shell: opened the ledger",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
        let newer = text.find("slow write").unwrap_or_default();
        let older = text.find("opened the ledger").unwrap_or_default();
        assert!(newer < older, "newest first:\n{text}");
    }

    #[test]
    fn an_event_captured_after_the_page_opens_shows_on_the_next_draw() {
        crate::locale::init_for_tests();
        let buffer = LogBuffer::new(8);
        let view = logs_page(&buffer);
        assert!(render_at(&view, 96, 30).contains("No log entries yet."));
        push(&buffer, Level::INFO, "bin_tui::shell", "late arrival");
        assert!(render_at(&view, 96, 30).contains("shell: late arrival"));
    }

    #[test]
    fn the_log_box_always_draws_its_scrollbar() {
        crate::locale::init_for_tests();
        let text = render_at(&logs_page(&LogBuffer::new(8)), 96, 30);
        // The box's right border is column 94 inside the View's margin; the bar sits beside it.
        let bar: String = text
            .lines()
            .skip(6)
            .take(3)
            .filter_map(|row| row.chars().nth(93))
            .collect();
        assert!(bar.chars().all(|ch| ch != ' '), "{bar:?}\n{text}");
    }

    #[test]
    fn left_and_right_narrow_and_widen_what_is_drawn_never_what_is_captured() {
        crate::locale::init_for_tests();
        let buffer = LogBuffer::new(8);
        push(&buffer, Level::ERROR, "bin_tui::shell", "broke");
        push(&buffer, Level::INFO, "bin_tui::shell", "fine");
        push(&buffer, Level::DEBUG, "bin_tui::shell", "chatter");
        let mut view = logs_page(&buffer);
        assert!(!render_at(&view, 96, 30).contains("chatter"));
        view.handle_key(key(KeyCode::Right));
        assert!(render_at(&view, 96, 30).contains("chatter"));
        for _ in 0..4 {
            view.handle_key(key(KeyCode::Left));
        }
        let text = render_at(&view, 96, 30);
        assert!(text.contains("broke") && !text.contains("fine"), "{text}");
        assert_eq!(buffer.snapshot().len(), 3, "the capture keeps everything");
    }

    #[test]
    fn capital_c_asks_first_then_enter_clears_and_raises_a_toast() {
        crate::locale::init_for_tests();
        let buffer = LogBuffer::new(8);
        push(&buffer, Level::INFO, "bin_tui::shell", "keep me");
        let mut view = logs_page(&buffer);

        view.handle_key(key(KeyCode::Char('C')));
        assert!(view.popup_open());
        assert!(render_at(&view, 96, 30).contains("Remove every captured log entry"));
        view.handle_key(key(KeyCode::Esc));
        assert!(!view.popup_open());
        assert_eq!(buffer.snapshot().len(), 1, "esc keeps the logs");

        view.handle_key(key(KeyCode::Char('C')));
        view.handle_key(key(KeyCode::Enter));
        assert!(!view.popup_open());
        assert!(buffer.snapshot().is_empty());
        assert_eq!(
            view.take_toasts(),
            vec![(ToastKind::Success, "Logs cleared".to_string())]
        );
    }

    #[test]
    fn y_copies_the_selected_line_and_j_moves_to_an_older_one() {
        let buffer = LogBuffer::new(8);
        push(&buffer, Level::INFO, "bin_tui::shell", "older");
        push(&buffer, Level::INFO, "bin_tui::shell", "newer");
        let mut view = logs_page(&buffer);
        let copied = |view: &mut SettingsView| match view.handle_key(key(KeyCode::Char('y'))) {
            Some(Action::CopyText(text)) => text,
            other => panic!("expected a copy, got {other:?}"),
        };
        assert!(copied(&mut view).ends_with("] shell: newer"));
        view.handle_key(key(KeyCode::Char('j')));
        assert!(copied(&mut view).ends_with("] shell: older"));
    }

    #[test]
    fn subsystem_is_the_first_module_after_our_crate_or_a_dependency_s_crate() {
        assert_eq!(subsystem("bin_tui::shell::draw"), "shell");
        assert_eq!(subsystem("lib_locale"), "locale");
        assert_eq!(subsystem("sqlx::query"), "sqlx");
    }
}
