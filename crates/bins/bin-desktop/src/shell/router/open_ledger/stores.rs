//! What an Open Ledger holds a Ledger by (#1085): the `LedgerPorts` [`super::OpenLedger::build`] is
//! given, and nothing a Client builds beside them. The Open Ledger keeps them for what runs after
//! it is built: a failed write's reload (`write_failures.rs`), a new month's Bill Schedule and the
//! `*_for_test` hooks `Root`'s test support calls.

use std::rc::Rc;
use std::sync::mpsc;

use lib_database::{LedgerPorts, PayeesStore};

/// What an Open Ledger holds a Ledger by: the `LedgerPorts` `lib-database` built, nothing a Client
/// builds beside them. The raw stores stay reachable through `ports.stores` for the reload after a
/// failed write and the `*_for_test` hooks.
pub struct LedgerStores {
    pub ports: LedgerPorts,
}

/// [`LedgerStores::build`]'s result: the stores, and the commit and failure channel `ports` came
/// with.
pub struct BuiltStores {
    pub stores: LedgerStores,
    pub commit: Rc<dyn lib_database::Commit>,
    pub failures: mpsc::Receiver<lib_database::WriteFailure>,
}

impl LedgerStores {
    /// The Sample Ledger's stores for `today`, in memory, for a port test that reads the same
    /// rows as the UI tests without building the whole Open Ledger.
    #[cfg(test)]
    pub fn sample(today: chrono::NaiveDate) -> Self {
        Self::build(LedgerPorts::new(
            lib_database::Stores::fake(lib_database::FakeStores::sample(today)).0,
            lib_core::Clock::fixed(today),
        ))
        .stores
    }

    /// Holds `ports`, taking its failure channel out for the Open Ledger to poll.
    pub fn build(mut ports: LedgerPorts) -> BuiltStores {
        let commit = ports.commit.clone();
        let failures = std::mem::replace(&mut ports.failures, mpsc::channel().1);
        BuiltStores {
            stores: Self { ports },
            commit,
            failures,
        }
    }

    /// Every domain's store, so a domain reloads by its `Domain` in one call: after a failed
    /// write, or a commit's cascade (#1081).
    pub fn shared(&self) -> &lib_database::SharedStores {
        &self.ports.stores
    }

    /// Adds a Location priced in `unit` to the Inventory store, so a test can delete a Unit one
    /// names (#885).
    #[doc(hidden)]
    pub fn add_location_in_for_test(&self, unit: lib_core::UnitId) {
        let mut scratch = lib_inventory::Inventory::default();
        let draft = lib_inventory::LocationDraft {
            name: "Beach shack".to_string(),
            ..lib_inventory::LocationDraft::default()
        };
        if scratch.add_location(&draft, unit).is_err() {
            return;
        }
        let Some(location) = scratch.locations.pop() else {
            return;
        };
        self.ports
            .stores
            .inventory
            .borrow_mut()
            .inventory_mut()
            .upsert_location(location);
    }

    /// Removes every Location from the Inventory store through the Interactor, as the Remove
    /// location Dialog does.
    #[doc(hidden)]
    pub fn empty_inventory_for_test(&self) {
        let ids: Vec<lib_core::LocationId> = self
            .ports
            .stores
            .inventory
            .borrow()
            .inventory()
            .locations()
            .iter()
            .map(|location| location.id)
            .collect();
        for id in ids {
            let removed = lib_inventory::interactor::RemoveLocation(id)
                .run(self.ports.stores.inventory.borrow_mut().inventory_mut());
            assert!(
                removed.is_ok(),
                "a Location listed a moment ago must still be removable"
            );
        }
    }

    /// Writes an empty default Budget in `unit` straight to the Budgets store, which then names
    /// the Unit.
    #[doc(hidden)]
    pub fn add_budget_in_for_test(&self, unit: lib_core::UnitId) {
        let budget = lib_budgets::personal_spending(unit, &[]);
        self.ports
            .stores
            .budgets
            .borrow_mut()
            .budgets_mut()
            .upsert_budget(&budget);
    }

    /// Writes a Payee called `name` straight to the Payees store, so a test can meet the refusal
    /// of an Import that creates the same name.
    #[doc(hidden)]
    pub fn add_payee_for_test(&self, name: &str) {
        PayeesStore::payees_mut(&mut *self.ports.stores.payees.borrow_mut()).upsert_payee(
            lib_payees::Payee {
                id: lib_core::PayeeId::new(),
                name: name.to_owned(),
                aliases: Vec::new(),
                default_category: None,
                is_active: true,
            },
        );
    }

    /// Gives Institution `id` Unit `unit` as its Usual unit, so a test can delete a Unit an
    /// Institution names.
    #[doc(hidden)]
    pub fn name_usual_unit_for_test(&self, id: lib_core::InstitutionId, unit: lib_core::UnitId) {
        let mut store = self.ports.stores.institutions.borrow_mut();
        let Some(mut institution) = store.institutions().institution(id) else {
            return;
        };
        institution.usual_unit = Some(unit);
        store.institutions_mut().upsert_institution(institution);
    }
}

#[cfg(test)]
mod tests {

    use gpui::{AppContext, Entity, TestAppContext};
    use lib_database::{Commit, PendingWrites, Stores};

    use super::*;

    /// The Desktop's start-up storage on the Ledger file in `dir`, the Sample Ledger as of `today`
    /// when there is none yet.
    fn storage_in(dir: &std::path::Path, today: chrono::NaiveDate) -> lib_database::Storage {
        let path = dir.join("ledger.pldb");
        if !path.exists() {
            lib_database::seed::sample_ledger(&path, today).expect("the Sample Ledger is created");
        }
        lib_database::open_storage(&path).expect("the Ledger opens")
    }

    /// The stores on `storage`, as an Entity a test reads them through, with the pending writes
    /// the test commits, as an input's `send` would.
    fn on_storage(
        cx: &mut TestAppContext,
        storage: lib_database::Storage,
    ) -> (Entity<LedgerStores>, PendingWrites) {
        let on = on_storage_with_bills(cx, storage);
        (on.store, on.pending)
    }

    /// The stores on the Ledger file, with the ports the modules that write beside them hold.
    struct OnStorage {
        store: Entity<LedgerStores>,
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
    fn on_storage_with_bills(cx: &mut TestAppContext, storage: lib_database::Storage) -> OnStorage {
        let pending = storage.pending.clone();
        let built = LedgerStores::build(LedgerPorts::new(
            Stores::from(storage),
            lib_core::Clock::fixed(chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a date")),
        ))
        .stores;
        let bills = built.ports.bills.clone();
        let payments = built.ports.bill_payments.clone();
        let deletion = built.ports.account_deletion.clone();
        let category_deletion = built.ports.category_deletion.clone();
        OnStorage {
            store: cx.new(|_| built),
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

    #[gpui::test]
    fn a_deleted_accounts_transactions_survive_a_restart(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let on = on_storage_with_bills(cx, storage);
        let store = on.store.clone();
        let wallet = lib_accounts::sample_accounts()
            .into_iter()
            .find(|account| account.name == "Wallet")
            .expect("a sample Account")
            .id;

        delete_account(&on, wallet).expect("nothing pays from the Wallet");
        on.pending.commit();
        let before = store.read_with(cx, |store, _| transactions_of(store));
        crate::quit(&writer);
        drop(store);
        drop(on);
        drop(writer);

        // The next day, so a re-seed would show as shifted dates.
        let (reopened, _) = on_storage(
            cx,
            storage_in(dir.path(), today.succ_opt().expect("a date")),
        );
        let after = reopened.read_with(cx, |store, _| transactions_of(store));

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
    #[gpui::test]
    fn a_deleted_categorys_rows_survive_a_restart(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let OnStorage {
            store,
            category_deletion,
            pending,
            ..
        } = on_storage_with_bills(cx, storage);
        let gone = store.read_with(cx, |store, _| {
            let payees = payees_of(store);
            categories_of(store)
                .iter()
                .map(|category| category.id)
                .find(|&id| {
                    plans_of(store).iter().any(|plan| plan.category_id == id)
                        && payees.iter().any(|p| p.default_category == Some(id))
                        && transactions_of(store)
                            .iter()
                            .any(|t| t.splits.iter().any(|s| s.category_id == id))
                })
                .expect("a sample Category with Splits, a Bill Plan and a Payee default")
        });

        let deleted = lib_category_deletion::interactor::DeleteCategory { id: gone }
            .run(&mut *category_deletion.borrow_mut())
            .expect("a leaf");
        // The port holds the stores; let them go before the restart reopens the file.
        drop(category_deletion);
        assert!(deleted.splits_moved > 0 && deleted.bill_plans_moved > 0);
        assert!(deleted.payee_defaults_cleared > 0);
        let reopened = restart(cx, (store, pending), writer, dir.path(), today);

        reopened.read_with(cx, |store, _| {
            let categories = categories_of(store);
            assert!(categories.iter().all(|c| c.id != gone));
            let home = lib_categories::find_by_name(&categories, "Uncategorised")
                .expect("the delete's Uncategorised is stored");
            let transactions = transactions_of(store);
            let splits = transactions.iter().flat_map(|t| &t.splits);
            assert!(splits.clone().all(|s| s.category_id != gone));
            assert_eq!(
                splits.filter(|s| s.category_id == home).count(),
                deleted.splits_moved
            );
            assert!(plans_of(store).iter().all(|p| p.category_id != gone));
            assert!(
                payees_of(store)
                    .iter()
                    .all(|p| p.default_category != Some(gone))
            );
        });
    }

    /// The Categories as the Categories store holds them.
    fn categories_of(store: &LedgerStores) -> Vec<lib_categories::Category> {
        store
            .ports
            .stores
            .categories
            .borrow()
            .categories()
            .categories()
    }

    /// The Payees as the Payees store holds them.
    fn payees_of(store: &LedgerStores) -> Vec<lib_payees::Payee> {
        PayeesStore::payees(&*store.ports.stores.payees.borrow()).payees()
    }

    /// The Bill Plans and the Bill Schedule as the Bills store holds them, the entries by due date
    /// and then Plan, since a port lists them in no particular order.
    fn bills_of(
        cx: &mut TestAppContext,
        store: &Entity<LedgerStores>,
    ) -> (Vec<lib_bills::BillPlan>, Vec<lib_bills::BillScheduleEntry>) {
        store.read_with(cx, |store, _| (plans_of(store), entries_of(store)))
    }

    /// The Transactions as the Transactions store holds them, newest first, since a store lists
    /// them in no particular order.
    fn transactions_of(store: &LedgerStores) -> Vec<lib_transactions::Transaction> {
        let mut transactions = store.ports.stores.transactions.borrow().all_transactions();
        transactions.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
        transactions
    }

    /// The Accounts as the Accounts store holds them.
    fn accounts_of(store: &LedgerStores) -> Vec<lib_accounts::Account> {
        store.ports.stores.accounts.borrow().accounts().accounts()
    }

    /// The Bill Plans as the Bills store holds them.
    fn plans_of(store: &LedgerStores) -> Vec<lib_bills::BillPlan> {
        store.ports.stores.bills.borrow().plans().bill_plans()
    }

    /// The Bill Schedule as the Bills store holds it, by due date and then Plan.
    fn entries_of(store: &LedgerStores) -> Vec<lib_bills::BillScheduleEntry> {
        let mut entries = store.ports.stores.bills.borrow().entries().entries();
        entries.sort_by_key(|entry| (entry.due, entry.plan_id));
        entries
    }

    /// Every Split as the Bills ports read them from the Transactions store.
    fn splits_of(store: &LedgerStores) -> Vec<lib_bills::BillSplit> {
        store
            .ports
            .bills
            .borrow()
            .transactions()
            .splits_between(chrono::NaiveDate::MIN, chrono::NaiveDate::MAX)
    }

    /// The Budgets as the Budgets store holds them.
    fn budgets_of(store: &LedgerStores) -> Vec<lib_budgets::Budget> {
        store.ports.stores.budgets.borrow().budgets().budgets()
    }

    /// Commits what was written, as the input's `send` would, then opens the Ledger in `dir`
    /// again a day later, so a re-seed would show as shifted dates.
    fn restart(
        cx: &mut TestAppContext,
        (store, pending): (Entity<LedgerStores>, PendingWrites),
        writer: lib_database::Writer,
        dir: &std::path::Path,
        today: chrono::NaiveDate,
    ) -> Entity<LedgerStores> {
        pending.commit();
        crate::quit(&writer);
        drop(store);
        drop(writer);
        on_storage(cx, storage_in(dir, today.succ_opt().expect("a date"))).0
    }

    #[gpui::test]
    fn paying_matching_skipping_and_editing_a_plan_survive_a_restart(cx: &mut TestAppContext) {
        use lib_bill_payments::interactor::PayBill;
        use lib_bills::interactor::{EditBillPlan, MatchBill, SkipBill};
        use lib_bills::{BillPlanDraft, Resolution};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let OnStorage {
            store,
            bills,
            payments,
            pending,
            ..
        } = on_storage_with_bills(cx, storage);
        let (plans, entries) = bills_of(cx, &store);
        let bill_accounts = bills.borrow().accounts().bill_accounts();
        let bill_categories = bills.borrow().categories().bill_categories();
        let (matched, split) = store.read_with(cx, |store, _| {
            entries
                .iter()
                .filter(|entry| entry.is_open())
                .find_map(|entry| {
                    lib_bills::match_candidates(
                        &plans_of(store),
                        &entries_of(store),
                        &splits_of(store),
                        &bill_accounts,
                        &bill_categories,
                        entry.id(),
                    )
                    .first()
                    .map(|split| (entry.id(), *split))
                })
                .expect("the sample leaves an entry with a Match candidate")
        });
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
        let before = bills_of(cx, &store);
        let transactions = store.read_with(cx, |store, _| transactions_of(store));

        let reopened = restart(cx, (store, pending), writer, dir.path(), today);

        assert_eq!(bills_of(cx, &reopened), before);
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
            reopened.read_with(cx, |store, _| transactions_of(store)
                .iter()
                .any(|t| t.id == payment.transaction_id)),
            "Pay's Transaction is stored with it"
        );
        assert_eq!(
            reopened.read_with(cx, |store, _| transactions_of(store)),
            transactions
        );
        assert_eq!(before.0[0].name, "Renamed plan");
    }

    #[gpui::test]
    fn deleting_an_account_leaves_the_entries_its_transactions_paid_unmatched_after_a_restart(
        cx: &mut TestAppContext,
    ) {
        use lib_bills::Resolution;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let on = on_storage_with_bills(cx, storage);
        let (entry, account) = on.store.read_with(cx, |store, _| {
            entries_of(store)
                .iter()
                .find_map(|entry| match entry.resolution {
                    Resolution::Paid(split) => transactions_of(store)
                        .iter()
                        .find(|t| t.id == split.transaction_id)
                        .map(|t| (entry.id(), t.account_id)),
                    _ => None,
                })
                .expect("the sample pays an entry")
        });
        let resolution = |cx: &mut TestAppContext, store: &Entity<LedgerStores>| {
            store.read_with(cx, |store, _| {
                lib_bills::entry(&entries_of(store), entry).map(|entry| entry.resolution.clone())
            })
        };

        // An Account a Bill Plan pays from can't be deleted, so its Plans move to another Account
        // first: what's tested is the Unmatch.
        let other = on.store.read_with(cx, |store, _| {
            accounts_of(store)
                .iter()
                .find(|a| a.id != account)
                .expect("a second Account")
                .id
        });
        {
            let mut ports = on.bills.borrow_mut();
            let plans = ports.bills().bill_plans();
            for mut plan in plans.into_iter().filter(|plan| plan.account_id == account) {
                plan.account_id = other;
                ports.split_mut().bills.upsert_bill_plan(plan);
            }
        }
        on.pending.commit();
        let budgets = on.store.read_with(cx, |store, _| budgets_of(store));
        assert!(
            budgets.iter().any(|b| b.account_ids.contains(&account)),
            "a sample Budget counts the Account"
        );

        delete_account(&on, account).expect("no Plan pays from it now");
        assert_eq!(resolution(cx, &on.store), Some(Resolution::Unresolved));
        on.store.read_with(cx, |store, _| {
            assert!(accounts_of(store).iter().all(|a| a.id != account));
            assert!(
                transactions_of(store)
                    .iter()
                    .all(|t| t.account_id != account)
            );
            assert_eq!(
                budgets_of(store),
                budgets,
                "the Budgets store keeps the id in memory until it reloads; a write skips it"
            );
        });

        let OnStorage { store, pending, .. } = on;
        let reopened = restart(cx, (store, pending), writer, dir.path(), today);

        assert_eq!(resolution(cx, &reopened), Some(Resolution::Unresolved));
        reopened.read_with(cx, |store, _| {
            assert!(accounts_of(store).iter().all(|a| a.id != account));
            assert!(
                budgets_of(store)
                    .iter()
                    .all(|b| !b.account_ids.contains(&account)),
                "the file dropped the Account from every Budget"
            );
        });
    }

    /// An Account a Bill Plan pays from is refused by `lib-account-deletion` before anything is
    /// written, so a restart finds the Account and its Transactions as they were.
    #[gpui::test]
    fn deleting_an_account_a_bill_plan_pays_from_is_refused_whole(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let on = on_storage_with_bills(cx, storage);
        let (account, plans, booked) = on.store.read_with(cx, |store, _| {
            let account = plans_of(store)[0].account_id;
            let plans = plans_of(store)
                .iter()
                .filter(|plan| plan.account_id == account)
                .count();
            let booked = transactions_of(store)
                .iter()
                .filter(|t| t.account_id == account)
                .count();
            (account, plans, booked)
        });
        assert!(booked > 0, "the sample books to a Bill Plan's Account");

        assert_eq!(
            delete_account(&on, account),
            Err(lib_account_deletion::Error::PaidFromByBillPlans { plans })
        );

        let OnStorage { store, pending, .. } = on;
        let reopened = restart(cx, (store, pending), writer, dir.path(), today);
        reopened.read_with(cx, |store, _| {
            assert!(accounts_of(store).iter().any(|a| a.id == account));
            assert_eq!(
                transactions_of(store)
                    .iter()
                    .filter(|t| t.account_id == account)
                    .count(),
                booked
            );
        });
    }
}
