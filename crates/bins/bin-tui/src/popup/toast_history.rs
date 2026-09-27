//! The session Toast history popup (`docs/toasts-design.md` "Session Toast history", #339):
//! every Toast raised this run, newest first, each with its Kind glyph in its mark colour, the
//! full untruncated Message, the time it was last raised and its `×N`. `j`/`k` or `↑`/`↓`
//! scroll, `Esc` closes; viewing dismisses nothing.
//!
//! The entries live on `Shell`'s `lib_toast::Toasts`; the popup owns only its scroll offset.

use chrono::{DateTime, Local};
use lib_locale::format::format_time;
use lib_toast::{History, HistoryEntry};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use crate::colours::Colours;

/// Width of the popup in cells: wide enough that most Messages sit on one line.
const POPUP_WIDTH: u16 = 72;

#[derive(Debug, Default)]
pub struct ToastHistoryPopup {
    /// The first entry drawn, counted from the newest.
    offset: usize,
}

impl ToastHistoryPopup {
    pub fn new() -> Self {
        Self::default()
    }

    /// Scrolls one entry, stopping with the oldest still on screen.
    pub fn scroll(&mut self, down: bool, len: usize) {
        self.offset = if down {
            (self.offset + 1).min(len.saturating_sub(1))
        } else {
            self.offset.saturating_sub(1)
        };
    }

    pub fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        history: &History<DateTime<Local>>,
        c: &Colours,
    ) {
        let width = POPUP_WIDTH.min(area.width);
        let inner_width = usize::from(width.saturating_sub(2).max(1));

        let mut lines = vec![
            Line::styled(
                lib_locale::msg::toast_history_title(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
        ];
        if history.is_empty() {
            lines.push(Line::styled(
                lib_locale::msg::toast_history_empty(),
                c.muted(),
            ));
        } else {
            lines.extend(
                history
                    .iter()
                    .skip(self.offset)
                    .map(|entry| entry_line(entry, c)),
            );
        }

        // Each line takes as many rows as it wraps onto; past the frame height the popup
        // fills the frame and the oldest entries fall off the bottom until scrolled to.
        let rows: usize = lines
            .iter()
            .map(|line| line.width().div_ceil(inner_width).max(1))
            .sum();
        let height = u16::try_from(rows + 2).unwrap_or(u16::MAX).min(area.height);
        let popup = Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + (area.height - height) / 2,
            width,
            height,
        };
        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }
}

/// `✓ 3:04:05 pm  Deleted tag Food ×2`: the time before the Message so the column lines up.
fn entry_line(entry: &HistoryEntry<DateTime<Local>>, c: &Colours) -> Line<'static> {
    let kind = entry.kind();
    let mut spans = vec![
        Span::styled(kind.glyph().to_string(), c.toast_mark(kind)),
        Span::raw(" "),
        Span::styled(format_time(entry.last_raised().time()), c.muted()),
        Span::raw("  "),
        Span::raw(entry.text().to_string()),
    ];
    if entry.count() > 1 {
        spans.push(Span::styled(format!(" ×{}", entry.count()), c.muted()));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use lib_toast::{ToastKind, Toasts};
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn at(second: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 9, 28, 15, 4, second)
            .single()
            .expect("test time is unambiguous")
    }

    fn render(popup: &ToastHistoryPopup, toasts: &Toasts<DateTime<Local>>) -> String {
        let mut terminal =
            Terminal::new(TestBackend::new(96, 20)).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area(), toasts.history(), &Colours::default()))
            .expect("rendering the popup should not error");
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn an_empty_history_says_so() {
        let text = render(&ToastHistoryPopup::new(), &Toasts::default());
        assert!(
            text.contains(&lib_locale::msg::toast_history_title()),
            "{text}"
        );
        assert!(
            text.contains(&lib_locale::msg::toast_history_empty()),
            "{text}"
        );
    }

    #[test]
    fn entries_list_newest_first() {
        let mut toasts = Toasts::default();
        toasts.raise(ToastKind::Info, "older", at(1));
        toasts.raise(ToastKind::Error, "newer", at(2));
        let text = render(&ToastHistoryPopup::new(), &toasts);
        let newer = text.find("✗").expect("the Error glyph drew");
        let older = text.find("i ").expect("the Info glyph drew");
        assert!(newer < older, "newest is not first:\n{text}");
        assert!(text.find("newer") < text.find("older"), "{text}");
    }

    #[test]
    fn a_merged_duplicate_is_one_entry_with_its_count() {
        let mut toasts = Toasts::default();
        toasts.raise(ToastKind::Success, "Deleted tag Food", at(1));
        toasts.raise(ToastKind::Success, "Deleted tag Food", at(2));
        let text = render(&ToastHistoryPopup::new(), &toasts);
        assert_eq!(text.matches("Deleted tag Food").count(), 1, "{text}");
        assert!(text.contains("Deleted tag Food ×2"), "{text}");
    }

    #[test]
    fn a_long_message_is_not_truncated() {
        let mut toasts = Toasts::default();
        let long = "word ".repeat(30);
        toasts.raise(ToastKind::Warning, long.trim(), at(1));
        let text = render(&ToastHistoryPopup::new(), &toasts);
        assert_eq!(text.matches("word").count(), 30, "{text}");
        assert!(!text.contains('…'), "{text}");
    }

    #[test]
    fn scrolling_stops_at_the_oldest_entry() {
        let mut popup = ToastHistoryPopup::new();
        for _ in 0..5 {
            popup.scroll(true, 2);
        }
        assert_eq!(popup.offset, 1);
        popup.scroll(false, 2);
        popup.scroll(false, 2);
        assert_eq!(popup.offset, 0);
    }
}
