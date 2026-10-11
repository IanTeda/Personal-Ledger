//! The Payees destination's Entity (ADR-0032). [`PayeesView`] owns the two selections (the Payees
//! page's row and the Settings list's id). `Shell` opens the dialogs and runs the rules, since the
//! dialog host and the Transactions store are `Shell`'s.

use gpui::Context;

/// The Payees page's own state. `selected` is a position in the stored rows; `settings_selected` is
/// the Payee id the Settings Payees list has under the cursor.
#[derive(Default)]
pub struct PayeesView {
    selected: usize,
    settings_selected: Option<u32>,
}

impl PayeesView {
    pub fn new() -> Self {
        Self::default()
    }

    /// The Payees page's stored row position. Clamp it before use: removing Payees can leave it
    /// past the end.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The Settings Payees list's stored id. It may name a Payee that no longer exists, so read it
    /// through the Shell's selection rule.
    pub fn settings_selected(&self) -> Option<u32> {
        self.settings_selected
    }

    pub fn set_selected(&mut self, selected: usize, cx: &mut Context<'_, Self>) {
        self.selected = selected;
        cx.notify();
    }

    pub fn set_settings_selected(&mut self, id: Option<u32>, cx: &mut Context<'_, Self>) {
        self.settings_selected = id;
        cx.notify();
    }
}
