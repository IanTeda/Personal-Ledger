//! [`PayeeFixture`]: the in-memory [`PayeeStore`] every ticket in the "Payees screen, views and
//! popup" map (issue #134) builds against, seeded with `docs/ux/tui/payees/README.md`'s own
//! mock payees and amounts (§*8a*'s Sunrise Payroll/Home Loan Direct/Woolworths/Coles Central/
//! Origin Energy/Telstra/WOOLIES rows, §*8a*'s Woolworths record/mix example).
//!
//! **Category paths are real, copied once from a temporary `crate::category::CategoryFixture`
//! at seed time** — mirroring `crate::tag::fixture`'s own `expense_leaf_paths` precedent
//! exactly, generalised here to pull leaves from either root (`Income` for Sunrise Payroll's
//! own mix, `Expenses` for everyone else). Only the plain path *strings* are copied; nothing
//! here holds a `RowID` back to Category. The fixture's leaf set has no dedicated "utilities"
//! category yet, so Origin Energy's/Telstra's mixes lean on the nearest available leaf
//! (`housing/insurance`) rather than inventing a category name that doesn't really exist — the
//! same "it's hand-drawn wireframe, not a spreadsheet" allowance `crate::category::fixture`'s
//! own module doc already claims.
//!
//! **The Woolworths/WOOLIES pair is this fixture's deliberate stand-in for the README's own
//! "`WOOLIES` also matches Coles Central" conflict example**: Woolworths holds a hand-authored
//! (`source = Manual`) alias `(?i)^WOOLIES$`, which collides with a *real*, separately-seeded
//! Payee actually named `WOOLIES` — a same-brand pair reads more plausibly than the handoff's
//! own arbitrary pairing, and exercises exactly the same "a pattern equal to another payee's
//! name is dead on arrival" rule the README's own resolution-order note describes.
//!
//! Seeded with a deliberate mix beyond the README's own named rows: `Bunnings Warehouse`'s
//! stored default disagrees with its real mix (exercises the "default follows the mix"
//! disagreement callout), and `Old Vendor` is inactive with no transactions or aliases
//! (exercises `za` and the one case `PayeeStore::delete` actually allows).

use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::NaiveDate;

use super::{
    AliasMode, AliasSource, Payee, PayeeAlias, PayeeCategoryShare, PayeeError, PayeeResolution,
    PayeeStore, PayeeTransaction, TransactionStatus,
};
use crate::category::{CategoryFixture, CategoryStore};
use lib_core::{Money, RowID};

/// The fixture's fixed "now" — matches `crate::account::fixture`'s/`crate::tag::fixture`'s own
/// `2026-09-08`, so anything cross-referencing every screen (the shell's status line, "today")
/// stays consistent.
pub const FIXTURE_NOW: NaiveDate = match NaiveDate::from_ymd_opt(2026, 9, 8) {
    Some(date) => date,
    None => panic!("fixed literal is a valid date"),
};

/// Builds an exact `Money` amount from a whole part and a fractional numerator over
/// `10^scale` — avoids both `f64` rounding error and `unwrap`/`expect` on parsed string
/// literals for seed data that's obviously always well-formed (mirrors
/// `crate::account::fixture`'s own `money` helper).
fn money(whole: i64, frac: i64, scale: u32) -> Money {
    let sign = if whole < 0 { -1 } else { 1 };
    let magnitude = BigDecimal::from(whole.abs())
        + BigDecimal::from(frac.abs()) / BigDecimal::from(10i64.pow(scale));
    Money(magnitude * BigDecimal::from(sign))
}

fn money_to_f64(value: &Money) -> f64 {
    value.0.to_string().parse().unwrap_or(0.0)
}

fn round_money(amount: f64) -> Money {
    Money(
        BigDecimal::from_f64(amount)
            .unwrap_or_default()
            .with_scale(2),
    )
}

/// A tiny xorshift PRNG step — deterministic across runs/platforms, the same technique
/// `crate::account::fixture`'s/`crate::tag::fixture`'s own fake data already uses.
fn xorshift(seed: u64) -> u64 {
    let mut seed = seed;
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}

/// A deterministic seed derived from a `RowID`, so the same Payee always generates the same
/// fake transactions — mirrors `crate::account::fixture::seed_from_id`.
fn seed_from_id(id: RowID) -> u64 {
    let uuid = id.into_uuid();
    let bytes = uuid.as_bytes();
    u64::from_be_bytes(
        bytes[8..16]
            .try_into()
            .expect("a uuid's byte array is always at least 16 bytes long"),
    )
}

/// Walks `id` up through `categories` to its root, lowercasing each name along the way — a
/// local copy of `crate::tag::fixture::ancestor_path`'s own technique (domain modules
/// shouldn't depend on each other for a private helper this small).
fn ancestor_path(categories: &CategoryFixture, id: RowID) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = categories.find(id);
    while let Some(node) = current {
        names.push(node.name.to_lowercase());
        current = node
            .parent_id
            .and_then(|parent_id| categories.find(parent_id));
    }
    names.reverse();
    names
}

/// Every leaf under `categories`' root named `root_name`, as a `"parent/child"`-style path
/// with the root segment itself dropped — generalises `crate::tag::fixture::
/// expense_leaf_paths` to either root, since Sunrise Payroll's own mix needs `Income` leaves
/// and everyone else needs `Expenses` leaves.
fn leaf_paths_under(categories: &CategoryFixture, root_name: &str) -> Vec<String> {
    let mut paths: Vec<String> = categories
        .nodes()
        .iter()
        .filter(|node| categories.children(node.id).is_empty())
        .filter(|node| {
            categories
                .root(node.id)
                .is_some_and(|root| root.name.eq_ignore_ascii_case(root_name))
        })
        .map(|node| {
            let mut segments = ancestor_path(categories, node.id);
            if segments.len() > 1 {
                segments.remove(0); // drop the root segment itself
            }
            segments.join("/")
        })
        .collect();
    paths.sort();
    paths
}

/// Picks one of `weights`' category paths, proportional to their relative weight — `seed`
/// drives a uniform pick over the cumulative weight, the same "one xorshift step, one choice"
/// technique the rest of this fixture's generators use for every other random decision.
fn weighted_pick(weights: &[(String, f64)], total_weight: f64, seed: u64) -> String {
    let target = (seed as f64 / u64::MAX as f64) * total_weight;
    let mut cumulative = 0.0;
    for (path, weight) in weights {
        cumulative += weight;
        if target <= cumulative {
            return path.clone();
        }
    }
    weights
        .last()
        .map(|(path, _)| path.clone())
        .unwrap_or_default()
}

/// Generates `payee`'s transaction rows, deterministically seeded from its id — see [`Payee`]'s
/// own doc on why these aren't stored. Every row shares `transactions_sum`'s own sign (a real
/// Payee's Transactions overwhelmingly flow one way — unlike an Account, which can hold both
/// debits and credits), with a residual folded into the last row so the rows' `BigDecimal` sum
/// is exactly `payee.transactions_sum`, mirroring `crate::account::fixture::ledger_for`'s own
/// technique.
pub(super) fn transactions_for(payee: &Payee) -> Vec<PayeeTransaction> {
    let count = payee.transaction_count as usize;
    let (Some(first), Some(last)) = (payee.first_posted, payee.last_posted) else {
        return Vec::new();
    };
    if count == 0 || payee.category_weights.is_empty() {
        return Vec::new();
    }

    let span_days = (last - first).num_days().max(0) as u64;
    let total_weight: f64 = payee
        .category_weights
        .iter()
        .map(|(_, weight)| weight)
        .sum();
    let sign = if payee.transactions_sum.0 < 0 {
        -1.0
    } else {
        1.0
    };
    let avg_magnitude = money_to_f64(&payee.transactions_sum).abs() / count as f64;

    let mut seed = seed_from_id(payee.id);
    let mut rows = Vec::with_capacity(count);
    let mut running = BigDecimal::from(0);

    for index in 0..count {
        let offset_days = if index == 0 {
            0
        } else if index == count - 1 {
            span_days
        } else {
            seed = xorshift(seed);
            seed % (span_days + 1)
        };
        let date = first + chrono::Duration::days(offset_days as i64);

        seed = xorshift(seed);
        let category_path = weighted_pick(&payee.category_weights, total_weight, seed);

        seed = xorshift(seed);
        let wobble = 0.4 + (seed % 120) as f64 / 100.0; // 0.4..1.6
        let amount = round_money(avg_magnitude * wobble * sign);
        running += amount.0.clone();

        rows.push(PayeeTransaction {
            date,
            category_path,
            amount,
            status: TransactionStatus::Reconciled,
        });
    }

    if let Some(last_row) = rows.last_mut() {
        let residual = payee.transactions_sum.0.clone() - running;
        last_row.amount.0 += residual;
    }

    rows.sort_by_key(|row| std::cmp::Reverse(row.date));
    let open_count = (payee.open_count as usize).min(rows.len());
    for row in rows[..open_count].iter_mut() {
        row.status = TransactionStatus::Open;
    }

    rows
}

/// `payee`'s spend grouped by category, biggest absolute total first — derived from
/// [`transactions_for`], never stored independently.
pub(super) fn category_mix_for(payee: &Payee) -> Vec<PayeeCategoryShare> {
    let rows = transactions_for(payee);
    let mut totals: Vec<(String, BigDecimal)> = Vec::new();
    for row in &rows {
        match totals
            .iter_mut()
            .find(|(path, _)| *path == row.category_path)
        {
            Some(entry) => entry.1 += row.amount.0.clone(),
            None => totals.push((row.category_path.clone(), row.amount.0.clone())),
        }
    }
    let total_abs: BigDecimal = totals.iter().map(|(_, amount)| amount.abs()).sum();
    totals.sort_by_key(|(_, amount)| std::cmp::Reverse(amount.abs()));

    totals
        .into_iter()
        .map(|(category_path, amount)| {
            let share = if total_abs == 0 {
                0.0
            } else {
                let amount_f64: f64 = amount.abs().to_string().parse().unwrap_or(0.0);
                let total_f64: f64 = total_abs.to_string().parse().unwrap_or(0.0);
                amount_f64 / total_f64
            };
            PayeeCategoryShare {
                category_path,
                amount: Money(amount),
                share,
            }
        })
        .collect()
}

/// An in-memory `PayeeStore`, seeded once with a fixed demo payee list plus their aliases.
/// Every mutating method mutates these same `Vec`s in place (discarded on quit, since nothing
/// here is persisted).
pub struct PayeeFixture {
    payees: Vec<Payee>,
    aliases: Vec<PayeeAlias>,
}

impl Default for PayeeFixture {
    fn default() -> Self {
        Self::new()
    }
}

impl PayeeFixture {
    /// Seeds the demo payee list. Ids are built directly via
    /// `uuid::Builder::from_unix_timestamp_millis` with a plain incrementing counter standing
    /// in for a real v7 UUID's random bits — **not** `RowID::from_timestamp`, which is *not*
    /// actually deterministic across runs (the bug found and fixed against
    /// `crate::account::fixture`, issue #116; still open against `crate::category::fixture`,
    /// issue #124).
    pub fn new() -> Self {
        use chrono::{DateTime, Utc};

        let mut next = DateTime::parse_from_rfc3339("2021-01-01T00:00:00Z")
            .expect("fixed literal is a valid RFC3339 timestamp")
            .with_timezone(&Utc);
        let mut counter: u64 = 0;
        let mut id = move || {
            let millis = next.timestamp_millis() as u64;
            next += chrono::Duration::seconds(1);
            counter += 1;
            let mut counter_bytes = [0u8; 10];
            counter_bytes[2..10].copy_from_slice(&counter.to_be_bytes());
            let uuid =
                uuid::Builder::from_unix_timestamp_millis(millis, &counter_bytes).into_uuid();
            RowID::from_uuid(uuid)
        };

        let date = |year, month, day| {
            NaiveDate::from_ymd_opt(year, month, day).expect("seed literal is a valid date")
        };

        // Read once, then discarded — see this module's own doc on why nothing here keeps a
        // `CategoryFixture` or a `RowID` back to it.
        let categories = CategoryFixture::new();
        let income_leaves = leaf_paths_under(&categories, "Income");
        let expense_leaves = leaf_paths_under(&categories, "Expenses");
        let pick = |paths: &[String], needle: &str| -> String {
            paths
                .iter()
                .find(|path| path.contains(needle))
                .cloned()
                .unwrap_or_else(|| paths.first().cloned().unwrap_or_default())
        };

        let salary = pick(&income_leaves, "primary job");
        let mortgage_principal = pick(&expense_leaves, "mortgage/principal");
        let mortgage_interest = pick(&expense_leaves, "mortgage/interest");
        let groceries = pick(&expense_leaves, "food/groceries");
        let takeaway = pick(&expense_leaves, "food/takeaway");
        let pharmacy = pick(&expense_leaves, "health/pharmacy");
        let parking = pick(&expense_leaves, "transport/parking");
        // No dedicated "utilities" leaf exists yet — `housing/insurance` is the nearest
        // available real leaf, see this module's own doc.
        let insurance = pick(&expense_leaves, "housing/insurance");
        let fuel = pick(&expense_leaves, "transport/fuel");

        let sunrise_payroll_id = id();
        let home_loan_direct_id = id();
        let woolworths_id = id();
        let coles_central_id = id();
        let origin_energy_id = id();
        let telstra_id = id();
        let woolies_id = id();
        let bunnings_warehouse_id = id();
        let old_vendor_id = id();

        let payees = vec![
            Payee {
                id: sunrise_payroll_id,
                name: "Sunrise Payroll".to_string(),
                is_active: true,
                website: None,
                icon_url: None,
                icon_derived: false,
                default_category_path: Some(salary.clone()),
                created_on: date(2024, 10, 15),
                updated_on: FIXTURE_NOW,
                transaction_count: 24,
                open_count: 0,
                first_posted: Some(date(2024, 10, 31)),
                last_posted: Some(date(2026, 9, 5)),
                transactions_sum: money(165_360, 0, 2),
                category_weights: vec![(salary, 1.0)],
            },
            Payee {
                id: home_loan_direct_id,
                name: "Home Loan Direct".to_string(),
                is_active: true,
                website: None,
                icon_url: None,
                icon_derived: false,
                default_category_path: Some(mortgage_principal.clone()),
                created_on: date(2023, 9, 20),
                updated_on: FIXTURE_NOW,
                transaction_count: 36,
                open_count: 0,
                first_posted: Some(date(2023, 10, 8)),
                last_posted: Some(date(2026, 9, 1)),
                transactions_sum: money(-42_180, 0, 2),
                category_weights: vec![(mortgage_principal, 0.75), (mortgage_interest, 0.25)],
            },
            Payee {
                id: woolworths_id,
                name: "Woolworths".to_string(),
                is_active: true,
                website: Some("woolworths.com.au".to_string()),
                icon_url: Some("woolworths.com.au/favicon.ico".to_string()),
                icon_derived: true,
                default_category_path: Some(groceries.clone()),
                created_on: date(2024, 10, 3),
                updated_on: FIXTURE_NOW,
                transaction_count: 184,
                open_count: 1,
                first_posted: Some(date(2024, 10, 5)),
                last_posted: Some(date(2026, 9, 6)),
                transactions_sum: money(-18_402, 55, 2),
                category_weights: vec![
                    (groceries.clone(), 0.84),
                    (takeaway.clone(), 0.11),
                    (pharmacy.clone(), 0.05),
                ],
            },
            Payee {
                id: coles_central_id,
                name: "Coles Central".to_string(),
                is_active: true,
                website: None,
                icon_url: None,
                icon_derived: false,
                default_category_path: None,
                created_on: date(2024, 6, 1),
                updated_on: FIXTURE_NOW,
                transaction_count: 96,
                open_count: 0,
                first_posted: Some(date(2024, 6, 10)),
                last_posted: Some(date(2026, 8, 29)),
                transactions_sum: money(-9_118, 40, 2),
                // Deliberately no dominant category — the "spread evenly, no default" case.
                category_weights: vec![
                    (groceries.clone(), 0.42),
                    (takeaway, 0.33),
                    (parking, 0.25),
                ],
            },
            Payee {
                id: origin_energy_id,
                name: "Origin Energy".to_string(),
                is_active: true,
                website: None,
                icon_url: None,
                icon_derived: false,
                default_category_path: Some(insurance.clone()),
                created_on: date(2022, 3, 1),
                updated_on: FIXTURE_NOW,
                transaction_count: 24,
                open_count: 0,
                first_posted: Some(date(2022, 4, 1)),
                last_posted: Some(date(2026, 8, 20)),
                transactions_sum: money(-5_784, 60, 2),
                category_weights: vec![(insurance.clone(), 1.0)],
            },
            Payee {
                id: telstra_id,
                name: "Telstra".to_string(),
                is_active: true,
                website: None,
                icon_url: None,
                icon_derived: false,
                default_category_path: None,
                created_on: date(2023, 1, 1),
                updated_on: FIXTURE_NOW,
                transaction_count: 36,
                open_count: 0,
                first_posted: Some(date(2023, 2, 1)),
                last_posted: Some(date(2026, 8, 15)),
                transactions_sum: money(-3_204, 0, 2),
                // Deliberately even — the "spread evenly, no default" case, a second time.
                category_weights: vec![(insurance.clone(), 0.5), (fuel.clone(), 0.5)],
            },
            Payee {
                id: woolies_id,
                name: "WOOLIES".to_string(),
                is_active: true,
                website: None,
                icon_url: None,
                icon_derived: false,
                default_category_path: None,
                created_on: date(2025, 6, 1),
                updated_on: FIXTURE_NOW,
                transaction_count: 6,
                open_count: 0,
                first_posted: Some(date(2025, 6, 10)),
                last_posted: Some(date(2026, 7, 1)),
                transactions_sum: money(-920, 10, 2),
                category_weights: vec![(groceries.clone(), 1.0)],
            },
            Payee {
                id: bunnings_warehouse_id,
                name: "Bunnings Warehouse".to_string(),
                is_active: true,
                website: None,
                icon_url: None,
                icon_derived: false,
                // Deliberately stale/wrong — exercises the "default disagrees with the mix"
                // callout (README §*Right pane*).
                default_category_path: Some(groceries),
                created_on: date(2023, 5, 1),
                updated_on: FIXTURE_NOW,
                transaction_count: 40,
                open_count: 0,
                first_posted: Some(date(2023, 5, 20)),
                last_posted: Some(date(2026, 8, 10)),
                transactions_sum: money(-6_250, 0, 2),
                category_weights: vec![(insurance.clone(), 0.9), (fuel, 0.1)],
            },
            Payee {
                id: old_vendor_id,
                name: "Old Vendor".to_string(),
                is_active: false,
                website: None,
                icon_url: None,
                icon_derived: false,
                default_category_path: None,
                created_on: date(2021, 1, 1),
                updated_on: date(2021, 1, 1),
                transaction_count: 0,
                open_count: 0,
                first_posted: None,
                last_posted: None,
                transactions_sum: money(0, 0, 2),
                category_weights: Vec::new(),
            },
        ];

        let aliases = vec![
            PayeeAlias {
                id: id(),
                payee_id: home_loan_direct_id,
                pattern: "(?i)^Bank Direct Debit$".to_string(),
                source: AliasSource::Rename,
                hits: 30,
            },
            PayeeAlias {
                id: id(),
                payee_id: woolworths_id,
                pattern: "(?i)^Woolworths Metro$".to_string(),
                source: AliasSource::Rename,
                hits: 12,
            },
            // Collides with the real Payee named "WOOLIES" below — see this module's own doc.
            PayeeAlias {
                id: id(),
                payee_id: woolworths_id,
                pattern: "(?i)^WOOLIES$".to_string(),
                source: AliasSource::Manual,
                hits: 3,
            },
            PayeeAlias {
                id: id(),
                payee_id: coles_central_id,
                pattern: "(?i)^Coles$".to_string(),
                source: AliasSource::Manual,
                hits: 40,
            },
            PayeeAlias {
                id: id(),
                payee_id: telstra_id,
                pattern: "(?i)^Telstra Corp$".to_string(),
                source: AliasSource::Manual,
                hits: 2,
            },
            PayeeAlias {
                id: id(),
                payee_id: woolies_id,
                pattern: "(?i)^WOOLLIES$".to_string(),
                source: AliasSource::Manual,
                hits: 1,
            },
        ];

        Self { payees, aliases }
    }

    fn find_holder(&self, name: &str, excluding: Option<RowID>) -> Option<&Payee> {
        self.payees
            .iter()
            .find(|payee| Some(payee.id) != excluding && payee.name.eq_ignore_ascii_case(name))
    }
}

impl PayeeStore for PayeeFixture {
    fn payees(&self) -> &[Payee] {
        &self.payees
    }

    fn find(&self, id: RowID) -> Option<&Payee> {
        self.payees.iter().find(|payee| payee.id == id)
    }

    fn find_by_name(&self, name: &str) -> Option<&Payee> {
        self.find_holder(name, None)
    }

    fn aliases(&self, id: RowID) -> Vec<&PayeeAlias> {
        self.aliases
            .iter()
            .filter(|alias| alias.payee_id == id)
            .collect()
    }

    fn total(&self, id: RowID) -> Money {
        self.find(id)
            .map(|payee| payee.transactions_sum.clone())
            .unwrap_or_else(|| money(0, 0, 2))
    }

    fn category_mix(&self, id: RowID) -> Vec<PayeeCategoryShare> {
        self.find(id).map(category_mix_for).unwrap_or_default()
    }

    fn transactions(&self, id: RowID) -> Vec<PayeeTransaction> {
        self.find(id).map(transactions_for).unwrap_or_default()
    }

    fn conflict_partners(&self, id: RowID) -> Vec<RowID> {
        let Some(payee) = self.find(id) else {
            return Vec::new();
        };
        let mine = self.aliases(id);

        self.payees
            .iter()
            .filter(|other| other.id != id)
            .filter(|other| {
                let theirs = self.aliases(other.id);
                let mine_hit_their_name = mine.iter().any(|alias| {
                    regex::Regex::new(&alias.pattern).is_ok_and(|re| re.is_match(&other.name))
                });
                let theirs_hit_my_name = theirs.iter().any(|alias| {
                    regex::Regex::new(&alias.pattern).is_ok_and(|re| re.is_match(&payee.name))
                });
                let shared_pattern = mine.iter().any(|mine_alias| {
                    theirs
                        .iter()
                        .any(|their_alias| their_alias.pattern == mine_alias.pattern)
                });
                mine_hit_their_name || theirs_hit_my_name || shared_pattern
            })
            .map(|other| other.id)
            .collect()
    }

    fn create(
        &mut self,
        name: String,
        website: Option<String>,
        icon_url: Option<String>,
        icon_derived: bool,
        default_category_path: Option<String>,
        active: bool,
    ) -> Result<RowID, PayeeError> {
        if let Some(holder) = self.find_holder(&name, None) {
            return Err(PayeeError::DuplicateName {
                holder: holder.name.clone(),
            });
        }
        let id = RowID::new();
        self.payees.push(Payee {
            id,
            name,
            is_active: active,
            website,
            icon_url,
            icon_derived,
            default_category_path,
            created_on: FIXTURE_NOW,
            updated_on: FIXTURE_NOW,
            transaction_count: 0,
            open_count: 0,
            first_posted: None,
            last_posted: None,
            transactions_sum: money(0, 0, 2),
            category_weights: Vec::new(),
        });
        Ok(id)
    }

    fn rename(&mut self, id: RowID, new_name: String) -> Result<(), PayeeError> {
        let current_name = self.find(id).ok_or(PayeeError::NotFound)?.name.clone();
        if new_name.eq_ignore_ascii_case(&current_name) {
            return Ok(());
        }
        if let Some(holder) = self.find_holder(&new_name, Some(id)) {
            return Err(PayeeError::DuplicateName {
                holder: holder.name.clone(),
            });
        }

        self.aliases.push(PayeeAlias {
            id: RowID::new(),
            payee_id: id,
            pattern: format!("(?i)^{}$", regex::escape(&current_name)),
            source: AliasSource::Rename,
            hits: 0,
        });

        let payee = self
            .payees
            .iter_mut()
            .find(|payee| payee.id == id)
            .expect("existence just checked above");
        payee.name = new_name;
        payee.updated_on = FIXTURE_NOW;
        Ok(())
    }

    fn update(
        &mut self,
        id: RowID,
        website: Option<String>,
        icon_url: Option<String>,
        icon_derived: bool,
        default_category_path: Option<String>,
    ) -> Result<(), PayeeError> {
        let payee = self
            .payees
            .iter_mut()
            .find(|payee| payee.id == id)
            .ok_or(PayeeError::NotFound)?;
        payee.website = website;
        payee.icon_url = icon_url;
        payee.icon_derived = icon_derived;
        payee.default_category_path = default_category_path;
        payee.updated_on = FIXTURE_NOW;
        Ok(())
    }

    fn set_active(&mut self, id: RowID, active: bool) -> Result<(), PayeeError> {
        let payee = self
            .payees
            .iter_mut()
            .find(|payee| payee.id == id)
            .ok_or(PayeeError::NotFound)?;
        payee.is_active = active;
        payee.updated_on = FIXTURE_NOW;
        Ok(())
    }

    fn delete(&mut self, id: RowID) -> Result<(), PayeeError> {
        let payee = self.find(id).ok_or(PayeeError::NotFound)?;
        let alias_count = self.aliases(id).len();
        if payee.transaction_count > 0 || alias_count > 0 {
            return Err(PayeeError::ReferencesExist {
                transaction_count: payee.transaction_count,
                alias_count,
            });
        }
        self.payees.retain(|payee| payee.id != id);
        Ok(())
    }

    fn add_alias(
        &mut self,
        payee_id: RowID,
        typed: &str,
        mode: AliasMode,
    ) -> Result<RowID, PayeeError> {
        self.find(payee_id).ok_or(PayeeError::NotFound)?;
        let pattern = match mode {
            AliasMode::ExactText => format!("(?i)^{}$", regex::escape(typed)),
            AliasMode::Regex => typed.to_string(),
        };

        for other in self.payees.iter().filter(|payee| payee.id != payee_id) {
            let matches_their_name =
                regex::Regex::new(&pattern).is_ok_and(|re| re.is_match(&other.name));
            if matches_their_name {
                return Err(PayeeError::PatternCollision {
                    pattern,
                    other: other.name.clone(),
                });
            }
            let duplicates_their_pattern = self
                .aliases(other.id)
                .iter()
                .any(|alias| alias.pattern == pattern);
            if duplicates_their_pattern {
                return Err(PayeeError::PatternCollision {
                    pattern,
                    other: other.name.clone(),
                });
            }
        }

        let id = RowID::new();
        self.aliases.push(PayeeAlias {
            id,
            payee_id,
            pattern,
            source: AliasSource::Manual,
            hits: 0,
        });
        Ok(id)
    }

    fn remove_alias(&mut self, alias_id: RowID) -> Result<(), PayeeError> {
        let alias = self
            .aliases
            .iter()
            .find(|alias| alias.id == alias_id)
            .ok_or(PayeeError::AliasNotFound)?;
        if alias.source == AliasSource::Rename {
            return Err(PayeeError::RenameProtectedAlias);
        }
        self.aliases.retain(|alias| alias.id != alias_id);
        Ok(())
    }

    fn resolve(&self, text: &str) -> PayeeResolution {
        let text = text.trim();
        if let Some(payee) = self.find_by_name(text) {
            return PayeeResolution::ExactName(payee.id);
        }
        for alias in &self.aliases {
            if regex::Regex::new(&alias.pattern).is_ok_and(|re| re.is_match(text)) {
                return PayeeResolution::Alias {
                    payee_id: alias.payee_id,
                    alias_id: alias.id,
                };
            }
        }
        PayeeResolution::WouldCreate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_by_name<'a>(store: &'a PayeeFixture, name: &str) -> &'a Payee {
        store
            .payees()
            .iter()
            .find(|payee| payee.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a payee named {name}"))
    }

    #[test]
    fn seeds_nine_payees_eight_active() {
        let store = PayeeFixture::new();
        assert_eq!(store.payees().len(), 9);
        assert_eq!(store.payees().iter().filter(|p| p.is_active).count(), 8);
    }

    #[test]
    fn totals_match_the_handoffs_displayed_figures() {
        let store = PayeeFixture::new();
        for (name, expected) in [
            ("Sunrise Payroll", money(165_360, 0, 2)),
            ("Home Loan Direct", money(-42_180, 0, 2)),
            ("Woolworths", money(-18_402, 55, 2)),
            ("Coles Central", money(-9_118, 40, 2)),
            ("Origin Energy", money(-5_784, 60, 2)),
            ("Telstra", money(-3_204, 0, 2)),
            ("WOOLIES", money(-920, 10, 2)),
        ] {
            let payee = find_by_name(&store, name);
            assert_eq!(store.total(payee.id), expected, "{name}");
        }
    }

    #[test]
    fn alias_counts_match_the_handoffs_m_column() {
        let store = PayeeFixture::new();
        for (name, expected) in [
            ("Sunrise Payroll", 0),
            ("Home Loan Direct", 1),
            ("Woolworths", 2),
            ("Coles Central", 1),
            ("Origin Energy", 0),
            ("Telstra", 1),
            ("WOOLIES", 1),
        ] {
            let payee = find_by_name(&store, name);
            assert_eq!(store.aliases(payee.id).len(), expected, "{name}");
        }
    }

    #[test]
    fn woolworths_and_woolies_are_flagged_as_conflict_partners() {
        let store = PayeeFixture::new();
        let woolworths = find_by_name(&store, "Woolworths").id;
        let woolies = find_by_name(&store, "WOOLIES").id;

        assert!(store.conflict_partners(woolworths).contains(&woolies));
        assert!(store.conflict_partners(woolies).contains(&woolworths));
    }

    #[test]
    fn unrelated_payees_have_no_conflict_partners() {
        let store = PayeeFixture::new();
        let sunrise_payroll = find_by_name(&store, "Sunrise Payroll").id;
        assert!(store.conflict_partners(sunrise_payroll).is_empty());
    }

    #[test]
    fn coles_central_and_telstra_have_no_default_category() {
        let store = PayeeFixture::new();
        assert_eq!(
            find_by_name(&store, "Coles Central").default_category_path,
            None
        );
        assert_eq!(find_by_name(&store, "Telstra").default_category_path, None);
    }

    #[test]
    fn woolworths_category_mix_agrees_with_its_stored_default() {
        let store = PayeeFixture::new();
        let woolworths = find_by_name(&store, "Woolworths");
        let mix = store.category_mix(woolworths.id);
        let top = mix.first().expect("woolworths has a mix");
        assert_eq!(
            Some(top.category_path.clone()),
            woolworths.default_category_path
        );
        assert!(top.share > 0.7, "groceries should dominate: {}", top.share);
    }

    #[test]
    fn bunnings_warehouses_category_mix_disagrees_with_its_stored_default() {
        let store = PayeeFixture::new();
        let bunnings = find_by_name(&store, "Bunnings Warehouse");
        let mix = store.category_mix(bunnings.id);
        let top = mix.first().expect("bunnings has a mix");
        assert_ne!(
            Some(top.category_path.clone()),
            bunnings.default_category_path
        );
    }

    #[test]
    fn category_mix_shares_sum_to_one_and_sort_biggest_first() {
        let store = PayeeFixture::new();
        let woolworths = find_by_name(&store, "Woolworths").id;
        let mix = store.category_mix(woolworths);

        let total_share: f64 = mix.iter().map(|row| row.share).sum();
        assert!(
            (total_share - 1.0).abs() < 0.01,
            "total share was {total_share}"
        );

        for pair in mix.windows(2) {
            assert!(pair[0].share >= pair[1].share);
        }
    }

    #[test]
    fn transactions_sum_to_transactions_sum_exactly() {
        let store = PayeeFixture::new();
        let woolworths = find_by_name(&store, "Woolworths");
        let rows = store.transactions(woolworths.id);
        assert_eq!(rows.len(), 184);

        let sum: BigDecimal = rows.iter().map(|row| row.amount.0.clone()).sum();
        assert_eq!(Money(sum), woolworths.transactions_sum);
    }

    #[test]
    fn rename_to_the_same_name_case_insensitively_is_a_no_op() {
        let mut store = PayeeFixture::new();
        let woolworths = find_by_name(&store, "Woolworths").id;
        let aliases_before = store.aliases(woolworths).len();

        store
            .rename(woolworths, "WOOLWORTHS".to_string())
            .expect("a case-only rename should succeed as a no-op");

        assert_eq!(store.aliases(woolworths).len(), aliases_before);
        assert_eq!(store.find(woolworths).unwrap().name, "Woolworths");
    }

    #[test]
    fn rename_to_a_new_name_writes_a_protected_rename_alias() {
        let mut store = PayeeFixture::new();
        let telstra = find_by_name(&store, "Telstra").id;
        let aliases_before = store.aliases(telstra).len();

        store
            .rename(telstra, "Telstra Retail".to_string())
            .expect("rename should succeed");

        let aliases_after = store.aliases(telstra);
        assert_eq!(aliases_after.len(), aliases_before + 1);
        let new_alias = aliases_after
            .iter()
            .find(|alias| alias.pattern == "(?i)^Telstra$")
            .expect("the prior name should be stored as the new alias's pattern");
        assert_eq!(new_alias.source, AliasSource::Rename);

        assert_eq!(
            store.remove_alias(new_alias.id),
            Err(PayeeError::RenameProtectedAlias)
        );
    }

    #[test]
    fn rename_refuses_a_case_insensitive_collision_with_another_payee() {
        let mut store = PayeeFixture::new();
        let coles_central = find_by_name(&store, "Coles Central").id;

        assert_eq!(
            store.rename(coles_central, "woolworths".to_string()),
            Err(PayeeError::DuplicateName {
                holder: "Woolworths".to_string()
            })
        );
    }

    #[test]
    fn delete_refuses_a_payee_with_transactions_or_aliases() {
        let mut store = PayeeFixture::new();
        let woolworths = find_by_name(&store, "Woolworths").id;
        assert_eq!(
            store.delete(woolworths),
            Err(PayeeError::ReferencesExist {
                transaction_count: 184,
                alias_count: 2,
            })
        );
    }

    #[test]
    fn delete_allows_an_unreferenced_payee() {
        let mut store = PayeeFixture::new();
        let old_vendor = find_by_name(&store, "Old Vendor").id;
        store
            .delete(old_vendor)
            .expect("an unreferenced payee should delete");
        assert!(store.find(old_vendor).is_none());
    }

    #[test]
    fn add_alias_refuses_a_pattern_matching_another_payees_name() {
        let mut store = PayeeFixture::new();
        let coles_central = find_by_name(&store, "Coles Central").id;

        assert_eq!(
            store.add_alias(coles_central, "Woolworths", AliasMode::ExactText),
            Err(PayeeError::PatternCollision {
                pattern: "(?i)^Woolworths$".to_string(),
                other: "Woolworths".to_string(),
            })
        );
    }

    #[test]
    fn add_alias_exact_text_mode_escapes_and_anchors_without_a_backslash_for_spaces() {
        let mut store = PayeeFixture::new();
        let coles_central = find_by_name(&store, "Coles Central").id;

        let alias_id = store
            .add_alias(coles_central, "WW Metro", AliasMode::ExactText)
            .expect("a fresh, non-colliding pattern should be accepted");

        let alias = store
            .aliases(coles_central)
            .into_iter()
            .find(|alias| alias.id == alias_id)
            .expect("just added");
        assert_eq!(alias.pattern, "(?i)^WW Metro$");
    }

    #[test]
    fn resolve_prefers_an_exact_name_over_a_matching_alias() {
        let store = PayeeFixture::new();
        let woolies = find_by_name(&store, "WOOLIES").id;

        // Woolworths holds a manual alias `(?i)^WOOLIES$`, but the real Payee named "WOOLIES"
        // always wins — step 1 beats step 2, the exact bug this fixture's own conflict pair
        // demonstrates.
        assert_eq!(
            store.resolve("woolies"),
            PayeeResolution::ExactName(woolies)
        );
    }

    #[test]
    fn resolve_falls_back_to_a_rename_alias() {
        let store = PayeeFixture::new();
        let home_loan_direct = find_by_name(&store, "Home Loan Direct").id;

        match store.resolve("Bank Direct Debit") {
            PayeeResolution::Alias { payee_id, .. } => assert_eq!(payee_id, home_loan_direct),
            other => panic!("expected an Alias resolution, got {other:?}"),
        }
    }

    #[test]
    fn resolve_reports_would_create_for_unknown_text() {
        let store = PayeeFixture::new();
        assert_eq!(store.resolve("Some New Shop"), PayeeResolution::WouldCreate);
    }
}
