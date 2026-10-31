//! The Ledger's Tags are a read copy of the Tags port (#862): a Client refreshes it from the port
//! after each Tags write and on rollback ([`Ledger::refresh_tags`]), for the Views that still read
//! Tags from the Ledger: the Tags page and Transactions. The Tags writers still run as
//! `LedgerCommand`s, which a Client forwards to the port, until the Tags page writes it itself
//! (#863) and Tag delete and merge move to `lib-reassign` (#813). The copy goes when the last
//! reader has its own port.
//!
//! [`Ledger::tag_uses`] counts what carries each Tag from the Ledger's Transactions, for a Client
//! whose Tags port is in memory and can't count them itself. The Tags adapter counts the same rows
//! in the file.

use std::collections::HashMap;

use lib_core::TagId;
use lib_tags::{Tag, TagUsage, TagUse};

use crate::Ledger;
use crate::tag_usage::TagRows;

impl Ledger {
    /// Replaces the read copy of the Tags with what the Tags port holds, in the port's order.
    /// Returns whether anything changed, so a Client announces only a real change.
    pub fn refresh_tags(&mut self, tags: Vec<Tag>) -> bool {
        if self.tags == tags {
            return false;
        }
        self.tags = tags;
        true
    }

    /// What carries each Tag, counted from the Transactions and Accounts; a Tag nothing carries
    /// is left out.
    pub fn tag_uses(&self) -> HashMap<TagId, TagUse> {
        let rows = TagRows {
            transactions: &self.transactions,
            accounts: &self.accounts,
            unit_code: &|unit| self.unit_code(unit).to_string(),
        };
        self.tags
            .iter()
            .map(|tag| (tag.id, rows.tag_usage(tag.id)))
            .filter(|(_, used)| used.transactions > 0)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use lib_core::Total;

    use super::*;
    use crate::tag_usage::transaction_count;

    #[test]
    fn a_refresh_takes_the_ports_rows_and_reports_only_a_change() {
        let mut ledger = Ledger::stub();
        let mut tags = ledger.tags().to_vec();

        assert!(!ledger.refresh_tags(tags.clone()));
        tags[0].name = "Japan".into();
        assert!(ledger.refresh_tags(tags.clone()));

        assert_eq!(ledger.tags(), tags);
    }

    #[test]
    fn tag_uses_count_the_transactions_carrying_each_tag_and_name_the_unit_by_code() {
        let ledger = Ledger::stub();
        let shared = lib_tags::find_by_name(ledger.tags(), "shared").expect("a stub Tag");

        let uses = ledger.tag_uses();
        let used = uses.get(&shared).expect("the stub's Splits carry it");

        assert_eq!(
            used.transactions,
            transaction_count(ledger.transactions(), shared)
        );
        assert!(used.last_used.is_some());
        assert!(matches!(used.total, Total::Single { .. } | Total::Mixed));
        assert!(!uses.contains_key(&TagId::seed(999)));
    }
}
