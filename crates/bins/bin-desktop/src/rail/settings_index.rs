//! The Settings index rail (`docs/ux/desktop/Settings/README.md`'s "2a resting state", "Settings
//! index rail" component): a list of the Settings pages, each entry swapping the
//! body to its own page. Deliberately **not** a reuse of `rail::context::ContextRail`'s "1c"
//! pattern -- that rail is a flat record list with no page-index mode, so this is a separate
//! mechanism (see issue #173's own ticket body).

use std::rc::Rc;

use gpui::{App, SharedString, Window, div, prelude::*, px};

use crate::{settings::SettingsSection, theme::color};

/// Fixed column width: `docs/ux/desktop/Settings/README.md`'s "Components" table.
pub const WIDTH: gpui::Pixels = px(214.0);

/// An index-entry click: swaps the body to that page; the entry takes the active dark treatment.
pub type OnEntryClick = Rc<dyn Fn(SettingsSection, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SettingsIndexRail {
    selected: SettingsSection,
    /// Whether keyboard focus is on the index rather than the page.
    focused: bool,
    on_click: OnEntryClick,
}

impl SettingsIndexRail {
    pub fn new(selected: SettingsSection, focused: bool, on_click: OnEntryClick) -> Self {
        Self {
            selected,
            focused,
            on_click,
        }
    }
}

impl RenderOnce for SettingsIndexRail {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w(WIDTH)
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(color::background(cx))
            .border_r(px(2.0))
            .border_color(color::structural_rule(cx))
            .when(self.focused, |this| {
                this.border_l(px(2.0)).border_color(color::foreground(cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .child(rail_label(
                        &lib_locale::format::upper(&crate::nav::Noun::Settings.label()),
                        cx,
                    ))
                    .children(SettingsSection::ALL.into_iter().map(|section| {
                        entry_row(section, section == self.selected, self.on_click.clone(), cx)
                    })),
            )
            .child(div().h(px(1.0)).bg(color::hairline(cx)))
            .child(footer(cx))
    }
}

fn rail_label(label: &str, cx: &App) -> impl IntoElement {
    div()
        .pt(px(10.0))
        .px(px(12.0))
        .pb(px(4.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::faint_text(cx))
        .child(label.to_string())
}

fn entry_row(
    section: SettingsSection,
    active: bool,
    on_click: OnEntryClick,
    cx: &App,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("settings-index-{section:?}")))
        .cursor_pointer()
        .py(px(8.0))
        .px(px(14.0))
        .when(active, |this| {
            this.bg(color::selection_background(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
        })
        .on_click(move |_event, window, cx| on_click(section, window, cx))
        .child(section.label())
}

/// The rail's own footer note, above the 1px rule this sits below (README's "Layout" bullet:
/// "Footer note, above a 1px rule").
fn footer(cx: &App) -> impl IntoElement {
    div()
        .py(px(10.0))
        .px(px(12.0))
        .text_size(px(11.5))
        .text_color(color::faint_text(cx))
        .child(crate::msg::desktop_settings_index_footer())
}
