//! The Settings body (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state", "Body"): one page
//! at a time, not a continuous scroll. The settings index rail (`chrome::rail::settings_index`) swaps the
//! page, and only the selected page is mounted. Each page keeps the same anatomy: one heading
//! ("Settings - <page>") with its right-aligned meta over a 2px rule, then the page's content.

mod about;
pub mod accounts;
pub(crate) mod apply;
pub mod categories;
pub mod clear_logs_dialog;
pub mod colour_theme;
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

use gpui::{AnyElement, App, Context, Entity, SharedString, Window, div, prelude::*, px};

use crate::{
    chrome::{dialog_host::Dialog, rail::settings_index::SettingsIndexRail},
    form::field::TextField,
    institutions::form::AddInstitutionForm,
    navigation::nav::Noun,
    theme::color,
    units::form::{DeleteUnitForm, UnitForm},
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
    pub categories: categories::CategoriesPageProps<'a>,
    pub tags: tags::TagsPageProps<'a>,
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
        SettingsPage::Categories => (
            categories::scope_note(props.categories.categories).into_any_element(),
            categories::render(&props.categories, page_focused, cx),
        ),
        SettingsPage::Tags => (
            tags::scope_note(props.tags.tags, props.tags.groups, cx),
            tags::render(&props.tags, page_focused, cx),
        ),
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
        | SettingsPage::Payees
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
        let payees = self.sources().payees.clone();
        payees.update(cx, |page, _| page.set_page_focused(page_focused));
        let (meta, content) = match self.data_page.take() {
            Some(page) => (page.meta, page.content),
            None if selected == SettingsPage::Accounts => (
                accounts.read(cx).scope_note(cx).into_any_element(),
                accounts.into_any_element(),
            ),
            None if selected == SettingsPage::Payees => {
                (payees.read(cx).scope_note(cx), payees.into_any_element())
            }
            None => (
                self.scope_note(cx).into_any_element(),
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
    /// The heading's scope note for a page this View draws: Units and Institutions count their
    /// rows; the others have a fixed note.
    fn scope_note(&self, cx: &App) -> String {
        match self.selected_section() {
            SettingsPage::Units => crate::msg::desktop_settings_scope_units(count(
                self.sources().units.read(cx).units(cx).len(),
            )),
            SettingsPage::Institutions => crate::msg::desktop_settings_scope_institutions(count(
                self.sources().institutions.read(cx).institutions(cx).len(),
            )),
            other => other.scope_note(),
        }
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
            SettingsPage::Display => {
                let display = self.display();
                display::render(
                    display.date_style,
                    display.row_density,
                    display.status_glyphs,
                    display.start_sidebar_minimised,
                    plain(entity, SettingsView::click_start_sidebar_minimised),
                    bind(entity, SettingsView::click_date_style),
                    bind(entity, SettingsView::click_row_density),
                    bind(entity, SettingsView::click_status_glyphs),
                    display.toasts_on,
                    bind(entity, SettingsView::click_toasts),
                    self.colour_theme_focus(),
                    self.display_field(),
                    bind(entity, SettingsView::click_colour_theme),
                    cx,
                )
            }
            SettingsPage::Units => units::render(
                self.sources().units.read(cx).units(cx),
                bind(entity, SettingsView::click_edit_unit),
                bind(entity, SettingsView::click_delete_unit),
                plain(entity, SettingsView::click_add_unit),
                self.sources().units.read(cx).price_sources(cx),
                row_stub(
                    entity,
                    crate::msg::desktop_status_test_price_source_not_yet_built,
                ),
                row_stub(
                    entity,
                    crate::msg::desktop_status_edit_price_source_not_yet_built,
                ),
                row_stub(
                    entity,
                    crate::msg::desktop_status_delete_price_source_not_yet_built,
                ),
                stub(
                    entity,
                    crate::msg::desktop_status_add_price_source_not_yet_built,
                ),
                cx,
            ),
            SettingsPage::Institutions => institutions::render(
                self.sources().institutions.read(cx).institutions(cx),
                row_stub(
                    entity,
                    crate::msg::desktop_status_edit_institution_not_yet_built,
                ),
                row_stub(
                    entity,
                    crate::msg::desktop_status_delete_institution_not_yet_built,
                ),
                plain(entity, SettingsView::click_add_institution),
                cx,
            ),
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
            // Accounts and Payees draw themselves; `Shell` hands the rest in as a `DataPage`, and
            // a frame without one draws nothing there.
            SettingsPage::Accounts
            | SettingsPage::Categories
            | SettingsPage::Tags
            | SettingsPage::Payees
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

/// A row count as the number a plural selector takes.
fn count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

/// A page that isn't rebuilt yet.
fn placeholder(cx: &App) -> AnyElement {
    div()
        .text_color(color::faint_text(cx))
        .child(crate::msg::desktop_settings_page_placeholder())
        .into_any_element()
}

/// An `Input`/`select`-styled box showing `value` as static text: `width:100%; padding:8px 10px;
/// border:1px solid rgba(32,30,29,.30); font-size:13px` -- `select_style` adds the `select`
/// row's own `background:#f3f2f2` (README: "selects add `background:#f3f2f2`").
///
/// Deliberately **not** a real editable text input or an openable dropdown -- see this map's own
/// Destination: every section here is stubbed/dummy data, and free-text editing/dropdown-open
/// behaviour is real interaction-pattern work with no existing precedent in this crate yet
/// (`gpui-component` ships an `Input` widget, but adopting it is a bigger, crate-wide styling
/// decision than one section's ticket should make on its own -- left for whichever future
/// ticket needs it first). "Save on change" therefore has nothing to save yet.
pub(super) fn field_value(
    value: impl Into<SharedString>,
    select_style: bool,
    cx: &App,
) -> impl IntoElement {
    div()
        .w_full()
        .py(px(8.0))
        .px(px(10.0))
        .border_1()
        .border_color(color::border(cx))
        .text_size(px(13.0))
        .when(select_style, |this| this.bg(color::background(cx)))
        .child(value.into())
}

/// Where keyboard focus sits within Settings: the index rail, or the page it has open. Only
/// meaningful while the View zone has focus on the Settings noun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsFocus {
    #[default]
    Index,
    Page,
}

/// The Display page's keyboard-reachable controls above the Colour Theme grid, top to bottom:
/// Date format, Row density, Status glyphs, Start sidebar minimised, Toasts. The grid is the stop
/// after the last one.
pub const DISPLAY_FIELD_COUNT: usize = 5;

/// The Display field that is a checkbox rather than a segmented or radio control.
pub const DISPLAY_FIELD_SIDEBAR: usize = 3;

/// `delta` steps along a control's choices, stopping at either end rather than wrapping (the
/// grid's own edge rule), for `h`/`l` on a segmented or radio control.
pub fn step_choice<T: Copy + PartialEq>(choices: &[T], current: T, delta: isize) -> T {
    let Some(position) = choices.iter().position(|choice| *choice == current) else {
        return current;
    };
    let last = choices.len() - 1;
    choices[position.saturating_add_signed(delta).min(last)]
}

/// Every Settings dialog `Shell` can have open, `None` when none is -- the README's own `State`
/// block (`dialog: Option<Dialog>`). One variant per dialog ticket; `EditUnit`/`DeleteUnit`'s own
/// `usize` is the row's index in `Shell::settings_units` -- `Shell::confirm_settings_dialog`
/// needs it to know which row to overwrite/remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsDialog {
    AddUnit(UnitForm),
    EditUnit(usize, UnitForm),
    DeleteUnit(usize, DeleteUnitForm),
    AddInstitution(AddInstitutionForm),
    /// Tracing's Clear logs confirm: nothing to fill in.
    ClearLogs,
}

impl Dialog for SettingsDialog {
    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::AddUnit(form) | Self::EditUnit(_, form) => Some(form.focused_mut()),
            Self::DeleteUnit(_, form) => Some(&mut form.confirm_input),
            Self::AddInstitution(form) => Some(&mut form.name),
            Self::ClearLogs => None,
        }
    }

    fn cycle_field(&mut self) {
        if let Self::AddUnit(form) | Self::EditUnit(_, form) = self {
            form.cycle_field();
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::AddUnit(form) | Self::EditUnit(_, form) => form.is_valid(),
            Self::DeleteUnit(_, form) => form.is_valid(),
            Self::AddInstitution(form) => form.is_valid(),
            Self::ClearLogs => true,
        }
    }

    /// Clear logs is a confirm; the Units and Institutions dialogs leave the page's own legend.
    fn hints(&self) -> Option<Vec<(&'static str, String)>> {
        match self {
            Self::ClearLogs => Some(hints::confirm_dialog_hints()),
            Self::AddUnit(_)
            | Self::EditUnit(..)
            | Self::DeleteUnit(..)
            | Self::AddInstitution(_) => None,
        }
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
