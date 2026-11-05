//! The Settings index rail (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state", "Settings
//! index rail" component): a list of the Settings pages under their THIS DEVICE and THIS LEDGER
//! headings, each entry swapping the body to its own page. Deliberately **not** a reuse of `shell::view::rail::context::ContextRail`'s "1c"
//! pattern -- that rail is a flat record list with no page-index mode, so this is a separate
//! mechanism (see issue #173's own ticket body).

use crate::navigation::nav::NounLabel;

use gpui::{App, SharedString, Window, div, prelude::*, px};
use lib_settings::{IndexGroup, IndexRow, SettingsFrameEvent};

use crate::{settings::state::SettingsView, shell::view::page_clicks::PageClicks, theme::color};

/// Fixed column width: `docs/ux/desktop-mockups/16-settings/README.md`'s "Components" table.
pub const WIDTH: gpui::Pixels = px(214.0);

/// The rail as the Settings frame's view model draws it: each group's rows, the page on show
/// selected and, with no Ledger open, the This Ledger rows muted. A row's click is the frame's
/// [`SettingsFrameEvent::ClickIndex`], which swaps the body to that page.
#[derive(IntoElement)]
pub struct SettingsIndexRail {
    groups: Vec<IndexGroup>,
    /// Whether keyboard focus is on the index rather than the page.
    focused: bool,
    clicks: PageClicks<SettingsView>,
}

impl SettingsIndexRail {
    pub fn new(groups: Vec<IndexGroup>, focused: bool, clicks: PageClicks<SettingsView>) -> Self {
        Self {
            groups,
            focused,
            clicks,
        }
    }
}

impl RenderOnce for SettingsIndexRail {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let cx: &App = cx;
        let clicks = self.clicks;
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
            .children(self.groups.iter().flat_map(|group| {
                std::iter::once(group_heading(group, cx).into_any_element())
                    .chain(
                        group
                            .rows
                            .iter()
                            .map(|row| entry_row(row, &clicks, cx).into_any_element()),
                    )
                    .collect::<Vec<_>>()
            }))
    }
}

/// A group's caps heading, **THIS DEVICE** or **THIS LEDGER**, which replaced the rail's footer
/// note on the Configuration and Preference split.
fn group_heading(group: &IndexGroup, cx: &App) -> impl IntoElement {
    let scope = group.group;
    div()
        .debug_selector(move || format!("settings-index-group-{scope:?}"))
        .pt(px(12.0))
        .px(px(14.0))
        .pb(px(6.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(9.5))
        .text_color(color::faint_text(cx))
        .child(lib_locale::format::upper(&group.label))
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

/// One page's entry; muted for a This Ledger page with no Ledger open, which a click refuses.
fn entry_row(row: &IndexRow, clicks: &PageClicks<SettingsView>, cx: &App) -> impl IntoElement {
    let section = row.page;
    div()
        .debug_selector(move || format!("settings-index-{section:?}"))
        .id(SharedString::from(format!("settings-index-{section:?}")))
        .cursor_pointer()
        .py(px(8.0))
        .px(px(14.0))
        .when(row.muted, |this| this.text_color(color::faint_text(cx)))
        .when(row.selected, |this| {
            this.bg(color::selection_background(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
        })
        .on_click(clicks.send(SettingsFrameEvent::ClickIndex(section)))
        .child(row.label.clone())
}
