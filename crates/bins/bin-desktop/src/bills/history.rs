//! The Bills Schedule tab's filters and the history figures behind its stat callout
//! (`docs/ux/desktop-mockups/12-bills/README.md`'s 8a, which absorbed 8f's History tab in #381) --
//! `gpui`-free and unit-tested, like `bills/mod.rs`.
//!
//! The rules are the Desktop Bills Surface map's History decisions (#369), as carried into the
//! Schedule by #381:
//!
//! - Status chips are multi-select (all five on by default); Bill Plan, Category and Account are
//!   single-select scopes over the rows `bills::schedule_rows` gives for the viewed month or All.
//! - The financial year starts in [`FINANCIAL_YEAR_START_MONTH`] until Settings' "Financial year
//!   starts" is wired to a real Preference.
//! - The stat callout reads only its one Bill Plan, never the other filters: Last paid is the Paid
//!   entry with the latest Matched-Transaction date (a later due date breaks a tie); Average is the
//!   mean of the previous complete financial year's Paid entries (by due date), Skipped ones left
//!   out rather than counted as zero; Same period last year looks back a year from Last paid's due
//!   date, by the Plan's Recurrence.

pub use lib_bills::history::*;

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use chrono::{Datelike, NaiveDate};
    use lib_core::Money;
    use lib_transactions::Transaction;

    use super::*;
    use crate::bills::{self, BillPlan, BillScheduleEntry, BillStatus, Resolution};
    use crate::{
        accounts::default_accounts, bills::default_bills, categories::default_categories,
        payees::default_payees, tags::default_tags, transactions::default_transactions,
    };

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn today() -> NaiveDate {
        date(2026, 9, 19)
    }

    fn cents(value: i64) -> Money {
        Money(BigDecimal::new(value.into(), 2))
    }

    struct World {
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
            transactions,
            plans: seed.plans,
            entries: seed.entries,
        }
    }

    fn plan_named<'a>(world: &'a World, name: &str) -> &'a BillPlan {
        world.plans.iter().find(|p| p.name == name).unwrap()
    }

    #[test]
    fn filters_keep_the_row_order_and_scope_by_plan() {
        let world = world();
        let all = bills::schedule_rows(&world.plans, &world.entries, None, today());
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
        let all = bills::schedule_rows(&world.plans, &world.entries, None, today());
        let transport = plan_named(&world, "Car Wash Membership").category_id;
        let filters = BillFilters {
            category_id: Some(transport),
            ..BillFilters::default()
        };
        let rows = filters.apply(&all, &world.plans);
        assert!(!rows.is_empty());
        assert!(
            rows.iter().all(|r| {
                bills::get(&world.plans, r.id.plan_id).unwrap().category_id == transport
            })
        );
    }

    #[test]
    fn telstra_stats_read_last_paid_average_and_last_year() {
        let world = world();
        let plan = plan_named(&world, "Telstra Internet");
        let stats = plan_stats(
            plan,
            &world.plans,
            &world.entries,
            &world.transactions,
            today(),
        );
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
        let before = plan_stats(
            &plan,
            &world.plans,
            &world.entries,
            &world.transactions,
            today(),
        );
        // Every Gym payment was 64.00, so a zero from a Skip would drag the mean down.
        assert_eq!(before.average, Some(cents(6_400)));
        for entry in world.entries.iter_mut().filter(|e| e.plan_id == plan.id) {
            if matches!(entry.resolution, Resolution::Paid(_)) && entry.due.month() == 1 {
                entry.resolution = Resolution::Skipped {
                    planned: cents(6_400),
                };
            }
        }
        let after = plan_stats(
            &plan,
            &world.plans,
            &world.entries,
            &world.transactions,
            today(),
        );
        assert_eq!(after.average, Some(cents(6_400)));
    }

    #[test]
    fn a_plan_never_paid_shows_dashes() {
        let world = world();
        let plan = plan_named(&world, "Rent");
        let stats = plan_stats(
            plan,
            &world.plans,
            &world.entries,
            &world.transactions,
            today(),
        );
        assert!(stats.never_paid);
        assert_eq!(stats.last_paid, None);
        assert_eq!(stats.average, None);
        assert_eq!(stats.same_period, SamePeriod::None);
    }
}
