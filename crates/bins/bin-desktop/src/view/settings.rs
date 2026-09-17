//! The Settings body (`docs/ux/desktop/Settings/README.md`'s "2a resting state", "Body"): one
//! continuous scroll, not a pane switcher -- every section stays mounted, and the settings index
//! rail (`rail::settings_index`) scrolls to a heading rather than swapping views. This ticket
//! (issue #173) only lays out the frame: each section's real content is a placeholder until its
//! own ticket lands (`crate::settings::SettingsSection::placeholder_issue`).
//!
//! Direct children of the scrollable container, in order: the page heading block (child `0`),
//! then each of the nine sections (children `1..=9`) -- `SettingsSection::body_child_index`
//! documents this offset, since `gpui::ScrollHandle::scroll_to_top_of_item` addresses direct
//! children by index.

use gpui::{ScrollHandle, div, prelude::*, px};

use crate::{settings::SettingsSection, theme::color};

pub fn render(focused: bool, scroll_handle: &ScrollHandle) -> gpui::AnyElement {
    div()
        .id("settings-body")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::INK)
        })
        .py(px(22.0))
        .px(px(28.0))
        .flex()
        .flex_col()
        .child(page_heading())
        .children(SettingsSection::ALL.into_iter().map(section_block))
        .into_any_element()
}

fn page_heading() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(28.0))
                        .text_color(color::INK)
                        .child("Settings"),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::INK_TERTIARY)
                        .child("preferences · synced"),
                ),
        )
        .child(
            div()
                .h(px(2.0))
                .bg(color::STRUCTURAL_RULE)
                .mt(px(14.0))
                .mb(px(18.0)),
        )
}

/// One section wrapper: heading + right-aligned scope note, a 2px rule, placeholder content,
/// then a **48px** bottom gap -- every section, no exceptions (README's implementation note 4:
/// mixed top/bottom margin ownership is how this gap goes missing, so it's carried on a single
/// edge, here).
fn section_block(section: SettingsSection) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .mb(px(48.0))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(20.0))
                        .text_color(color::INK)
                        .child(section.label()),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::INK_TERTIARY)
                        .child(section.scope_note()),
                ),
        )
        .child(
            div()
                .h(px(2.0))
                .bg(color::STRUCTURAL_RULE)
                .mt(px(14.0))
                .mb(px(18.0)),
        )
        .child(div().text_color(color::INK_TERTIARY).child(format!(
            "{} -- not yet built (see issue #{})",
            section.label(),
            section.placeholder_issue()
        )))
}
