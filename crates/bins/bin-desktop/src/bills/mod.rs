//! Pure Bills-surface domain types and the stub dataset behind them (`docs/ux/desktop-mockups/12-bills/`,
//! `docs/bills.md`, ADR-0019 and its amendments) -- `gpui`-free and in-memory, the same "pure
//! state, chrome renders it" split `tags/mod.rs` uses. Nothing here reads `lib_database`.
//!
//! The rules are the Desktop Bills Surface map's settled decisions (#365–#368):
//!
//! - A [`BillPlan`]'s due dates step from its First Due: 7 or 14 days for Weekly and Fortnightly,
//!   and for Monthly, Quarterly and Annually its day of the month, clamped to a shorter month's last
//!   day but always computed from the anchor (31 Jan gives 28 Feb, then 31 Mar). Ends On is
//!   inclusive.
//! - A [`BillScheduleEntry`] is identified by its Bill Plan and due date ([`EntryId`]), the stub's
//!   stand-in for ADR-0019's deterministic UUIDv5, so regenerating a date finds the same row.
//! - Generation ([`populate`]) runs through the end of next calendar month and always keeps at
//!   least one unresolved entry due today or later per active Bill Plan. It skips a due date whose
//!   Recurrence period already holds a Paid or Skipped entry, and clears `superseded` on a row an
//!   edit brings back rather than adding a second one.
//! - An edit to Recurrence, First Due or Ends On, or deactivating, supersedes (never deletes) the
//!   unresolved entries due today or later; Overdue, Paid and Skipped entries are never touched.
//! - Upcoming, Due and Overdue are derived from `today` by calendar month; Needs Attention is a date
//!   rule (unresolved and due on or before today plus the Attention Lead).
//! - Settlement is a Match to one Split, one-to-one. The stub records the Match on the entry as a
//!   [`SplitRef`] rather than as a `bill_schedule_id` on the Split, so the shared Transactions stub
//!   keeps its shape; a Transaction "carries a Bill" when any entry points at one of its Splits.
//!
//! Plan and Transaction ids are the stubs' `u32`s, matching every other desktop stub.

pub(crate) mod form;
pub(crate) mod history;
pub(crate) mod pay_form;
pub(crate) mod store;

use crate::{
    bills::form::BillPlanForm,
    bills::pay_form::PayForm,
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    form::field::TextField,
};

pub use lib_bills::*;
pub use store::{BillsStore, edit_bills, edit_bills_and_transactions};

/// Which tab of the Bills surface shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BillsTab {
    #[default]
    Schedule,
    Planner,
}

impl BillsTab {
    /// `tab` cycles forward, wrapping.
    pub fn next(self) -> Self {
        match self {
            BillsTab::Schedule => BillsTab::Planner,
            BillsTab::Planner => BillsTab::Schedule,
        }
    }
}

/// The open Bills dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillsDialog {
    Add(BillPlanForm),
    /// Editing the Bill Plan with this [`BillPlan::id`].
    Edit(u32, BillPlanForm),
    Pay(PayForm),
    /// Confirming a skip of this entry: nothing to edit, so no form.
    Skip(EntryId),
}

impl BillsDialog {
    /// The form behind the Add and Edit bill plan dialogs.
    pub fn plan_form_mut(&mut self) -> Option<&mut BillPlanForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Pay(_) | Self::Skip(_) => None,
        }
    }

    /// The form behind the dialog, which Skip has none of: it confirms with nothing to fill in.
    fn inner(&self) -> Option<&dyn Dialog> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Pay(form) => Some(form),
            Self::Skip(_) => None,
        }
    }

    fn inner_mut(&mut self) -> Option<&mut dyn Dialog> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Pay(form) => Some(form),
            Self::Skip(_) => None,
        }
    }
}

impl Dialog for BillsDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.inner_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.inner_mut()?.focused_text()
    }

    fn cycle_field(&mut self) {
        if let Some(form) = self.inner_mut() {
            form.cycle_field();
        }
    }

    fn is_valid(&self) -> bool {
        self.inner().is_none_or(Dialog::is_valid)
    }

    fn close_open_select(&mut self) -> bool {
        self.inner_mut().is_some_and(Dialog::close_open_select)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::{Account, default_accounts},
        categories::{Category, default_categories},
        payees::default_payees,
        period::Period,
        tags::default_tags,
        transactions::{Transaction, default_transactions},
    };
    use bigdecimal::{BigDecimal, Signed};
    use chrono::{Datelike, NaiveDate};
    use lib_core::{Money, Total, TransactionStatus};

    fn cents_money(cents: i64) -> Money {
        Money(BigDecimal::new(cents.into(), 2))
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn today() -> NaiveDate {
        date(2026, 9, 19)
    }

    struct World {
        accounts: Vec<Account>,
        categories: Vec<Category>,
        transactions: Vec<Transaction>,
        plans: Vec<BillPlan>,
        entries: Vec<BillScheduleEntry>,
    }

    fn world() -> World {
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let mut transactions =
            default_transactions(&accounts, &categories, &payees, &default_tags(), today());
        let seed = default_bills(&accounts, &categories, &payees, &mut transactions, today());
        World {
            accounts,
            categories,
            transactions,
            plans: seed.plans,
            entries: seed.entries,
        }
    }

    fn draft(recurrence: Recurrence, first_due: NaiveDate) -> BillPlanDraft {
        BillPlanDraft {
            name: "Water".to_string(),
            category_id: 5,
            unit: "aud".to_string(),
            account_id: 2,
            payee_id: Some(15),
            planned_amount: cents_money(9_000),
            amount_kind: AmountKind::Estimated,
            recurrence,
            first_due,
            ends_on: None,
            attention_lead: None,
        }
    }

    fn dues(w: &World, plan_id: u32) -> Vec<NaiveDate> {
        let mut dates: Vec<NaiveDate> = w
            .entries
            .iter()
            .filter(|e| e.plan_id == plan_id && !e.superseded)
            .map(|e| e.due)
            .collect();
        dates.sort();
        dates
    }

    fn plan_id(w: &World, name: &str) -> u32 {
        w.plans.iter().find(|p| p.name == name).unwrap().id
    }

    fn id(plan_id: u32, due: NaiveDate) -> EntryId {
        EntryId { plan_id, due }
    }

    // --- generation ---

    #[test]
    fn populate_is_idempotent() {
        let mut w = world();
        let before = w.entries.clone();
        populate(&w.plans, &mut w.entries, today());
        assert_eq!(w.entries, before);
    }

    // --- settlement ---

    #[test]
    fn pay_writes_an_open_transaction_and_matches_its_split() {
        let mut w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let due = id(telstra, date(2026, 9, 22));
        let before = w.transactions.len();
        let tid = pay(
            &w.plans,
            &mut w.entries,
            &mut w.transactions,
            due,
            &cents_money(8_900),
            today(),
        )
        .unwrap();
        assert_eq!(w.transactions.len(), before + 1);
        let t = w.transactions.iter().find(|t| t.id == tid).unwrap();
        assert_eq!(t.status, TransactionStatus::Open);
        assert_eq!(t.total(), cents_money(-8_900));
        assert_eq!(t.account_id, 2);
        assert!(carries_bill(&w.entries, tid));
        let e = entry(&w.entries, due).unwrap();
        assert_eq!(status(e, today()), BillStatus::Paid);
        assert_eq!(paid_on(e, &w.transactions), Some(today()));
        // Still newest first.
        assert!(w.transactions.windows(2).all(|p| p[0].date >= p[1].date));
    }

    #[test]
    fn pay_refuses_a_non_positive_amount_and_a_resolved_entry() {
        let mut w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let due = id(telstra, date(2026, 9, 22));
        let zero = pay(
            &w.plans,
            &mut w.entries,
            &mut w.transactions,
            due,
            &cents_money(0),
            today(),
        );
        assert_eq!(zero, Err(BillError::AmountNotPositive));
        skip(&w.plans, &mut w.entries, due).unwrap();
        let again = pay(
            &w.plans,
            &mut w.entries,
            &mut w.transactions,
            due,
            &cents_money(1),
            today(),
        );
        assert_eq!(again, Err(BillError::EntryNotOpen));
    }

    #[test]
    fn match_candidates_put_the_payee_match_first_and_preselect_it() {
        let w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let due = id(telstra, date(2026, 9, 22));
        let candidates = match_candidates(
            &w.plans,
            &w.entries,
            &w.transactions,
            &w.accounts,
            &w.categories,
            due,
        );
        assert!(!candidates.is_empty());
        let plan = get(&w.plans, telstra).unwrap();
        let first = preselected_candidate(plan, &candidates, &w.transactions).unwrap();
        let split = split_of(&w.transactions, first).unwrap();
        assert_eq!(split.payee_id, plan.payee_id);
        assert_eq!(split.amount, cents_money(-7_999));
        // Nothing already Matched, nothing income, nothing outside the window.
        for c in &candidates {
            assert!(!is_matched(&w.entries, *c));
            let t = w
                .transactions
                .iter()
                .find(|t| t.id == c.transaction_id)
                .unwrap();
            assert!((t.date - due.due).num_days().abs() <= MATCH_WINDOW_DAYS);
            assert!(t.splits[c.split_index].amount.0.is_negative());
        }
    }

    #[test]
    fn match_split_pays_and_unmatch_reverses_leaving_the_transaction() {
        let mut w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let due = id(telstra, date(2026, 9, 22));
        let candidates = match_candidates(
            &w.plans,
            &w.entries,
            &w.transactions,
            &w.accounts,
            &w.categories,
            due,
        );
        let chosen = candidates[0];
        let before = w.transactions.len();
        match_split(
            &w.plans,
            &mut w.entries,
            &w.transactions,
            &w.accounts,
            &w.categories,
            due,
            chosen,
        )
        .unwrap();
        let e = entry(&w.entries, due).unwrap();
        assert_eq!(
            amount(e, &w.plans, &w.transactions),
            Some(cents_money(7_999))
        );
        // A Matched Split is no longer anyone's candidate.
        let netflix = plan_id(&w, "Netflix");
        let other = match_candidates(
            &w.plans,
            &w.entries,
            &w.transactions,
            &w.accounts,
            &w.categories,
            id(netflix, date(2026, 9, 18)),
        );
        assert!(!other.contains(&chosen));
        unmatch(&mut w.entries, due).unwrap();
        assert_eq!(
            status(entry(&w.entries, due).unwrap(), today()),
            BillStatus::Due
        );
        assert_eq!(w.transactions.len(), before);
    }

    #[test]
    fn match_split_refuses_a_non_candidate() {
        let mut w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let due = id(telstra, date(2026, 9, 22));
        let far = SplitRef {
            transaction_id: w.transactions.last().unwrap().id,
            split_index: 0,
        };
        let result = match_split(
            &w.plans,
            &mut w.entries,
            &w.transactions,
            &w.accounts,
            &w.categories,
            due,
            far,
        );
        assert_eq!(result, Err(BillError::NotACandidate));
    }

    #[test]
    fn skip_snapshots_the_planned_amount_and_unskip_reverses() {
        let mut w = world();
        let origin = plan_id(&w, "Origin Energy");
        let due = id(origin, date(2026, 9, 24));
        skip(&w.plans, &mut w.entries, due).unwrap();
        let (c, a) = (w.categories.clone(), w.accounts.clone());
        let plan = get(&w.plans, origin).unwrap().clone();
        let mut edit = draft(plan.recurrence, plan.first_due);
        edit.name = plan.name.clone();
        edit.category_id = plan.category_id;
        edit.payee_id = plan.payee_id;
        edit.planned_amount = cents_money(99_999);
        edit_plan(&mut w.plans, &mut w.entries, origin, &edit, &c, &a, today()).unwrap();
        let e = entry(&w.entries, due).unwrap();
        assert_eq!(
            amount(e, &w.plans, &w.transactions),
            Some(cents_money(15_000))
        );
        unskip(&mut w.entries, due).unwrap();
        assert_eq!(unskip(&mut w.entries, due), Err(BillError::NotSkipped));
    }

    #[test]
    fn deleting_a_matched_transaction_unmatches_its_entry() {
        let mut w = world();
        let streaming = plan_id(&w, "Streaming Bundle");
        let due = id(streaming, date(2026, 9, 5));
        let Resolution::Paid(split) = entry(&w.entries, due).unwrap().resolution else {
            panic!("the seed pays Streaming Bundle this month");
        };
        unmatch_transaction(&mut w.entries, split.transaction_id);
        assert_eq!(
            status(entry(&w.entries, due).unwrap(), today()),
            BillStatus::Overdue
        );
    }

    // --- the seed, against the handoff ---

    #[test]
    fn the_seed_has_the_handoff_plans_with_one_inactive() {
        let w = world();
        assert_eq!(w.plans.len(), 10);
        assert_eq!(inactive_count(&w.plans), 1);
        assert!(
            !get(&w.plans, plan_id(&w, "Basic Fitness (closed)"))
                .unwrap()
                .is_active
        );
        let order: Vec<&str> = planner_order(&w.plans)
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(order.last(), Some(&"Basic Fitness (closed)"));
    }

    #[test]
    fn the_september_schedule_matches_8a() {
        let w = world();
        let rows = schedule_rows(&w.plans, &w.entries, Some(Period::of(today())), today());
        let shown: Vec<(String, u32, BillStatus, bool)> = rows
            .iter()
            .map(|r| {
                let name = get(&w.plans, r.id.plan_id).unwrap().name.clone();
                (name, r.id.due.day(), r.status, r.needs_attention)
            })
            .collect();
        use BillStatus::*;
        // Unresolved first, next due first; then resolved, most recent first.
        let expected = [
            ("Gym \u{2014} Fitness First", 15, Overdue, true),
            ("Netflix", 18, Overdue, true),
            ("Telstra Internet", 22, Due, true),
            ("Origin Energy", 24, Due, false),
            ("Car Insurance \u{2014} AAMI", 30, Due, false),
            ("Car Wash Membership", 10, Skipped, false),
            ("Streaming Bundle", 5, Paid, false),
        ];
        let expected: Vec<(String, u32, BillStatus, bool)> = expected
            .iter()
            .map(|(n, d, s, a)| (n.to_string(), *d, *s, *a))
            .collect();
        assert_eq!(shown, expected);
        let attention: Vec<EntryId> = rows
            .iter()
            .filter(|r| r.needs_attention)
            .map(|r| r.id)
            .collect();
        assert_eq!(attention_entries(&w.plans, &w.entries, today()), attention);
    }

    #[test]
    fn rent_and_council_rates_are_upcoming_in_later_months() {
        let w = world();
        let october = schedule_rows(
            &w.plans,
            &w.entries,
            Some(Period::of(today()).next()),
            today(),
        );
        let rent = plan_id(&w, "Rent");
        assert!(
            october
                .iter()
                .any(|r| r.id == id(rent, date(2026, 10, 1)) && r.status == BillStatus::Upcoming)
        );
        let council = plan_id(&w, "Council Rates");
        assert!(entry(&w.entries, id(council, date(2026, 11, 5))).is_some());
    }

    #[test]
    fn an_earlier_overdue_row_carries_into_the_current_month() {
        let mut w = world();
        let netflix = plan_id(&w, "Netflix");
        let august = id(netflix, date(2026, 8, 18));
        unmatch(&mut w.entries, august).unwrap();
        let september = schedule_rows(&w.plans, &w.entries, Some(Period::of(today())), today());
        let carried = september.iter().find(|r| r.id == august).unwrap();
        assert!(carried.carried);
        let own = schedule_rows(
            &w.plans,
            &w.entries,
            Some(Period::of(today()).prev()),
            today(),
        );
        assert!(own.iter().any(|r| r.id == august && !r.carried));
        let summary = period_summary(&september, &w.plans, &w.entries, &w.transactions);
        assert_eq!(summary.overdue, 3);
    }

    #[test]
    fn unresolved_rows_carry_into_any_other_month_once() {
        let w = world();
        let august = schedule_rows(
            &w.plans,
            &w.entries,
            Some(Period::of(today()).prev()),
            today(),
        );
        let actionable: Vec<&ScheduleRow> = august
            .iter()
            .filter(|r| matches!(r.status, BillStatus::Overdue | BillStatus::Due))
            .collect();
        // September's two Overdue and three Due rows all reach back into August, carried.
        assert_eq!(actionable.len(), 5);
        assert!(actionable.iter().all(|r| r.carried));
        let ids: std::collections::HashSet<EntryId> = august.iter().map(|r| r.id).collect();
        assert_eq!(ids.len(), august.len());
    }

    #[test]
    fn all_shows_every_entry_unresolved_first_then_latest_resolved() {
        let w = world();
        let all = schedule_rows(&w.plans, &w.entries, None, today());
        assert_eq!(
            all.len(),
            w.entries.iter().filter(|e| !e.superseded).count()
        );
        assert!(all.iter().all(|r| !r.carried && !r.preview));
        let split = all.iter().position(ScheduleRow::is_resolved).unwrap();
        let (open, settled) = all.split_at(split);
        assert!(settled.iter().all(ScheduleRow::is_resolved));
        assert!(open.windows(2).all(|w| w[0].id.due <= w[1].id.due));
        assert!(settled.windows(2).all(|w| w[0].id.due >= w[1].id.due));
    }

    #[test]
    fn a_carried_entry_is_shown_in_the_current_period_and_leads_needs_attention() {
        let mut w = world();
        let netflix = plan_id(&w, "Netflix");
        let august = id(netflix, date(2026, 8, 18));
        unmatch(&mut w.entries, august).unwrap();
        let carried = entry(&w.entries, august).unwrap();
        assert_eq!(schedule_period(carried, today()), Period::of(today()));
        assert_eq!(attention_entries(&w.plans, &w.entries, today())[0], august);
        let rent = entry(&w.entries, id(plan_id(&w, "Rent"), date(2026, 10, 1))).unwrap();
        assert_eq!(schedule_period(rent, today()), Period::of(today()).next());
    }

    #[test]
    fn a_lead_reaching_into_next_month_brings_its_entry_into_needs_attention() {
        let mut w = world();
        let rent = plan_id(&w, "Rent");
        let october = id(rent, date(2026, 10, 1));
        assert!(!attention_entries(&w.plans, &w.entries, today()).contains(&october));
        w.plans
            .iter_mut()
            .find(|p| p.id == rent)
            .unwrap()
            .attention_lead = Some(31);
        assert!(attention_entries(&w.plans, &w.entries, today()).contains(&october));
    }

    #[test]
    fn periods_past_the_horizon_show_previews() {
        let w = world();
        let january = Period {
            year: 2027,
            month: 1,
        };
        let rows = schedule_rows(&w.plans, &w.entries, Some(january), today());
        assert!(rows.iter().any(|r| r.preview));
        let rent = plan_id(&w, "Rent");
        assert!(rows.iter().any(|r| r.id == id(rent, date(2027, 1, 1))));
        // The inactive Plan never previews.
        let closed = plan_id(&w, "Basic Fitness (closed)");
        assert!(rows.iter().all(|r| r.id.plan_id != closed));
    }

    #[test]
    fn the_period_summary_counts_and_totals_its_own_rows() {
        let w = world();
        let rows = schedule_rows(&w.plans, &w.entries, Some(Period::of(today())), today());
        let summary = period_summary(&rows, &w.plans, &w.entries, &w.transactions);
        assert_eq!((summary.due, summary.overdue, summary.paid), (3, 2, 1));
        // 29.99 + 25.00 + 64.00 + 22.99 + 89.00 + 150.00 + 1180.00
        assert_eq!(
            summary.planned,
            Total::Single {
                unit: "aud".into(),
                amount: cents_money(156_098),
            }
        );
    }

    #[test]
    fn history_past_months_are_settled_by_transactions() {
        let w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let past: Vec<&BillScheduleEntry> = w
            .entries
            .iter()
            .filter(|e| e.plan_id == telstra && e.due < Period::of(today()).first_day())
            .collect();
        assert_eq!(past.len(), 15);
        assert!(
            past.iter()
                .all(|e| matches!(e.resolution, Resolution::Paid(_)))
        );
        // Same month last year exists for the History figures.
        assert!(past.iter().any(|e| e.due == date(2025, 9, 22)));
        let gym = plan_id(&w, "Gym \u{2014} Fitness First");
        assert!(
            w.entries
                .iter()
                .any(|e| e.plan_id == gym && matches!(e.resolution, Resolution::Skipped { .. }))
        );
        // The closed Plan stopped at its Ends On.
        let closed = plan_id(&w, "Basic Fitness (closed)");
        assert_eq!(dues(&w, closed).last(), Some(&date(2026, 5, 12)));
    }

    #[test]
    fn the_seed_is_deterministic() {
        let first = world();
        let second = world();
        assert_eq!(first.entries, second.entries);
        assert_eq!(first.transactions, second.transactions);
    }
}
