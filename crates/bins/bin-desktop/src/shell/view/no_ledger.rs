//! The "1a" no Ledger open content: "No ledger open" centered in the main pane, `:open`/`:new`
//! named in the body copy (`docs/ux/desktop-mockups/01-shell/README.md`'s "Main pane" bullet). It
//! is the Shell's, drawn in its frame, since the Dashboard module exists only once a Ledger is open
//! (#975). `gpui` 0.2's `Styled` trait has no letter-spacing hook, so the title's `-.01em`
//! tracking from the spec has no equivalent here -- a real, not merely unverified, gap.
//!
//! Both command names are real click targets (issue #167), not just copy: a click goes through
//! [`PageClicks`] to `Root`, which runs the command exactly as the palette's `enter` would (#1024).

use gpui::{App, SharedString, div, prelude::*, px, relative};
use lib_dashboard::NoLedgerCommand;

use crate::shell::router::Root;
use crate::shell::view::page_clicks::PageClicks;
use crate::{msg, theme::color};

/// The content, its `:open` and `:new` sending their [`NoLedgerCommand`] through `clicks`.
pub fn render(clicks: &PageClicks<Root>, cx: &App) -> gpui::AnyElement {
    let command = |command: NoLedgerCommand, text: String| {
        let name = command.name();
        div()
            .id(SharedString::from(format!("no-ledger-{name}")))
            .debug_selector(move || format!("no-ledger-{name}"))
            .cursor_pointer()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::foreground(cx))
            .on_click(clicks.send(command))
            .child(text)
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(14.0))
        .p(px(24.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(26.0))
                .line_height(relative(1.1))
                .child(msg::desktop_no_ledger_title()),
        )
        .child(
            div()
                .max_w(px(380.0))
                .flex()
                .flex_wrap()
                .justify_center()
                .text_align(gpui::TextAlign::Center)
                .text_size(px(13.0))
                .text_color(color::muted(cx))
                .children(
                    msg::desktop_no_ledger_hint(":open", ":new")
                        .into_iter()
                        .map(|segment| match segment.tag.as_deref() {
                            Some("open") => {
                                command(NoLedgerCommand::Open, segment.text).into_any_element()
                            }
                            Some("new") => {
                                command(NoLedgerCommand::New, segment.text).into_any_element()
                            }
                            _ => div().child(segment.text).into_any_element(),
                        }),
                ),
        )
        .into_any_element()
}
