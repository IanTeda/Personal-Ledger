//! The in-memory deletion port, for tests and UI-first work, the reads the Dialog quotes, a Router
//! that records what the Delete Dialog's Presenter asked for, and [`world`], the small Ledger the
//! tests and the contract delete from. Each holds its rows in plain fields so a test can set up or
//! read state directly.

use std::collections::HashMap;

use lib_accounts::fake::{InMemoryAccountUsage, InMemoryAccounts};
use lib_accounts::{Account, AccountRepository, AccountUsage, AccountUse};
use lib_bills::fake::InMemoryBills;
use lib_bills::{BillPlanRepository, BillScheduleEntry, BillScheduleRepository};
use lib_core::{AccountId, TransactionId, UnitId};
use lib_toast::{Notice, Notify};
use lib_transactions::TransactionRepository;
use lib_transactions::fake::InMemoryTransactions;

use crate::protocols::{
    AccountDeleted, AccountDeletion, AccountDeletionRouter, AccountRemoval, DeletionReads,
};

/// A delete's three stores in memory, the Accounts, the Transactions and the Bills, written
/// together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryDeletion {
    pub accounts: InMemoryAccounts,
    pub transactions: InMemoryTransactions,
    pub bills: InMemoryBills,
}

impl AccountDeletion for InMemoryDeletion {
    fn account(&self, id: AccountId) -> Option<Account> {
        self.accounts.account(id)
    }

    fn bill_plans_paying_from(&self, id: AccountId) -> usize {
        self.bills
            .bill_plans()
            .iter()
            .filter(|plan| plan.account_id == id)
            .count()
    }

    fn transactions_booked_to(&self, id: AccountId) -> Vec<TransactionId> {
        self.transactions
            .transactions
            .iter()
            .filter(|transaction| transaction.account_id == id)
            .map(|transaction| transaction.id)
            .collect()
    }

    fn bill_entries(&self) -> Vec<BillScheduleEntry> {
        self.bills.entries()
    }

    fn remove_account(&mut self, removal: AccountRemoval) {
        for entry in removal.unmatched {
            self.bills.upsert_entry(entry);
        }
        for id in removal.transactions {
            self.transactions.delete_transaction(id);
        }
        self.accounts.delete_account(removal.account);
    }
}

/// The figures the Delete Dialog quotes, as a test sets them: what names each Account, and each
/// Unit's code.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryReads {
    pub usage: InMemoryAccountUsage,
    pub unit_codes: HashMap<UnitId, String>,
}

impl AccountUsage for InMemoryReads {
    fn account_usage(&self, id: AccountId) -> AccountUse {
        self.usage.account_usage(id)
    }
}

impl DeletionReads for InMemoryReads {
    fn unit_code(&self, unit: UnitId) -> Option<String> {
        self.unit_codes.get(&unit).cloned()
    }
}

/// A Router that records each close and each Notice, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub closed: Vec<Option<AccountDeleted>>,
    pub notices: Vec<Notice>,
}

impl AccountDeletionRouter for RecordingRouter {
    fn close_dialog(&mut self, outcome: Option<AccountDeleted>) {
        self.closed.push(outcome);
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.notices.push(notice);
    }
}

/// The small Ledger the tests and the contract delete from: three Accounts, one Bill Plan paying
/// from Everyday with two entries Paid, one by a Wallet Transaction and one by an Everyday one, and
/// a third still open.
pub mod world {
    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_accounts::{Account, NO_INSTITUTION};
    use lib_bills::fake::InMemoryBills;
    use lib_bills::{
        AmountKind, BillPlan, BillScheduleEntry, EntryId, Recurrence, Resolution, SplitRef,
    };
    use lib_core::{
        AccountId, AccountType, BillPlanId, BillScheduleEntryId, CategoryId, Money, TransactionId,
        TransactionStatus, UnitId,
    };
    use lib_transactions::fake::InMemoryTransactions;
    use lib_transactions::{Split, Transaction};

    use super::InMemoryDeletion;

    /// Nothing pays from it; two Transactions are booked to it, one paying a Bill.
    pub const WALLET: AccountId = AccountId::seed(1);
    /// The Internet Plan pays from it; one Transaction is booked to it, paying a Bill.
    pub const EVERYDAY: AccountId = AccountId::seed(2);
    /// Nothing names it.
    pub const OFFSET: AccountId = AccountId::seed(3);
    pub const INTERNET: BillPlanId = BillPlanId::seed(1);
    /// The Wallet's Transaction that paid September's Internet.
    pub const WALLET_PAYMENT: TransactionId = TransactionId::seed(1);
    pub const WALLET_GROCERIES: TransactionId = TransactionId::seed(2);
    /// Everyday's Transaction that paid August's Internet.
    pub const EVERYDAY_PAYMENT: TransactionId = TransactionId::seed(3);

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap_or_default()
    }

    fn cents(cents: i64) -> Money {
        Money(BigDecimal::new(cents.into(), 2))
    }

    /// The Internet entry the Wallet paid.
    pub fn paid_by_wallet() -> EntryId {
        EntryId {
            plan_id: INTERNET,
            due: date(9, 1),
        }
    }

    /// The Internet entry Everyday paid.
    pub fn paid_by_everyday() -> EntryId {
        EntryId {
            plan_id: INTERNET,
            due: date(8, 1),
        }
    }

    fn account(id: AccountId, name: &str, account_type: AccountType) -> Account {
        Account {
            id,
            name: name.to_string(),
            institution: NO_INSTITUTION,
            account_type,
            unit: UnitId::SEED_AUD,
            balance: cents(-231_844),
            account_number: None,
            opened_at: date(1, 1),
        }
    }

    fn transaction(id: TransactionId, account_id: AccountId, day: NaiveDate) -> Transaction {
        Transaction {
            id,
            date: day,
            account_id,
            status: TransactionStatus::Cleared,
            is_flagged: false,
            description: Some("Internet".to_string()),
            splits: vec![Split {
                amount: cents(-8_999),
                category_id: CategoryId::seed(7),
                payee_id: None,
                tag_ids: Vec::new(),
            }],
        }
    }

    fn entry(row: u32, id: EntryId, resolution: Resolution) -> BillScheduleEntry {
        BillScheduleEntry {
            row_id: BillScheduleEntryId::seed(row),
            plan_id: id.plan_id,
            due: id.due,
            resolution,
            superseded: false,
        }
    }

    fn paid_by(transaction_id: TransactionId) -> Resolution {
        Resolution::Paid(SplitRef {
            transaction_id,
            split_index: 0,
        })
    }

    /// The Accounts, Transactions and Bills described above.
    pub fn world() -> InMemoryDeletion {
        let plan = BillPlan {
            id: INTERNET,
            name: "Internet".to_string(),
            category_id: CategoryId::seed(7),
            unit: UnitId::SEED_AUD,
            account_id: EVERYDAY,
            payee_id: None,
            planned_amount: cents(8_999),
            amount_kind: AmountKind::Fixed,
            recurrence: Recurrence::Monthly,
            first_due: date(8, 1),
            ends_on: None,
            attention_lead: Some(5),
            is_active: true,
            generate_from: date(8, 1),
        };
        let open = EntryId {
            plan_id: INTERNET,
            due: date(10, 1),
        };
        InMemoryDeletion {
            accounts: lib_accounts::fake::InMemoryAccounts {
                accounts: vec![
                    account(WALLET, "Wallet", AccountType::Cash),
                    account(EVERYDAY, "Everyday", AccountType::Bank),
                    account(OFFSET, "Offset", AccountType::Bank),
                ],
            },
            transactions: InMemoryTransactions {
                transactions: vec![
                    transaction(WALLET_PAYMENT, WALLET, date(9, 1)),
                    Transaction {
                        description: Some("Groceries".to_string()),
                        ..transaction(WALLET_GROCERIES, WALLET, date(9, 3))
                    },
                    transaction(EVERYDAY_PAYMENT, EVERYDAY, date(8, 1)),
                ],
            },
            bills: InMemoryBills {
                plans: vec![plan],
                entries: vec![
                    entry(1, paid_by_everyday(), paid_by(EVERYDAY_PAYMENT)),
                    entry(2, paid_by_wallet(), paid_by(WALLET_PAYMENT)),
                    entry(3, open, Resolution::Unresolved),
                ],
            },
        }
    }
}
