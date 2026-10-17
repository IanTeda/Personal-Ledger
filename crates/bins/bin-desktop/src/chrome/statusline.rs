//! The shell's status line (`docs/ux/desktop-mockups/01-shell/README.md`'s "1a" spec,
//! "Status line" component): mode badge, hint strip. Its own bottom-right breadcrumb (`ledger ·
//! <noun>`) was dropped -- the top bar's brand tile now names the active screen instead
//! (`crate::chrome::topbar::brand_mark`), and showing it in both places was a plain duplicate.

use std::rc::Rc;

use gpui::{App, FontWeight, Pixels, Window, div, prelude::*, px};

use lib_toast::ToastKind;

use crate::{
    chrome::{
        text_fit::{hints_that_fit, text_width},
        window_size::WindowSize,
    },
    navigation::nav::InputMode,
    theme::color,
};

const PADDING_X: Pixels = px(12.0);
const GAP: Pixels = px(14.0);
const TEXT_SIZE: Pixels = px(11.5);
const BADGE_SIZE: Pixels = px(10.0);
const BADGE_PADDING_X: Pixels = px(7.0);
/// Between a hint's key and its action, and either side of the `·` between hints.
const HINT_GAP: Pixels = px(4.0);

/// Band height: `docs/ux/desktop-mockups/01-shell/README.md`'s "Layout" table.
pub const HEIGHT: Pixels = px(28.0);

/// A page's own status-line content, replacing the shell-wide hint strip and file path while that
/// page is showing: its key legend (`(key, action)` pairs) and its right-aligned text.
#[derive(Debug, Clone)]
pub struct PageStatus {
    pub hints: Vec<(&'static str, String)>,
    pub right: String,
}

/// The shell-wide hint strip's clickable entries -- each does what its key does in `Normal` mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintAction {
    Command,
    Search,
    Help,
    ToggleRail,
}

pub type OnHint = Rc<dyn Fn(HintAction, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct StatusLine {
    on_hint: Option<OnHint>,
    mode: InputMode,
    /// Replaces the hint strip when `Some` -- the handoff's "Loading and error states" rule
    /// ("the hint strip is replaced by the error ... cleared by any keypress"), reused here
    /// for the `g`-jump prefix's own "flash the hint strip" abort message and the command
    /// palette's own "not yet built" message (`Shell`'s `status_message`), not just write
    /// failures. Ignored while `palette_query` is `Some` -- the "1d" spec's own COMMAND-mode
    /// status line has no room for both the live query and a message, and a message can only
    /// be showing here right after the palette that set it has already closed.
    status_message: Option<String>,
    /// The active command-mode echo text and its own "esc closes ..." hint, `Some` while
    /// `InputMode::Command` has something other than the ordinary hint strip to show: either
    /// the palette's live input (`docs/ux/desktop-mockups/01-shell/README.md`'s "1d" spec:
    /// "Status line in COMMAND mode: ... the live query echoes ...; right side reads 'esc close
    /// command window'"), or -- once `:open` is confirmed -- the "1e" file explorer's own
    /// frozen `"open"` echo and "esc close file explorer" hint, since `Shell` keeps `mode` at
    /// `Command` for as long as that dialog is open (see `Shell::run_command`'s own doc).
    command_echo: Option<(String, String)>,
    /// `Some` while a page with its own legend is showing (`docs/ux/desktop-mockups/17-accounts/README.md`'s
    /// 3a status bar). Ignored in command mode, which owns the whole line.
    page: Option<PageStatus>,
    /// The status-line echo (ADR-0027): the Toast carried here while Toasts are off. Below
    /// `status_message`, above the page legend and hint strip.
    toast_echo: Option<(ToastKind, String)>,
    /// Replaces the mode badge's text: a page's own mode (6e's `IMPORT`), drawn like `Normal`.
    mode_label: Option<String>,
}

impl StatusLine {
    pub fn new(
        mode: InputMode,
        status_message: Option<String>,
        command_echo: Option<(String, String)>,
    ) -> Self {
        Self {
            on_hint: None,
            mode,
            status_message,
            command_echo,
            page: None,
            toast_echo: None,
            mode_label: None,
        }
    }

    pub fn toast_echo(mut self, echo: Option<(ToastKind, String)>) -> Self {
        self.toast_echo = echo;
        self
    }

    /// Makes the shell-wide hint strip's entries clickable.
    pub fn on_hint(mut self, on_hint: OnHint) -> Self {
        self.on_hint = Some(on_hint);
        self
    }

    pub fn mode_label(mut self, label: Option<String>) -> Self {
        self.mode_label = label;
        self
    }

    pub fn page(mut self, page: Option<PageStatus>) -> Self {
        self.page = page;
        self
    }
}

impl RenderOnce for StatusLine {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let window_size = WindowSize::of(window);
        let right = match (&self.command_echo, &self.page) {
            (Some((_, hint)), _) => hint.clone(),
            (None, Some(page)) => page.right.clone(),
            (None, None) => file_path(window_size),
        };
        let badge_label = lib_locale::format::upper(
            &self
                .mode_label
                .clone()
                .unwrap_or_else(|| mode_label(self.mode)),
        );
        // What the mode chip and the context note leave the hints; they never shrink.
        let room = window.viewport_size().width
            - PADDING_X * 2.0
            - GAP * 3.0
            - text_width(window, &badge_label, BADGE_SIZE, FontWeight::EXTRA_BOLD)
            - BADGE_PADDING_X * 2.0
            - text_width(window, &right, TEXT_SIZE, FontWeight::NORMAL);
        let fit = |hints: &[(&str, String)]| -> usize {
            let widths: Vec<Pixels> = hints
                .iter()
                .map(|(key, action)| {
                    text_width(window, key, TEXT_SIZE, FontWeight::EXTRA_BOLD)
                        + HINT_GAP
                        + text_width(window, action, TEXT_SIZE, FontWeight::NORMAL)
                })
                .collect();
            let separator =
                HINT_GAP * 2.0 + text_width(window, "\u{b7}", TEXT_SIZE, FontWeight::NORMAL);
            hints_that_fit(&widths, separator, room)
        };
        div()
            .h(HEIGHT)
            .flex_none()
            .flex()
            .items_center()
            .gap(GAP)
            .px(PADDING_X)
            .bg(color::chrome(cx))
            .border_t(px(2.0))
            .border_color(color::structural_rule(cx))
            .text_size(TEXT_SIZE)
            .text_color(color::muted(cx))
            .whitespace_nowrap()
            .child(mode_badge(self.mode, badge_label, cx))
            .child(
                match left(
                    self.command_echo.as_ref().map(|(query, _)| query.as_str()),
                    self.status_message,
                    self.toast_echo,
                    self.page.as_ref(),
                ) {
                    Left::CommandEcho(query) => command_query_echo(query, cx).into_any_element(),
                    Left::StatusMessage(message) => div()
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .text_color(color::accent_text(cx))
                        .child(message)
                        .into_any_element(),
                    Left::ToastEcho(kind, text) => toast_echo(kind, text, cx).into_any_element(),
                    Left::PageHints(hints) => {
                        let shown = fit(hints);
                        page_hint_strip(&hints[..shown], cx).into_any_element()
                    }
                    Left::HintStrip => {
                        let shown = fit(&shell_hints());
                        hint_strip(self.on_hint.clone(), shown, cx).into_any_element()
                    }
                },
            )
            .child(div().flex_1())
            .child(
                div()
                    .debug_selector(|| "status-right".to_string())
                    .flex_none()
                    .child(right),
            )
    }
}

/// What the left side of the status line shows, highest precedence first (ADR-0027): command
/// echo > status-line message > Toast echo > page legend / shell hint strip.
#[derive(Debug, PartialEq, Eq)]
enum Left<'a> {
    CommandEcho(&'a str),
    StatusMessage(String),
    ToastEcho(ToastKind, String),
    PageHints(&'a [(&'static str, String)]),
    HintStrip,
}

fn left<'a>(
    command_echo: Option<&'a str>,
    status_message: Option<String>,
    toast_echo: Option<(ToastKind, String)>,
    page: Option<&'a PageStatus>,
) -> Left<'a> {
    if let Some(query) = command_echo {
        Left::CommandEcho(query)
    } else if let Some(message) = status_message {
        Left::StatusMessage(message)
    } else if let Some((kind, text)) = toast_echo {
        Left::ToastEcho(kind, text)
    } else if let Some(page) = page {
        Left::PageHints(&page.hints)
    } else {
        Left::HintStrip
    }
}

/// The Kind glyph in its mark colour, then the Message in `foreground`, as a Toast reads.
fn toast_echo(kind: ToastKind, text: String, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .text_color(color::foreground(cx))
        .child(
            div()
                .font_weight(FontWeight::EXTRA_BOLD)
                .text_color(color::toast_mark(kind, cx))
                .child(kind.glyph().to_string()),
        )
        .child(text)
}

/// The mode chip's own label, before a View's override.
fn mode_label(mode: InputMode) -> String {
    match mode {
        InputMode::Normal => crate::msg::desktop_mode_normal(),
        InputMode::Insert => crate::msg::desktop_mode_insert(),
        InputMode::Search => crate::msg::desktop_mode_search(),
        InputMode::Command => crate::msg::desktop_mode_command(),
        InputMode::Dialog => crate::msg::desktop_mode_dialog(),
        InputMode::Filter => crate::msg::desktop_mode_filter(),
    }
}

fn mode_badge(mode: InputMode, label: String, cx: &App) -> impl IntoElement {
    let bg = match mode {
        InputMode::Normal | InputMode::Insert | InputMode::Search => color::foreground(cx),
        InputMode::Command | InputMode::Dialog | InputMode::Filter => color::accent(cx),
    };

    div()
        .flex_none()
        .bg(bg)
        .text_color(color::background(cx))
        .font_weight(FontWeight::EXTRA_BOLD)
        .text_size(BADGE_SIZE)
        .py(px(2.0))
        .px(BADGE_PADDING_X)
        .child(label)
}

/// The shell-wide hints, in the order they drop from the end: `(id, action, key, label)`.
fn shell_hint_entries() -> [(&'static str, HintAction, &'static str, String); 4] {
    [
        (
            "hint-command",
            HintAction::Command,
            ":",
            crate::msg::desktop_hint_command(),
        ),
        (
            "hint-search",
            HintAction::Search,
            "/",
            crate::msg::desktop_hint_search(),
        ),
        (
            "hint-help",
            HintAction::Help,
            "?",
            crate::msg::desktop_hint_help(),
        ),
        (
            "hint-rail",
            HintAction::ToggleRail,
            "b",
            crate::msg::desktop_hint_toggle_sidebar(),
        ),
    ]
}

/// The shell-wide hints as `(key, label)`, for measuring.
fn shell_hints() -> Vec<(&'static str, String)> {
    shell_hint_entries()
        .into_iter()
        .map(|(_, _, key, label)| (key, label))
        .collect()
}

/// The shell-wide hints, the first `shown` of them, each a click target for its key's action.
fn hint_strip(on_hint: Option<OnHint>, shown: usize, cx: &App) -> impl IntoElement {
    let entry = |id: &'static str, action: HintAction, key: &'static str, label: String| {
        let on_hint = on_hint.clone();
        div()
            .id(id)
            .debug_selector(|| id.to_string())
            .flex()
            .items_center()
            .gap(HINT_GAP)
            .when(on_hint.is_some(), |this| {
                this.cursor_pointer()
                    .hover(|style| style.bg(color::hover(cx)))
            })
            .when_some(on_hint, |this, on_hint| {
                this.on_click(move |_event, window, cx| on_hint(action, window, cx))
            })
            .child(
                div()
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .text_color(color::foreground(cx))
                    .child(key),
            )
            .child(label)
            .into_any_element()
    };

    div().flex().items_center().gap(HINT_GAP).children(
        shell_hint_entries()
            .into_iter()
            .take(shown)
            .enumerate()
            .flat_map(|(index, (id, action, key, label))| {
                let separator = (index > 0).then(|| div().child("\u{b7}").into_any_element());
                separator.into_iter().chain([entry(id, action, key, label)])
            }),
    )
}

/// A page's key legend: `j/k row · enter open ledger · ...`, each key at weight 800 like the
/// shell-wide hint strip's own.
fn page_hint_strip(hints: &[(&'static str, String)], cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(HINT_GAP)
        .children(hints.iter().enumerate().flat_map(|(index, (key, action))| {
            let separator = (index > 0).then(|| div().child("\u{b7}").into_any_element());
            separator.into_iter().chain([div()
                .debug_selector(move || format!("status-hint-{index}"))
                .flex()
                .items_center()
                .gap(HINT_GAP)
                .child(
                    div()
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .text_color(color::foreground(cx))
                        .child(*key),
                )
                .child(action.clone())
                .into_any_element()])
        }))
}

/// The "1d" spec's COMMAND-mode echo: `:{query}` at weight 800, then the block caret --
/// the status line's own smaller echo of the palette's input row, kept in sync by `Shell`
/// passing the same `Palette::input()` string to both.
fn command_query_echo(query: &str, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .font_weight(FontWeight::EXTRA_BOLD)
        .text_color(color::foreground(cx))
        .child(format!(":{query}"))
        .child(div().w(px(2.0)).h(px(13.0)).bg(color::cursor(cx)))
}

/// The open Ledger's own file path, moved here from the top bar's brand tile (`crate::chrome::topbar`)
/// -- representative content, not a real path yet (lands with the sync/open-file tickets,
/// #164/#165). Shortened to its file name at Medium (DESKTOP-116).
fn file_path(window_size: WindowSize) -> String {
    match window_size {
        WindowSize::Medium => format!("\u{2026}/{STUB_LEDGER_FILE} · aud"),
        WindowSize::Large | WindowSize::XLarge => {
            format!("~/Documents/{STUB_LEDGER_FILE} · aud")
        }
    }
}

/// The stub Ledger's file name, shared with the Documents status line's "stored beside" note until
/// a real open-file path exists.
pub const STUB_LEDGER_FILE: &str = "My-Personal-Ledger.pldb";

#[cfg(test)]
mod tests {
    use super::*;

    fn echo() -> Option<(ToastKind, String)> {
        Some((ToastKind::Success, "Deleted tag Food".to_string()))
    }

    #[test]
    fn the_toast_echo_replaces_the_hint_strip_and_the_page_legend() {
        let page = PageStatus {
            hints: vec![("j/k", "row".to_string())],
            right: String::new(),
        };
        assert_eq!(left(None, None, None, None), Left::HintStrip);
        assert_eq!(
            left(None, None, None, Some(&page)),
            Left::PageHints(&page.hints)
        );
        let echoed = Left::ToastEcho(ToastKind::Success, "Deleted tag Food".to_string());
        assert_eq!(left(None, None, echo(), None), echoed);
        assert_eq!(left(None, None, echo(), Some(&page)), echoed);
    }

    #[test]
    fn a_status_line_message_and_the_command_echo_outrank_the_toast_echo() {
        let message = Some("not yet built".to_string());
        assert_eq!(
            left(None, message.clone(), echo(), None),
            Left::StatusMessage("not yet built".to_string())
        );
        assert_eq!(
            left(Some("acc"), message, echo(), None),
            Left::CommandEcho("acc")
        );
    }
}
