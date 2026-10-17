//! The Settings body (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state", "Body"): one page
//! at a time, not a continuous scroll. The settings index rail (`chrome::rail::settings_index`) swaps the
//! page, and only the selected page is mounted. Each page keeps the same anatomy: one heading
//! ("Settings - <page>") with its right-aligned meta over a 2px rule, then the page's content.

pub mod about;
pub mod accounts;
pub mod categories;
pub mod data_backup;
pub mod display;
pub mod documents;
mod general;
pub(crate) mod hints;
pub mod institutions;
pub mod inventory;
pub mod page;
pub mod payees;
pub mod picker;
pub mod snapshot;
pub mod state;
pub mod sync_server;
pub mod tags;
pub mod tracing;
pub mod units;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, Pixels, SharedString, Window, div, prelude::*, px};

use crate::{chrome::rail::settings_index::SettingsIndexRail, navigation::nav::Noun, theme::color};

use picker::PagePicker;
use state::SettingsView;

/// The widest a page's content draws, at every Window Size; the View past it stays empty
/// (DESKTOP-107).
pub const CONTENT_MAX_WIDTH: Pixels = px(840.0);

/// The page picker's own width, under the page name.
const PICKER_WIDTH: Pixels = px(280.0);

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;
type OnPageClick = Rc<dyn Fn(SettingsPage, &mut Window, &mut App)>;

pub use page::SettingsPage;

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
                .flex_1()
                .min_w(px(0.0))
                .h_full()
                .overflow_y_scroll()
                .track_scroll(&self.scroll_handle)
                .when(index_focused, |this| {
                    this.border_l(px(2.0)).border_color(color::foreground(cx))
                })
                .py(px(22.0))
                .px(px(28.0))
                .child(index_page(selected, on_row_click, cx))
                .into_any_element();
        }
        let accounts = self.sources().accounts.clone();
        accounts.update(cx, |page, _| page.set_page_focused(page_focused));
        let categories = self.sources().categories.clone();
        categories.update(cx, |page, _| page.set_page_focused(page_focused));
        let payees = self.sources().payees.clone();
        payees.update(cx, |page, _| page.set_page_focused(page_focused));
        let tags = self.sources().tags.clone();
        tags.update(cx, |page, _| page.set_page_focused(page_focused));
        let documents = self.sources().documents.clone();
        documents.update(cx, |page, _| page.set_page_focused(page_focused));
        let inventory = self.sources().inventory.clone();
        inventory.update(cx, |page, _| page.set_page_focused(page_focused));
        let tracing = self.sources().tracing.clone();
        let display = self.sources().display.clone();
        let units = self.sources().units.clone();
        let institutions = self.sources().institutions.clone();
        let sync_server = self.sources().sync_server.clone();
        let data_backup = self.sources().data_backup.clone();
        let about = self.sources().about.clone();
        let (meta, content) = match selected {
            SettingsPage::Accounts => (
                accounts.read(cx).scope_note(cx).into_any_element(),
                accounts.into_any_element(),
            ),
            SettingsPage::Categories => (
                categories.read(cx).scope_note(cx).into_any_element(),
                categories.into_any_element(),
            ),
            SettingsPage::Payees => (payees.read(cx).scope_note(cx), payees.into_any_element()),
            SettingsPage::Tags => (tags.read(cx).scope_note(cx), tags.into_any_element()),
            SettingsPage::Documents => (
                documents.read(cx).scope_note(cx).into_any_element(),
                documents.into_any_element(),
            ),
            SettingsPage::Inventory => (
                inventory.read(cx).scope_note(cx).into_any_element(),
                inventory.into_any_element(),
            ),
            SettingsPage::Display => (
                self.scope_note().into_any_element(),
                display.into_any_element(),
            ),
            SettingsPage::Units => (
                units.read(cx).scope_note(cx).into_any_element(),
                units.into_any_element(),
            ),
            SettingsPage::Institutions => (
                institutions.read(cx).scope_note(cx).into_any_element(),
                institutions.into_any_element(),
            ),
            SettingsPage::SyncServer => (
                self.scope_note().into_any_element(),
                sync_server.into_any_element(),
            ),
            SettingsPage::DataBackup => (
                self.scope_note().into_any_element(),
                data_backup.into_any_element(),
            ),
            SettingsPage::About => (
                self.scope_note().into_any_element(),
                about.into_any_element(),
            ),
            SettingsPage::Tracing => (
                self.scope_note().into_any_element(),
                tracing.into_any_element(),
            ),
            SettingsPage::General => (self.scope_note().into_any_element(), general::render(cx)),
        };
        // Tracing's log box fills the page and scrolls itself, so its body must not scroll too.
        let fills = selected.fills_page();
        let body = div()
            .id("settings-body")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .when(fills, |this| this.overflow_hidden())
            .when(!fills, |this| {
                this.overflow_y_scroll().track_scroll(&self.scroll_handle)
            })
            .when(page_focused, |this| {
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
            crumb_heading(selected, self.picker(), on_crumb, on_caret, on_pick, cx)
                .into_any_element()
        } else {
            plain_heading(selected, cx).into_any_element()
        };
        let body = body.child(section_block(selected, heading, meta, content, cx));
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
                    on_index_click,
                ))
            })
            .child(body)
            .into_any_element()
    }
}

impl SettingsView {
    /// The heading's scope note for a page this View draws: a fixed note.
    fn scope_note(&self) -> String {
        self.selected_section().scope_note()
    }
}

/// One page: heading + right-aligned scope note, a 2px rule, the page's content, then a **48px**
/// bottom gap -- every page, no exceptions (README's implementation note 4: mixed top/bottom
/// margin ownership is how this gap goes missing, so it's carried on a single edge, here). The
/// whole block caps at [`CONTENT_MAX_WIDTH`].
fn section_block(
    section: SettingsPage,
    heading: AnyElement,
    meta: AnyElement,
    content: AnyElement,
    cx: &App,
) -> impl IntoElement {
    div()
        .debug_selector(|| "settings-page".to_string())
        .w_full()
        .max_w(CONTENT_MAX_WIDTH)
        .flex()
        .flex_col()
        .when(section.fills_page(), |this| this.flex_1().min_h(px(0.0)))
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
        .child(format!("{} - {}", Noun::Settings.label(), section.label()))
}

/// The page heading at Medium, a breadcrumb that is the only trail on screen (DESKTOP-120):
/// **Settings** goes back to the index, and the caret after the page name opens the picker under it.
fn crumb_heading(
    section: SettingsPage,
    picker: Option<&PagePicker>,
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
        .children(picker.map(|picker| gpui::deferred(page_picker(picker, on_pick, cx))))
}

/// The picker under the page name: its filter, then the matching pages in index order.
fn page_picker(picker: &PagePicker, on_pick: OnPageClick, cx: &App) -> AnyElement {
    let chosen = picker.chosen();
    let query = picker.query();
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
        .child(
            div()
                .py(px(8.0))
                .px(px(12.0))
                .border_b_1()
                .border_color(color::hairline(cx))
                .text_color(if query.is_empty() {
                    color::faint_text(cx)
                } else {
                    color::foreground(cx)
                })
                .child(if query.is_empty() {
                    crate::msg::desktop_settings_picker_filter()
                } else {
                    query.to_string()
                }),
        )
        .children(picker.matches().into_iter().map(|page| {
            let on_pick = on_pick.clone();
            div()
                .id(SharedString::from(format!("settings-picker-{page:?}")))
                .debug_selector(move || format!("settings-picker-{page:?}"))
                .cursor_pointer()
                .py(px(7.0))
                .px(px(12.0))
                .when(Some(page) == chosen, |this| {
                    this.bg(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .hover(|this| this.bg(color::hover(cx)))
                .on_click(move |_event, window, cx| on_pick(page, window, cx))
                .child(page.label())
        }))
        .into_any_element()
}

/// The Settings index as a page of its own, at Medium (DESKTOP-119): a plain **Settings** heading,
/// then one row per page with its one-line description. The page on show stays selected.
fn index_page(selected: SettingsPage, on_row_click: OnPageClick, cx: &App) -> impl IntoElement {
    let count = i64::try_from(SettingsPage::ALL.len()).unwrap_or(i64::MAX);
    div()
        .debug_selector(|| "settings-index-page".to_string())
        .w_full()
        .max_w(CONTENT_MAX_WIDTH)
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
                        .text_size(px(28.0))
                        .text_color(color::foreground(cx))
                        .child(Noun::Settings.label()),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::faint_text(cx))
                        .child(crate::msg::desktop_settings_index_count(count)),
                ),
        )
        .child(
            div()
                .h(px(2.0))
                .bg(color::structural_rule(cx))
                .mt(px(14.0))
                .mb(px(6.0)),
        )
        .children(SettingsPage::ALL.into_iter().map(|page| {
            let on_row_click = on_row_click.clone();
            let active = page == selected;
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
                .on_click(move |_event, window, cx| on_row_click(page, window, cx))
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
        }))
        .child(
            div()
                .pt(px(10.0))
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_settings_index_footer()),
        )
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
