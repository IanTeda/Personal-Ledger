//! The shell's status line (`docs/ux/desktop/Shell & Navigation/README.md`'s "1a" spec,
//! "Status line" component): mode badge, hint strip. Its own bottom-right breadcrumb (`ledger ·
//! <noun>`) was dropped -- the top bar's brand tile now names the active screen instead
//! (`crate::topbar::brand_mark`), and showing it in both places was a plain duplicate.

use std::rc::Rc;

use gpui::{App, Window, div, prelude::*, px};

use crate::{nav::InputMode, theme::color};

/// Band height: `docs/ux/desktop/Shell & Navigation/README.md`'s "Layout" table.
pub const HEIGHT: gpui::Pixels = px(28.0);

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
    /// the palette's live input (`docs/ux/desktop/Shell & Navigation/README.md`'s "1d" spec:
    /// "Status line in COMMAND mode: ... the live query echoes ...; right side reads 'esc close
    /// command window'"), or -- once `:open` is confirmed -- the "1e" file explorer's own
    /// frozen `"open"` echo and "esc close file explorer" hint, since `Shell` keeps `mode` at
    /// `Command` for as long as that dialog is open (see `Shell::run_command`'s own doc).
    command_echo: Option<(String, String)>,
    /// `Some` while a page with its own legend is showing (`docs/ux/desktop/Accounts/README.md`'s
    /// 3a status bar). Ignored in command mode, which owns the whole line.
    page: Option<PageStatus>,
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
        }
    }

    /// Makes the shell-wide hint strip's entries clickable.
    pub fn on_hint(mut self, on_hint: OnHint) -> Self {
        self.on_hint = Some(on_hint);
        self
    }

    pub fn page(mut self, page: Option<PageStatus>) -> Self {
        self.page = page;
        self
    }
}

impl RenderOnce for StatusLine {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .h(HEIGHT)
            .flex_none()
            .flex()
            .items_center()
            .gap(px(14.0))
            .px(px(12.0))
            .bg(color::CHROME)
            .border_t(px(2.0))
            .border_color(color::STRUCTURAL_RULE)
            .text_size(px(11.5))
            .text_color(color::INK_SECONDARY)
            .child(mode_badge(self.mode))
            .child(match (&self.command_echo, self.status_message) {
                (Some((query, _)), _) => command_query_echo(query).into_any_element(),
                (None, Some(message)) => div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_color(color::ACCENT_TEXT)
                    .child(message)
                    .into_any_element(),
                (None, None) => match &self.page {
                    Some(page) => page_hint_strip(&page.hints).into_any_element(),
                    None => hint_strip(self.on_hint.clone()).into_any_element(),
                },
            })
            .child(div().flex_1())
            .child(match (self.command_echo, self.page) {
                (Some((_, hint)), _) => div().child(hint).into_any_element(),
                (None, Some(page)) => div().child(page.right).into_any_element(),
                (None, None) => file_path().into_any_element(),
            })
    }
}

fn mode_badge(mode: InputMode) -> impl IntoElement {
    let (label, bg) = match mode {
        InputMode::Normal => (crate::msg::desktop_mode_normal(), color::INK),
        InputMode::Insert => (crate::msg::desktop_mode_insert(), color::INK),
        InputMode::Search => (crate::msg::desktop_mode_search(), color::INK),
        // The handoff's own COMMAND-mode callout: the badge fill becomes the accent.
        InputMode::Command => (crate::msg::desktop_mode_command(), color::ACCENT),
        // Same accent callout as `Command` -- a modal dialog is exactly as attention-grabbing.
        InputMode::Dialog => (crate::msg::desktop_mode_dialog(), color::ACCENT),
        // The same accent callout as the other modal surfaces.
        InputMode::Filter => (crate::msg::desktop_mode_filter(), color::ACCENT),
        InputMode::Help => (crate::msg::desktop_mode_help(), color::ACCENT),
    };

    div()
        .bg(bg)
        .text_color(color::INK_ON_DARK)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .py(px(2.0))
        .px(px(7.0))
        .child(lib_locale::format::upper(&label))
}

fn hint_strip(on_hint: Option<OnHint>) -> impl IntoElement {
    let entry = |id: &'static str, action: HintAction, key: &'static str, label: String| {
        let on_hint = on_hint.clone();
        div()
            .id(id)
            .flex()
            .items_center()
            .gap(px(4.0))
            .when(on_hint.is_some(), |this| {
                this.cursor_pointer()
                    .hover(|style| style.bg(color::HOVER_TINT))
            })
            .when_some(on_hint, |this, on_hint| {
                this.on_click(move |_event, window, cx| on_hint(action, window, cx))
            })
            .child(
                div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_color(color::INK)
                    .child(key),
            )
            .child(label)
    };
    let dot = || div().child("\u{b7}");

    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(entry(
            "hint-command",
            HintAction::Command,
            ":",
            crate::msg::desktop_hint_command(),
        ))
        .child(dot())
        .child(entry(
            "hint-search",
            HintAction::Search,
            "/",
            crate::msg::desktop_hint_search(),
        ))
        .child(dot())
        .child(entry(
            "hint-help",
            HintAction::Help,
            "?",
            crate::msg::desktop_hint_help(),
        ))
        .child(dot())
        .child(entry(
            "hint-rail",
            HintAction::ToggleRail,
            "b",
            crate::msg::desktop_hint_toggle_sidebar(),
        ))
}

/// A page's key legend: `j/k row · enter open ledger · ...`, each key at weight 800 like the
/// shell-wide hint strip's own.
fn page_hint_strip(hints: &[(&'static str, String)]) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .children(hints.iter().enumerate().flat_map(|(index, (key, action))| {
            let separator = (index > 0).then(|| div().child("\u{b7}").into_any_element());
            separator.into_iter().chain([
                div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_color(color::INK)
                    .child(*key)
                    .into_any_element(),
                div().child(action.clone()).into_any_element(),
            ])
        }))
}

/// The "1d" spec's COMMAND-mode echo: `:{query}` at weight 800, then the accent block caret --
/// the status line's own smaller echo of the palette's input row, kept in sync by `Shell`
/// passing the same `Palette::input()` string to both.
fn command_query_echo(query: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::INK)
        .child(format!(":{query}"))
        .child(div().w(px(2.0)).h(px(13.0)).bg(color::ACCENT))
}

/// The open Ledger's own file path, moved here from the top bar's brand tile (`crate::topbar`)
/// -- representative content, not a real path yet (lands with the sync/open-file tickets,
/// #164/#165).
fn file_path() -> impl IntoElement {
    div().child("~/Documents/My-Personal-Ledger.pldb · aud")
}
