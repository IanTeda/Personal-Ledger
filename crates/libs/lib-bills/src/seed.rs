//! The stub Bills seed: the handoff's Bill Plans laid out against the Accounts, Categories and
//! Payees Bills reads, with the payments that settle their earlier cycles. Deterministic (no
//! randomness; `today` is injected) and `gpui`-free, so both Clients can build the same Ledger.
//!
//! A payment writes a Transaction too, so the seed only lists them ([`StubBills::payments`]): the
//! stub Ledger books them through `lib-bill-payments`' `PayBill`, and `fake`'s stub stands in for
//! it.

use bigdecimal::BigDecimal;
use chrono::{Datelike, Duration, NaiveDate};
use lib_core::{
    AccountId, BillPlanId, BillScheduleEntryId, CategoryId, Money, PayeeId, Period, TransactionId,
    TransactionStatus,
};
use lib_transactions::{Split, Transaction};

use crate::{
    AmountKind, BillAccount, BillCategory, BillPayee, BillPlan, BillPlanDraft, BillScheduleEntry,
    EntryId, Recurrence, insert_plan_as, set_active, skip,
};

// ---------------------------------------------------------------------------------------------
// The stub dataset
// ---------------------------------------------------------------------------------------------

/// The seeded Bills: Plans and their Schedule.
#[derive(Debug, Clone, PartialEq)]
pub struct BillsSeed {
    pub plans: Vec<BillPlan>,
    pub entries: Vec<BillScheduleEntry>,
}

/// The stub Bills before their history is booked: the entries [`Self::payments`] settles are
/// still open, and the Match candidate is not yet a Transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct StubBills {
    pub seed: BillsSeed,
    /// In the order they are booked, each Transaction taking the next seed id.
    pub payments: Vec<SeedPayment>,
    pub match_candidate: Option<SeedDebit>,
}

/// One earlier cycle paid directly: what `PayBill` is given, and the status its Transaction is
/// left at once booked.
#[derive(Debug, Clone, PartialEq)]
pub struct SeedPayment {
    pub entry: EntryId,
    pub amount: Money,
    pub date: NaiveDate,
    pub status: TransactionStatus,
}

/// The unmatched Telstra payment yesterday, the handoff's 8d Match candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct SeedDebit {
    pub date: NaiveDate,
    pub account_id: AccountId,
    pub category_id: CategoryId,
    pub payee_id: PayeeId,
    /// A positive magnitude; the Split is the negative.
    pub amount: Money,
}

impl SeedDebit {
    /// The Transaction it is, under the seed id the booking side gives it.
    pub fn transaction(&self, id: TransactionId) -> Transaction {
        Transaction {
            id,
            date: self.date,
            account_id: self.account_id,
            status: TransactionStatus::Cleared,
            is_flagged: false,
            description: Some("TELSTRA DIRECT DEBIT".to_string()),
            splits: vec![Split {
                amount: Money(-self.amount.0.clone()),
                category_id: self.category_id,
                payee_id: Some(self.payee_id),
                tag_ids: Vec::new(),
            }],
        }
    }
}

/// A seeded entry's id, fixed by its Bill Plan's seed number and its due date rather than by its
/// place in the seed, so the same due date gets the same id whatever day the seed is built on and a
/// re-seed upserts over it. Meaningful only for a Plan with a [`BillPlanId::seed`] id below 4096.
pub fn seed_entry_id(plan_id: BillPlanId, due: NaiveDate) -> BillScheduleEntryId {
    let day = u32::try_from(due.num_days_from_ce()).unwrap_or_default() & 0x000F_FFFF;
    BillScheduleEntryId::seed(plan_id.seed_number() << 20 | day)
}

/// Builds an exact `Money` from a count of cents.
fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

/// `day` of the month `offset` months from `today`'s, clamped to that month's last day.
fn day_in(today: NaiveDate, offset: i32, day: u32) -> NaiveDate {
    let period = Period::of(today).shift(offset);
    let last = period.last_day().day();
    NaiveDate::from_ymd_opt(period.year, period.month, day.min(last)).unwrap_or(today)
}

/// How this month's entry of a seeded Plan is left.
#[derive(Clone, Copy, PartialEq)]
enum ThisMonth {
    Open,
    Paid,
    Skipped,
}

struct PlanSeed {
    name: &'static str,
    category: &'static str,
    account: &'static str,
    payee: Option<&'static str>,
    cents: i64,
    kind: AmountKind,
    recurrence: Recurrence,
    /// `(months from today's month, day)`.
    first_due: (i32, u32),
    ends_on: Option<(i32, u32)>,
    lead: Option<u32>,
    active: bool,
    this_month: ThisMonth,
    /// Earlier-month cycles (by occurrence index) that were skipped rather than paid.
    skipped: &'static [u32],
}

/// The handoff's ten sample Bill Plans (8b), laid out so that on 19 September the Schedule (8a)
/// shows its nine rows and statuses: Streaming Bundle Paid, Car Wash Skipped, Gym and Netflix
/// Overdue, Telstra Due inside its 3-day lead, Origin and AAMI Due, Rent and Council Rates
/// Upcoming. Rent's first due date is next month, so Rent has no September row.
const PLAN_SEEDS: &[PlanSeed] = &[
    PlanSeed {
        name: "Rent",
        category: "Rent",
        account: "ANZ Everyday",
        payee: Some("Ray White Rentals"),
        cents: 240_000,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (1, 1),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Netflix",
        category: "Household",
        account: "Amex Platinum",
        payee: Some("Netflix"),
        cents: 2_299,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-14, 18),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Gym \u{2014} Fitness First",
        category: "Household",
        account: "ANZ Everyday",
        payee: Some("Fitness First"),
        cents: 6_400,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-10, 15),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[4],
    },
    PlanSeed {
        name: "Telstra Internet",
        category: "Household",
        account: "ANZ Everyday",
        payee: Some("Telstra"),
        cents: 8_900,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-15, 22),
        ends_on: None,
        lead: Some(3),
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Origin Energy",
        category: "Electricity",
        account: "ANZ Everyday",
        payee: Some("Origin Energy"),
        cents: 15_000,
        kind: AmountKind::Estimated,
        recurrence: Recurrence::Quarterly,
        first_due: (-15, 24),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Car Insurance \u{2014} AAMI",
        category: "Transport",
        account: "Amex Platinum",
        payee: Some("AAMI"),
        cents: 118_000,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Annually,
        first_due: (-12, 30),
        ends_on: None,
        lead: Some(5),
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Council Rates",
        category: "Household",
        account: "ANZ Everyday",
        payee: Some("Brisbane City Council"),
        cents: 52_000,
        kind: AmountKind::Estimated,
        recurrence: Recurrence::Quarterly,
        first_due: (2, 5),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Streaming Bundle",
        category: "Household",
        account: "Amex Platinum",
        payee: None,
        cents: 2_999,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-6, 5),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Paid,
        skipped: &[],
    },
    PlanSeed {
        name: "Car Wash Membership",
        category: "Transport",
        account: "Amex Platinum",
        payee: None,
        cents: 2_500,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-4, 10),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Skipped,
        skipped: &[1],
    },
    PlanSeed {
        name: "Basic Fitness (closed)",
        category: "Household",
        account: "ANZ Everyday",
        payee: None,
        cents: 1_995,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-16, 12),
        ends_on: Some((-4, 12)),
        lead: None,
        active: false,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
];

/// A deterministic spread for a paid cycle's actual amount: an Estimated Plan varies from 85% to
/// 114% of its estimate, and Telstra's older cycles were billed at 76.20 then 79.99, so the History
/// figures have something to show.
fn paid_cents(seed: &PlanSeed, index: u32, cycles: u32) -> i64 {
    match seed.kind {
        AmountKind::Estimated => seed.cents * (85 + i64::from(index * 7 % 30)) / 100,
        AmountKind::Fixed if seed.name == "Telstra Internet" && index + 3 < cycles => {
            if index < 6 {
                7_620
            } else {
                7_999
            }
        }
        AmountKind::Fixed => seed.cents,
    }
}

/// Seeds the handoff's Bill Plans against `accounts`, `categories` and `payees` and lays out their
/// history: every earlier-month cycle is a [`SeedPayment`] (Reconciled, dated a day or two either
/// side of the due date) unless its Plan skipped it, and this month's is left as the handoff
/// shows it. Also names one unmatched Telstra payment yesterday, the 8d Match candidate. A Plan
/// whose Account, Category or Payee is missing is left out.
pub fn default_bills(
    accounts: &[BillAccount],
    categories: &[BillCategory],
    payees: &[BillPayee],
    today: NaiveDate,
) -> StubBills {
    let mut plans: Vec<BillPlan> = Vec::new();
    let mut entries: Vec<BillScheduleEntry> = Vec::new();
    let mut payments: Vec<SeedPayment> = Vec::new();
    let this_month = Period::of(today);
    let category_named = |name: &str| categories.iter().find(|c| c.name == name).map(|c| c.id);
    let payee_named = |name: &str| payees.iter().find(|p| p.name == name).map(|p| p.id);

    for seed in PLAN_SEEDS {
        let Some(account) = accounts.iter().find(|a| a.name == seed.account) else {
            continue;
        };
        let Some(category_id) = category_named(seed.category) else {
            continue;
        };
        let payee_id = match seed.payee {
            Some(name) => match payee_named(name) {
                Some(id) => Some(id),
                None => continue,
            },
            None => None,
        };
        let first_due = day_in(today, seed.first_due.0, seed.first_due.1);
        let draft = BillPlanDraft {
            name: seed.name.to_string(),
            category_id,
            unit: account.unit,
            account_id: account.id,
            payee_id,
            planned_amount: cents_money(seed.cents),
            amount_kind: seed.kind,
            recurrence: seed.recurrence,
            first_due,
            ends_on: seed.ends_on.map(|(offset, day)| day_in(today, offset, day)),
            attention_lead: seed.lead,
        };
        let Ok(id) = insert_plan_as(
            BillPlanId::seed(u32::try_from(plans.len() + 1).unwrap_or_default()),
            &mut plans,
            &mut entries,
            &draft,
            categories,
            accounts,
            today,
        ) else {
            continue;
        };

        let mine: Vec<EntryId> = entries
            .iter()
            .filter(|e| e.plan_id == id && Period::of(e.due) <= this_month)
            .map(BillScheduleEntry::id)
            .collect();
        let cycles = mine.len() as u32;
        for (index, entry_id) in (0u32..).zip(mine) {
            let resolve = if Period::of(entry_id.due) < this_month {
                if seed.skipped.contains(&index) {
                    ThisMonth::Skipped
                } else {
                    ThisMonth::Paid
                }
            } else {
                seed.this_month
            };
            match resolve {
                ThisMonth::Open => {}
                ThisMonth::Skipped => {
                    let _ = skip(&plans, &mut entries, entry_id);
                }
                ThisMonth::Paid => {
                    let shift = i64::from(index % 3) - 1;
                    let date = (entry_id.due + Duration::days(shift)).min(today);
                    payments.push(SeedPayment {
                        entry: entry_id,
                        amount: cents_money(paid_cents(seed, index, cycles)),
                        date,
                        status: if Period::of(date) < this_month {
                            TransactionStatus::Reconciled
                        } else {
                            TransactionStatus::Cleared
                        },
                    });
                }
            }
        }
        if !seed.active {
            let _ = set_active(&mut plans, &mut entries, id, false, today);
        }
    }

    let match_candidate = match (
        accounts.iter().find(|a| a.name == "ANZ Everyday"),
        category_named("Household"),
        payee_named("Telstra"),
    ) {
        (Some(account), Some(category_id), Some(payee_id)) => Some(SeedDebit {
            date: today - Duration::days(1),
            account_id: account.id,
            category_id,
            payee_id,
            amount: cents_money(7_999),
        }),
        _ => None,
    };

    for entry in &mut entries {
        entry.row_id = seed_entry_id(entry.plan_id, entry.due);
    }
    entries.sort_by_key(|entry| (entry.due, entry.plan_id));
    StubBills {
        seed: BillsSeed { plans, entries },
        payments,
        match_candidate,
    }
}
