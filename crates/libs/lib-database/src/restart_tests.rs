//! A Ledger file's writes, made through the ports an Open Ledger holds, read back after a
//! restart: the file's foreign keys let each cross-domain write through, and a reopen reads the
//! result as it was left.

use crate::{Commit, LedgerPorts, PayeesStore, PendingWrites, SharedStores, Stores};

/// An Open Ledger's ports with the stores they were built over, which a test reads the rows
/// back through, since `LedgerPorts` hands out only ports (#1212).
struct Opened {
    ports: LedgerPorts,
    stores: SharedStores,
}

/// Opens the storage on the Ledger file in `dir`, the Sample Ledger as of `today` when there is
/// none yet.
fn storage_in(dir: &std::path::Path, today: chrono::NaiveDate) -> crate::Storage {
    let path = dir.join("ledger.pldb");
    if !path.exists() {
        crate::seed::sample_ledger(&path, today).expect("the Sample Ledger is created");
    }
    crate::open_storage(&path).expect("the Ledger opens")
}

/// The stores on `storage`, as the plain `Opened` a test reads them through, with the pending
/// writes the test commits, as a Client commits an input's writes.
fn on_storage(storage: crate::Storage) -> (Opened, PendingWrites) {
    let on = on_storage_with_bills(storage);
    (on.store, on.pending)
}

/// The stores on the Ledger file, with the ports the modules that write beside them hold.
struct OnStorage {
    store: Opened,
    /// The Bills ports the store shares, which the Bills module writes.
    bills: lib_bills::SharedPorts,
    /// The payments port over the Transactions and Bills stores, the Bill Payments module's.
    payments: lib_bill_payments::SharedPayments,
    /// The deletion port over the Accounts, Transactions and Bills stores, the Account
    /// Deletion module's.
    deletion: lib_account_deletion::SharedDeletion,
    /// The deletion port over the Categories, Transactions, Bills, Payees and Budgets stores,
    /// the Category Deletion module's.
    category_deletion: lib_category_deletion::SharedCategoryDeletion,
    pending: PendingWrites,
}

/// [`on_storage`] with the ports the Bills, Bill Payments, Account Deletion and Category
/// Deletion modules write.
fn on_storage_with_bills(storage: crate::Storage) -> OnStorage {
    let pending = storage.pending.clone();
    let shared = Stores::from(storage).share();
    let stores = shared.stores.clone();
    let (built, _) = LedgerPorts::over(
        shared,
        lib_core::Clock::fixed(crate::ports::test_support::today()),
        crate::ports::test_support::secret(),
    );
    let bills = built.bills();
    let payments = built.bill_payments();
    let deletion = built.account_deletion().deletion;
    let category_deletion = built.category_deletion().deletion;
    OnStorage {
        store: Opened {
            ports: built,
            stores,
        },
        bills,
        payments,
        deletion,
        category_deletion,
        pending,
    }
}

/// Deletes Account `id` through the Account Deletion module's port, as its Dialog's confirm
/// does.
fn delete_account(
    on: &OnStorage,
    id: lib_core::AccountId,
) -> lib_account_deletion::Result<lib_account_deletion::AccountDeleted> {
    lib_account_deletion::interactor::DeleteAccount { id }.run(&mut *on.deletion.borrow_mut())
}

#[test]
fn a_deleted_accounts_transactions_survive_a_restart() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let today = crate::ports::test_support::today();
    let storage = storage_in(dir.path(), today);
    let writer = storage.writer.clone();
    let on = on_storage_with_bills(storage);
    let wallet = lib_accounts::sample_accounts()
        .into_iter()
        .find(|account| account.name == "Wallet")
        .expect("a sample Account")
        .id;

    delete_account(&on, wallet).expect("nothing pays from the Wallet");
    on.pending.commit();
    let before = transactions_of(&on.store);
    let OnStorage { store, pending, .. } = on;
    let reopened = restart((store, pending), writer, dir.path(), today);
    let after = transactions_of(&reopened);

    assert_eq!(after, before);
    assert!(
        after
            .iter()
            .all(|transaction| transaction.account_id != wallet)
    );
}

/// Category delete, through the Category Deletion module's port, re-points the Splits and Bill
/// Plans to an Uncategorised it creates and clears the Payee defaults before the delete, so the
/// file's foreign keys let it through, and a restart reads the result back.
#[test]
fn a_deleted_categorys_rows_survive_a_restart() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let today = crate::ports::test_support::today();
    let storage = storage_in(dir.path(), today);
    let writer = storage.writer.clone();
    let OnStorage {
        store,
        category_deletion,
        pending,
        ..
    } = on_storage_with_bills(storage);
    let gone = {
        let payees = payees_of(&store);
        categories_of(&store)
            .iter()
            .map(|category| category.id)
            .find(|&id| {
                plans_of(&store).iter().any(|plan| plan.category_id == id)
                    && payees.iter().any(|p| p.default_category == Some(id))
                    && transactions_of(&store)
                        .iter()
                        .any(|t| t.splits.iter().any(|s| s.category_id == id))
            })
            .expect("a sample Category with Splits, a Bill Plan and a Payee default")
    };

    let deleted = lib_category_deletion::interactor::DeleteCategory { id: gone }
        .run(&mut *category_deletion.borrow_mut())
        .expect("a leaf");
    // The port holds the stores; let them go before the restart reopens the file.
    drop(category_deletion);
    assert!(deleted.splits_moved > 0 && deleted.bill_plans_moved > 0);
    assert!(deleted.payee_defaults_cleared > 0);
    let reopened = restart((store, pending), writer, dir.path(), today);

    let categories = categories_of(&reopened);
    assert!(categories.iter().all(|c| c.id != gone));
    let home = lib_categories::find_by_name(&categories, "Uncategorised")
        .expect("the delete's Uncategorised is stored");
    let transactions = transactions_of(&reopened);
    let splits = transactions.iter().flat_map(|t| &t.splits);
    assert!(splits.clone().all(|s| s.category_id != gone));
    assert_eq!(
        splits.filter(|s| s.category_id == home).count(),
        deleted.splits_moved
    );
    assert!(plans_of(&reopened).iter().all(|p| p.category_id != gone));
    assert!(
        payees_of(&reopened)
            .iter()
            .all(|p| p.default_category != Some(gone))
    );
}

/// The Categories as the Categories store holds them.
fn categories_of(store: &Opened) -> Vec<lib_categories::Category> {
    store.stores.categories.borrow().categories().categories()
}

/// The Payees as the Payees store holds them.
fn payees_of(store: &Opened) -> Vec<lib_payees::Payee> {
    PayeesStore::payees(&*store.stores.payees.borrow()).payees()
}

/// The Bill Plans and the Bill Schedule as the Bills store holds them, the entries by due date
/// and then Plan, since a port lists them in no particular order.
fn bills_of(store: &Opened) -> (Vec<lib_bills::BillPlan>, Vec<lib_bills::BillScheduleEntry>) {
    (plans_of(store), entries_of(store))
}

/// The Transactions as the Transactions store holds them, newest first, since a store lists
/// them in no particular order.
fn transactions_of(store: &Opened) -> Vec<lib_transactions::Transaction> {
    let mut transactions = store.stores.transactions.borrow().all_transactions();
    transactions.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
    transactions
}

/// The Accounts as the Accounts store holds them.
fn accounts_of(store: &Opened) -> Vec<lib_accounts::Account> {
    store.stores.accounts.borrow().accounts().accounts()
}

/// The Bill Plans as the Bills store holds them.
fn plans_of(store: &Opened) -> Vec<lib_bills::BillPlan> {
    store.stores.bills.borrow().plans().bill_plans()
}

/// The Bill Schedule as the Bills store holds it, by due date and then Plan.
fn entries_of(store: &Opened) -> Vec<lib_bills::BillScheduleEntry> {
    let mut entries = store.stores.bills.borrow().entries().entries();
    entries.sort_by_key(|entry| (entry.due, entry.plan_id));
    entries
}

/// Every Split as the Bills ports read them from the Transactions store.
fn splits_of(store: &Opened) -> Vec<lib_bills::BillSplit> {
    store
        .ports
        .bills()
        .borrow()
        .transactions()
        .splits_between(chrono::NaiveDate::MIN, chrono::NaiveDate::MAX)
}

/// The Budgets as the Budgets store holds them.
fn budgets_of(store: &Opened) -> Vec<lib_budgets::Budget> {
    store.stores.budgets.borrow().budgets().budgets()
}

/// Commits what was written, as a Client commits an input's writes, then opens the Ledger in
/// `dir` again a day later, so a re-seed would show as shifted dates.
fn restart(
    (store, pending): (Opened, PendingWrites),
    writer: crate::Writer,
    dir: &std::path::Path,
    today: chrono::NaiveDate,
) -> Opened {
    pending.commit();
    // Stands for the writes a quit commits: the writer thread dies with the process.
    writer.flush().expect("the writer is running");
    drop(store);
    drop(writer);
    on_storage(storage_in(dir, today.succ_opt().expect("a date"))).0
}

#[test]
fn paying_matching_skipping_and_editing_a_plan_survive_a_restart() {
    use lib_bill_payments::interactor::PayBill;
    use lib_bills::interactor::{EditBillPlan, MatchBill, SkipBill};
    use lib_bills::{BillPlanDraft, Resolution};

    let dir = tempfile::tempdir().expect("a temporary directory");
    let today = crate::ports::test_support::today();
    let storage = storage_in(dir.path(), today);
    let writer = storage.writer.clone();
    let OnStorage {
        store,
        bills,
        payments,
        pending,
        ..
    } = on_storage_with_bills(storage);
    let (plans, entries) = bills_of(&store);
    let bill_accounts = bills.borrow().accounts().bill_accounts();
    let bill_categories = bills.borrow().categories().bill_categories();
    let (matched, split) = {
        entries
            .iter()
            .filter(|entry| entry.is_open())
            .find_map(|entry| {
                lib_bills::match_candidates(
                    &plans_of(&store),
                    &entries_of(&store),
                    &splits_of(&store),
                    &bill_accounts,
                    &bill_categories,
                    entry.id(),
                )
                .first()
                .map(|split| (entry.id(), *split))
            })
            .expect("the sample leaves an entry with a Match candidate")
    };
    let mut open = entries
        .iter()
        .filter(|entry| entry.is_open() && entry.id() != matched)
        .map(lib_bills::BillScheduleEntry::id);
    let paid = open.next().expect("a second open entry");
    let skipped = open
        .find(|entry| entry.plan_id != paid.plan_id)
        .expect("a third open entry");
    let plan = plans[0].clone();

    // Pay is the Bill Payments module's, through its port; Match, Skip and the edit are the
    // Bills module's, through its Interactors. One commit lands them all.
    PayBill {
        entry: paid,
        amount: "42.50".parse().expect("a decimal"),
        date: today,
    }
    .run(&mut *payments.borrow_mut())
    .expect("the entry is paid");
    {
        let mut ports = bills.borrow_mut();
        let ports = ports.split_mut();
        MatchBill {
            entry: matched,
            split,
        }
        .run(
            ports.bills,
            ports.transactions,
            ports.accounts,
            ports.categories,
        )
        .expect("the entry is matched");
        SkipBill { entry: skipped }
            .run(ports.bills)
            .expect("the entry is skipped");
        EditBillPlan {
            id: plan.id,
            draft: BillPlanDraft {
                name: "Renamed plan".into(),
                category_id: plan.category_id,
                unit: plan.unit,
                account_id: plan.account_id,
                payee_id: plan.payee_id,
                planned_amount: plan.planned_amount.clone(),
                amount_kind: plan.amount_kind,
                recurrence: plan.recurrence,
                first_due: plan.first_due,
                ends_on: plan.ends_on,
                attention_lead: plan.attention_lead,
            },
            is_active: plan.is_active,
            today,
        }
        .run(ports.bills, ports.accounts, ports.categories)
        .expect("the plan is edited");
    }
    let before = bills_of(&store);
    let transactions = transactions_of(&store);

    let reopened = restart((store, pending), writer, dir.path(), today);

    assert_eq!(bills_of(&reopened), before);
    let after = &before.1;
    let resolution = |id| lib_bills::entry(after, id).map(|entry| entry.resolution.clone());
    assert_eq!(resolution(matched), Some(Resolution::Paid(split)));
    assert!(matches!(
        resolution(skipped),
        Some(Resolution::Skipped { .. })
    ));
    let Some(Resolution::Paid(payment)) = resolution(paid) else {
        panic!("the paid entry stays Paid");
    };
    assert!(
        transactions_of(&reopened)
            .iter()
            .any(|t| t.id == payment.transaction_id),
        "Pay's Transaction is stored with it"
    );
    assert_eq!(transactions_of(&reopened), transactions);
    assert_eq!(before.0[0].name, "Renamed plan");
}

#[test]
fn deleting_an_account_leaves_the_entries_its_transactions_paid_unmatched_after_a_restart() {
    use lib_bills::Resolution;

    let dir = tempfile::tempdir().expect("a temporary directory");
    let today = crate::ports::test_support::today();
    let storage = storage_in(dir.path(), today);
    let writer = storage.writer.clone();
    let on = on_storage_with_bills(storage);
    let (entry, account) = entries_of(&on.store)
        .iter()
        .find_map(|entry| match entry.resolution {
            Resolution::Paid(split) => transactions_of(&on.store)
                .iter()
                .find(|t| t.id == split.transaction_id)
                .map(|t| (entry.id(), t.account_id)),
            _ => None,
        })
        .expect("the sample pays an entry");
    let resolution = |store: &Opened| {
        lib_bills::entry(&entries_of(store), entry).map(|entry| entry.resolution.clone())
    };

    // An Account a Bill Plan pays from can't be deleted, so its Plans move to another Account
    // first: what's tested is the Unmatch.
    let other = accounts_of(&on.store)
        .iter()
        .find(|a| a.id != account)
        .expect("a second Account")
        .id;
    {
        let mut ports = on.bills.borrow_mut();
        let plans = ports.bills().bill_plans();
        for mut plan in plans.into_iter().filter(|plan| plan.account_id == account) {
            plan.account_id = other;
            ports.split_mut().bills.upsert_bill_plan(plan);
        }
    }
    on.pending.commit();
    let budgets = budgets_of(&on.store);
    assert!(
        budgets.iter().any(|b| b.account_ids.contains(&account)),
        "a sample Budget counts the Account"
    );

    delete_account(&on, account).expect("no Plan pays from it now");
    assert_eq!(resolution(&on.store), Some(Resolution::Unresolved));
    assert!(accounts_of(&on.store).iter().all(|a| a.id != account));
    assert!(
        transactions_of(&on.store)
            .iter()
            .all(|t| t.account_id != account)
    );
    assert_eq!(
        budgets_of(&on.store),
        budgets,
        "the Budgets store keeps the id in memory until it reloads; a write skips it"
    );

    let OnStorage { store, pending, .. } = on;
    let reopened = restart((store, pending), writer, dir.path(), today);

    assert_eq!(resolution(&reopened), Some(Resolution::Unresolved));
    assert!(accounts_of(&reopened).iter().all(|a| a.id != account));
    assert!(
        budgets_of(&reopened)
            .iter()
            .all(|b| !b.account_ids.contains(&account)),
        "the file dropped the Account from every Budget"
    );
}

/// An Account a Bill Plan pays from is refused by `lib-account-deletion` before anything is
/// written, so a restart finds the Account and its Transactions as they were.
#[test]
fn deleting_an_account_a_bill_plan_pays_from_is_refused_whole() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let today = crate::ports::test_support::today();
    let storage = storage_in(dir.path(), today);
    let writer = storage.writer.clone();
    let on = on_storage_with_bills(storage);
    let (account, plans, booked) = {
        let account = plans_of(&on.store)[0].account_id;
        let plans = plans_of(&on.store)
            .iter()
            .filter(|plan| plan.account_id == account)
            .count();
        let booked = transactions_of(&on.store)
            .iter()
            .filter(|t| t.account_id == account)
            .count();
        (account, plans, booked)
    };
    assert!(booked > 0, "the sample books to a Bill Plan's Account");

    assert_eq!(
        delete_account(&on, account),
        Err(lib_account_deletion::Error::PaidFromByBillPlans { plans })
    );

    let OnStorage { store, pending, .. } = on;
    let reopened = restart((store, pending), writer, dir.path(), today);
    assert!(accounts_of(&reopened).iter().any(|a| a.id == account));
    assert_eq!(
        transactions_of(&reopened)
            .iter()
            .filter(|t| t.account_id == account)
            .count(),
        booked
    );
}
