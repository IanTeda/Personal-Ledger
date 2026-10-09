//! The Tags destination's wiring: keys, clicks, the Add, Edit, Remove and Merge dialogs, and the Settings Tags list. The pure rules live in `lib_tags`; the chrome in `view::tags`, and its View state in the `TagsView` Entity.

use super::Shell;

use crate::{
    accounts,
    chrome::dialog_host::OpenDialog,
    navigation::{
        key_router::Movement,
        nav::{FocusZone, Noun},
    },
    settings::{SettingsFocus, SettingsSection},
    tags::{self, Tag},
    transactions::edit_transactions,
};

use gpui::{App, Context, Keystroke};
use lib_toast::ToastKind;

/// `Ctrl-d`/`Ctrl-u` on the Settings Tags page: rows per half page.
const SETTINGS_TAGS_HALF_PAGE: isize = 5;

impl Shell {
    /// The Tags rows, read through their store (ADR-0032).
    pub(super) fn tags_list<'a>(&self, cx: &'a App) -> &'a [Tag] {
        self.tags_store.read(cx).tags()
    }

    /// Writes `rows` back to the Tags store, which notifies its readers. The rules work on a copy
    /// of the rows, so a refused change never reaches the store.
    fn store_tags(&mut self, rows: Vec<Tag>, cx: &mut App) {
        self.tags_store
            .update(cx, |store, cx| store.mutate(cx, |tags| *tags = rows));
    }

    /// The Tags page's selected row, a position in `tags::sorted_by_usage`'s order.
    fn tags_page_selected(&self, cx: &App) -> usize {
        self.tags_view.read(cx).selected()
    }

    fn set_tags_page_selected(&mut self, selected: usize, cx: &mut App) {
        self.tags_view
            .update(cx, |view, cx| view.set_selected(selected, cx));
    }

    /// The Settings Tags list's stored id. It may name a Tag that has since been removed.
    fn settings_tags_selected(&self, cx: &App) -> Option<u32> {
        self.tags_view.read(cx).settings_selected()
    }

    fn set_settings_tags_selected(&mut self, id: Option<u32>, cx: &mut App) {
        self.tags_view
            .update(cx, |view, cx| view.set_settings_selected(id, cx));
    }

    /// Whether Settings' Tags list owns the keyboard: the page, not the index, has focus.
    pub(super) fn settings_tags_page_has_focus(&self) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_focus == SettingsFocus::Page
            && self.settings_selected_section == SettingsSection::Tags
    }

    /// The Settings Tags page's selected Tag: the stored id while it still exists, else the first
    /// row, so a removed or merged-away Tag never leaves the page with nothing under the cursor.
    pub(super) fn settings_tags_selected_id(&self, cx: &App) -> Option<u32> {
        let sorted = tags::sorted_by_name(self.tags_list(cx));
        self.settings_tags_selected(cx)
            .filter(|id| sorted.iter().any(|tag| tag.id == *id))
            .or_else(|| sorted.first().map(|tag| tag.id))
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the Tags list A–Z. `enter` has no hand-off here.
    pub(super) fn apply_settings_tags_movement(&mut self, movement: Movement, cx: &mut App) {
        let current_id = self.settings_tags_selected_id(cx);
        let sorted = tags::sorted_by_name(self.tags_list(cx));
        let len = sorted.len();
        let current = current_id
            .and_then(|id| sorted.iter().position(|tag| tag.id == id))
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => accounts::step_selection(current, len, 1),
            Movement::Prev => accounts::step_selection(current, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => {
                accounts::step_selection(current, len, SETTINGS_TAGS_HALF_PAGE)
            }
            Movement::HalfPageUp => {
                accounts::step_selection(current, len, -SETTINGS_TAGS_HALF_PAGE)
            }
            Movement::Enter => return,
        };
        let next_id = sorted.get(next).map(|tag| tag.id);
        self.set_settings_tags_selected(next_id, cx);
    }

    /// The selected Tag: the Tags page's row indexes the usage order, not the rows.
    pub(super) fn selected_tag_id(&self, cx: &App) -> Option<u32> {
        if self.settings_tags_page_has_focus() {
            return self.settings_tags_selected_id(cx);
        }
        let sorted = tags::sorted_by_usage(self.tags_list(cx), self.transactions(cx));
        sorted
            .get(
                self.tags_page_selected(cx)
                    .min(sorted.len().saturating_sub(1)),
            )
            .map(|tag| tag.id)
    }

    /// Selects the Tag with `id`, if it still exists.
    pub(super) fn select_tag(&mut self, id: u32, cx: &mut App) {
        self.set_settings_tags_selected(Some(id), cx);
        let index = tags::sorted_by_usage(self.tags_list(cx), self.transactions(cx))
            .iter()
            .position(|tag| tag.id == id);
        if let Some(index) = index {
            self.set_tags_page_selected(index, cx);
        }
    }

    /// The Tags page's own `n`/`e`/`x`/`m` (only while it is the active noun and the view has
    /// focus, in `Normal` mode).
    pub(super) fn handle_tags_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if !self.settings_tags_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_tag_dialog(cx),
            "e" => {
                if let Some(id) = self.selected_tag_id(cx) {
                    self.open_edit_tag_dialog(id, cx);
                }
            }
            "x" => {
                if let Some(id) = self.selected_tag_id(cx) {
                    self.open_remove_tag_dialog(id, cx);
                }
            }
            "m" => {
                if let Some(id) = self.selected_tag_id(cx) {
                    self.open_merge_tags_dialog(Some(id), cx);
                }
            }
            _ => return false,
        }
        true
    }

    pub(super) fn open_add_tag_dialog(&mut self, cx: &App) {
        let form = tags::form::TagForm::new(self.tags_list(cx).to_vec());
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Add(form)));
    }

    /// The form behind the open Add or Edit tag dialog, if that is what's open.
    pub(super) fn tag_form_mut(&mut self) -> Option<&mut tags::form::TagForm> {
        match self.tags_dialog_mut() {
            Some(tags::form::TagsDialog::Add(form) | tags::form::TagsDialog::Edit(_, form)) => {
                Some(form)
            }
            _ => None,
        }
    }

    /// Applies a confirmed Tags dialog (`Enter` and the confirm button): adds or saves the Tag
    /// and selects it, removes it, or merges it into another, toasting the last two.
    pub(super) fn apply_tags_dialog(
        &mut self,
        dialog: tags::form::TagsDialog,
        cx: &mut Context<'_, Self>,
    ) {
        match dialog {
            tags::form::TagsDialog::Add(form) => {
                let Some(draft) = form.draft() else {
                    return;
                };
                let mut rows = self.tags_list(cx).to_vec();
                // `is_valid` ran the same name check, so a refusal can only leave the dialog open.
                match tags::insert_tag(&mut rows, &draft) {
                    Ok(id) => {
                        self.store_tags(rows, cx);
                        self.select_tag(id, cx);
                    }
                    Err(_) => self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Add(form))),
                }
            }
            tags::form::TagsDialog::Edit(id, form) => {
                let Some(draft) = form.draft() else {
                    return;
                };
                let mut rows = self.tags_list(cx).to_vec();
                let saved = tags::edit_tag(&mut rows, id, &draft)
                    .and_then(|()| tags::set_active(&mut rows, id, form.is_active));
                match saved {
                    Ok(()) => {
                        self.store_tags(rows, cx);
                        self.select_tag(id, cx);
                    }
                    Err(_) => {
                        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Edit(id, form)))
                    }
                }
            }
            tags::form::TagsDialog::Remove(id, _) => self.apply_remove_tag(id, cx),
            tags::form::TagsDialog::Merge(form) => {
                if let Some((source, target)) = form.pair() {
                    self.apply_merge_tags(source, target, cx);
                }
            }
        }
    }

    pub(super) fn handle_tags_dialog_field_click(
        &mut self,
        field: tags::form::TagField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.tag_form_mut() {
            form.focus(field);
        }
        cx.notify();
    }

    pub(super) fn handle_tags_dialog_pick(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(form) = self.tag_form_mut() {
            form.focus(tags::form::TagField::Swatches);
            form.pick(index);
        }
        cx.notify();
    }

    pub(super) fn handle_tags_dialog_toggle_active(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(form) = self.tag_form_mut() {
            form.toggle_active();
        }
        cx.notify();
    }

    pub(super) fn handle_tags_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    pub(super) fn handle_tags_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    pub(super) fn open_edit_tag_dialog(&mut self, id: u32, cx: &App) {
        let rows = self.tags_list(cx);
        let Some(tag) = tags::get(rows, id) else {
            return;
        };
        let form = tags::form::TagForm::for_edit(tag, rows.to_vec());
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Edit(id, form)));
    }

    pub(super) fn open_remove_tag_dialog(&mut self, id: u32, cx: &App) {
        let Some(tag) = tags::get(self.tags_list(cx), id) else {
            return;
        };
        let form = tags::form::RemoveTagForm::new(
            &tag.name,
            tags::transaction_count(self.transactions(cx), id),
        );
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Remove(id, form)));
    }

    /// Untags every Split, deletes the Tag and toasts it. The selection keeps its position, so it
    /// lands on the next Tag in usage order (or the new last one).
    pub(super) fn apply_remove_tag(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(name) = tags::get(self.tags_list(cx), id).map(|tag| tag.name.clone()) else {
            return;
        };
        let mut rows = self.tags_list(cx).to_vec();
        let removed = edit_transactions(&self.transactions_store, cx, |transactions| {
            tags::remove_tag(&mut rows, transactions, id)
        });
        let (kind, text) = match removed {
            Ok(()) => {
                self.store_tags(rows, cx);
                (
                    ToastKind::Success,
                    lib_locale::msg::toast_tag_deleted(&name),
                )
            }
            Err(error) => (
                ToastKind::Error,
                lib_locale::msg::toast_save_failed(
                    &lib_locale::msg::toast_entity_tag(),
                    &error.to_string(),
                ),
            ),
        };
        self.raise_toast(kind, text);
        let last = self.tags_list(cx).len().saturating_sub(1);
        let selected = self.tags_page_selected(cx).min(last);
        self.set_tags_page_selected(selected, cx);
    }

    /// The 7e selects' options, labelled `Shared (9 txns)`.
    pub(super) fn merge_tag_options(&self, cx: &App) -> Vec<tags::form::MergeOption> {
        tags::form::merge_options(self.tags_list(cx), self.transactions(cx), |name, count| {
            crate::msg::desktop_tags_merge_option(name, i64::try_from(count).unwrap_or(i64::MAX))
        })
    }

    /// Opens 7e with `source` as the source Tag and, when it is flagged as a likely duplicate, its
    /// suggested target (#354). `None` (the palette's `tags merge`, or the subline link with
    /// nothing flagged) leaves both selects empty.
    pub(super) fn open_merge_tags_dialog(&mut self, source: Option<u32>, cx: &App) {
        let groups = tags::duplicate_groups(self.tags_list(cx), self.transactions(cx));
        let target = source.and_then(|id| tags::duplicate_of(&groups, id));
        let form = tags::form::MergeTagsForm::new(self.merge_tag_options(cx), source, target);
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Merge(form)));
    }

    /// Retags the source's Splits with the target, deletes the source, toasts it and selects the
    /// target.
    pub(super) fn apply_merge_tags(
        &mut self,
        source: u32,
        target: u32,
        cx: &mut Context<'_, Self>,
    ) {
        let rows = self.tags_list(cx);
        let (Some(source_name), Some(target_name)) = (
            tags::get(rows, source).map(|tag| tag.name.clone()),
            tags::get(rows, target).map(|tag| tag.name.clone()),
        ) else {
            return;
        };
        let retagged = tags::transaction_count(self.transactions(cx), source);
        let mut rows = rows.to_vec();
        let merged = edit_transactions(&self.transactions_store, cx, |transactions| {
            tags::merge_tags(&mut rows, transactions, source, target)
        });
        let (kind, text) = match merged {
            Ok(()) => {
                self.store_tags(rows, cx);
                (
                    ToastKind::Success,
                    lib_locale::msg::toast_tag_merged(
                        &source_name,
                        &target_name,
                        i64::try_from(retagged).unwrap_or(i64::MAX),
                    ),
                )
            }
            Err(error) => (
                ToastKind::Error,
                lib_locale::msg::toast_save_failed(
                    &lib_locale::msg::toast_entity_tag(),
                    &error.to_string(),
                ),
            ),
        };
        self.raise_toast(kind, text);
        self.select_tag(target, cx);
    }

    pub(super) fn handle_merge_tags_field_click(
        &mut self,
        field: tags::form::MergeField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(tags::form::TagsDialog::Merge(form)) = self.tags_dialog_mut() {
            form.toggle(field);
        }
        cx.notify();
    }

    pub(super) fn handle_merge_tags_option_click(
        &mut self,
        field: tags::form::MergeField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(tags::form::TagsDialog::Merge(form)) = self.tags_dialog_mut() {
            form.choose(field, index);
        }
        cx.notify();
    }

    pub(super) fn handle_tags_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_tag_dialog(cx);
        cx.notify();
    }

    /// A click on a row of Settings' Tags list: selects it and moves focus into the page.
    pub(super) fn handle_settings_tags_row_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id, cx);
        self.focus_settings_page();
        cx.notify();
    }

    pub(super) fn handle_tags_duplicate_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id, cx);
        self.open_merge_tags_dialog(Some(id), cx);
        cx.notify();
    }

    pub(super) fn handle_tags_edit_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id, cx);
        self.open_edit_tag_dialog(id, cx);
        cx.notify();
    }

    pub(super) fn handle_tags_remove_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id, cx);
        self.open_remove_tag_dialog(id, cx);
        cx.notify();
    }
}
