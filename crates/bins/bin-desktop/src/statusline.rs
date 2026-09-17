//! The shell's status line (`docs/ux/desktop/Shell & Navigation/README.md`'s "1a" spec,
//! "Status line" component): mode badge, hint strip, breadcrumb.

use gpui::{App, Window, div, prelude::*, px};

use crate::{
    nav::{InputMode, Noun},
    theme::color,
};

/// Band height: `docs/ux/desktop/Shell & Navigation/README.md`'s "Layout" table.
pub const HEIGHT: gpui::Pixels = px(28.0);

#[derive(IntoElement)]
pub struct StatusLine {
    mode: InputMode,
    noun: Noun,
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
    command_echo: Option<(String, &'static str)>,
}

impl StatusLine {
    pub fn new(
        mode: InputMode,
        noun: Noun,
        status_message: Option<String>,
        command_echo: Option<(String, &'static str)>,
    ) -> Self {
        Self {
            mode,
            noun,
            status_message,
            command_echo,
        }
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
                (None, None) => hint_strip().into_any_element(),
            })
            .child(div().flex_1())
            .child(match self.command_echo {
                Some((_, hint)) => div().child(hint).into_any_element(),
                None => breadcrumb(self.noun).into_any_element(),
            })
    }
}

fn mode_badge(mode: InputMode) -> impl IntoElement {
    let (label, bg) = match mode {
        InputMode::Normal => ("NORMAL", color::INK),
        InputMode::Insert => ("INSERT", color::INK),
        InputMode::Search => ("SEARCH", color::INK),
        // The handoff's own COMMAND-mode callout: the badge fill becomes the accent.
        InputMode::Command => ("COMMAND", color::ACCENT),
    };

    div()
        .bg(bg)
        .text_color(color::INK_ON_DARK)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .py(px(2.0))
        .px(px(7.0))
        .child(label)
}

fn hint_strip() -> impl IntoElement {
    let key = |text: &'static str| {
        div()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::INK)
            .child(text)
    };

    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(key(":"))
        .child("command ·")
        .child(key("/"))
        .child("search ·")
        .child(key("?"))
        .child("help ·")
        .child(key("b"))
        .child("toggle sidebar")
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

fn breadcrumb(noun: Noun) -> impl IntoElement {
    let noun_label = match noun {
        Noun::Dashboard => "dashboard",
        Noun::Transactions => "transactions",
        Noun::Accounts => "accounts",
        Noun::Categories => "categories",
        Noun::Payees => "payees",
        Noun::Tags => "tags",
        Noun::Bills => "bills",
        Noun::Budgets => "budgets",
        Noun::Reports => "reports",
        Noun::Settings => "settings",
    };

    div().child(format!("ledger · {noun_label}"))
}
