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
}

impl StatusLine {
    pub fn new(mode: InputMode, noun: Noun) -> Self {
        Self { mode, noun }
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
            .child(hint_strip())
            .child(div().flex_1())
            .child(breadcrumb(self.noun))
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
        .child(key("a"))
        .child("add txn ·")
        .child(key("?"))
        .child("help ·")
        .child(key("b"))
        .child("toggle sidebar")
}

fn breadcrumb(noun: Noun) -> impl IntoElement {
    let noun_label = match noun {
        Noun::Dashboard => "dashboard",
        Noun::Transactions => "transactions",
        Noun::Accounts => "accounts",
        Noun::Reconcile => "reconcile",
        Noun::Budgets => "budgets",
        Noun::Reports => "reports",
        Noun::Categories => "categories",
        Noun::Payees => "payees",
        Noun::Units => "units",
        Noun::Settings => "settings",
    };

    div().child(format!("ledger · {noun_label}"))
}
