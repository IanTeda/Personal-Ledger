//! The Settings index rail (`docs/ux/desktop/Settings/README.md`'s "2a resting state", "Settings
//! index rail" component): a live-filtered table of contents for the Settings body's own
//! continuous scroll. Deliberately **not** a reuse of `rail::context::ContextRail`'s "1c"
//! pattern -- that rail is a flat record list with no scroll-to-section/index mode, so this is a
//! new mechanism (see issue #173's own ticket body).

use std::rc::Rc;

use gpui::{App, SharedString, Window, div, prelude::*, px};

use crate::{settings::SettingsSection, theme::color};

/// Fixed column width: `docs/ux/desktop/Settings/README.md`'s "Components" table.
pub const WIDTH: gpui::Pixels = px(214.0);

/// An index-entry click (`docs/ux/desktop/Settings/README.md`'s "Navigation" bullet: "Index
/// entry click -> scroll the body to that section's heading; the entry takes the active dark
/// treatment").
pub type OnEntryClick = Rc<dyn Fn(SettingsSection, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SettingsIndexRail {
    selected: SettingsSection,
    /// The live `/ filter` query -- filters entries only, the body's own sections stay mounted
    /// regardless (README's "Navigation" bullet).
    filter: String,
    on_click: OnEntryClick,
}

impl SettingsIndexRail {
    pub fn new(selected: SettingsSection, filter: String, on_click: OnEntryClick) -> Self {
        Self {
            selected,
            filter,
            on_click,
        }
    }
}

impl RenderOnce for SettingsIndexRail {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .w(WIDTH)
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(color::GROUND)
            .border_r(px(2.0))
            .border_color(color::STRUCTURAL_RULE)
            .child(filter_box(&self.filter))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .child(rail_label("SETTINGS"))
                    .children(
                        SettingsSection::ALL
                            .into_iter()
                            .filter(|section| section.matches_filter(&self.filter))
                            .map(|section| {
                                entry_row(section, section == self.selected, self.on_click.clone())
                            }),
                    ),
            )
            .child(div().h(px(1.0)).bg(color::HAIRLINE))
            .child(footer())
    }
}

/// `/ filter` box: `height:30px; min-height:30px; font-size:12.5px`, wrapper padding
/// `22px 12px 25.4px` closed by a 2px rule -- tuned so that rule aligns with the one under the
/// body's own "Settings" page heading (`docs/ux/desktop/Settings/README.md`'s "Layout" bullet
/// and implementation note 5).
fn filter_box(filter: &str) -> impl IntoElement {
    div()
        .pt(px(22.0))
        .px(px(12.0))
        .pb(px(25.4))
        .border_b(px(2.0))
        .border_color(color::STRUCTURAL_RULE)
        .child(
            div()
                .h(px(30.0))
                .min_h(px(30.0))
                .flex()
                .items_center()
                .px(px(10.0))
                .border_1()
                .border_color(color::STRUCTURAL_RULE)
                .text_size(px(12.5))
                .child(div().text_color(color::INK_TERTIARY).child("/ "))
                .child(if filter.is_empty() {
                    div()
                        .text_color(color::INK_TERTIARY)
                        .child("filter")
                        .into_any_element()
                } else {
                    div()
                        .text_color(color::INK)
                        .child(filter.to_string())
                        .into_any_element()
                }),
        )
}

fn rail_label(label: &str) -> impl IntoElement {
    div()
        .pt(px(10.0))
        .px(px(12.0))
        .pb(px(4.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_TERTIARY)
        .child(label.to_string())
}

fn entry_row(section: SettingsSection, active: bool, on_click: OnEntryClick) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("settings-index-{section:?}")))
        .cursor_pointer()
        .py(px(8.0))
        .px(px(14.0))
        .when(active, |this| {
            this.bg(color::INK)
                .text_color(color::INK_ON_DARK)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
        })
        .on_click(move |_event, window, cx| on_click(section, window, cx))
        .child(section.label())
}

/// The rail's own footer note, above the 1px rule this sits below (README's "Layout" bullet:
/// "Footer note, above a 1px rule").
fn footer() -> impl IntoElement {
    div()
        .py(px(10.0))
        .px(px(12.0))
        .text_size(px(11.5))
        .text_color(color::INK_TERTIARY)
        .child("preferences sync \u{b7} configuration local")
}
