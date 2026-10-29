//! One Presenter per Units View: the Settings page and each of its Dialogs. Each takes events
//! through `handle` and builds a plain-data view model through `view_model`, its text already
//! localised from [`crate::msg`], so a View only draws. They share one set of ports
//! ([`SharedPorts`](crate::protocols::SharedPorts)) and ask for navigation through the
//! [`UnitsRouter`](crate::protocols::UnitsRouter) the Client passes in.
//!
//! The Presenters hold field values, focus and which fields show, and build their forms from
//! `lib_viper::view`'s field, select and Dialog views.

mod delete_price_source;
mod delete_unit;
mod price_source_form;
mod set_base_unit;
mod set_price;
mod unit_form;
mod units_page;
mod units_pane;

use std::rc::Rc;

use chrono::{DateTime, Utc};
use lib_core::PriceSourceId;
use lib_list_cursor::Step;
use lib_viper::keymap::{BACK, NEXT_FIELD, SELECT_NEXT, SELECT_PREVIOUS};
use lib_viper::view::{Choice, DialogHint};

pub use delete_price_source::{
    DeletePriceSourceEvent, DeletePriceSourcePresenter, DeletePriceSourceViewModel,
};
pub use delete_unit::{
    DeleteUnitEvent, DeleteUnitPresenter, DeleteUnitViewModel, InUseView, NewBaseView,
    active_unit_choices, confirms_delete, delete_refusal,
};
pub use price_source_form::{
    AuthChoice, PriceSourceField, PriceSourceFormEvent, PriceSourceFormPresenter,
    PriceSourceFormViewModel,
};
pub use set_base_unit::{SetBaseUnitEvent, SetBaseUnitPresenter, SetBaseUnitViewModel};
pub use set_price::{SetPriceEvent, SetPriceField, SetPricePresenter, SetPriceViewModel};
pub use unit_form::{UnitField, UnitFormEvent, UnitFormPresenter, UnitFormViewModel};
pub use units_page::{
    ActivityLine, Hint, MenuAction, MenuEntry, RowMenu, SourceColumns, SourceKind, SourceLine,
    UnitColumns, UnitLine, UnitsPageEvent, UnitsPagePresenter, UnitsPageViewModel, UnitsRow,
};
pub use units_pane::{
    AccountLine, AccountsGroup, ActivityGroup, DetailPane, Fact, NOT_FETCHED_GLYPH, PaneButton,
    PaneGroup, SourceGroup, UnitGroup, ValueLine, ValuesGroup,
};

use crate::protocols::UnitsPorts;
pub use crate::protocols::{UnitChange, UnitsOutcome};
use crate::{UnitUse, Updates, msg};

/// A form Dialog's keys besides its submit: next field, choose and cancel.
fn form_hints() -> Vec<DialogHint> {
    vec![next_field_hint(), choose_hint(), cancel_hint()]
}

fn next_field_hint() -> DialogHint {
    DialogHint {
        commands: &[NEXT_FIELD],
        action: lib_locale::msg::dialog_hint_next_field(),
    }
}

/// Steps a select, for a Dialog that has one.
fn choose_hint() -> DialogHint {
    DialogHint {
        commands: &[SELECT_PREVIOUS, SELECT_NEXT],
        action: lib_locale::msg::dialog_hint_choose(),
    }
}

fn cancel_hint() -> DialogHint {
    DialogHint {
        commands: &[BACK],
        action: lib_locale::msg::dialog_hint_cancel(),
    }
}

/// Where a Presenter reads the time from: `Utc::now` in a Client, a fixed instant in a test.
pub type Clock = Rc<dyn Fn() -> DateTime<Utc>>;

impl UnitsOutcome {
    /// The Toast raised while the detail pane isn't docked; a saved Unit or a new base raises none.
    fn toast(&self) -> Option<String> {
        Some(match self {
            Self::Unit { change, code } => match change {
                UnitChange::Saved | UnitChange::BaseSet => return None,
                UnitChange::Deleted => msg::units_toast_deleted(code),
                UnitChange::Deactivated => msg::units_toast_deactivated(code),
                UnitChange::Reactivated => msg::units_toast_reactivated(code),
            },
            Self::PriceSet { code, week } => msg::units_toast_price_set(code, &week_text(*week)),
            Self::SourceSaved { name } => msg::units_toast_source_saved(name),
            Self::SourceDeleted { name, moved } => {
                msg::units_toast_source_deleted(name, count(*moved))
            }
        })
    }

    /// The line ACTIVITY logs while the detail pane is docked.
    fn activity(&self) -> String {
        match self {
            Self::Unit { change, code } => match change {
                UnitChange::Saved => msg::units_activity_saved(code),
                UnitChange::Deleted => msg::units_activity_deleted(code),
                UnitChange::Deactivated => msg::units_activity_deactivated(code),
                UnitChange::Reactivated => msg::units_activity_reactivated(code),
                UnitChange::BaseSet => msg::units_activity_base_set(code),
            },
            Self::PriceSet { code, week } => msg::units_activity_price_set(code, &week_text(*week)),
            Self::SourceSaved { name } => msg::units_activity_source_saved(name),
            Self::SourceDeleted { name, .. } => msg::units_activity_source_deleted(name),
        }
    }
}

/// A Weekly Price's week, as its Monday in the Locale's medium form.
fn week_text(week: chrono::NaiveDate) -> String {
    lib_locale::format::format_date(week, None)
}

/// The codes of the Units source `id` prices, in the Ledger's order.
fn linked_codes(ports: &dyn UnitsPorts, id: PriceSourceId) -> Vec<String> {
    ports
        .units()
        .units()
        .into_iter()
        .filter(|unit| unit.link.source_id == id)
        .map(|unit| unit.code)
        .collect()
}

/// A count as a Message's plural argument.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// What names the Unit, as one phrase: `2 accounts, 1 bill plan, 3 budgets, 1 location`.
pub(super) fn uses_text(uses: UnitUse) -> String {
    let parts: Vec<String> = [
        (
            uses.accounts,
            msg::units_in_use_accounts as fn(i64) -> String,
        ),
        (uses.bill_plans, msg::units_in_use_bill_plans),
        (uses.budgets, msg::units_in_use_budgets),
        (uses.locations, msg::units_in_use_locations),
        (uses.institutions, msg::units_in_use_institutions),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, text)| text(count(n)))
    .collect();
    parts.join(", ")
}

fn updates_label(updates: Updates) -> String {
    match updates {
        Updates::Every15Minutes => msg::units_price_source_updates_every_15_minutes(),
        Updates::Hourly => msg::units_price_source_updates_hourly(),
        Updates::Daily => msg::units_price_source_updates_daily(),
        Updates::ManualOnly => msg::units_price_source_updates_manual_only(),
    }
}

/// The Updates options, in the control's order.
fn updates_choices() -> Vec<Choice<Updates>> {
    [
        Updates::Every15Minutes,
        Updates::Hourly,
        Updates::Daily,
        Updates::ManualOnly,
    ]
    .into_iter()
    .map(|value| Choice {
        value,
        label: updates_label(value),
    })
    .collect()
}

/// How long before `now` something happened, in the largest whole unit: `5 minutes ago`.
fn ago(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let elapsed = (now - at).num_seconds().max(0);
    let whole = |unit: i64| elapsed / unit;
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    match elapsed {
        _ if elapsed < MINUTE => msg::units_ago_just_now(),
        _ if elapsed < HOUR => msg::units_ago_minutes(whole(MINUTE)),
        _ if elapsed < DAY => msg::units_ago_hours(whole(HOUR)),
        _ if elapsed < 7 * DAY => msg::units_ago_days(whole(DAY)),
        _ if elapsed < 30 * DAY => msg::units_ago_weeks(whole(7 * DAY)),
        _ if elapsed < 365 * DAY => msg::units_ago_months(whole(30 * DAY)),
        _ => msg::units_ago_years(whole(365 * DAY)),
    }
}

/// `label`, suffixed ` (2)`, ` (3)`... when an earlier option already shows it, so two Custom
/// sources sharing a name can still be told apart in a select.
fn unique_label(label: &str, seen: &mut Vec<String>) -> String {
    let mut candidate = label.to_string();
    let mut n = 1;
    while seen.contains(&candidate) {
        n += 1;
        candidate = format!("{label} ({n})");
    }
    seen.push(candidate.clone());
    candidate
}

/// The `n`th field of `order` after (or, `backward`, before) `current`, skipping any `shows`
/// refuses, wrapping at either end.
fn cycle<T: Copy + PartialEq>(
    order: &[T],
    current: T,
    backward: bool,
    shows: impl Fn(T) -> bool,
) -> T {
    let count = order.len();
    let mut index = order
        .iter()
        .position(|field| *field == current)
        .unwrap_or(0);
    for _ in 0..count {
        index = if backward {
            (index + count - 1) % count
        } else {
            (index + 1) % count
        };
        if shows(order[index]) {
            return order[index];
        }
    }
    current
}

/// The option `delta` places from `current`, stopping at either end.
fn step<T: Copy + PartialEq>(options: &[T], current: T, delta: isize) -> T {
    let index = options
        .iter()
        .position(|option| *option == current)
        .unwrap_or(0);
    let direction = match delta.signum() {
        1 => Step::Next,
        -1 => Step::Prev,
        _ => return current,
    };
    let next = (0..delta.unsigned_abs()).fold(index, |at, _| {
        lib_list_cursor::step(at, options.len(), direction)
    });
    options.get(next).copied().unwrap_or(current)
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use chrono::{DateTime, TimeZone, Utc};

    use super::Clock;
    use crate::fake::InMemoryPorts;
    use crate::protocols::SharedPorts;

    /// Composes this crate's layer into the process Locale, which every test that reads text
    /// needs: the first explicit `init` wins, so each test calls it before reading any.
    pub fn locale() {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
    }

    /// The instant the tests' clock reads.
    pub fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 21, 9, 30, 0)
            .single()
            .expect("a valid, unambiguous instant")
    }

    pub fn clock() -> Clock {
        Rc::new(now)
    }

    /// `frame`'s hints as `(commands, action)`, the commands joined by `/`, for asserting which
    /// keys a Dialog takes.
    pub fn hints(frame: &lib_viper::view::DialogFrame) -> Vec<(String, String)> {
        frame
            .hints
            .iter()
            .map(|hint| (hint.commands.join("/"), hint.action.clone()))
            .collect()
    }

    /// `(commands, action)` pairs as [`hints`] returns them.
    pub fn pairs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(commands, action)| (commands.to_string(), action.to_string()))
            .collect()
    }

    /// The stub seed, shared, with the handle the test reads it back through.
    pub fn stub() -> (Rc<RefCell<InMemoryPorts>>, SharedPorts) {
        locale();
        InMemoryPorts::stub(now()).shared()
    }

    /// Prices every other stub Unit in itself, so nothing is quoted in aud and the base can be
    /// deleted; the stub's btc and vas are quoted in aud, which a delete refuses.
    pub fn unquote_aud(ports: &mut InMemoryPorts) {
        for unit in &mut ports.units.units {
            if unit.link.quote == crate::UnitId::SEED_AUD {
                unit.link.quote = unit.id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn ago_is_the_largest_whole_unit_since() {
        test_support::locale();
        let now = test_support::now();
        let cases = [
            (Duration::seconds(30), "just now"),
            (Duration::minutes(5), "5 minutes ago"),
            (Duration::minutes(61), "1 hour ago"),
            (Duration::days(2), "2 days ago"),
            (Duration::days(14), "2 weeks ago"),
            (Duration::days(90), "3 months ago"),
            (Duration::days(800), "2 years ago"),
        ];
        for (before, text) in cases {
            assert_eq!(ago(now - before, now), text);
        }
    }

    #[test]
    fn same_labels_are_suffixed_in_order() {
        let mut seen = Vec::new();
        let labels: Vec<_> = ["Mine", "Mine", "Other", "Mine"]
            .iter()
            .map(|label| unique_label(label, &mut seen))
            .collect();
        assert_eq!(labels, ["Mine", "Mine (2)", "Other", "Mine (3)"]);
    }
}
