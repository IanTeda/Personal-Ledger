//! The stub Transactions the other domains' seeds name by id, booked after the generated history:
//! the payments that settle the stub Bills' earlier cycles and the unmatched Telstra debit the
//! Bills' Match offers, then the Documents' sample purchases. Each set is numbered from the id it
//! is given, in the order listed, so the Bills and Documents seeds can name them.
//!
//! The payments follow `lib-bills`' stub Bill Plans (`PLAN_SEEDS` in its `seed.rs`), which this
//! crate can't reach: [`PAID_BILLS`] repeats what a payment reads of each Plan, and `lib-ledger`'s
//! seed tests check the two agree.

use chrono::{Datelike, Duration, Months, NaiveDate};
use lib_core::{Period, TransactionId, TransactionStatus};

use super::seed::Lookup;
use super::{SeedAccount, SeedNames};
use crate::Transaction;

/// What a payment reads of one stub Bill Plan that has earlier cycles to pay. Rent and Council
/// Rates are first due after this month, so they have none and aren't listed.
struct PaidBill {
    /// The Plan's name, which its payments carry as their description.
    name: &'static str,
    category: &'static str,
    account: &'static str,
    payee: Option<&'static str>,
    cents: i64,
    /// An Estimated Plan's payments vary around `cents`.
    estimated: bool,
    /// The months between due dates.
    months: u32,
    /// `(months from today's month, day)`.
    first_due: (i32, u32),
    ends_on: Option<(i32, u32)>,
    /// Whether this month's cycle is paid too.
    paid_this_month: bool,
    /// Earlier-month cycles (by occurrence index) that were skipped rather than paid.
    skipped: &'static [u32],
}

const PAID_BILLS: &[PaidBill] = &[
    PaidBill {
        name: "Netflix",
        category: "Household",
        account: "Amex Platinum",
        payee: Some("Netflix"),
        cents: 2_299,
        estimated: false,
        months: 1,
        first_due: (-14, 18),
        ends_on: None,
        paid_this_month: false,
        skipped: &[],
    },
    PaidBill {
        name: "Gym \u{2014} Fitness First",
        category: "Household",
        account: "ANZ Everyday",
        payee: Some("Fitness First"),
        cents: 6_400,
        estimated: false,
        months: 1,
        first_due: (-10, 15),
        ends_on: None,
        paid_this_month: false,
        skipped: &[4],
    },
    PaidBill {
        name: "Telstra Internet",
        category: "Household",
        account: "ANZ Everyday",
        payee: Some("Telstra"),
        cents: 8_900,
        estimated: false,
        months: 1,
        first_due: (-15, 22),
        ends_on: None,
        paid_this_month: false,
        skipped: &[],
    },
    PaidBill {
        name: "Origin Energy",
        category: "Electricity",
        account: "ANZ Everyday",
        payee: Some("Origin Energy"),
        cents: 15_000,
        estimated: true,
        months: 3,
        first_due: (-15, 24),
        ends_on: None,
        paid_this_month: false,
        skipped: &[],
    },
    PaidBill {
        name: "Car Insurance \u{2014} AAMI",
        category: "Transport",
        account: "Amex Platinum",
        payee: Some("AAMI"),
        cents: 118_000,
        estimated: false,
        months: 12,
        first_due: (-12, 30),
        ends_on: None,
        paid_this_month: false,
        skipped: &[],
    },
    PaidBill {
        name: "Streaming Bundle",
        category: "Household",
        account: "Amex Platinum",
        payee: None,
        cents: 2_999,
        estimated: false,
        months: 1,
        first_due: (-6, 5),
        ends_on: None,
        paid_this_month: true,
        skipped: &[],
    },
    PaidBill {
        name: "Car Wash Membership",
        category: "Transport",
        account: "Amex Platinum",
        payee: None,
        cents: 2_500,
        estimated: false,
        months: 1,
        first_due: (-4, 10),
        ends_on: None,
        paid_this_month: false,
        skipped: &[1],
    },
    PaidBill {
        name: "Basic Fitness (closed)",
        category: "Household",
        account: "ANZ Everyday",
        payee: None,
        cents: 1_995,
        estimated: false,
        months: 1,
        first_due: (-16, 12),
        ends_on: Some((-4, 12)),
        paid_this_month: false,
        skipped: &[],
    },
];

/// `day` of the month `offset` months from `today`'s, clamped to that month's last day.
fn day_in(today: NaiveDate, offset: i32, day: u32) -> NaiveDate {
    let period = Period::of(today).shift(offset);
    let last = period.last_day().day();
    NaiveDate::from_ymd_opt(period.year, period.month, day.min(last)).unwrap_or(today)
}

/// A paid cycle's amount: an Estimated Plan varies from 85% to 114% of its estimate, and Telstra's
/// older cycles were billed at 76.20 then 79.99.
fn paid_cents(bill: &PaidBill, index: u32, cycles: u32) -> i64 {
    if bill.estimated {
        bill.cents * (85 + i64::from(index * 7 % 30)) / 100
    } else if bill.name == "Telstra Internet" && index + 3 < cycles {
        if index < 6 { 7_620 } else { 7_999 }
    } else {
        bill.cents
    }
}

/// One Transaction of one Split on `account`, before it takes its id.
#[expect(
    clippy::too_many_arguments,
    reason = "one row of hand-authored seed data"
)]
fn debit(
    lookup: &Lookup<'_>,
    accounts: &[SeedAccount],
    account: &str,
    date: NaiveDate,
    status: TransactionStatus,
    cents: i64,
    category: &str,
    payee: Option<&str>,
    description: &str,
) -> Option<Transaction> {
    Some(Transaction {
        id: TransactionId::seed(0),
        date,
        account_id: accounts.iter().find(|a| a.name == account)?.id,
        status,
        is_flagged: false,
        description: Some(description.to_string()),
        splits: vec![lookup.split(-cents, category, payee, &[])?],
    })
}

/// Numbers `transactions` from `first`, in order.
fn numbered_from(first: TransactionId, transactions: Vec<Transaction>) -> Vec<Transaction> {
    transactions
        .into_iter()
        .zip(first.seed_number()..)
        .map(|(transaction, n)| Transaction {
            id: TransactionId::seed(n),
            ..transaction
        })
        .collect()
}

/// The payments of the stub Bills' earlier cycles, Plan by Plan and cycle by cycle, then the
/// unmatched Telstra debit yesterday, numbered from `first`. A cycle is paid a day either side of
/// its due date (never after `today`), Reconciled before this month and Cleared in it.
pub fn bill_transactions(
    accounts: &[SeedAccount],
    names: &dyn SeedNames,
    first: TransactionId,
    today: NaiveDate,
) -> Vec<Transaction> {
    let lookup = Lookup { names };
    let this_month = Period::of(today);
    let mut booked = Vec::new();
    for bill in PAID_BILLS {
        let first_due = day_in(today, bill.first_due.0, bill.first_due.1);
        let ends_on = bill
            .ends_on
            .map_or(NaiveDate::MAX, |(offset, day)| day_in(today, offset, day));
        let dues: Vec<NaiveDate> = (0..)
            .map_while(|n: u32| first_due.checked_add_months(Months::new(n * bill.months)))
            .take_while(|due| *due <= ends_on && Period::of(*due) <= this_month)
            .collect();
        let cycles = dues.len() as u32;
        for (index, due) in (0u32..).zip(dues) {
            let paid = if Period::of(due) < this_month {
                !bill.skipped.contains(&index)
            } else {
                bill.paid_this_month
            };
            if !paid {
                continue;
            }
            let date = (due + Duration::days(i64::from(index % 3) - 1)).min(today);
            let status = if Period::of(date) < this_month {
                TransactionStatus::Reconciled
            } else {
                TransactionStatus::Cleared
            };
            booked.extend(debit(
                &lookup,
                accounts,
                bill.account,
                date,
                status,
                paid_cents(bill, index, cycles),
                bill.category,
                bill.payee,
                bill.name,
            ));
        }
    }
    booked.extend(debit(
        &lookup,
        accounts,
        "ANZ Everyday",
        today - Duration::days(1),
        TransactionStatus::Cleared,
        7_999,
        "Household",
        Some("Telstra"),
        "TELSTRA DIRECT DEBIT",
    ));
    numbered_from(first, booked)
}

/// The Documents' sample purchases, numbered from `first`: the Camera House receipt's, then the
/// Inbox's Strong pair (Woolworths, Bunnings) and its Likely Sonos order, which has no Sonos Payee
/// so only amount and date agree. A Payee the stubs lack is left off rather than the purchase.
pub fn document_transactions(
    accounts: &[SeedAccount],
    names: &dyn SeedNames,
    first: TransactionId,
    today: NaiveDate,
) -> Vec<Transaction> {
    let lookup = Lookup { names };
    let day = |offset: i64| today + Duration::days(offset);
    let purchase = |account, offset, cents, category, payee: Option<&str>, description| {
        let payee = payee.filter(|name| names.payee(name).is_some());
        debit(
            &lookup,
            accounts,
            account,
            day(offset),
            TransactionStatus::Cleared,
            cents,
            category,
            payee,
            description,
        )
    };
    let booked = [
        purchase(
            "Amex Platinum",
            -212,
            438_000,
            "Household",
            None,
            "CAMERA HOUSE \u{2014} Sony A7 IV",
        ),
        purchase(
            "Amex Platinum",
            -3,
            21_240,
            "Groceries",
            Some("Woolworths"),
            "WOOLWORTHS METRO",
        ),
        purchase(
            "ANZ Everyday",
            -6,
            61_235,
            "Household",
            Some("Bunnings Warehouse"),
            "BUNNINGS INV88213",
        ),
        purchase(
            "Amex Platinum",
            -9,
            49_900,
            "Household",
            None,
            "SONOS ONLINE",
        ),
    ];
    numbered_from(first, booked.into_iter().flatten().collect())
}
