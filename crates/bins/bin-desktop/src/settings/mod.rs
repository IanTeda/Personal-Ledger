//! The Settings body (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state", "Body"): one page
//! at a time, not a continuous scroll. The settings index rail (`shell::view::rail::settings_index`) swaps the
//! page, and only the selected page is mounted. Each page keeps the same anatomy: one heading
//! ("Settings - <page>") with its right-aligned meta over a 2px rule, then the page's content.

pub mod about;
pub mod data_backup;
pub mod display;
pub mod general;
pub(crate) mod hints;
pub mod ledger;
pub mod page;
pub mod pages;
pub mod picker;
pub mod snapshot;
pub mod state;
pub mod sync_server;
pub mod tracing;

use crate::navigation::nav::NounLabel;
use std::rc::Rc;

use gpui::{AnyElement, App, Context, Pixels, SharedString, Window, div, prelude::*, px};

use gpui_component::{
    Sizable,
    scroll::{Scrollbar, ScrollbarShow},
};

use crate::{
    navigation::nav::Noun, shell::view::rail::settings_index::SettingsIndexRail, theme::color,
};

use picker::PagePicker;
use state::SettingsView;

/// The widest a page's content draws, at every Window Size; the View past it stays empty
/// (DESKTOP-107).
pub const CONTENT_MAX_WIDTH: Pixels = px(840.0);

/// The page picker's own width, under the page name.
const PICKER_WIDTH: Pixels = px(280.0);

/// Where the Medium index page's groups start among its scrolled children, after the heading and
/// its rule.
const INDEX_PAGE_FIRST_GROUP: usize = 2;

/// Where `page`'s row sits among the Medium index page's scrolled children, so the selected row
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

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;
type OnPageClick = Rc<dyn Fn(SettingsPage, &mut Window, &mut App)>;

pub use page::{ScopeGroup, SettingsPage};

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let selected = self.selected_section();
        let index_focused = self.zone_focused && self.focus() == SettingsFocus::Index;
        let page_focused = self.zone_focused && self.focus() == SettingsFocus::Page;
        let on_index_click = {
            let entity = entity.clone();
            Rc::new(move |section, _window: &mut Window, cx: &mut App| {
                entity.update(cx, |view, cx| view.click_index(section, cx));
            })
        };
        let drill_down = self.drill_down();
        // At Medium the index is a page of its own, in the page's place (DESKTOP-119).
        if drill_down && self.focus() == SettingsFocus::Index {
            let on_row_click: OnPageClick = {
                let entity = entity.clone();
                Rc::new(move |section, _window: &mut Window, cx: &mut App| {
                    entity.update(cx, |view, cx| view.click_index_page(section, cx));
                })
            };
            return div()
                .id("settings")
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
                        .children(index_page(selected, self.ledger_open(), on_row_click, cx)),
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
                )
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
        let heading = if drill_down {
            let on_crumb: OnClick = {
                let entity = entity.clone();
                Rc::new(move |_window: &mut Window, cx: &mut App| {
                    entity.update(cx, |view, cx| view.click_index_crumb(cx));
                })
            };
            let on_caret: OnClick = {
                let entity = entity.clone();
                Rc::new(move |_window: &mut Window, cx: &mut App| {
                    entity.update(cx, |view, cx| view.click_picker_caret(cx));
                })
            };
            let on_pick: OnPageClick = {
                let entity = entity.clone();
                Rc::new(move |section, _window: &mut Window, cx: &mut App| {
                    entity.update(cx, |view, cx| view.click_picker_page(section, cx));
                })
            };
            crumb_heading(
                selected,
                self.picker(),
                self.ledger_open(),
                on_crumb,
                on_caret,
                on_pick,
                cx,
            )
            .into_any_element()
        } else {
            plain_heading(selected, cx).into_any_element()
        };
        let body = body.child(section_block(heading, meta, content, fills, full_width, cx));
        // A page can dock a pane at the View's right edge (Units' detail pane at X-Large).
        div()
            .id("settings")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .when(!drill_down, |this| {
                this.child(SettingsIndexRail::new(
                    selected,
                    index_focused,
                    self.ledger_open(),
                    on_index_click,
                ))
            })
            .child(body)
            .children(pane)
            .into_any_element()
    }
}

/// One page: heading + right-aligned scope note, a 2px rule, the page's content, then a **48px**
/// bottom gap -- every page, no exceptions (README's implementation note 4: mixed top/bottom
/// margin ownership is how this gap goes missing, so it's carried on a single edge, here). The
/// whole block caps at [`CONTENT_MAX_WIDTH`], except a [`pages::Page::FULL_WIDTH`] page (Units, whose
/// tables fill the View's width down to their 610px minimum). A [`pages::Page::FILLS_PAGE`] page
/// fills the body's height.
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
fn plain_heading(section: SettingsPage, cx: &App) -> impl IntoElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(28.0))
        .text_color(color::foreground(cx))
        .child(crate::msg::desktop_settings_page_heading(
            &Noun::Settings.label(),
            &section.label(),
        ))
}

/// The page heading at Medium, a breadcrumb that is the only trail on screen (DESKTOP-120):
/// **Settings** goes back to the index, and the caret after the page name opens the picker under it.
fn crumb_heading(
    section: SettingsPage,
    picker: Option<&PagePicker>,
    ledger_open: bool,
    on_crumb: OnClick,
    on_caret: OnClick,
    on_pick: OnPageClick,
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
                .on_click(move |_event, window, cx| on_crumb(window, cx))
                .child(Noun::Settings.label()),
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
                .on_click(move |_event, window, cx| on_caret(window, cx))
                .child(section.label())
                .child(
                    div()
                        .text_size(px(16.0))
                        .text_color(color::muted(cx))
                        .child("\u{25be}"),
                ),
        )
        // Deferred so it paints after the page below it; otherwise the page draws over the menu.
        .children(
            picker.map(|picker| gpui::deferred(page_picker(picker, ledger_open, on_pick, cx))),
        )
}

/// The picker under the page name: every page, in index order under its group's heading, with the
/// chosen one highlighted.
fn page_picker(
    picker: &PagePicker,
    ledger_open: bool,
    on_pick: OnPageClick,
    cx: &App,
) -> AnyElement {
    let chosen = picker.chosen();
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
        .children(ScopeGroup::ALL.into_iter().flat_map(|group| {
            let on_pick = on_pick.clone();
            std::iter::once(
                div()
                    .pt(px(10.0))
                    .pb(px(4.0))
                    .px(px(12.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_size(px(9.5))
                    .text_color(color::faint_text(cx))
                    .child(lib_locale::format::upper(&group.label()))
                    .into_any_element(),
            )
            .chain(group.pages().map(move |page| {
                let on_pick = on_pick.clone();
                div()
                    .id(SharedString::from(format!("settings-picker-{page:?}")))
                    .debug_selector(move || format!("settings-picker-{page:?}"))
                    .cursor_pointer()
                    .py(px(7.0))
                    .px(px(12.0))
                    .when(page.needs_ledger() && !ledger_open, |this| {
                        this.text_color(color::faint_text(cx))
                    })
                    .when(page == chosen, |this| {
                        this.bg(color::selection_background(cx))
                            .text_color(color::selection_text(cx))
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    })
                    .hover(|this| this.bg(color::hover(cx)))
                    .on_click(move |_event, window, cx| on_pick(page, window, cx))
                    .child(page.label())
                    .into_any_element()
            }))
        }))
        .into_any_element()
}

/// The Settings index as a page of its own, at Medium (DESKTOP-119): a plain **Settings** heading,
/// then each group's heading, with its scope line and what it holds, over one row per page with
/// its one-line description. The page on show stays selected. Each is a child of the scroll on
/// its own ([`index_page_child`]), so a row can be scrolled to.
fn index_page(
    selected: SettingsPage,
    ledger_open: bool,
    on_row_click: OnPageClick,
    cx: &App,
) -> Vec<AnyElement> {
    let count = i64::try_from(SettingsPage::ALL.len()).unwrap_or(i64::MAX);
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
                .child(crate::msg::desktop_settings_index_count(count)),
        );
    let rule = div()
        .h(px(2.0))
        .bg(color::structural_rule(cx))
        .mt(px(14.0))
        .mb(px(6.0));
    let row = |page: SettingsPage| {
        let on_row_click = on_row_click.clone();
        let active = page == selected;
        let muted = page.needs_ledger() && !ledger_open;
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
            .when(muted, |this| this.text_color(color::faint_text(cx)))
            .on_click(move |_event, window, cx| on_row_click(page, window, cx))
            .child(
                div()
                    .debug_selector(move || format!("settings-index-page-{page:?}-icon"))
                    .flex_none()
                    .child(
                        page.icon()
                            .icon()
                            .with_size(px(16.0))
                            .text_color(if active {
                                color::selection_text(cx)
                            } else {
                                color::foreground(cx)
                            }),
                    ),
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
                            .child(page.label()),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .when(!active, |this| this.text_color(color::muted(cx)))
                            .truncate()
                            .child(page.description()),
                    ),
            )
            .child("\u{203a}")
            .into_any_element()
    };
    let groups = ScopeGroup::ALL
        .into_iter()
        .flat_map(|group| std::iter::once(group_heading(group, cx)).chain(group.pages().map(row)));
    [heading.into_any_element(), rule.into_any_element()]
        .into_iter()
        .chain(groups.collect::<Vec<_>>())
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

/// A group's heading on the Medium index page: its name with the page count and sync scope at the
/// right, and a line saying where its settings live.
fn group_heading(group: ScopeGroup, cx: &App) -> AnyElement {
    div()
        .debug_selector(move || format!("settings-index-page-group-{group:?}"))
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
                        .child(lib_locale::format::upper(&group.label())),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::faint_text(cx))
                        .child(group.scope_line()),
                ),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(group.description()),
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

/// Where keyboard focus sits within Settings: the index rail, or the page it has open. Only
/// meaningful while the View zone has focus on the Settings noun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsFocus {
    #[default]
    Index,
    Page,
}

/// `delta` steps along a control's choices, stopping at either end rather than wrapping (the
/// grid's own edge rule), for `h`/`l` on a segmented or radio control.
pub fn step_choice<T: Copy + PartialEq>(choices: &[T], current: T, delta: isize) -> T {
    let Some(position) = choices.iter().position(|choice| *choice == current) else {
        return current;
    };
    let last = choices.len() - 1;
    choices[position.saturating_add_signed(delta).min(last)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_choice_stops_at_both_ends() {
        let choices = [1, 2, 3];
        assert_eq!(step_choice(&choices, 2, 1), 3);
        assert_eq!(step_choice(&choices, 3, 1), 3);
        assert_eq!(step_choice(&choices, 1, -1), 1);
        assert_eq!(step_choice(&choices, 9, 1), 9);
    }
}
