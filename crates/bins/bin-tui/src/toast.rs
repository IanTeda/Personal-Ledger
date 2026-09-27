//! The Toast overlay: `Shell`'s `lib_toast` stack drawn bottom-right over the view, per
//! `docs/toasts-design.md` "Placement and layout". Hand-rolled (#311): each Toast is a 3-row
//! bordered box, newest nearest the footer rule, with a muted `+N more` line above the stack.
//! `Shell` draws it last, above popups and their `Dim`, so an outcome is never hidden by the
//! surface that caused it.

use lib_toast::{Toast, Toasts};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::colours::Colours;

/// The widest a Toast box draws, border included.
const MAX_WIDTH: u16 = 48;
/// A Toast box's rows: border, the one line of content, border.
const HEIGHT: u16 = 3;
/// Below this frame size no Toast is drawn; the status-line echo carries them instead (#309).
pub const MIN_FRAME: (u16, u16) = (40, 8);
const BAR: &str = "▌";
const ELLIPSIS: char = '…';

/// Draws the visible Toasts inside `area` (the view region, so they never cover the footer or
/// status line), stacking up from its bottom edge and stopping at whatever fits.
pub fn render<S>(frame: &mut Frame<'_>, area: Rect, toasts: &Toasts<S>, c: &Colours) {
    let full = frame.area();
    if full.width < MIN_FRAME.0 || full.height < MIN_FRAME.1 {
        return;
    }

    let max_width = MAX_WIDTH.min(area.width);
    let fits = usize::from(area.height / HEIGHT);
    let visible = toasts.visible();
    let shown = visible.len().min(fits);

    let mut bottom = area.bottom();
    // Newest is last in `visible` and sits nearest the footer.
    for toast in visible.iter().rev().take(shown) {
        let width = u16::try_from(FIXED + text_width(toast.text()) + badge_width(toast))
            .unwrap_or(u16::MAX)
            .min(max_width);
        let rect = Rect::new(area.right() - width, bottom - HEIGHT, width, HEIGHT);
        render_toast(frame, rect, toast, c);
        bottom -= HEIGHT;
    }

    let more = toasts.more_count() + (visible.len() - shown);
    if more > 0 && bottom > area.top() {
        let text = crate::msg::tui_toast_more(&more.to_string());
        let line = Line::from(text).style(c.muted()).right_aligned();
        let rect = Rect::new(area.right() - max_width, bottom - 1, max_width, 1);
        frame.render_widget(Clear, rect);
        frame.render_widget(Paragraph::new(line), rect);
    }
}

fn render_toast(frame: &mut Frame<'_>, rect: Rect, toast: &Toast, c: &Colours) {
    let kind = toast.kind();
    let mark = c.toast_mark(kind);
    let badge = badge(toast);
    let room = usize::from(rect.width).saturating_sub(FIXED + badge_width(toast));

    let mut spans = vec![
        Span::styled(BAR, mark),
        Span::raw(" "),
        Span::styled(kind.glyph().to_string(), mark),
        Span::raw(" "),
        Span::raw(truncate(toast.text(), room)),
    ];
    if let Some(badge) = badge {
        spans.push(Span::styled(badge, c.muted()));
    }

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(c.toast_border())
        .style(c.toast());
    frame.render_widget(Clear, rect);
    frame.render_widget(Paragraph::new(Line::from(spans)).block(block), rect);
}

/// Columns around the text: both borders, the bar, the glyph, the spaces after each and one
/// before the right border.
const FIXED: usize = 2 + 4 + 1;

/// The muted `×N` badge for a merged duplicate; never truncated.
fn badge(toast: &Toast) -> Option<String> {
    (toast.count() > 1).then(|| format!(" ×{}", toast.count()))
}

fn badge_width(toast: &Toast) -> usize {
    badge(toast).as_deref().map_or(0, text_width)
}

fn text_width(text: &str) -> usize {
    Span::raw(text).width()
}

/// `text` cut to at most `room` columns, ending in `…` when anything was cut.
fn truncate(text: &str, room: usize) -> String {
    if text_width(text) <= room {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = text_width(ch.encode_utf8(&mut [0; 4]));
        if used + w + 1 > room {
            break;
        }
        out.push(ch);
        used += w;
    }
    if room > 0 {
        out.push(ELLIPSIS);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_untouched() {
        assert_eq!(truncate("Deleted tag Food", 20), "Deleted tag Food");
    }

    #[test]
    fn long_text_ends_in_an_ellipsis_within_its_room() {
        let cut = truncate("Deleted account Everyday Spending", 12);
        assert_eq!(cut, "Deleted acc…");
        assert_eq!(text_width(&cut), 12);
    }

    #[test]
    fn wide_characters_count_as_two_columns() {
        assert_eq!(truncate("日本語テキスト", 7), "日本語…");
    }
}
