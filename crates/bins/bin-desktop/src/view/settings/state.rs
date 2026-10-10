//! The Settings destination's Entity (ADR-0032). [`SettingsView`] owns the View state that
//! belongs to Settings alone: the page on show, where keyboard focus sits, the Display page's
//! focused control, and the Documents and Inventory pages' selections. Preferences that Chrome
//! and other Views read stay on `Shell` (ADR-0032's "Shell keeps what is cross-cutting").

use std::collections::HashSet;

use gpui::Context;

use crate::{
    settings::{SettingsFocus, SettingsSection},
    view::settings::inventory::InventoryRow,
};

/// The Settings View's own state. The page is the last one visited, so it survives leaving and
/// re-entering Settings, as `g s` relies on.
#[derive(Default)]
pub struct SettingsView {
    selected_section: SettingsSection,
    focus: SettingsFocus,
    display_field: Option<usize>,
    documents_selected: Option<u32>,
    inventory_selected: Option<InventoryRow>,
    inventory_expanded: HashSet<u32>,
}

impl SettingsView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn selected_section(&self) -> SettingsSection {
        self.selected_section
    }

    pub fn focus(&self) -> SettingsFocus {
        self.focus
    }

    pub fn display_field(&self) -> Option<usize> {
        self.display_field
    }

    pub fn documents_selected(&self) -> Option<u32> {
        self.documents_selected
    }

    pub fn inventory_selected(&self) -> Option<InventoryRow> {
        self.inventory_selected
    }

    /// The Properties shown open on the Inventory page.
    pub fn inventory_expanded(&self) -> &HashSet<u32> {
        &self.inventory_expanded
    }

    pub fn set_selected_section(&mut self, cx: &mut Context<'_, Self>, section: SettingsSection) {
        self.selected_section = section;
        cx.notify();
    }

    pub fn set_focus(&mut self, cx: &mut Context<'_, Self>, focus: SettingsFocus) {
        self.focus = focus;
        cx.notify();
    }

    pub fn set_display_field(&mut self, cx: &mut Context<'_, Self>, field: Option<usize>) {
        self.display_field = field;
        cx.notify();
    }

    pub fn set_documents_selected(&mut self, cx: &mut Context<'_, Self>, id: Option<u32>) {
        self.documents_selected = id;
        cx.notify();
    }

    pub fn set_inventory_selected(
        &mut self,
        cx: &mut Context<'_, Self>,
        row: Option<InventoryRow>,
    ) {
        self.inventory_selected = row;
        cx.notify();
    }

    /// Shows Property `id` open. Returns whether it was closed before, so the caller can tell a
    /// real change from a no-op.
    pub fn expand(&mut self, cx: &mut Context<'_, Self>, id: u32) -> bool {
        let inserted = self.inventory_expanded.insert(id);
        cx.notify();
        inserted
    }

    /// Shows Property `id` closed. Returns whether it was open before.
    pub fn collapse(&mut self, cx: &mut Context<'_, Self>, id: u32) -> bool {
        let removed = self.inventory_expanded.remove(&id);
        cx.notify();
        removed
    }

    /// Edits the expanded Properties in place, for the fold and disclosure rules to run on.
    pub fn update_inventory_expanded(
        &mut self,
        cx: &mut Context<'_, Self>,
        edit: impl FnOnce(&mut HashSet<u32>),
    ) {
        edit(&mut self.inventory_expanded);
        cx.notify();
    }
}
