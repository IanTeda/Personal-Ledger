//! The Settings body (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state", "Body"): one page
//! at a time, not a continuous scroll. The settings index rail (`chrome::rail::settings_index`) swaps the
//! page, and only the selected page is mounted. Each page keeps the same anatomy: one heading
//! ("Settings - <page>") with its right-aligned meta over a 2px rule, then the page's content.

mod about;
pub mod accounts;
pub(crate) mod apply;
pub mod categories;
pub mod clear_logs_dialog;
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
pub mod tracing_log;
pub mod units;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, Entity, Window, div, prelude::*, px};

use crate::{
    chrome::{dialog_host::Dialog, rail::settings_index::SettingsIndexRail},
    form::field::TextField,
    navigation::nav::Noun,
    theme::color,
};

use state::SettingsView;

pub use page::SettingsPage;

/// A click handler the Settings View binds for a page, given the clicked value.
type OnValue<A> = Rc<dyn Fn(A, &mut Window, &mut App)>;
/// As [`OnValue`], for a click that carries no value.
type OnPlain = Rc<dyn Fn(&mut Window, &mut App)>;

/// A page `Shell` still draws (the pages with Ledger data, until each moves into this View):
/// its heading's meta and its content, handed to [`SettingsView`] for the frame they are drawn
/// in.
pub struct DataPage {
    pub meta: AnyElement,
    pub content: AnyElement,
}

/// What `Shell` draws the pages with Ledger data from, until each moves into this View.
pub struct DataPageProps<'a> {
    pub documents: documents::DocumentsPageProps<'a>,
    pub inventory: inventory::InventoryPageProps<'a>,
}

/// The page `section` as `Shell` draws it, or `None` for a page [`SettingsView`] draws itself.
pub fn data_page(
    section: SettingsPage,
    page_focused: bool,
    props: &DataPageProps<'_>,
    cx: &App,
) -> Option<DataPage> {
    let (meta, content) = match section {
        SettingsPage::Documents => (
            documents::scope_text(props.documents.types).into_any_element(),
            documents::render(&props.documents, page_focused, cx),
        ),
        SettingsPage::Inventory => (
            inventory::scope_text(props.inventory.inventory).into_any_element(),
            inventory::render(&props.inventory, page_focused, cx),
        ),
        SettingsPage::General
        | SettingsPage::Accounts
        | SettingsPage::Categories
        | SettingsPage::Payees
        | SettingsPage::Tags
        | SettingsPage::Display
        | SettingsPage::Units
        | SettingsPage::Institutions
        | SettingsPage::SyncServer
        | SettingsPage::DataBackup
        | SettingsPage::Tracing
        | SettingsPage::About => return None,
    };
    Some(DataPage { meta, content })
}

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
        let display = self.sources().display.clone();
        let units = self.sources().units.clone();
        let institutions = self.sources().institutions.clone();
        let (meta, content) = match self.data_page.take() {
            Some(page) => (page.meta, page.content),
            None if selected == SettingsPage::Accounts => (
                accounts.read(cx).scope_note(cx).into_any_element(),
                accounts.into_any_element(),
            ),
            None if selected == SettingsPage::Categories => (
                categories.read(cx).scope_note(cx).into_any_element(),
                categories.into_any_element(),
            ),
            None if selected == SettingsPage::Payees => {
                (payees.read(cx).scope_note(cx), payees.into_any_element())
            }
            None if selected == SettingsPage::Tags => {
                (tags.read(cx).scope_note(cx), tags.into_any_element())
            }
            None if selected == SettingsPage::Display => (
                self.scope_note().into_any_element(),
                display.into_any_element(),
            ),
            None if selected == SettingsPage::Units => (
                units.read(cx).scope_note(cx).into_any_element(),
                units.into_any_element(),
            ),
            None if selected == SettingsPage::Institutions => (
                institutions.read(cx).scope_note(cx).into_any_element(),
                institutions.into_any_element(),
            ),
            None => (
                self.scope_note().into_any_element(),
                self.page_content(&entity, cx),
            ),
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

    /// The content of a page this View draws, its clicks bound to `entity`.
    fn page_content(&self, entity: &Entity<Self>, cx: &App) -> AnyElement {
        // A click handler that runs `act` on this View with the clicked value.
        fn bind<A: 'static>(
            entity: &Entity<SettingsView>,
            act: fn(&mut SettingsView, A, &mut Context<'_, SettingsView>),
        ) -> OnValue<A> {
            let entity = entity.clone();
            Rc::new(move |value, _window, cx| entity.update(cx, |view, cx| act(view, value, cx)))
        }
        // As `bind`, for a click that carries no value.
        fn plain(
            entity: &Entity<SettingsView>,
            act: fn(&mut SettingsView, &mut Context<'_, SettingsView>),
        ) -> OnPlain {
            let entity = entity.clone();
            Rc::new(move |_window, cx| entity.update(cx, act))
        }
        // As `plain`, for a button with nothing behind it yet: it says so on the status line.
        fn stub(entity: &Entity<SettingsView>, message: fn() -> String) -> OnPlain {
            let entity = entity.clone();
            Rc::new(move |_window, cx| entity.update(cx, |view, cx| view.not_built(message(), cx)))
        }
        // As `stub`, for a row's button, whose row it ignores.
        fn row_stub(entity: &Entity<SettingsView>, message: fn() -> String) -> OnValue<usize> {
            let entity = entity.clone();
            Rc::new(move |_index, _window, cx| {
                entity.update(cx, |view, cx| view.not_built(message(), cx))
            })
        }

        match self.selected_section() {
            SettingsPage::General => general::render(cx),
            SettingsPage::SyncServer => sync_server::render(
                stub(entity, crate::msg::desktop_status_sync_now_not_implemented),
                cx,
            ),
            SettingsPage::DataBackup => data_backup::render(
                stub(
                    entity,
                    crate::msg::desktop_status_backup_now_not_implemented,
                ),
                stub(
                    entity,
                    crate::msg::desktop_status_export_ledger_not_implemented,
                ),
                cx,
            ),
            SettingsPage::Tracing => tracing::render(
                &tracing::LogBoxProps {
                    level: self.log().level(),
                    entries: self.log().visible(),
                    hidden: self.log().hidden_count(),
                    list: self.log_list().clone(),
                },
                bind(entity, SettingsView::set_tracing_level),
                plain(entity, SettingsView::click_clear_logs),
                cx,
            ),
            SettingsPage::About => about::render(cx),
            // Accounts, Categories, Payees, Tags, Display, Units and Institutions draw themselves;
            // `Shell` hands the rest in as a `DataPage`, and a frame without one draws nothing there.
            SettingsPage::Accounts
            | SettingsPage::Display
            | SettingsPage::Units
            | SettingsPage::Institutions
            | SettingsPage::Categories
            | SettingsPage::Payees
            | SettingsPage::Tags
            | SettingsPage::Documents
            | SettingsPage::Inventory => gpui::Empty.into_any_element(),
        }
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

/// The Settings dialogs no page owns: each page's own Dialogs live in its folder (ADR-0037's
/// amendment), so only Tracing's Clear logs confirm is left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsDialog {
    /// Tracing's Clear logs confirm: nothing to fill in.
    ClearLogs,
}

impl Dialog for SettingsDialog {
    fn focused_text(&mut self) -> Option<&mut TextField> {
        None
    }

    fn is_valid(&self) -> bool {
        true
    }

    /// Clear logs is a confirm.
    fn hints(&self) -> Option<Vec<(&'static str, String)>> {
        Some(hints::confirm_dialog_hints())
    }
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
