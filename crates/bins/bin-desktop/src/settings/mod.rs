//! The Settings body (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state",
//! "Body"): the Settings index, Settings' resting view at every Window Size, or one page at a time,
//! not a continuous scroll. At Large and X-Large the index is also docked beside it
//! (`shell::view::rail::settings_index`), at X-Large the index on show docks a detail pane
//! following its cursor ([`pane`]), and only the selected page is mounted. Each page keeps the same
//! anatomy: one heading ("Settings - <page>") with its right-aligned meta over a 2px rule, then the
//! page's content.
//!
//! The frame (which page shows, whether the keyboard is on the index, in a page or on the docked
//! index beside it, the Medium drill-down and the page picker) is `lib-settings`' Presenter, held
//! by [`state::SettingsView`] and drawn here from its view model, its clicks going through
//! [`PageClicks`]. What stays the Desktop's of a page's identity is its icon ([`icon`]) and which
//! page shows a revealed record ([`for_reveal`]).

pub mod about;
pub(crate) mod controls;
pub mod data_backup;
pub mod device;
pub mod display;
pub mod general;
pub(crate) mod hints;
pub mod ledger;
pub mod pages;
mod pane;
pub mod router;
pub mod snapshot;
pub mod state;
pub mod sync_server;
pub mod tracing;

use crate::navigation::nav::NounLabel;

use gpui::{AnyElement, App, Context, Pixels, SharedString, Window, div, prelude::*, px};

use gpui_component::{
    Sizable,
    scroll::{Scrollbar, ScrollbarShow},
};
use lib_settings::{Breadcrumb, IndexGroup, IndexRow, PickerView, ScopeFacts, SettingsFrameEvent};

use crate::{
    assets::icon::DesktopIcon,
    navigation::nav::Noun,
    shell::view::{page_clicks::PageClicks, rail::settings_index::SettingsIndexRail},
    theme::color,
    view::event::Reveal,
};

use state::SettingsView;

pub use lib_settings::{ScopeGroup, SettingsFocus, SettingsPage};

/// The widest a page's content draws, at every Window Size; the View past it stays empty
/// (DESKTOP-107).
pub const CONTENT_MAX_WIDTH: Pixels = px(840.0);

/// The page picker's own width, under the page name.
const PICKER_WIDTH: Pixels = px(280.0);

/// Where the index page's groups start among its scrolled children, after the heading and
/// its rule.
const INDEX_PAGE_FIRST_GROUP: usize = 2;

/// Where `page`'s row sits among the index page's scrolled children, so the selected row
/// can be scrolled to: past the heading and its rule, the rows before it and one heading per group
/// up to its own.
fn index_page_child(page: SettingsPage) -> usize {
    let groups = ScopeGroup::ALL
        .iter()
        .position(|group| *group == page.scope_group())
        .unwrap_or(0)
        + 1;
    INDEX_PAGE_FIRST_GROUP + groups + page.index()
}

/// The page's scope note, right of its heading and on the status line. The Tracing note counts the
/// entries the log capture keeps, which is `lib-tracing`'s to say.
pub fn scope_note(page: SettingsPage) -> String {
    page.scope_note(ScopeFacts {
        log_capacity: lib_tracing::LOG_CAPACITY,
    })
}

/// The icon beside the page's name on the index page (DESKTOP-119). A page that was a
/// rail noun keeps that noun's icon, so it reads the same wherever it shows; Institutions
/// shares Loans' bank.
pub fn icon(page: SettingsPage) -> DesktopIcon {
    match page {
        SettingsPage::General => DesktopIcon::SettingsGeneral,
        SettingsPage::Display => DesktopIcon::SettingsDisplay,
        SettingsPage::Ledger => DesktopIcon::SettingsLedger,
        SettingsPage::Units => DesktopIcon::SettingsUnits,
        SettingsPage::Institutions => DesktopIcon::Loans,
        SettingsPage::Accounts => DesktopIcon::Accounts,
        SettingsPage::Categories => DesktopIcon::Categories,
        SettingsPage::Tags => DesktopIcon::Tags,
        SettingsPage::Payees => DesktopIcon::Payees,
        SettingsPage::Documents => DesktopIcon::Documents,
        SettingsPage::Inventory => DesktopIcon::Inventory,
        SettingsPage::SyncServer => DesktopIcon::SettingsSyncServer,
        SettingsPage::DataBackup => DesktopIcon::SettingsDataBackup,
        SettingsPage::Tracing => DesktopIcon::SettingsTracing,
        SettingsPage::About => DesktopIcon::SettingsAbout,
    }
}

/// The page that owns a revealed record, or `None` for a record another View owns. Which page
/// shows a record is a Router decision, and `Reveal` is the Desktop's routing vocabulary.
pub fn for_reveal(reveal: &Reveal) -> Option<SettingsPage> {
    match reveal {
        Reveal::Account(_) => Some(SettingsPage::Accounts),
        Reveal::Payee(_) => Some(SettingsPage::Payees),
        Reveal::BillEntry(_) | Reveal::BillPlan(_) | Reveal::BillsPeriod(_) => None,
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let model = self.view_model();
        let clicks = PageClicks::new(cx);
        let page_focused = self.zone_focused && model.focus == SettingsFocus::Page;
        let drill_down = model.drill_down;
        let docked =
            (!drill_down).then(|| SettingsIndexRail::new(model.groups.clone(), clicks.clone()));
        // The index is Settings' resting view, in the page's place at every Window Size: alone at
        // Medium (DESKTOP-119), beside the docked index at Large and X-Large (DESKTOP-181, 182).
        if model.index_page {
            let index = div()
                .relative()
                .flex_1()
                .min_w(px(0.0))
                .h_full()
                .child(
                    div()
                        .id("settings-index-scroll")
                        .debug_selector(|| "settings-index-page".to_string())
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(&self.index_scroll)
                        .pt(px(22.0))
                        .pb(px(70.0))
                        .px(px(28.0))
                        .flex()
                        .flex_col()
                        .children(index_page(&model.groups, &model.index_count, &clicks, cx)),
                )
                .child(
                    div()
                        .debug_selector(|| "settings-index-scrollbar".to_string())
                        .occlude()
                        .absolute()
                        .top_0()
                        .right_0()
                        .bottom_0()
                        .w(px(12.0))
                        .child(
                            Scrollbar::vertical(&self.index_scroll)
                                .scrollbar_show(ScrollbarShow::Always),
                        ),
                );
            // At X-Large a detail pane follows the cursor at the View's right edge (DESKTOP-182).
            let pane = self
                .pane_summary(cx)
                .map(|summary| pane::index_pane(&summary, &clicks, cx));
            return div()
                .id("settings")
                .flex_1()
                .min_w(px(0.0))
                .h_full()
                .flex()
                .children(docked)
                .child(index)
                .children(pane)
                .into_any_element();
        }
        self.sync_frame(cx);
        let page = self.selected_page();
        page.set_page_focused(page_focused, cx);
        let meta = page.heading_meta(cx);
        let content = page.view().into_any_element();
        let pane = page.side_pane(cx);
        // Tracing's log box fills the page and scrolls itself, so its body must not scroll too.
        let fills = page.fills_page();
        let full_width = page.full_width();
        let body = div()
            .id("settings-body")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .when(fills, |this| this.overflow_hidden())
            .when(!fills, |this| {
                this.overflow_y_scroll().track_scroll(&self.scroll_handle)
            })
            // The focus rule tells the index from the page; at Medium only one of them shows.
            .when(page_focused && !drill_down, |this| {
                this.border_l(px(2.0)).border_color(color::foreground(cx))
            })
            .py(px(22.0))
            .px(px(28.0))
            .flex()
            .flex_col();
        let heading = match &model.breadcrumb {
            Some(crumb) => {
                crumb_heading(crumb, model.picker.as_ref(), &clicks, cx).into_any_element()
            }
            None => plain_heading(&model.heading, cx).into_any_element(),
        };
        let body = body.child(section_block(heading, meta, content, fills, full_width, cx));
        // A page can dock a pane at the View's right edge (Units' detail pane at X-Large).
        div()
            .id("settings")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .children(docked)
            .child(body)
            .children(pane)
            .into_any_element()
    }
}

/// One page: heading + right-aligned scope note, a 2px rule, the page's content, then a **48px**
/// bottom gap -- every page, no exceptions (README's implementation note 4: mixed top/bottom
/// margin ownership is how this gap goes missing, so it's carried on a single edge, here). The
/// whole block caps at [`CONTENT_MAX_WIDTH`], except a [`SettingsPageView::FULL_WIDTH`] page
/// (Units, whose tables fill the View's width down to their 610px minimum). A
/// [`SettingsPageView::FILLS_PAGE`] page fills the body's height.
///
/// [`SettingsPageView::FULL_WIDTH`]: crate::shell::view::module_page::SettingsPageView::FULL_WIDTH
/// [`SettingsPageView::FILLS_PAGE`]: crate::shell::view::module_page::SettingsPageView::FILLS_PAGE
fn section_block(
    heading: AnyElement,
    meta: AnyElement,
    content: AnyElement,
    fills: bool,
    full_width: bool,
    cx: &App,
) -> impl IntoElement {
    div()
        .debug_selector(|| "settings-page".to_string())
        .w_full()
        .when(!full_width, |this| this.max_w(CONTENT_MAX_WIDTH))
        .flex()
        .flex_col()
        .when(fills, |this| this.flex_1().min_h(px(0.0)))
        .mb(px(48.0))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap(px(14.0))
                .child(heading)
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::faint_text(cx))
                        .child(meta),
                ),
        )
        .child(
            div()
                .h(px(2.0))
                .bg(color::structural_rule(cx))
                .mt(px(14.0))
                .mb(px(18.0)),
        )
        .child(content)
}

/// The page heading at Large and X-Large, beside the docked index: `Settings - Display`.
fn plain_heading(heading: &str, cx: &App) -> impl IntoElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(28.0))
        .text_color(color::foreground(cx))
        .child(heading.to_string())
}

/// The page heading at Medium, a breadcrumb that is the only trail on screen (DESKTOP-120):
/// **Settings** goes back to the index, and the caret after the page name opens the picker under it.
fn crumb_heading(
    crumb: &Breadcrumb,
    picker: Option<&PickerView>,
    clicks: &PageClicks<SettingsView>,
    cx: &App,
) -> impl IntoElement {
    div()
        .relative()
        .flex()
        .items_baseline()
        .gap(px(10.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(28.0))
        .text_color(color::foreground(cx))
        .child(
            div()
                .id("settings-crumb-index")
                .debug_selector(|| "settings-crumb-index".to_string())
                .cursor_pointer()
                .text_color(color::muted(cx))
                .hover(|this| this.text_color(color::foreground(cx)))
                .on_click(clicks.send(SettingsFrameEvent::ClickCrumb))
                .child(crumb.settings.clone()),
        )
        .child(div().text_color(color::faint_text(cx)).child("\u{203a}"))
        .child(
            div()
                .id("settings-crumb-picker")
                .debug_selector(|| "settings-crumb-picker".to_string())
                .cursor_pointer()
                .flex()
                .items_baseline()
                .gap(px(8.0))
                .on_click(clicks.send(SettingsFrameEvent::ClickCaret))
                .child(crumb.page.clone())
                .child(
                    div()
                        .text_size(px(16.0))
                        .text_color(color::muted(cx))
                        .child("\u{25be}"),
                ),
        )
        // Deferred so it paints after the page below it; otherwise the page draws over the menu.
        .children(picker.map(|picker| gpui::deferred(page_picker(picker, clicks, cx))))
}

/// The picker under the page name: every page, in index order under its group's heading, with the
/// chosen one highlighted.
fn page_picker(picker: &PickerView, clicks: &PageClicks<SettingsView>, cx: &App) -> AnyElement {
    div()
        .absolute()
        .top_full()
        .left_0()
        .mt(px(6.0))
        .w(PICKER_WIDTH)
        .flex()
        .flex_col()
        .bg(color::background(cx))
        .border_2()
        .border_color(color::foreground(cx))
        .font_weight(gpui::FontWeight::NORMAL)
        .text_size(px(13.0))
        .occlude()
        .children(picker.groups.iter().flat_map(|group| {
            std::iter::once(
                div()
                    .pt(px(10.0))
                    .pb(px(4.0))
                    .px(px(12.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_size(px(9.5))
                    .text_color(color::faint_text(cx))
                    .child(lib_locale::format::upper(&group.label))
                    .into_any_element(),
            )
            .chain(group.rows.iter().map(|row| {
                let page = row.page;
                div()
                    .id(SharedString::from(format!("settings-picker-{page:?}")))
                    .debug_selector(move || format!("settings-picker-{page:?}"))
                    .cursor_pointer()
                    .py(px(7.0))
                    .px(px(12.0))
                    .when(row.muted, |this| this.text_color(color::faint_text(cx)))
                    .when(row.chosen, |this| {
                        this.bg(color::selection_background(cx))
                            .text_color(color::selection_text(cx))
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    })
                    .hover(|this| this.bg(color::hover(cx)))
                    .on_click(clicks.send(SettingsFrameEvent::ClickPicker(page)))
                    .child(row.label.clone())
                    .into_any_element()
            }))
            .collect::<Vec<_>>()
        }))
        .into_any_element()
}

/// The Settings index as a page of its own, at every Window Size (DESKTOP-119, 181, 182): a plain
/// **Settings** heading, then each group's heading, with its scope line and what it holds, over
/// one row per page with its one-line description. The row under the cursor is highlighted. Each
/// is a child of the scroll on its own ([`index_page_child`]), so a row can be scrolled to.
fn index_page(
    groups: &[IndexGroup],
    index_count: &str,
    clicks: &PageClicks<SettingsView>,
    cx: &App,
) -> Vec<AnyElement> {
    let heading = div()
        .flex()
        .items_baseline()
        .justify_between()
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(28.0))
                .text_color(color::foreground(cx))
                .child(Noun::Settings.label()),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child(index_count.to_string()),
        );
    let rule = div()
        .h(px(2.0))
        .bg(color::structural_rule(cx))
        .mt(px(14.0))
        .mb(px(6.0));
    let row = |row: &IndexRow| {
        let page = row.page;
        let active = row.cursor;
        div()
            .id(SharedString::from(format!("settings-index-page-{page:?}")))
            .debug_selector(move || format!("settings-index-page-{page:?}"))
            .cursor_pointer()
            .flex()
            .items_center()
            .gap(px(14.0))
            .py(px(10.0))
            .px(px(12.0))
            .border_b_1()
            .border_color(color::hairline(cx))
            .when(active, |this| {
                this.bg(color::selection_background(cx))
                    .text_color(color::selection_text(cx))
            })
            .when(!active, |this| this.hover(|this| this.bg(color::hover(cx))))
            .when(row.muted, |this| this.text_color(color::faint_text(cx)))
            .on_click(clicks.send(SettingsFrameEvent::ClickIndexPage(page)))
            .child(
                div()
                    .debug_selector(move || format!("settings-index-page-{page:?}-icon"))
                    .flex_none()
                    .child(icon(page).icon().with_size(px(16.0)).text_color(if active {
                        color::selection_text(cx)
                    } else {
                        color::foreground(cx)
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .child(row.label.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .when(!active, |this| this.text_color(color::muted(cx)))
                            .truncate()
                            .child(row.description.clone()),
                    ),
            )
            .child("\u{203a}")
            .into_any_element()
    };
    let groups = groups.iter().flat_map(|group| {
        std::iter::once(group_heading(group, cx))
            .chain(group.rows.iter().map(row))
            .collect::<Vec<_>>()
    });
    [heading.into_any_element(), rule.into_any_element()]
        .into_iter()
        .chain(groups)
        .map(|child| {
            div()
                .w_full()
                .max_w(CONTENT_MAX_WIDTH)
                .flex_none()
                .child(child)
                .into_any_element()
        })
        .collect()
}

/// A group's heading on the index page: its name with the page count and sync scope at the
/// right, and a line saying where its settings live.
fn group_heading(group: &IndexGroup, cx: &App) -> AnyElement {
    let scope = group.group;
    div()
        .debug_selector(move || format!("settings-index-page-group-{scope:?}"))
        .pt(px(18.0))
        .pb(px(8.0))
        .border_b_1()
        .border_color(color::hairline(cx))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(10.0))
                        .text_color(color::faint_text(cx))
                        .child(lib_locale::format::upper(&group.label)),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::faint_text(cx))
                        .child(group.scope_line.clone()),
                ),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(group.description.clone()),
        )
        .into_any_element()
}

/// A page that isn't rebuilt yet.
fn placeholder(cx: &App) -> AnyElement {
    div()
        .text_color(color::faint_text(cx))
        .child(crate::msg::desktop_settings_page_placeholder())
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_revealed_account_or_payee_opens_its_page_and_a_bills_record_opens_none() {
        assert_eq!(
            for_reveal(&Reveal::Account(lib_core::AccountId::new())),
            Some(SettingsPage::Accounts)
        );
        assert_eq!(
            for_reveal(&Reveal::Payee(lib_core::PayeeId::new())),
            Some(SettingsPage::Payees)
        );
        assert_eq!(
            for_reveal(&Reveal::BillPlan(lib_core::BillPlanId::new())),
            None
        );
    }
}
