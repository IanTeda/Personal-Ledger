//! The Transactions writes (ADR-0038). Import's commit is the one write the Desktop makes today:
//! it creates the new Payees and books one Transaction per statement row, so it crosses domains.
//! Like every command it checks before writing, so a refusal leaves the Ledger untouched, and it
//! records an upsert per row it wrote (ADR-0039).

use chrono::NaiveDate;
use lib_core::{Money, TransactionStatus};
use lib_payees::{Payee, PayeeDraft, PayeeError, alias_owner, insert_payee};
use lib_transactions::{Split, Transaction};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// Who an imported row's Split pays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookingPayee {
    Existing(u32),
    /// A Payee to create, once per name however many rows name it. `alias` is the match rule to
    /// remember on it, skipped when another Payee already owns it (#282).
    Create {
        name: String,
        alias: Option<String>,
    },
}

/// One resolved statement row, ready to book as a single-Split Transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportBooking {
    pub date: NaiveDate,
    /// The statement line as it came, kept as the Transaction's memo.
    pub description: String,
    pub amount: Money,
    pub payee: BookingPayee,
    pub category_id: u32,
}

/// What an Import's commit wrote, for its Toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportCommitted {
    pub transactions: usize,
    pub new_payees: usize,
}

impl Ledger {
    /// Creates each new Payee in `bookings` once, then books each row to `account_id`, newest
    /// first.
    pub fn commit_import(
        &mut self,
        account_id: u32,
        bookings: Vec<ImportBooking>,
    ) -> Result<ImportCommitted, PayeeError> {
        let mut payees = self.payees.clone();
        let mut created: Vec<(String, u32)> = Vec::new();
        let mut touched: Vec<u32> = Vec::new();
        let mut payee_ids = Vec::with_capacity(bookings.len());
        for booking in &bookings {
            let id = match &booking.payee {
                BookingPayee::Existing(id) => *id,
                BookingPayee::Create { name, alias } => {
                    let folded = name.to_lowercase();
                    let id = match created.iter().find(|(known, _)| *known == folded) {
                        Some((_, id)) => *id,
                        None => {
                            let id = insert_payee(
                                &mut payees,
                                &PayeeDraft {
                                    name: name.clone(),
                                    default_category: None,
                                    aliases: Vec::new(),
                                },
                            )?;
                            created.push((folded, id));
                            touched.push(id);
                            id
                        }
                    };
                    if let Some(token) = alias
                        && alias_owner(&payees, token).is_none_or(|owner| owner.id == id)
                        && let Some(payee) = payees.iter_mut().find(|p| p.id == id)
                        && !payee.aliases.contains(token)
                    {
                        payee.aliases.push(token.clone());
                    }
                    id
                }
            };
            payee_ids.push(id);
        }

        self.payees = payees;
        for id in touched {
            if let Some(payee) = self.payees.iter().find(|p| p.id == id) {
                self.pending
                    .push(LedgerChange::UpsertPayee(Payee::clone(payee)));
            }
        }

        let first_id = self.transactions.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let count = bookings.len();
        for (id, (booking, payee_id)) in (first_id..).zip(bookings.into_iter().zip(payee_ids)) {
            let transaction = Transaction {
                id,
                date: booking.date,
                account_id,
                status: TransactionStatus::Cleared,
                is_flagged: false,
                description: Some(booking.description),
                splits: vec![Split {
                    amount: booking.amount,
                    category_id: booking.category_id,
                    payee_id: Some(payee_id),
                    tag_ids: Vec::new(),
                }],
            };
            self.pending
                .push(LedgerChange::UpsertTransaction(transaction.clone()));
            self.transactions.push(transaction);
        }
        self.transactions
            .sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));

        Ok(ImportCommitted {
            transactions: count,
            new_payees: created.len(),
        })
    }
}

/// [`Ledger::commit_import`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitImport {
    pub account_id: u32,
    pub bookings: Vec<ImportBooking>,
}

impl LedgerCommand for CommitImport {
    type Outcome = ImportCommitted;
    type Refusal = PayeeError;
    fn run(self, ledger: &mut Ledger) -> Result<ImportCommitted, PayeeError> {
        ledger.commit_import(self.account_id, self.bookings)
    }
}

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use lib_payees::{find_by_name, get, match_alias};

    use super::*;

    fn booking(description: &str, payee: BookingPayee, day: u32) -> ImportBooking {
        ImportBooking {
            date: NaiveDate::from_ymd_opt(2026, 9, day).unwrap(),
            description: description.to_owned(),
            amount: Money(BigDecimal::new((-100).into(), 2)),
            payee,
            category_id: 10,
        }
    }

    fn create(name: &str, alias: Option<&str>) -> BookingPayee {
        BookingPayee::Create {
            name: name.to_owned(),
            alias: alias.map(str::to_owned),
        }
    }

    #[test]
    fn books_each_row_newest_first_and_records_the_upserts() {
        let mut ledger = Ledger::stub();
        let before = ledger.transactions().len();
        let existing = ledger.payees()[0].id;
        ledger.drain_changes();

        let committed = CommitImport {
            account_id: 2,
            bookings: vec![
                booking("OLD", BookingPayee::Existing(existing), 1),
                booking("NEW", create("Corner Shop", None), 28),
            ],
        }
        .run(&mut ledger)
        .unwrap();

        assert_eq!(
            committed,
            ImportCommitted {
                transactions: 2,
                new_payees: 1
            }
        );
        assert_eq!(ledger.transactions().len(), before + 2);
        assert!(
            ledger
                .transactions()
                .windows(2)
                .all(|w| w[0].date >= w[1].date)
        );
        let booked = ledger
            .transactions()
            .iter()
            .find(|t| t.description.as_deref() == Some("NEW"))
            .unwrap();
        let shop = find_by_name(ledger.payees(), "Corner Shop").unwrap();
        assert_eq!(booked.account_id, 2);
        assert_eq!(booked.status, TransactionStatus::Cleared);
        assert_eq!(booked.splits[0].payee_id, Some(shop));
        assert_eq!(booked.splits[0].category_id, 10);

        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 3);
        assert_eq!(
            changes[0],
            LedgerChange::UpsertPayee(get(ledger.payees(), shop).unwrap().clone())
        );
        assert!(
            matches!(&changes[1], LedgerChange::UpsertTransaction(t) if t.description.as_deref() == Some("OLD"))
        );
        assert!(
            matches!(&changes[2], LedgerChange::UpsertTransaction(t) if t.description.as_deref() == Some("NEW"))
        );
    }

    #[test]
    fn rows_sharing_a_new_name_create_one_payee_and_remember_its_alias() {
        let mut ledger = Ledger::stub();
        let before = ledger.payees().len();

        let committed = ledger
            .commit_import(
                2,
                vec![
                    booking("SP CANDLE", create("Candle Co", Some("CANDLE CO")), 3),
                    booking("SP CANDLE 2", create("candle co", Some("CANDLE CO")), 4),
                ],
            )
            .unwrap();

        assert_eq!(committed.new_payees, 1);
        assert_eq!(ledger.payees().len(), before + 1);
        let candle = find_by_name(ledger.payees(), "Candle Co").unwrap();
        assert_eq!(
            get(ledger.payees(), candle).unwrap().aliases,
            vec!["CANDLE CO"]
        );
        assert_eq!(match_alias(ledger.payees(), "SP CANDLE CO"), Some(candle));
    }

    #[test]
    fn an_alias_another_payee_owns_is_skipped() {
        let mut ledger = Ledger::stub();
        let owner = ledger
            .payees()
            .iter()
            .find(|p| !p.aliases.is_empty())
            .unwrap()
            .clone();

        ledger
            .commit_import(
                2,
                vec![booking(
                    "X",
                    create("Fresh Name", Some(&owner.aliases[0])),
                    3,
                )],
            )
            .unwrap();

        let fresh = find_by_name(ledger.payees(), "Fresh Name").unwrap();
        assert!(get(ledger.payees(), fresh).unwrap().aliases.is_empty());
    }

    #[test]
    fn a_refused_commit_changes_nothing_and_records_nothing() {
        let mut ledger = Ledger::stub();
        let payees = ledger.payees().to_vec();
        let transactions = ledger.transactions().to_vec();
        let taken = payees[0].name.clone();
        ledger.drain_changes();

        let refused = CommitImport {
            account_id: 2,
            bookings: vec![
                booking("FIRST", create("Fine New Name", None), 3),
                booking("SECOND", create(&taken, None), 4),
            ],
        }
        .run(&mut ledger);

        assert!(matches!(refused, Err(PayeeError::DuplicateName(_))));
        assert_eq!(ledger.payees(), payees);
        assert_eq!(ledger.transactions(), transactions);
        assert!(ledger.drain_changes().is_empty());
    }
}
