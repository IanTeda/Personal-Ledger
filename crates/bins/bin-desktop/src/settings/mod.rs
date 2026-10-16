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
pub mod snapshot;
pub mod state;
pub mod sync_server;
pub mod tags;
pub mod tracing;
pub mod units;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, Window, div, prelude::*, px};

use crate::{chrome::rail::settings_index::SettingsIndexRail, navigation::nav::Noun, theme::color};

use state::SettingsView;

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
            .flex_col()
            .child(section_block(selected, meta, content, cx));
        div()
            .id("settings")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .child(SettingsIndexRail::new(
                selected,
                index_focused,
                on_index_click,
            ))
            .child(body)
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
/// margin ownership is how this gap goes missing, so it's carried on a single edge, here).
fn section_block(
    section: SettingsPage,
    meta: AnyElement,
    content: AnyElement,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .when(section.fills_page(), |this| this.flex_1().min_h(px(0.0)))
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
                        .child(format!("{} - {}", Noun::Settings.label(), section.label())),
                )
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
