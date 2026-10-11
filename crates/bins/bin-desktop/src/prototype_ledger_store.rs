//! PROTOTYPE (#593), throwaway: the single `LedgerStore` Entity ADR-0038 names, run beside the
//! per-domain stores so one flow (Payee delete) can go through it. Never merged.

use std::collections::BTreeSet;

use gpui::{Context, EventEmitter};
use lib_ledger::{
    Ledger,
    prototype_commands::{Domain, LedgerChange, LedgerCommand, domains},
};

/// Emitted after a command that changed something, naming every domain it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerChanged {
    pub what: BTreeSet<Domain>,
}

pub struct LedgerStore {
    ledger: Ledger,
    /// Stands in for the stub repository adapter (ADR-0039): every drained change lands here.
    applied: Vec<LedgerChange>,
}

impl EventEmitter<LedgerChanged> for LedgerStore {}

impl LedgerStore {
    pub fn new(ledger: Ledger) -> Self {
        Self {
            ledger,
            applied: Vec::new(),
        }
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    #[cfg_attr(not(test), expect(dead_code, reason = "prototype: read by tests only"))]
    pub fn applied(&self) -> &[LedgerChange] {
        &self.applied
    }

    /// The Dialog host's `Apply` for every command: run it, hand its changes to the repository,
    /// and announce the domains it touched. A refusal changes nothing and emits nothing.
    pub fn apply<C: LedgerCommand>(
        &mut self,
        cx: &mut Context<'_, Self>,
        command: C,
    ) -> Result<C::Outcome, C::Refusal> {
        let outcome = command.run(&mut self.ledger)?;
        let changes = self.ledger.drain_changes();
        if !changes.is_empty() {
            let what = domains(&changes);
            self.applied.extend(changes);
            cx.emit(LedgerChanged { what });
        }
        Ok(outcome)
    }
}
