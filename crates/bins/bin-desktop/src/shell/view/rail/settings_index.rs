//! The Settings index rail (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state", "Settings
//! index rail" component): a list of the Settings pages under their THIS DEVICE and THIS LEDGER
//! headings, each entry swapping the body to its own page. Deliberately **not** a reuse of `shell::view::rail::context::ContextRail`'s "1c"
//! pattern -- that rail is a flat record list with no page-index mode, so this is a separate
//! mechanism (see issue #173's own ticket body).

use crate::navigation::nav::NounLabel;
use std::rc::Rc;

use gpui::{App, SharedString, Window, div, prelude::*, px};

use crate::{
    settings::{ScopeGroup, SettingsPage},
    theme::color,
};

/// Fixed column width: `docs/ux/desktop-mockups/16-settings/README.md`'s "Components" table.
pub const WIDTH: gpui::Pixels = px(214.0);

/// An index-entry click: swaps the body to that page; the entry takes the active dark treatment.
pub type OnEntryClick = Rc<dyn Fn(SettingsPage, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SettingsIndexRail {
    selected: SettingsPage,
    /// Whether keyboard focus is on the index rather than the page.
    focused: bool,
    on_click: OnEntryClick,
}

impl SettingsIndexRail {
    pub fn new(selected: SettingsPage, focused: bool, on_click: OnEntryClick) -> Self {
        Self {
            selected,
            focused,
            on_click,
        }
    }
}

impl RenderOnce for SettingsIndexRail {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let cx: &App = cx;
        div()
            .w(WIDTH)
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(color::background(cx))
            .border_r(px(2.0))
            .border_color(if self.focused {
                color::foreground(cx)
            } else {
                color::structural_rule(cx)
            })
            .child(rail_label(
                &lib_locale::format::upper(&crate::navigation::nav::Noun::Settings.label()),
                cx,
            ))
            .children(ScopeGroup::ALL.into_iter().flat_map(|group| {
                let on_click = self.on_click.clone();
                let selected = self.selected;
                std::iter::once(group_heading(group, cx).into_any_element()).chain(
                    group.pages().map(move |section| {
                        entry_row(section, section == selected, on_click.clone(), cx)
                            .into_any_element()
                    }),
                )
            }))
    }
}

/// A group's caps heading, **THIS DEVICE** or **THIS LEDGER**, which replaced the rail's footer
/// note on the Configuration and Preference split.
fn group_heading(group: ScopeGroup, cx: &App) -> impl IntoElement {
    div()
        .debug_selector(move || format!("settings-index-group-{group:?}"))
        .pt(px(12.0))
        .px(px(14.0))
        .pb(px(6.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(9.5))
        .text_color(color::faint_text(cx))
        .child(lib_locale::format::upper(&group.label()))
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
    section: SettingsPage,
    active: bool,
    on_click: OnEntryClick,
    cx: &App,
) -> impl IntoElement {
    div()
        .debug_selector(move || format!("settings-index-{section:?}"))
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
