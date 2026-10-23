//! The in-memory Bills ports, for tests and UI-first work. Each holds its rows in plain fields so a
//! test can set up or read state directly. The stub seed is `seed`'s [`default_bills`](crate::default_bills),
//! joined in [`InMemoryPorts::stub`] with fixed Accounts, Categories and Payees ([`stub_accounts`],
//! [`stub_categories`], [`stub_payees`]) that carry the stub's ids and names, so this crate needs
//! none of those domains. [`RecordingRouter`] records what a Presenter asked for.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use lib_core::{AccountId, BillPlanId, CategoryId, Money, PayeeId, TransactionId, UnitId};
use lib_toast::{Notice, Notify};

use crate::protocols::{
    BillUnits, BillsOutcome, BillsPorts, BillsRepository, BillsRouter, PayRequest, PortsMut,
    SharedPorts,
};
use crate::{
    BillAccount, BillAccounts, BillCategories, BillCategory, BillPayee, BillPayees, BillPlan,
    BillPlanRepository, BillScheduleEntry, BillScheduleRepository, BillSplit, BillTransactions,
    BillsSeed, EntryId, Resolution, SplitRef, StubBills,
};

/// The Bill Plans and the Bill Schedule, stored in memory.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryBills {
    pub plans: Vec<BillPlan>,
    pub entries: Vec<BillScheduleEntry>,
}

impl From<BillsSeed> for InMemoryBills {
    fn from(seed: BillsSeed) -> Self {
        Self {
            plans: seed.plans,
            entries: seed.entries,
        }
    }
}

impl BillPlanRepository for InMemoryBills {
    fn bill_plans(&self) -> Vec<BillPlan> {
        self.plans.clone()
    }

    fn upsert_bill_plan(&mut self, plan: BillPlan) {
        match self.plans.iter_mut().find(|row| row.id == plan.id) {
            Some(row) => *row = plan,
            None => self.plans.push(plan),
        }
    }
}

impl BillScheduleRepository for InMemoryBills {
    fn entries(&self) -> Vec<BillScheduleEntry> {
        self.entries.clone()
    }

    fn upsert_entry(&mut self, entry: BillScheduleEntry) {
        match self.entries.iter_mut().find(|row| row.id() == entry.id()) {
            Some(row) => {
                *row = BillScheduleEntry {
                    row_id: row.row_id,
                    ..entry
                }
            }
            None => self.entries.push(entry),
        }
    }
}

/// The other domains' rows Bills reads, held in memory.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryBillReads {
    pub accounts: Vec<BillAccount>,
    pub categories: Vec<BillCategory>,
    pub payees: Vec<BillPayee>,
    pub splits: Vec<BillSplit>,
}

impl BillTransactions for InMemoryBillReads {
    fn splits_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<BillSplit> {
        crate::splits_between(&self.splits, from, to)
    }

    fn paid_split(&self, split: SplitRef) -> Option<BillSplit> {
        crate::split_of(&self.splits, split).cloned()
    }
}

impl BillAccounts for InMemoryBillReads {
    fn bill_accounts(&self) -> Vec<BillAccount> {
        self.accounts.clone()
    }
}

impl BillCategories for InMemoryBillReads {
    fn bill_categories(&self) -> Vec<BillCategory> {
        self.categories.clone()
    }
}

impl BillPayees for InMemoryBillReads {
    fn bill_payees(&self) -> Vec<BillPayee> {
        self.payees.clone()
    }
}

/// The Units Bills names, held in memory as `(id, code)`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryBillUnits {
    pub codes: Vec<(UnitId, String)>,
    pub base: Option<UnitId>,
}

impl InMemoryBillUnits {
    /// The stub's Units, aud the base.
    pub fn stub() -> Self {
        Self {
            codes: vec![
                (UnitId::SEED_AUD, "aud".to_string()),
                (UnitId::SEED_BTC, "btc".to_string()),
                (UnitId::SEED_VAS, "vas".to_string()),
            ],
            base: Some(UnitId::SEED_AUD),
        }
    }
}

impl BillUnits for InMemoryBillUnits {
    fn unit_code(&self, id: UnitId) -> Option<String> {
        self.codes
            .iter()
            .find(|(unit, _)| *unit == id)
            .map(|(_, code)| code.clone())
    }

    fn base_unit(&self) -> Option<UnitId> {
        self.base
    }
}

/// Every Bills port in memory, as the Presenters share them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryPorts {
    pub bills: InMemoryBills,
    pub reads: InMemoryBillReads,
    pub units: InMemoryBillUnits,
}

impl InMemoryPorts {
    /// The stub Bill Plans and their Schedule as of `today`, with the fixed Accounts, Categories
    /// and Payees they name and the Splits their Paid entries were paid by: the stub Ledger's
    /// Bills, with the Transactions numbered from 1 rather than after the other stubs'.
    pub fn stub(today: NaiveDate) -> Self {
        let accounts = stub_accounts();
        let categories = stub_categories();
        let payees = stub_payees();
        let mut splits = Vec::new();
        let seed = book(
            crate::default_bills(&accounts, &categories, &payees, today),
            &mut splits,
        );
        Self {
            bills: seed.into(),
            reads: InMemoryBillReads {
                accounts,
                categories,
                payees,
                splits,
            },
            units: InMemoryBillUnits::stub(),
        }
    }

    /// These ports shared as the Presenters hold them, with the handle a test reads them through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedPorts) {
        let ports = Rc::new(RefCell::new(self));
        let shared: SharedPorts = ports.clone();
        (ports, shared)
    }
}

impl BillsPorts for InMemoryPorts {
    fn bills(&self) -> &dyn BillsRepository {
        &self.bills
    }

    fn transactions(&self) -> &dyn BillTransactions {
        &self.reads
    }

    fn accounts(&self) -> &dyn BillAccounts {
        &self.reads
    }

    fn categories(&self) -> &dyn BillCategories {
        &self.reads
    }

    fn payees(&self) -> &dyn BillPayees {
        &self.reads
    }

    fn units(&self) -> &dyn BillUnits {
        &self.units
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            bills: &mut self.bills,
            transactions: &self.reads,
            accounts: &self.reads,
            categories: &self.reads,
        }
    }
}

/// Books `stub`'s payments and Match candidate into `splits`, as the stub Ledger does through
/// `lib-bill-payments`' `PayBill`, which this crate can't reach: each payment a Transaction in its
/// Plan's Account of one Split of `-amount` carrying the Plan's Category and Payee, newest first.
/// `lib-ledger` checks the two agree.
fn book(stub: StubBills, splits: &mut Vec<BillSplit>) -> BillsSeed {
    let StubBills {
        seed: BillsSeed { plans, mut entries },
        payments,
        match_candidate,
    } = stub;
    // Every booked Transaction has one Split, so the next seed id follows the Split count.
    let next = |splits: &Vec<BillSplit>| SplitRef {
        transaction_id: TransactionId::seed(u32::try_from(splits.len() + 1).unwrap_or_default()),
        split_index: 0,
    };
    for payment in payments {
        let Some(plan) = crate::get(&plans, payment.entry.plan_id) else {
            continue;
        };
        let Some(found) = crate::entry_mut(&mut entries, payment.entry) else {
            continue;
        };
        let split = next(splits);
        found.resolution = Resolution::Paid(split);
        insert_newest_first(
            splits,
            BillSplit {
                split,
                date: payment.date,
                account_id: plan.account_id,
                description: Some(plan.name.clone()),
                amount: Money(-payment.amount.0),
                category_id: plan.category_id,
                payee_id: plan.payee_id,
            },
        );
    }
    if let Some(debit) = match_candidate {
        let split = BillSplit {
            split: next(splits),
            date: debit.date,
            account_id: debit.account_id,
            description: Some(debit.description),
            amount: Money(-debit.amount.0),
            category_id: debit.category_id,
            payee_id: Some(debit.payee_id),
        };
        insert_newest_first(splits, split);
    }
    BillsSeed { plans, entries }
}

fn insert_newest_first(splits: &mut Vec<BillSplit>, split: BillSplit) {
    let at = splits
        .iter()
        .position(|s| s.date <= split.date)
        .unwrap_or(splits.len());
    splits.insert(at, split);
}

/// The stub Accounts as Bills reads them: the Accounts stub's ids, names, Units and whether each
/// takes Transactions.
pub fn stub_accounts() -> Vec<BillAccount> {
    let account = |n, name: &str, unit, takes_transactions| BillAccount {
        id: AccountId::seed(n),
        name: name.to_string(),
        unit,
        takes_transactions,
    };
    vec![
        account(1, "Wallet", UnitId::SEED_AUD, true),
        account(2, "ANZ Everyday", UnitId::SEED_AUD, true),
        account(3, "ANZ Offset", UnitId::SEED_AUD, true),
        account(4, "Amex Platinum", UnitId::SEED_AUD, true),
        account(5, "Home Loan", UnitId::SEED_AUD, false),
        account(6, "Vanguard VAS", UnitId::SEED_VAS, false),
        account(7, "Bitcoin", UnitId::SEED_BTC, false),
    ]
}

/// The stub Categories as Bills reads them, the Categories stub's tree flattened to paths.
pub fn stub_categories() -> Vec<BillCategory> {
    let category = |n, name: &str, path: &str, is_expense, is_leaf| BillCategory {
        id: CategoryId::seed(n),
        name: name.to_string(),
        path: path.to_string(),
        is_expense,
        is_leaf,
    };
    vec![
        category(1, "Housing", "Housing", true, false),
        category(2, "Rent", "Housing \u{203a} Rent", true, true),
        category(3, "Utilities", "Housing \u{203a} Utilities", true, false),
        category(
            4,
            "Electricity",
            "Housing \u{203a} Utilities \u{203a} Electricity",
            true,
            true,
        ),
        category(
            5,
            "Water",
            "Housing \u{203a} Utilities \u{203a} Water",
            true,
            true,
        ),
        category(6, "Food", "Food", true, false),
        category(7, "Groceries", "Food \u{203a} Groceries", true, true),
        category(8, "Dining", "Food \u{203a} Dining", true, true),
        category(9, "Transport", "Transport", true, true),
        category(10, "Household", "Household", true, true),
        category(11, "Salary", "Salary", false, true),
        category(12, "Interest", "Interest", false, true),
    ]
}

/// The stub Payees Bills' seed and tests name, with the Payees stub's ids, and its one inactive
/// Payee.
pub fn stub_payees() -> Vec<BillPayee> {
    let payee = |n, name: &str, is_active| BillPayee {
        id: PayeeId::seed(n),
        name: name.to_string(),
        is_active,
    };
    vec![
        payee(3, "Netflix", true),
        payee(5, "Origin Energy", true),
        payee(15, "Sydney Water", true),
        payee(18, "Don Quijote", false),
        payee(19, "Ray White Rentals", true),
        payee(20, "Telstra", true),
        payee(21, "Fitness First", true),
        payee(22, "AAMI", true),
        payee(23, "Brisbane City Council", true),
    ]
}

/// What a Presenter asked its Router for, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    PlanForm(Option<BillPlanId>),
    Skip(EntryId),
    Pay(PayRequest),
    Closed(Option<BillsOutcome>),
    Transaction(TransactionId),
    Notice(Notice),
}

/// A Router that records every request and opens every Dialog it is asked for, unless told not
/// to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub routed: Vec<Routed>,
    /// Refuses to open any Dialog, as a Client does on a row gone under it.
    pub refuse: bool,
}

impl RecordingRouter {
    /// The last status-line message the Presenter set, if any.
    pub fn status(&self) -> Option<&str> {
        self.routed.iter().rev().find_map(|routed| match routed {
            Routed::Notice(Notice::Status(text)) => Some(text.as_str()),
            _ => None,
        })
    }
}

impl BillsRouter for RecordingRouter {
    fn open_plan_form(&mut self, plan: Option<BillPlanId>) -> bool {
        self.routed.push(Routed::PlanForm(plan));
        !self.refuse
    }

    fn open_skip(&mut self, entry: EntryId) -> bool {
        self.routed.push(Routed::Skip(entry));
        !self.refuse
    }

    fn open_pay(&mut self, request: PayRequest) -> bool {
        self.routed.push(Routed::Pay(request));
        !self.refuse
    }

    fn close_dialog(&mut self, outcome: Option<BillsOutcome>) {
        self.routed.push(Routed::Closed(outcome));
    }

    fn show_transaction(&mut self, id: TransactionId) {
        self.routed.push(Routed::Transaction(id));
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.routed.push(Routed::Notice(notice));
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{AccountId, BillPlanId, BillScheduleEntryId};

    use super::*;
    use crate::Resolution;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, day).expect("a September day")
    }

    #[test]
    fn an_upserted_entry_keeps_the_stored_id_for_its_plan_and_due_date() {
        let stored = BillScheduleEntry {
            row_id: BillScheduleEntryId::seed(1),
            plan_id: BillPlanId::seed(1),
            due: date(1),
            resolution: Resolution::Unresolved,
            superseded: false,
        };
        let mut bills = InMemoryBills {
            plans: Vec::new(),
            entries: vec![stored.clone()],
        };

        bills.upsert_entry(BillScheduleEntry {
            row_id: BillScheduleEntryId::seed(2),
            superseded: true,
            ..stored.clone()
        });

        assert_eq!(
            bills.entries,
            [BillScheduleEntry {
                superseded: true,
                ..stored
            }]
        );
    }

    #[test]
    fn the_window_includes_both_ends() {
        let on = |day| BillSplit {
            split: SplitRef {
                transaction_id: TransactionId::seed(day),
                split_index: 0,
            },
            date: date(day),
            account_id: AccountId::seed(1),
            description: None,
            amount: Money(bigdecimal::BigDecimal::from(-1)),
            category_id: CategoryId::seed(7),
            payee_id: None,
        };
        let reads = InMemoryBillReads {
            splits: (1..=5).map(on).collect(),
            ..InMemoryBillReads::default()
        };

        let mut days: Vec<u32> = reads
            .splits_between(date(2), date(4))
            .iter()
            .map(|s| s.split.transaction_id.seed_number())
            .collect();
        days.sort_unstable();

        assert_eq!(days, [2, 3, 4]);
        assert_eq!(reads.paid_split(on(5).split), Some(on(5)));
    }
}
