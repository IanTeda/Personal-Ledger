//! The Ledger's Inventory is a read copy of the Inventory port (#866): a Client refreshes it from
//! the port after each Inventory write and on rollback ([`Ledger::refresh_inventory`]), for the
//! readers still on the Ledger: Documents, which resolve a Link to an Item by id until they have
//! their own port (#812), and the Inventory page until it runs on `lib-inventory`'s Presenters
//! (#867). The Inventory writers still run as `LedgerCommand`s, which a Client forwards to the
//! port, until the page writes it itself (#867) and Remove Location moves to `lib-reassign`
//! (#813).

use lib_inventory::Inventory;

use crate::Ledger;

impl Ledger {
    /// Replaces the read copy of the Inventory with what the Inventory port holds, in the port's
    /// order. Returns whether anything changed, so a Client announces only a real change.
    pub fn refresh_inventory(&mut self, inventory: Inventory) -> bool {
        if self.inventory == inventory {
            return false;
        }
        self.inventory = inventory;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refresh_takes_the_ports_rows_and_reports_only_a_change() {
        let mut ledger = Ledger::stub();
        let mut inventory = ledger.inventory().clone();

        assert!(!ledger.refresh_inventory(inventory.clone()));
        inventory.locations[0].areas.swap(0, 1);
        assert!(ledger.refresh_inventory(inventory.clone()));

        assert_eq!(ledger.inventory(), &inventory);
    }
}
