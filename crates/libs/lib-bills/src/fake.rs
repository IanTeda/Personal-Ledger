//! The in-memory Bills ports, for tests and UI-first work. Each holds its rows in plain fields so a
//! test can set up or read state directly. The stub seed is `seed`'s [`default_bills`](crate::default_bills),
//! joined in [`InMemoryPorts::stub`] with the stub Accounts, Categories and Payees it names.
//! [`RecordingRouter`] records what a Presenter asked for.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use lib_accounts::Account;
use lib_categories::Category;
use lib_core::{BillPlanId, TransactionId, UnitId};
use lib_payees::Payee;
use lib_toast::ToastKind;
use lib_transactions::Transaction;

use crate::protocols::{
    BillUnits, BillsOutcome, BillsPorts, BillsRepository, BillsRouter, PayRequest, PortsMut,
    SharedPorts,
};
use crate::{
    BillAccounts, BillCategories, BillPayees, BillPlan, BillPlanRepository, BillScheduleEntry,
    BillScheduleRepository, BillTransactions, BillsSeed, EntryId,
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
    pub accounts: Vec<Account>,
    pub categories: Vec<Category>,
    pub payees: Vec<Payee>,
    pub transactions: Vec<Transaction>,
}

impl BillTransactions for InMemoryBillReads {
    fn transactions_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<Transaction> {
        crate::transactions_between(&self.transactions, from, to)
    }

    fn paid_transaction(&self, id: TransactionId) -> Option<Transaction> {
        self.transactions.iter().find(|t| t.id == id).cloned()
    }
}

impl BillAccounts for InMemoryBillReads {
    fn bill_accounts(&self) -> Vec<Account> {
        self.accounts.clone()
    }
}

impl BillCategories for InMemoryBillReads {
    fn bill_categories(&self) -> Vec<Category> {
        self.categories.clone()
    }
}

impl BillPayees for InMemoryBillReads {
    fn bill_payees(&self) -> Vec<Payee> {
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
    /// The stub Bill Plans and their Schedule as of `today`, with the stub Accounts, Categories
    /// and Payees they name and the Transactions their Paid entries were paid by.
    pub fn stub(today: NaiveDate) -> Self {
        let accounts = lib_accounts::default_accounts();
        let categories = lib_categories::default_categories();
        let payees = lib_payees::default_payees();
        let mut transactions = Vec::new();
        let seed = crate::default_bills(&accounts, &categories, &payees, &mut transactions, today);
        Self {
            bills: seed.into(),
            reads: InMemoryBillReads {
                accounts,
                categories,
                payees,
                transactions,
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

/// What a Presenter asked its Router for, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    PlanForm(Option<BillPlanId>),
    Skip(EntryId),
    Pay(PayRequest),
    Closed(Option<BillsOutcome>),
    Transaction(TransactionId),
    Toast(ToastKind, String),
    Status(Option<String>),
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
    /// The last status the Presenter set, if any.
    pub fn status(&self) -> Option<&Option<String>> {
        self.routed.iter().rev().find_map(|routed| match routed {
            Routed::Status(text) => Some(text),
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

    fn raise_toast(&mut self, kind: ToastKind, text: String) {
        self.routed.push(Routed::Toast(kind, text));
    }

    fn set_status(&mut self, text: Option<String>) {
        self.routed.push(Routed::Status(text));
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{AccountId, BillPlanId, BillScheduleEntryId, TransactionStatus};

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
        let on = |day| Transaction {
            id: TransactionId::seed(day),
            date: date(day),
            account_id: AccountId::seed(1),
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits: Vec::new(),
        };
        let reads = InMemoryBillReads {
            transactions: (1..=5).map(on).collect(),
            ..InMemoryBillReads::default()
        };

        let mut days: Vec<u32> = reads
            .transactions_between(date(2), date(4))
            .iter()
            .map(|t| t.id.seed_number())
            .collect();
        days.sort_unstable();

        assert_eq!(days, [2, 3, 4]);
        assert_eq!(reads.paid_transaction(TransactionId::seed(5)), Some(on(5)));
    }
}
