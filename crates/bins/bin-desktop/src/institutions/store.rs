//! The shared Institutions store (ADR-0032).

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_institutions::{InstitutionRow, add_institution};
use lib_ledger::Domain;

use crate::ledger_data::LedgerDataEvent;
use crate::ledger_store::{LedgerChanged, LedgerStore};

/// The shared Institutions table, read through [`LedgerStore`]. Settings edits it, and the Accounts
/// form reads it.
pub struct InstitutionsStore {
    ledger: Entity<LedgerStore>,
    _ledger_changed: Subscription,
}

impl EventEmitter<LedgerDataEvent> for InstitutionsStore {}

impl InstitutionsStore {
    /// A store over `ledger`'s Institutions.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Institutions) {
                cx.emit(LedgerDataEvent::Changed);
            }
        });
        Self {
            ledger,
            _ledger_changed,
        }
    }

    pub fn institutions<'a>(&self, cx: &'a App) -> &'a [InstitutionRow] {
        self.ledger.read(cx).ledger().institutions()
    }

    #[expect(
        deprecated,
        reason = "Institutions writes move onto Ledger commands in #647"
    )]
    pub fn add(&mut self, cx: &mut App, name: String, account_type: String) {
        self.ledger.update(cx, |ledger, cx| {
            ledger.write(cx, &[Domain::Institutions], |ledger| {
                add_institution(ledger.institutions, name, account_type)
            })
        });
    }
}
