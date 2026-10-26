//! The Accounts store as the Desktop holds it (#853): the store `shell::router::build` is given,
//! shared by the Ledger's forwarding shim and read copy, the Units ports (`UnitHoldings` and the
//! Accounts half of `UnitUsage`) and the Bills ports (`BillAccounts`). The writes are committed by
//! the input they were made in (`crate::input::send`), not here.
//!
//! The legacy Accounts page's Add and Edit are still `LedgerCommand`s until the page writes the
//! port itself (#855), and [`forward`] carries them to the store. Account delete writes it through
//! `lib-account-deletion`'s port (#854, `crate::account_deletion`).
//!
//! An in-memory store's Transactions count is recounted from the Ledger before each read
//! ([`recount`]), since it has no file to count in; the adapter counts in the file. The Budgets
//! count is the Ledger's until Budgets persist (#811).

use lib_accounts::AccountUse;
use lib_core::AccountId;
use lib_database::{AccountsStore, SharedAccountsStore};
use lib_ledger::{Ledger, LedgerChange};

/// `ledger`'s own Accounts in memory, for a test that needs no storage.
pub fn in_memory(ledger: &Ledger) -> SharedAccountsStore {
    let store = std::rc::Rc::new(std::cell::RefCell::new(lib_accounts::fake::InMemoryPorts {
        accounts: lib_accounts::fake::InMemoryAccounts {
            accounts: ledger.accounts().to_vec(),
        },
        ..Default::default()
    }));
    recount(&*store, ledger);
    store
}

/// The forwarding shim (#853): sends each Account row `changes` upserts to `store`, in the order
/// recorded, returning whether it wrote anything. Run before the Transactions, which may name a new
/// Account. Writers carried: the legacy Accounts page's Add and Edit.
pub fn forward(store: &mut dyn AccountsStore, changes: &[LedgerChange]) -> bool {
    let port = store.accounts_mut();
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::UpsertAccount(account) = change {
            port.upsert_account(account.clone());
            wrote = true;
        }
    }
    wrote
}

/// Recounts an in-memory store's usage from `ledger`'s Transactions and Budgets; the adapter
/// counts in the file, so it is left alone.
pub fn recount(store: &std::cell::RefCell<dyn AccountsStore>, ledger: &Ledger) {
    if let Some(usage) = store.borrow_mut().usage_to_count() {
        usage.uses = ledger
            .accounts()
            .iter()
            .map(|account| (account.id, ledger.account_use(account.id)))
            .collect();
    }
}

/// What names Account `id`, as its Edit and Delete Dialogs show: the Transactions from the store,
/// the Budgets from `ledger` until Budgets persist (#811).
pub fn usage(
    store: &std::cell::RefCell<dyn AccountsStore>,
    ledger: &Ledger,
    id: AccountId,
) -> AccountUse {
    recount(store, ledger);
    AccountUse {
        transactions: store.borrow().usage().account_usage(id).transactions,
        budgets: ledger.account_use(id).budgets,
    }
}

#[cfg(test)]
mod tests {
    use lib_core::TagId;

    use super::*;

    #[test]
    fn account_upserts_reach_the_store() {
        let ledger = Ledger::stub();
        let store = in_memory(&ledger);
        let mut wallet = ledger.accounts()[0].clone();
        wallet.name = "Purse".into();
        let changes = [
            LedgerChange::UpsertAccount(wallet.clone()),
            LedgerChange::DeleteTag(TagId::seed(1)),
        ];

        assert!(forward(&mut *store.borrow_mut(), &changes));

        assert_eq!(store.borrow().accounts().account(wallet.id), Some(wallet));
        assert!(!forward(
            &mut lib_accounts::fake::InMemoryPorts::default(),
            &changes[1..]
        ));
    }

    #[test]
    fn an_in_memory_stores_usage_is_counted_from_the_ledger() {
        let ledger = Ledger::stub();
        let store = in_memory(&ledger);
        let everyday = AccountId::seed(2);

        assert_eq!(
            usage(&*store, &ledger, everyday),
            ledger.account_use(everyday)
        );
        assert!(usage(&*store, &ledger, everyday).transactions > 0);
    }
}
