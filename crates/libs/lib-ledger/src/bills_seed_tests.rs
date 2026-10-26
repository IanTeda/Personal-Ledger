//! `lib-bills`' rules over the stub Ledger's Bills, against the handoff's 8a: generation,
//! settlement, the Schedule's rows and figures. Here rather than in `lib-bills`, since the stub
//! joins the Bills seed to the other domains' stubs and books its payments through
//! `lib-bill-payments`.

use bigdecimal::{BigDecimal, Signed};
use chrono::{Datelike, NaiveDate};
use lib_bills::*;
use lib_core::{AccountId, BillPlanId, CategoryId, PayeeId, Period};
use lib_core::{Money, Total, UnitId};

use crate::Ledger;

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
    accounts: Vec<BillAccount>,
    categories: Vec<BillCategory>,
    splits: Vec<BillSplit>,
    plans: Vec<BillPlan>,
    entries: Vec<BillScheduleEntry>,
}

fn world() -> World {
    let ledger = Ledger::stub_on(today());
    World {
        accounts: crate::bill_reads::bill_accounts(ledger.accounts()),
        categories: crate::bill_reads::bill_categories(ledger.categories()),
        splits: ledger.bill_splits(),
        plans: ledger.bill_plans().to_vec(),
        entries: ledger.bill_entries().to_vec(),
    }
}

fn draft(recurrence: Recurrence, first_due: NaiveDate) -> BillPlanDraft {
    BillPlanDraft {
        name: "Water".to_string(),
        category_id: CategoryId::seed(5),
        unit: UnitId::SEED_AUD,
        account_id: AccountId::seed(2),
        payee_id: Some(PayeeId::seed(15)),
        planned_amount: cents_money(9_000),
        amount_kind: AmountKind::Estimated,
        recurrence,
        first_due,
        ends_on: None,
        attention_lead: None,
    }
}

fn dues(w: &World, plan_id: BillPlanId) -> Vec<NaiveDate> {
    let mut dates: Vec<NaiveDate> = w
        .entries
        .iter()
        .filter(|e| e.plan_id == plan_id && !e.superseded)
        .map(|e| e.due)
        .collect();
    dates.sort();
    dates
}

fn plan_id(w: &World, name: &str) -> BillPlanId {
    w.plans.iter().find(|p| p.name == name).unwrap().id
}

fn id(plan_id: BillPlanId, due: NaiveDate) -> EntryId {
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
fn match_candidates_put_the_payee_match_first_and_preselect_it() {
    let w = world();
    let telstra = plan_id(&w, "Telstra Internet");
    let due = id(telstra, date(2026, 9, 22));
    let candidates = match_candidates(
        &w.plans,
        &w.entries,
        &w.splits,
        &w.accounts,
        &w.categories,
        due,
    );
    assert!(!candidates.is_empty());
    let plan = get(&w.plans, telstra).unwrap();
    let first = preselected_candidate(plan, &candidates, &w.splits).unwrap();
    let split = split_of(&w.splits, first).unwrap();
    assert_eq!(split.payee_id, plan.payee_id);
    assert_eq!(split.amount, cents_money(-7_999));
    // Nothing already Matched, nothing income, nothing outside the window.
    for c in &candidates {
        assert!(!is_matched(&w.entries, *c));
        let s = split_of(&w.splits, *c).unwrap();
        assert!((s.date - due.due).num_days().abs() <= MATCH_WINDOW_DAYS);
        assert!(s.amount.0.is_negative());
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
        &w.splits,
        &w.accounts,
        &w.categories,
        due,
    );
    let chosen = candidates[0];
    let before = w.splits.len();
    match_split(
        &w.plans,
        &mut w.entries,
        &w.splits,
        &w.accounts,
        &w.categories,
        due,
        chosen,
    )
    .unwrap();
    let e = entry(&w.entries, due).unwrap();
    assert_eq!(amount(e, &w.plans, &w.splits), Some(cents_money(7_999)));
    // A Matched Split is no longer anyone's candidate.
    let netflix = plan_id(&w, "Netflix");
    let other = match_candidates(
        &w.plans,
        &w.entries,
        &w.splits,
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
    assert_eq!(w.splits.len(), before);
}

#[test]
fn match_split_refuses_a_non_candidate() {
    let mut w = world();
    let telstra = plan_id(&w, "Telstra Internet");
    let due = id(telstra, date(2026, 9, 22));
    let far = SplitRef {
        transaction_id: w.splits.last().unwrap().split.transaction_id,
        split_index: 0,
    };
    let result = match_split(
        &w.plans,
        &mut w.entries,
        &w.splits,
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
    assert_eq!(amount(e, &w.plans, &w.splits), Some(cents_money(15_000)));
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
    let summary = period_summary(&september, &w.plans, &w.entries, &w.splits, |_| {
        "aud".to_string()
    });
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
    let summary = period_summary(&rows, &w.plans, &w.entries, &w.splits, |_| {
        "aud".to_string()
    });
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
    assert_eq!(first.splits, second.splits);
}

// --- the Schedule's filters and stat callout ---

fn cents(value: i64) -> Money {
    cents_money(value)
}

fn plan_named<'a>(world: &'a World, name: &str) -> &'a BillPlan {
    world.plans.iter().find(|p| p.name == name).unwrap()
}

#[test]
fn filters_keep_the_row_order_and_scope_by_plan() {
    let world = world();
    let all = schedule_rows(&world.plans, &world.entries, None, today());
    assert_eq!(BillFilters::default().apply(&all, &world.plans), all);

    let telstra = plan_named(&world, "Telstra Internet").id;
    let filters = BillFilters {
        plan_id: Some(telstra),
        statuses: vec![BillStatus::Paid],
        ..BillFilters::default()
    };
    let rows = filters.apply(&all, &world.plans);
    assert!(!rows.is_empty());
    assert!(
        rows.iter()
            .all(|r| r.id.plan_id == telstra && r.status == BillStatus::Paid)
    );
    assert!(rows.windows(2).all(|w| w[0].id.due > w[1].id.due));
}

#[test]
fn category_and_account_scope_by_plan() {
    let world = world();
    let all = schedule_rows(&world.plans, &world.entries, None, today());
    let transport = plan_named(&world, "Car Wash Membership").category_id;
    let filters = BillFilters {
        category_id: Some(transport),
        ..BillFilters::default()
    };
    let rows = filters.apply(&all, &world.plans);
    assert!(!rows.is_empty());
    assert!(
        rows.iter()
            .all(|r| { get(&world.plans, r.id.plan_id).unwrap().category_id == transport })
    );
}

#[test]
fn telstra_stats_read_last_paid_average_and_last_year() {
    let world = world();
    let plan = plan_named(&world, "Telstra Internet");
    let stats = plan_stats(plan, &world.plans, &world.entries, &world.splits, today());
    let (amount, _) = stats.last_paid.clone().unwrap();
    assert_eq!(amount, cents(8_900));
    assert_eq!(stats.average_year, 2025);
    assert!(stats.average.is_some());
    assert!(matches!(stats.same_period, SamePeriod::Payment(_)));
    assert!(!stats.never_paid);
}

#[test]
fn skipped_rows_are_left_out_of_the_average() {
    let mut world = world();
    let plan = plan_named(&world, "Gym \u{2014} Fitness First").clone();
    let before = plan_stats(&plan, &world.plans, &world.entries, &world.splits, today());
    // Every Gym payment was 64.00, so a zero from a Skip would drag the mean down.
    assert_eq!(before.average, Some(cents(6_400)));
    for entry in world.entries.iter_mut().filter(|e| e.plan_id == plan.id) {
        if matches!(entry.resolution, Resolution::Paid(_)) && entry.due.month() == 1 {
            entry.resolution = Resolution::Skipped {
                planned: cents(6_400),
            };
        }
    }
    let after = plan_stats(&plan, &world.plans, &world.entries, &world.splits, today());
    assert_eq!(after.average, Some(cents(6_400)));
}

#[test]
fn a_plan_never_paid_shows_dashes() {
    let world = world();
    let plan = plan_named(&world, "Rent");
    let stats = plan_stats(plan, &world.plans, &world.entries, &world.splits, today());
    assert!(stats.never_paid);
    assert_eq!(stats.last_paid, None);
    assert_eq!(stats.average, None);
    assert_eq!(stats.same_period, SamePeriod::None);
}
