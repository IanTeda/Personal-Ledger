//! The Payees destination's Entities (ADR-0032). [`PayeesStore`] owns the rows, read through
//! `lib_payees`' service; [`PayeesView`] owns the two selections (the Payees page's row and the
//! Settings list's id). `Shell` opens the dialogs and runs the rules, since the dialog host and the
//! Transactions store are `Shell`'s.

use gpui::{Context, Entity, EventEmitter};
use lib_payees::{Payee, PayeeService};

/// Emitted by [`PayeesStore`] after any write, so `Shell` can refresh the Views that read the rows
/// (ADR-0032's View events).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayeesEvent {
    Changed,
}

/// The shared Payees rows. Every reader (the Payees page, Settings, the Transactions form and
/// Import, Bills, Documents and Budgets) reads them from here, so a change is seen everywhere on
/// the next render.
pub struct PayeesStore {
    service: PayeeService,
}

impl EventEmitter<PayeesEvent> for PayeesStore {}

impl PayeesStore {
    /// A store over `payees`, which the Desktop seeds from `lib_payees::default_payees`.
    pub fn new(payees: Vec<Payee>) -> Self {
        Self {
            service: PayeeService::new(payees),
        }
    }

    pub fn payees(&self) -> &[Payee] {
        self.service.payees()
    }

    /// Applies `change` to the rows, then emits [`PayeesEvent::Changed`]. Mutations go through
    /// here so no write can skip the event.
    pub fn mutate<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut Vec<Payee>) -> R,
    ) -> R {
        let result = change(self.service.payees_mut());
        cx.emit(PayeesEvent::Changed);
        result
    }
}

/// The Payees page's own state. `selected` is a position in the stored rows; `settings_selected` is
/// the Payee id the Settings Payees list has under the cursor.
pub struct PayeesView {
    store: Entity<PayeesStore>,
    selected: usize,
    settings_selected: Option<u32>,
}

impl PayeesView {
    pub fn new(store: Entity<PayeesStore>) -> Self {
        Self {
            store,
            selected: 0,
            settings_selected: None,
        }
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
