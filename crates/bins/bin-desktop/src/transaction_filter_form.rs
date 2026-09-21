//! The filter popover's draft state (`docs/ux/desktop/Transactions/README.md`'s 4b): every filter
//! dimension as the user is editing it, kept apart from the applied filters until **apply**
//! commits it. `gpui`-free and unit-tested.
//!
//! The behaviour is the Desktop Transactions map's decisions:
//!
//! - The fields, in `Tab` order: Account and Category (the shared dropdown; Category lists every
//!   category by its path and a parent matches its descendants), Payee and Tag (plain text,
//!   substring match), From and To (text dates) and Status (a segmented control).
//! - **From / To** accept `today` and dates in the chosen Date format (`12 sep 2026`, or day-first
//!   `12/09/2026`), with ISO `2026-09-12` always accepted; an empty side is unbounded. A value that
//!   parses as none of these is an error, and **apply** is refused until it is fixed.
//! - `reset` returns the draft to the defaults (this year, everything else empty); it never touches
//!   the applied filters. `apply` commits the draft; `Esc` discards it.

use chrono::NaiveDate;
use lib_core::DateStyle;
use lib_locale::format::{
    DateInputError, DateInputOptions, date_error_message, format_date_input, parse_date_with,
};

use crate::{
    accounts::Account,
    categories::{self, Category},
    select::SelectState,
    transaction_chips::FilterField,
    transaction_query::{StatusFilter, TransactionFilters},
};

/// The Account select's first option, meaning no account filter.
pub fn all_accounts() -> String {
    crate::msg::desktop_transactions_filter_all_accounts()
}

/// The Category select's first option, meaning no category filter.
pub fn any_category() -> String {
    crate::msg::desktop_transactions_filter_any_category()
}

/// The popover's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormField {
    #[default]
    Account,
    Category,
    Payee,
    Tag,
    From,
    To,
    Status,
}

impl FormField {
    pub const ORDER: [FormField; 7] = [
        Self::Account,
        Self::Category,
        Self::Payee,
        Self::Tag,
        Self::From,
        Self::To,
        Self::Status,
    ];

    pub fn is_select(self) -> bool {
        matches!(self, Self::Account | Self::Category)
    }

    pub fn is_text(self) -> bool {
        matches!(self, Self::Payee | Self::Tag | Self::From | Self::To)
    }

    /// The field a chip opens the popover on: the date chip focuses From.
    pub fn for_chip(chip: FilterField) -> Self {
        match chip {
            FilterField::Account => Self::Account,
            FilterField::Category => Self::Category,
            FilterField::Payee => Self::Payee,
            FilterField::Tag => Self::Tag,
            FilterField::Date => Self::From,
            FilterField::Status => Self::Status,
        }
    }
}

/// The two selects' options: each a label with the id it stands for (`None` for the "all" and
/// "any" entries).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormOptions {
    accounts: Vec<(Option<u32>, String)>,
    categories: Vec<(Option<u32>, String)>,
    account_labels: Vec<String>,
    category_labels: Vec<String>,
}

impl FormOptions {
    pub fn new(accounts: &[Account], all_categories: &[Category]) -> Self {
        let mut account_options = vec![(None, all_accounts())];
        account_options.extend(
            accounts
                .iter()
                .map(|account| (Some(account.id), account.name.clone())),
        );
        let mut category_options = vec![(None, any_category())];
        category_options.extend(
            categories::paths_in_tree_order(all_categories)
                .into_iter()
                .map(|(id, path)| (Some(id), path)),
        );
        let labels = |options: &[(Option<u32>, String)]| {
            options.iter().map(|(_, label)| label.clone()).collect()
        };
        Self {
            account_labels: labels(&account_options),
            category_labels: labels(&category_options),
            accounts: account_options,
            categories: category_options,
        }
    }

    /// The labels behind `field`'s select; empty for any other field.
    pub fn for_field(&self, field: FormField) -> &[String] {
        match field {
            FormField::Account => &self.account_labels,
            FormField::Category => &self.category_labels,
            _ => &[],
        }
    }

    fn account_label(&self, id: Option<u32>) -> String {
        self.accounts
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map_or_else(all_accounts, |(_, label)| label.clone())
    }

    fn category_label(&self, id: Option<u32>) -> String {
        self.categories
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map_or_else(any_category, |(_, label)| label.clone())
    }

    fn account_id(&self, label: &str) -> Option<u32> {
        self.accounts
            .iter()
            .find(|(_, candidate)| candidate == label)
            .and_then(|(id, _)| *id)
    }

    fn category_id(&self, label: &str) -> Option<u32> {
        self.categories
            .iter()
            .find(|(_, candidate)| candidate == label)
            .and_then(|(id, _)| *id)
    }
}

/// A key a focused select understands, parsed from a keystroke by `Shell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectKey {
    Up,
    Down,
    /// `Enter` or `Space`: open a closed list, commit an open one.
    Activate,
}

/// Parses a From / To field: empty is `Ok(None)` (no bound), and otherwise the Locale's short
/// numeric form, ISO, or a word such as `today`, with the year optional (a filter can say `3/9`).
/// An ISO date style makes ISO the only numeric form. The error is the hint shown under the field,
/// a Message built from today's date.
pub fn parse_date(
    text: &str,
    today: NaiveDate,
    date_style: Option<DateStyle>,
) -> Result<Option<NaiveDate>, String> {
    let options = DateInputOptions {
        style: date_style,
        allow_yearless: true,
    };
    match parse_date_with(text, today, &options) {
        Ok(date) => Ok(Some(date)),
        Err(DateInputError::Empty) => Ok(None),
        Err(error) => Err(date_error_message(&error, today, &options)),
    }
}

/// The popover's draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterForm {
    pub account: SelectState,
    pub category: SelectState,
    pub payee: String,
    pub tag: String,
    /// The From text as typed.
    pub from: String,
    /// The To text as typed.
    pub to: String,
    pub status: StatusFilter,
    pub focused: FormField,
}

impl FilterForm {
    /// A draft holding `filters`: the dates in the chosen style in full (`01 jan 2026`), and a
    /// To of today as the word `today` (the mockup's own defaults).
    pub fn from_filters(
        filters: &TransactionFilters,
        options: &FormOptions,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Self {
        Self {
            account: SelectState::new(Some(options.account_label(filters.account))),
            category: SelectState::new(Some(options.category_label(filters.category))),
            payee: filters.payee.clone(),
            tag: filters.tag.clone(),
            from: filters
                .from
                .map(|date| format_date_input(date, date_style))
                .unwrap_or_default(),
            to: match filters.to {
                Some(date) if date == today => lib_locale::msg::date_word_today(),
                Some(date) => format_date_input(date, date_style),
                None => String::new(),
            },
            status: filters.status,
            focused: FormField::default(),
        }
    }

    /// `reset`: the draft back to the defaults (this year, everything else empty). Focus stays put
    /// and the applied filters are untouched.
    pub fn reset(
        &mut self,
        options: &FormOptions,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) {
        let focused = self.focused;
        *self = Self::from_filters(
            &TransactionFilters::defaults(today),
            options,
            today,
            date_style,
        );
        self.focused = focused;
    }

    /// The hint under From when its text is not a date.
    pub fn start_hint(&self, today: NaiveDate, date_style: Option<DateStyle>) -> Option<String> {
        parse_date(&self.from, today, date_style).err()
    }

    /// The hint under To when its text is not a date.
    pub fn end_hint(&self, today: NaiveDate, date_style: Option<DateStyle>) -> Option<String> {
        parse_date(&self.to, today, date_style).err()
    }

    /// Whether **apply** may run: both dates parse.
    pub fn is_valid(&self, today: NaiveDate, date_style: Option<DateStyle>) -> bool {
        self.start_hint(today, date_style).is_none() && self.end_hint(today, date_style).is_none()
    }

    /// The filters this draft describes, or `None` while a date is unparseable.
    pub fn to_filters(
        &self,
        options: &FormOptions,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Option<TransactionFilters> {
        Some(TransactionFilters {
            account: self
                .account
                .value()
                .and_then(|label| options.account_id(label)),
            category: self
                .category
                .value()
                .and_then(|label| options.category_id(label)),
            payee: self.payee.trim().to_string(),
            tag: self.tag.trim().to_string(),
            from: parse_date(&self.from, today, date_style).ok()?,
            to: parse_date(&self.to, today, date_style).ok()?,
            status: self.status,
        })
    }

    fn select_mut(&mut self, field: FormField) -> Option<&mut SelectState> {
        match field {
            FormField::Account => Some(&mut self.account),
            FormField::Category => Some(&mut self.category),
            _ => None,
        }
    }

    pub fn any_select_open(&self) -> bool {
        self.account.is_open() || self.category.is_open()
    }

    /// Closes whichever list is open, discarding its highlight (the first `Esc`). Returns whether
    /// one was open; if not, `Esc` should cancel the popover.
    pub fn close_open_select(&mut self) -> bool {
        let was_open = self.any_select_open();
        self.account.cancel();
        self.category.cancel();
        was_open
    }

    /// Moves focus to `field`, closing a list open on another field.
    pub fn focus(&mut self, field: FormField) {
        if field != self.focused {
            self.close_open_select();
        }
        self.focused = field;
    }

    /// `Tab` / `Shift-Tab`: commits an open list's highlight, then moves on, wrapping.
    pub fn cycle_focus(&mut self, backward: bool, options: &FormOptions) {
        let focused = self.focused;
        if let Some(state) = self.select_mut(focused) {
            state.commit(options.for_field(focused));
        }
        let count = FormField::ORDER.len();
        let index = FormField::ORDER
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        let next = if backward {
            (index + count - 1) % count
        } else {
            (index + 1) % count
        };
        self.focused = FormField::ORDER[next];
    }

    /// A key on the focused select. Returns whether the focused field is a select.
    pub fn handle_select_key(&mut self, key: SelectKey, options: &FormOptions) -> bool {
        let focused = self.focused;
        let list = options.for_field(focused);
        let Some(state) = self.select_mut(focused) else {
            return false;
        };
        match (key, state.is_open()) {
            (SelectKey::Up, true) => state.move_highlight(list, -1),
            (SelectKey::Down, true) => state.move_highlight(list, 1),
            (SelectKey::Up, false) => state.step(list, -1),
            (SelectKey::Down, false) => state.step(list, 1),
            (SelectKey::Activate, true) => state.commit(list),
            (SelectKey::Activate, false) => state.open(list),
        }
        true
    }

    /// A click on a select's closed field: focuses it and toggles its list.
    pub fn click_select(&mut self, field: FormField, options: &FormOptions) {
        if !field.is_select() {
            return;
        }
        self.focus(field);
        let list = options.for_field(field);
        if let Some(state) = self.select_mut(field) {
            if state.is_open() {
                state.cancel();
            } else {
                state.open(list);
            }
        }
    }

    /// A click on row `index` of an open list.
    pub fn choose_option(&mut self, field: FormField, index: usize, options: &FormOptions) {
        let list = options.for_field(field);
        if let Some(state) = self.select_mut(field) {
            state.choose(list, index);
        }
    }

    /// Types `ch` into the focused text field.
    pub fn push_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        match self.focused {
            FormField::Payee => self.payee.push(ch),
            FormField::Tag => self.tag.push(ch),
            FormField::From => self.from.push(ch),
            FormField::To => self.to.push(ch),
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.focused {
            FormField::Payee => {
                self.payee.pop();
            }
            FormField::Tag => {
                self.tag.pop();
            }
            FormField::From => {
                self.from.pop();
            }
            FormField::To => {
                self.to.pop();
            }
            _ => {}
        }
    }

    /// Steps the Status segmented control, wrapping: `Left` / `Right` while it is focused.
    pub fn step_status(&mut self, delta: isize) {
        let all = StatusFilter::ALL;
        let index = all.iter().position(|s| *s == self.status).unwrap_or(0);
        let next = (index as isize + delta).rem_euclid(all.len() as isize) as usize;
        self.status = all[next];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{accounts::default_accounts, categories::default_categories};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn options() -> FormOptions {
        FormOptions::new(&default_accounts(), &default_categories())
    }

    fn fmt() -> Option<DateStyle> {
        None
    }

    /// The typed forms are Locale-dependent, so each test pins one.
    fn au<T>(f: impl FnOnce() -> T) -> T {
        lib_locale::with_locale(lib_locale::Locale::EnAu, f)
    }

    fn defaults_form() -> FilterForm {
        au(|| {
            FilterForm::from_filters(
                &TransactionFilters::defaults(today()),
                &options(),
                today(),
                fmt(),
            )
        })
    }

    #[test]
    fn empty_is_unbounded_and_today_is_today() {
        assert_eq!(parse_date("", today(), fmt()), Ok(None));
        assert_eq!(parse_date("   ", today(), fmt()), Ok(None));
        assert_eq!(parse_date("today", today(), fmt()), Ok(Some(today())));
        assert_eq!(parse_date(" TODAY ", today(), fmt()), Ok(Some(today())));
    }

    #[test]
    fn dates_parse_in_the_locales_short_form() {
        au(|| {
            for text in ["12/9/2026", "12/09/2026", "2026-09-12", "2026-9-12"] {
                assert_eq!(
                    parse_date(text, today(), None),
                    Ok(Some(day(2026, 9, 12))),
                    "{text}"
                );
            }
        });
        lib_locale::with_locale(lib_locale::Locale::EnUs, || {
            assert_eq!(
                parse_date("9/12/2026", today(), None),
                Ok(Some(day(2026, 9, 12)))
            );
        });
    }

    #[test]
    fn a_year_is_optional_in_a_filter() {
        au(|| {
            assert_eq!(parse_date("3/9", today(), None), Ok(Some(day(2026, 9, 3))));
        });
    }

    #[test]
    fn relative_words_are_accepted() {
        au(|| {
            assert_eq!(
                parse_date("yesterday", today(), None),
                Ok(Some(day(2026, 9, 18)))
            );
        });
    }

    #[test]
    fn iso_is_accepted_whatever_the_preference() {
        au(|| {
            for style in crate::settings::DATE_STYLE_CHOICES {
                assert_eq!(
                    parse_date("2026-09-12", today(), style),
                    Ok(Some(day(2026, 9, 12)))
                );
            }
        });
    }

    #[test]
    fn an_iso_style_accepts_only_iso() {
        au(|| {
            assert!(parse_date("12/9/2026", today(), Some(DateStyle::Iso)).is_err());
            assert!(parse_date("2026-09-12", today(), Some(DateStyle::Iso)).is_ok());
        });
    }

    #[test]
    fn impossible_or_malformed_dates_are_errors_with_a_hint() {
        au(|| {
            for bad in ["31/2/2026", "12/9/26", "12 sep 2026", "2026-13-01", "abc"] {
                assert!(
                    parse_date(bad, today(), fmt()).is_err(),
                    "{bad:?} should not parse"
                );
            }
            assert_eq!(
                parse_date("nope", today(), None),
                Err("Enter a date like 19/9/2026 or 2026-09-19, or a word such as today, yesterday, tomorrow.".to_string())
            );
            assert_eq!(
                parse_date("nope", today(), Some(DateStyle::Iso)),
                Err(
                    "Enter a date like 2026-09-19, or a word such as today, yesterday, tomorrow."
                        .to_string()
                )
            );
        });
    }

    #[test]
    fn the_defaults_draft_holds_the_mockups_own_values() {
        au(|| {
            let form = defaults_form();
            assert_eq!(form.from, "1/1/2026");
            assert_eq!(form.to, "today");
            assert_eq!(form.account.value(), Some(all_accounts().as_str()));
            assert_eq!(form.category.value(), Some(any_category().as_str()));
            assert_eq!(form.status, StatusFilter::All);
            assert_eq!(form.focused, FormField::Account);
        });
    }

    #[test]
    fn a_draft_round_trips_through_every_date_style() {
        let mut filters = TransactionFilters::defaults(today());
        filters.account = Some(default_accounts()[1].id);
        filters.category = categories::find_by_name(&default_categories(), "Groceries");
        filters.payee = "wool".to_string();
        filters.tag = "japan".to_string();
        filters.status = StatusFilter::Cleared;
        filters.from = Some(day(2025, 3, 4));
        filters.to = Some(day(2026, 8, 30));
        au(|| {
            for style in crate::settings::DATE_STYLE_CHOICES {
                let form = FilterForm::from_filters(&filters, &options(), today(), style);
                assert_eq!(
                    form.to_filters(&options(), today(), style),
                    Some(filters.clone()),
                    "{style:?}"
                );
            }
        });
    }

    #[test]
    fn defaults_and_empty_dates_round_trip() {
        let defaults = TransactionFilters::defaults(today());
        assert_eq!(
            defaults_form().to_filters(&options(), today(), fmt()),
            Some(defaults)
        );
        let mut unbounded = TransactionFilters::defaults(today());
        unbounded.from = None;
        unbounded.to = None;
        let form = FilterForm::from_filters(&unbounded, &options(), today(), fmt());
        assert_eq!((form.from.as_str(), form.to.as_str()), ("", ""));
        assert_eq!(form.to_filters(&options(), today(), fmt()), Some(unbounded));
    }

    #[test]
    fn payee_and_tag_are_trimmed_and_the_selects_map_back_to_ids() {
        let mut form = defaults_form();
        form.payee = "  wool  ".to_string();
        form.tag = " japan ".to_string();
        form.account = SelectState::new(Some("Amex Platinum".to_string()));
        form.category = SelectState::new(Some("Food \u{203a} Dining".to_string()));
        let filters = form.to_filters(&options(), today(), fmt()).unwrap();
        assert_eq!(filters.payee, "wool");
        assert_eq!(filters.tag, "japan");
        assert_eq!(
            filters.account,
            Some(
                default_accounts()
                    .iter()
                    .find(|a| a.name == "Amex Platinum")
                    .unwrap()
                    .id
            )
        );
        assert_eq!(
            filters.category,
            categories::find_by_name(&default_categories(), "Dining")
        );
    }

    #[test]
    fn an_unparseable_date_blocks_apply_and_names_the_field() {
        let mut form = defaults_form();
        assert!(form.is_valid(today(), fmt()));
        form.from = "31 feb 2026".to_string();
        assert!(!form.is_valid(today(), fmt()));
        assert!(form.start_hint(today(), fmt()).is_some());
        assert!(form.end_hint(today(), fmt()).is_none());
        assert_eq!(form.to_filters(&options(), today(), fmt()), None);
        form.from = String::new();
        form.to = "later".to_string();
        assert!(form.end_hint(today(), fmt()).is_some());
        assert!(form.start_hint(today(), fmt()).is_none());
    }

    #[test]
    fn reset_returns_the_draft_to_the_defaults_and_keeps_focus() {
        let mut applied = TransactionFilters::defaults(today());
        applied.payee = "wool".to_string();
        applied.status = StatusFilter::Open;
        let mut form = FilterForm::from_filters(&applied, &options(), today(), fmt());
        form.focused = FormField::Tag;
        form.tag = "typed".to_string();
        form.reset(&options(), today(), fmt());
        assert_eq!(form, {
            let mut expected = defaults_form();
            expected.focused = FormField::Tag;
            expected
        });
        assert_eq!(applied.payee, "wool", "the applied filters are separate");
    }

    #[test]
    fn a_chip_opens_the_popover_on_its_own_field() {
        assert_eq!(
            FormField::for_chip(FilterField::Account),
            FormField::Account
        );
        assert_eq!(
            FormField::for_chip(FilterField::Category),
            FormField::Category
        );
        assert_eq!(FormField::for_chip(FilterField::Payee), FormField::Payee);
        assert_eq!(FormField::for_chip(FilterField::Tag), FormField::Tag);
        assert_eq!(FormField::for_chip(FilterField::Date), FormField::From);
        assert_eq!(FormField::for_chip(FilterField::Status), FormField::Status);
    }

    #[test]
    fn tab_walks_the_seven_fields_and_wraps_both_ways() {
        let options = options();
        let mut form = defaults_form();
        let mut seen = vec![form.focused];
        for _ in 0..7 {
            form.cycle_focus(false, &options);
            seen.push(form.focused);
        }
        assert_eq!(seen[..7], FormField::ORDER);
        assert_eq!(seen[7], FormField::Account, "wraps");
        form.cycle_focus(true, &options);
        assert_eq!(
            form.focused,
            FormField::Status,
            "backwards wraps to the last"
        );
    }

    #[test]
    fn tab_commits_an_open_lists_highlight_before_moving_on() {
        let options = options();
        let mut form = defaults_form();
        form.handle_select_key(SelectKey::Activate, &options);
        form.handle_select_key(SelectKey::Down, &options);
        assert!(form.account.is_open());
        form.cycle_focus(false, &options);
        assert!(!form.account.is_open());
        assert_eq!(
            form.account.value(),
            Some("Wallet"),
            "the highlight was committed"
        );
        assert_eq!(form.focused, FormField::Category);
    }

    #[test]
    fn selects_step_open_navigate_and_the_first_escape_closes_only_the_list() {
        let options = options();
        let mut form = defaults_form();
        form.focus(FormField::Category);
        assert!(form.handle_select_key(SelectKey::Down, &options));
        assert_eq!(form.category.value(), Some("Housing"));
        form.handle_select_key(SelectKey::Activate, &options);
        assert!(form.category.is_open());
        assert!(form.close_open_select());
        assert!(
            !form.close_open_select(),
            "the next Esc cancels the popover instead"
        );
        assert_eq!(form.category.value(), Some("Housing"));
    }

    #[test]
    fn a_select_key_on_a_text_field_is_not_taken() {
        let options = options();
        let mut form = defaults_form();
        form.focus(FormField::Payee);
        assert!(!form.handle_select_key(SelectKey::Down, &options));
    }

    #[test]
    fn typing_goes_to_the_focused_text_field_only() {
        let mut form = defaults_form();
        form.focus(FormField::Payee);
        form.push_char('w');
        form.focus(FormField::Tag);
        form.push_char('j');
        form.push_char('\n');
        form.focus(FormField::Account);
        form.push_char('x');
        assert_eq!((form.payee.as_str(), form.tag.as_str()), ("w", "j"));
        assert_eq!(form.account.value(), Some(all_accounts().as_str()));
        form.focus(FormField::From);
        form.backspace();
        assert_eq!(form.from, "1/1/202");
    }

    #[test]
    fn the_status_control_steps_and_wraps() {
        let mut form = defaults_form();
        form.step_status(1);
        assert_eq!(form.status, StatusFilter::Open);
        form.step_status(-1);
        form.step_status(-1);
        assert_eq!(form.status, StatusFilter::Reconciled, "wraps back past all");
        form.step_status(1);
        assert_eq!(form.status, StatusFilter::All);
    }

    #[test]
    fn the_category_options_are_paths_in_tree_order_led_by_any() {
        let options = options();
        let labels = options.for_field(FormField::Category);
        assert_eq!(labels[0], any_category());
        assert_eq!(labels.len(), 13);
        assert_eq!(labels[1], "Housing");
        assert!(labels.contains(&"Housing \u{203a} Utilities \u{203a} Electricity".to_string()));
        assert_eq!(options.for_field(FormField::Account)[0], all_accounts());
        assert_eq!(options.for_field(FormField::Account).len(), 8);
        assert!(options.for_field(FormField::Payee).is_empty());
    }
}
