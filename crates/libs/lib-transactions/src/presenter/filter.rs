//! The filter popover (`docs/ux/desktop-mockups/03-transactions/README.md`'s 4b): a draft of every
//! filter dimension, kept apart from the applied filters until **apply** hands them back.
//!
//! - The fields, in `Tab` order: Account and Category (selects; Category lists every Category by
//!   its path and a parent matches its descendants), Payee and Tag (plain text, substring match),
//!   From and To (typed dates) and Status (a segmented control).
//! - **From / To** accept `today` and dates in the chosen Date format, with ISO always accepted and
//!   the year optional; an empty side is unbounded. A value that parses as none of these shows a
//!   hint under its field, and **apply** is refused until it is fixed.
//! - A Payee or Tag hand-off (the Payees and Tags pages filter by id) pre-fills its text field with
//!   the name and keeps the id while the field still reads exactly that name; editing the text
//!   turns it back into a plain substring filter.
//! - `reset` returns the draft to the defaults (this year, everything else empty); it never touches
//!   the applied filters. **apply** hands the draft back as the filters; `cancel` discards it.
//!
//! The choices are read when the popover opens, as it is modal and nothing changes them under it.
//! A select's open list and highlight are the View's widget state: the Presenter holds the chosen
//! option, and steps it while the list is closed.

use chrono::NaiveDate;
use lib_core::{AccountId, CategoryId, DateStyle, PayeeId, TagId};
use lib_locale::format::{format_date_input, parse_optional_date};
use lib_viper::view::{Choice, DialogFrame, SelectView, TextFieldView};

use crate::msg;
use crate::query::{StatusFilter, TransactionFilters};
use crate::{TransactionAccounts, TransactionCategories, TransactionPayees, TransactionTags};

/// The popover's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterField {
    #[default]
    Account,
    Category,
    Payee,
    Tag,
    From,
    To,
    Status,
}

impl FilterField {
    const ORDER: [Self; 7] = [
        Self::Account,
        Self::Category,
        Self::Payee,
        Self::Tag,
        Self::From,
        Self::To,
        Self::Status,
    ];

    /// Whether the field is a select, whose open list the View keeps.
    pub fn is_select(self) -> bool {
        matches!(self, Self::Account | Self::Category)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterEvent {
    /// `Tab`/`Shift-Tab`, wrapping.
    NextField,
    PreviousField,
    /// A click on a field.
    Focus(FilterField),
    /// `Down`/`Up` on a closed select: the next or previous option, wrapping. Nothing elsewhere.
    SelectNext,
    SelectPrevious,
    /// An Account or Category picked from its open list.
    Account(Option<AccountId>),
    Category(Option<CategoryId>),
    /// A click on a Status segment, which focuses the control.
    Status(StatusFilter),
    /// `Right`/`Space` and `Left` on the Status control: the next or previous Status, wrapping.
    StatusNext,
    StatusPrevious,
    /// A character typed into the focused text field, and `Backspace` on it.
    Type(char),
    Backspace,
    Reset,
    Apply,
    Cancel,
}

/// What an event closed the popover with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOutcome {
    /// **apply**: the draft as the filters.
    Applied(TransactionFilters),
    /// The draft discarded.
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterViewModel {
    /// The title, **apply** as the submit and whether it may run (both dates parse).
    pub frame: DialogFrame,
    pub account: SelectView<Option<AccountId>>,
    pub category: SelectView<Option<CategoryId>>,
    pub payee: TextFieldView,
    pub tag: TextFieldView,
    pub from: TextFieldView,
    /// The hint under From while its text is not a date.
    pub from_error: Option<String>,
    pub to: TextFieldView,
    pub to_error: Option<String>,
    pub status: SelectView<StatusFilter>,
    /// The **reset** button.
    pub reset: String,
    pub focused: FilterField,
}

/// What the selects choose from, and the names a Payee or Tag hand-off pre-fills.
struct Choices {
    accounts: Vec<Choice<Option<AccountId>>>,
    categories: Vec<Choice<Option<CategoryId>>>,
    payees: Vec<(PayeeId, String)>,
    tags: Vec<(TagId, String)>,
}

pub struct FilterPresenter {
    today: NaiveDate,
    date_style: Option<DateStyle>,
    choices: Choices,
    account: Option<AccountId>,
    /// A Budget's on-budget Accounts from the Budgets detail's hand-off, which no field shows: kept
    /// through the draft until the Account select is changed or the draft reset.
    account_ids: Vec<AccountId>,
    category: Option<CategoryId>,
    payee: String,
    /// The Payees page's hand-off id, kept while `payee` reads its name.
    payee_id: Option<PayeeId>,
    tag: String,
    /// The Tags page's hand-off id, kept while `tag` reads its name.
    tag_id: Option<TagId>,
    /// The From and To text as typed.
    from: String,
    to: String,
    status: StatusFilter,
    focused: FilterField,
}

impl FilterPresenter {
    /// The popover on a draft of `applied`, its choices read from `reads`: the dates in the chosen
    /// style, and a To of today as the word `today`. A chip focuses its own field (the date chip
    /// focuses From); `None` (the `f` key) the first.
    pub fn open<R>(
        reads: &R,
        applied: &TransactionFilters,
        chip: Option<ChipField>,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Self
    where
        R: ?Sized
            + TransactionAccounts
            + TransactionCategories
            + TransactionPayees
            + TransactionTags,
    {
        let choices = Choices {
            accounts: std::iter::once(Choice {
                value: None,
                label: msg::transactions_filter_all_accounts(),
            })
            .chain(reads.transaction_accounts().into_iter().map(|a| Choice {
                value: Some(a.id),
                label: a.name,
            }))
            .collect(),
            categories: std::iter::once(Choice {
                value: None,
                label: msg::transactions_filter_any_category(),
            })
            .chain(reads.transaction_categories().into_iter().map(|c| Choice {
                value: Some(c.id),
                label: c.path,
            }))
            .collect(),
            payees: reads
                .transaction_payees()
                .into_iter()
                .map(|p| (p.id, p.name))
                .collect(),
            tags: reads
                .transaction_tags()
                .into_iter()
                .map(|t| (t.id, t.name))
                .collect(),
        };
        let mut presenter = Self {
            today,
            date_style,
            choices,
            account: None,
            account_ids: Vec::new(),
            category: None,
            payee: String::new(),
            payee_id: None,
            tag: String::new(),
            tag_id: None,
            from: String::new(),
            to: String::new(),
            status: StatusFilter::All,
            focused: chip.map(ChipField::field).unwrap_or_default(),
        };
        presenter.fill(applied);
        presenter
    }

    /// The draft holding `filters`; focus stays put.
    fn fill(&mut self, filters: &TransactionFilters) {
        let (payee, payee_id) = prefill(
            &filters.payee,
            named(&self.choices.payees, filters.payee_id),
        );
        let (tag, tag_id) = prefill(&filters.tag, named(&self.choices.tags, filters.tag_id));
        self.account = filters.account;
        self.account_ids.clone_from(&filters.account_ids);
        self.category = filters.category;
        self.payee = payee;
        self.payee_id = payee_id;
        self.tag = tag;
        self.tag_id = tag_id;
        self.from = filters
            .from
            .map(|date| format_date_input(date, self.date_style))
            .unwrap_or_default();
        self.to = self.to_text(filters.to);
        self.status = filters.status;
    }

    /// To as the draft shows it: today as the word `today`.
    fn to_text(&self, to: Option<NaiveDate>) -> String {
        match to {
            Some(date) if date == self.today => lib_locale::msg::date_word_today(),
            Some(date) => format_date_input(date, self.date_style),
            None => String::new(),
        }
    }

    /// Follows a change to the Display Preference while the popover is open: From and To are read
    /// in the old style and shown in the new, so a day-first date isn't read month-first. Text
    /// that isn't a date yet is left as typed.
    pub fn set_date_style(&mut self, date_style: Option<DateStyle>) {
        let from = self.parse(&self.from);
        let to = self.parse(&self.to);
        self.date_style = date_style;
        if let Ok(from) = from {
            self.from = from
                .map(|date| format_date_input(date, date_style))
                .unwrap_or_default();
        }
        if let Ok(to) = to {
            self.to = self.to_text(to);
        }
    }

    pub fn focused(&self) -> FilterField {
        self.focused
    }

    pub fn handle(&mut self, event: FilterEvent) -> Option<FilterOutcome> {
        match event {
            FilterEvent::NextField | FilterEvent::PreviousField => {
                let order = FilterField::ORDER;
                let index = order.iter().position(|f| *f == self.focused).unwrap_or(0);
                let next = if event == FilterEvent::PreviousField {
                    index + order.len() - 1
                } else {
                    index + 1
                };
                self.focused = order[next % order.len()];
            }
            FilterEvent::Focus(field) => self.focused = field,
            FilterEvent::SelectNext => self.step_select(1),
            FilterEvent::SelectPrevious => self.step_select(-1),
            FilterEvent::Account(id) => {
                if self.choices.accounts.iter().any(|c| c.value == id) {
                    self.account = id;
                    self.account_ids.clear();
                }
            }
            FilterEvent::Category(id) => {
                if self.choices.categories.iter().any(|c| c.value == id) {
                    self.category = id;
                }
            }
            FilterEvent::Status(status) => {
                self.focused = FilterField::Status;
                self.status = status;
            }
            FilterEvent::StatusNext => self.status = stepped(&StatusFilter::ALL, self.status, 1),
            FilterEvent::StatusPrevious => {
                self.status = stepped(&StatusFilter::ALL, self.status, -1);
            }
            FilterEvent::Type(ch) => {
                if let Some(text) = self.text_mut()
                    && !ch.is_control()
                {
                    text.push(ch);
                }
            }
            FilterEvent::Backspace => {
                if let Some(text) = self.text_mut() {
                    text.pop();
                }
            }
            FilterEvent::Reset => self.fill(&TransactionFilters::defaults(self.today)),
            FilterEvent::Apply => return self.filters().map(FilterOutcome::Applied),
            FilterEvent::Cancel => return Some(FilterOutcome::Cancelled),
        }
        None
    }

    /// Steps the focused select's choice by `delta`, wrapping; nothing on any other field.
    fn step_select(&mut self, delta: isize) {
        fn values<T: Copy>(choices: &[Choice<T>]) -> Vec<T> {
            choices.iter().map(|choice| choice.value).collect()
        }
        match self.focused {
            FilterField::Account => {
                self.account = stepped(&values(&self.choices.accounts), self.account, delta);
                self.account_ids.clear();
            }
            FilterField::Category => {
                self.category = stepped(&values(&self.choices.categories), self.category, delta);
            }
            _ => {}
        }
    }

    /// The focused field's text, when it is a text field.
    fn text_mut(&mut self) -> Option<&mut String> {
        match self.focused {
            FilterField::Payee => Some(&mut self.payee),
            FilterField::Tag => Some(&mut self.tag),
            FilterField::From => Some(&mut self.from),
            FilterField::To => Some(&mut self.to),
            _ => None,
        }
    }

    fn parse(&self, text: &str) -> Result<Option<NaiveDate>, String> {
        parse_optional_date(text, self.today, self.date_style)
    }

    /// The filters the draft describes, or `None` while a date doesn't parse.
    fn filters(&self) -> Option<TransactionFilters> {
        let (payee, payee_id) = resolve(&self.payee, named(&self.choices.payees, self.payee_id));
        let (tag, tag_id) = resolve(&self.tag, named(&self.choices.tags, self.tag_id));
        Some(TransactionFilters {
            account: self.account,
            account_ids: self.account_ids.clone(),
            category: self.category,
            payee,
            payee_id,
            tag,
            tag_id,
            from: self.parse(&self.from).ok()?,
            to: self.parse(&self.to).ok()?,
            status: self.status,
        })
    }

    pub fn view_model(&self) -> FilterViewModel {
        let text = |label: String, text: &str, placeholder: String, field| TextFieldView {
            label,
            text: text.to_string(),
            placeholder,
            focused: self.focused == field,
        };
        let from_error = self.parse(&self.from).err();
        let to_error = self.parse(&self.to).err();
        FilterViewModel {
            frame: DialogFrame {
                title: msg::transactions_filter_title(),
                submit: msg::transactions_filter_apply(),
                can_confirm: from_error.is_none() && to_error.is_none(),
                destructive: false,
                hints: Vec::new(),
            },
            account: SelectView {
                label: lib_locale::msg::column_account(),
                options: self.choices.accounts.clone(),
                chosen: Some(self.account),
                focused: self.focused == FilterField::Account,
            },
            category: SelectView {
                label: lib_locale::msg::column_category(),
                options: self.choices.categories.clone(),
                chosen: Some(self.category),
                focused: self.focused == FilterField::Category,
            },
            payee: text(
                lib_locale::msg::column_payee(),
                &self.payee,
                msg::transactions_filter_any_payee(),
                FilterField::Payee,
            ),
            tag: text(
                msg::transactions_filter_field_tag(),
                &self.tag,
                msg::transactions_filter_any_tag(),
                FilterField::Tag,
            ),
            from: text(
                lib_locale::msg::column_from(),
                &self.from,
                msg::transactions_filter_no_start(),
                FilterField::From,
            ),
            from_error,
            to: text(
                msg::transactions_filter_field_to(),
                &self.to,
                msg::transactions_filter_no_end(),
                FilterField::To,
            ),
            to_error,
            status: SelectView {
                label: lib_locale::msg::column_status(),
                options: StatusFilter::ALL
                    .into_iter()
                    .map(|value| Choice {
                        value,
                        label: status_filter_label(value),
                    })
                    .collect(),
                chosen: Some(self.status),
                focused: self.focused == FilterField::Status,
            },
            reset: msg::transactions_filter_reset(),
            focused: self.focused,
        }
    }
}

/// The chip row and its clears, which act on the applied filters rather than a draft. A chip at
/// its default is an outline chip (`account: all accounts`); one changed from the default is an
/// accent chip whose `✕` resets just that filter. The default this-year range reads `this year`, a
/// custom one reads as itself (`12 Aug 2026 – today`), and no bounds reads `all dates`.
impl FilterPresenter {
    /// The six chips for `filters`, in order, naming the rows they filter by from `reads`.
    pub fn chips<R>(
        reads: &R,
        filters: &TransactionFilters,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Vec<ChipView>
    where
        R: ?Sized
            + TransactionAccounts
            + TransactionCategories
            + TransactionPayees
            + TransactionTags,
    {
        let defaults = TransactionFilters::defaults(today);
        // A Budget's Account set, which the single-Account filter can't say, reads as its size.
        let account = match filters.account {
            Some(id) => Some(
                reads
                    .transaction_accounts()
                    .into_iter()
                    .find(|account| account.id == id)
                    .map_or_else(msg::transactions_chip_unknown, |account| account.name),
            ),
            None if !filters.account_ids.is_empty() => Some(msg::transactions_chip_some_accounts(
                i64::try_from(filters.account_ids.len()).unwrap_or(i64::MAX),
            )),
            None => None,
        };
        let category = filters.category.map(|id| {
            reads
                .transaction_categories()
                .into_iter()
                .find(|category| category.id == id)
                .map_or_else(msg::transactions_chip_unknown, |category| category.path)
        });
        // An id-only filter (the Payees and Tags pages' hand-off) names its Payee or Tag.
        let payee = match filters.payee_id {
            Some(id) if filters.payee.trim().is_empty() => reads
                .transaction_payees()
                .into_iter()
                .find(|payee| payee.id == id)
                .map_or_else(msg::transactions_chip_unknown, |payee| payee.name),
            _ => filters.payee.trim().to_string(),
        };
        let tag = match filters.tag_id {
            Some(id) if filters.tag.trim().is_empty() => reads
                .transaction_tags()
                .into_iter()
                .find(|tag| tag.id == id)
                .map_or_else(msg::transactions_chip_unknown, |tag| tag.name),
            _ => filters.tag.trim().to_string(),
        };
        let or_any = |text: &str| {
            if text.is_empty() {
                msg::transactions_chip_any()
            } else {
                text.to_string()
            }
        };
        let chip = |field, label, active| ChipView {
            field,
            label,
            active,
        };
        vec![
            chip(
                ChipField::Account,
                msg::transactions_chip_account(
                    &account
                        .clone()
                        .unwrap_or_else(msg::transactions_chip_all_accounts),
                ),
                account.is_some(),
            ),
            chip(
                ChipField::Category,
                msg::transactions_chip_category(
                    &category.clone().unwrap_or_else(msg::transactions_chip_any),
                ),
                category.is_some(),
            ),
            chip(
                ChipField::Payee,
                msg::transactions_chip_payee(&or_any(&payee)),
                !payee.is_empty(),
            ),
            chip(
                ChipField::Tag,
                msg::transactions_chip_tag(&or_any(&tag)),
                !tag.is_empty(),
            ),
            chip(
                ChipField::Date,
                date_label(filters.from, filters.to, today, date_style),
                filters.from != defaults.from || filters.to != defaults.to,
            ),
            chip(
                ChipField::Status,
                msg::transactions_chip_status(&status_filter_label(filters.status)),
                filters.status != defaults.status,
            ),
        ]
    }

    /// The `✕` on an accent chip: resets just that filter. The date range returns to this year,
    /// the default, not to `all dates`.
    pub fn clear_chip(filters: &mut TransactionFilters, chip: ChipField, today: NaiveDate) {
        let defaults = TransactionFilters::defaults(today);
        match chip {
            ChipField::Account => {
                filters.account = defaults.account;
                filters.account_ids = defaults.account_ids;
            }
            ChipField::Category => filters.category = defaults.category,
            ChipField::Payee => {
                filters.payee = defaults.payee;
                filters.payee_id = defaults.payee_id;
            }
            ChipField::Tag => {
                filters.tag = defaults.tag;
                filters.tag_id = defaults.tag_id;
            }
            ChipField::Date => {
                filters.from = defaults.from;
                filters.to = defaults.to;
            }
            ChipField::Status => filters.status = defaults.status,
        }
    }

    /// `clear filters`: every filter back to the defaults (this year, nothing else).
    pub fn clear_all(filters: &mut TransactionFilters, today: NaiveDate) {
        *filters = TransactionFilters::defaults(today);
    }
}

/// The date chip's label for the range `from` to `to`; a range ending today reads `… – today`.
fn date_label(
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    today: NaiveDate,
    date_style: Option<DateStyle>,
) -> String {
    let (default_from, default_to) = crate::query::this_year(today);
    if from == Some(default_from) && to == Some(default_to) {
        return msg::transactions_date_this_year();
    }
    let show = |date: NaiveDate| lib_locale::format::format_date(date, date_style);
    let show_end = |date: NaiveDate| {
        if date == today {
            lib_locale::msg::date_word_today()
        } else {
            show(date)
        }
    };
    match (from, to) {
        (None, None) => msg::transactions_date_all(),
        (Some(from), None) => msg::transactions_date_from(&show(from)),
        (None, Some(to)) => msg::transactions_date_until(&show_end(to)),
        (Some(from), Some(to)) => msg::transactions_date_range(&show(from), &show_end(to)),
    }
}

/// The value `delta` places from `current` in `values`, wrapping; the first when `current` is
/// missing from them.
fn stepped<T: Copy + PartialEq>(values: &[T], current: T, delta: isize) -> T {
    let Some(index) = values.iter().position(|value| *value == current) else {
        return values.first().copied().unwrap_or(current);
    };
    let len = values.len() as isize;
    values[(index as isize + delta).rem_euclid(len) as usize]
}

fn named<T: Copy + PartialEq>(names: &[(T, String)], id: Option<T>) -> Option<(T, &str)> {
    let id = id?;
    names
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .map(|(id, name)| (*id, name.as_str()))
}

/// A text field's draft for an id hand-off: `(id, name)` pre-fills the empty field with the name
/// and keeps the id; otherwise the text as applied.
fn prefill<T: Copy>(text: &str, handed_off: Option<(T, &str)>) -> (String, Option<T>) {
    match handed_off {
        Some((id, name)) if text.trim().is_empty() => (name.to_string(), Some(id)),
        _ => (text.to_string(), None),
    }
}

/// The applied `(text, id)` for a text field: the id alone while the text still reads the
/// hand-off's name, else the trimmed text as a substring filter.
fn resolve<T: Copy>(text: &str, handed_off: Option<(T, &str)>) -> (String, Option<T>) {
    let text = text.trim();
    match handed_off {
        Some((id, name)) if name == text => (String::new(), Some(id)),
        _ => (text.to_string(), None),
    }
}

/// A filter dimension as its chip names it, in the order the chips appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChipField {
    Account,
    Category,
    Payee,
    Tag,
    Date,
    Status,
}

impl ChipField {
    pub const ALL: [Self; 6] = [
        Self::Account,
        Self::Category,
        Self::Payee,
        Self::Tag,
        Self::Date,
        Self::Status,
    ];

    /// The popover field the chip opens it on: the date chip focuses From.
    fn field(self) -> FilterField {
        match self {
            Self::Account => FilterField::Account,
            Self::Category => FilterField::Category,
            Self::Payee => FilterField::Payee,
            Self::Tag => FilterField::Tag,
            Self::Date => FilterField::From,
            Self::Status => FilterField::Status,
        }
    }
}

/// One chip in the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipView {
    pub field: ChipField,
    /// The text without its trailing `▾` or `✕`, which the View adds.
    pub label: String,
    /// Changed from the default: drawn as an accent chip with a `✕`.
    pub active: bool,
}

/// The word for a Status filter: the shared Status label, or "All".
pub fn status_filter_label(filter: StatusFilter) -> String {
    use lib_core::TransactionStatus;
    use lib_locale::Label;
    match filter {
        StatusFilter::All => msg::transactions_status_all(),
        StatusFilter::Open => TransactionStatus::Open.label(),
        StatusFilter::Cleared => TransactionStatus::Cleared.label(),
        StatusFilter::Reconciled => TransactionStatus::Reconciled.label(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presenter::test_support::*;

    fn open(applied: &TransactionFilters) -> FilterPresenter {
        FilterPresenter::open(&reads(), applied, None, today(), None)
    }

    fn type_text(presenter: &mut FilterPresenter, text: &str) {
        for ch in text.chars() {
            presenter.handle(FilterEvent::Type(ch));
        }
    }

    #[test]
    fn the_draft_reaches_the_filters_only_on_apply() {
        let applied = TransactionFilters::defaults(today());
        let mut presenter = au(|| open(&applied));

        presenter.handle(FilterEvent::Focus(FilterField::Payee));
        type_text(&mut presenter, "wool");

        assert_eq!(presenter.view_model().payee.text, "wool");
        let outcome = au(|| presenter.handle(FilterEvent::Apply));
        assert_eq!(
            outcome,
            Some(FilterOutcome::Applied(TransactionFilters {
                payee: "wool".to_string(),
                ..TransactionFilters::defaults(today())
            }))
        );
    }

    #[test]
    fn cancel_discards_the_draft_and_the_applied_filters_reopen_unchanged() {
        let applied = TransactionFilters::defaults(today());
        let mut presenter = au(|| open(&applied));
        presenter.handle(FilterEvent::Focus(FilterField::Tag));
        type_text(&mut presenter, "japan");

        assert_eq!(
            presenter.handle(FilterEvent::Cancel),
            Some(FilterOutcome::Cancelled)
        );
        assert_eq!(au(|| open(&applied)).view_model().tag.text, "");
    }

    #[test]
    fn reset_returns_the_draft_to_the_defaults_and_keeps_focus() {
        let applied = TransactionFilters {
            account: Some(WALLET),
            payee: "wool".to_string(),
            status: StatusFilter::Open,
            from: None,
            ..TransactionFilters::defaults(today())
        };
        let mut presenter = au(|| open(&applied));
        presenter.handle(FilterEvent::Focus(FilterField::Tag));

        au(|| presenter.handle(FilterEvent::Reset));

        let view = au(|| presenter.view_model());
        assert_eq!(view.account.chosen, Some(None));
        assert_eq!(view.payee.text, "");
        assert_eq!(view.status.chosen, Some(StatusFilter::All));
        assert_eq!(
            (view.from.text.as_str(), view.to.text.as_str()),
            ("1/1/2026", "today")
        );
        assert_eq!(view.focused, FilterField::Tag);
        assert_eq!(
            au(|| presenter.handle(FilterEvent::Apply)),
            Some(FilterOutcome::Applied(
                TransactionFilters::defaults(today())
            ))
        );
    }

    #[test]
    fn tab_walks_the_seven_fields_and_wraps_both_ways() {
        let mut presenter = au(|| open(&TransactionFilters::defaults(today())));
        let mut seen = vec![presenter.focused()];
        for _ in 0..7 {
            presenter.handle(FilterEvent::NextField);
            seen.push(presenter.focused());
        }
        assert_eq!(seen[..7], FilterField::ORDER);
        assert_eq!(seen[7], FilterField::Account, "wraps");
        presenter.handle(FilterEvent::PreviousField);
        assert_eq!(presenter.focused(), FilterField::Status, "wraps backwards");
    }

    #[test]
    fn a_closed_select_steps_its_choice_and_wraps() {
        let mut presenter = au(|| open(&TransactionFilters::defaults(today())));
        presenter.handle(FilterEvent::Focus(FilterField::Category));

        presenter.handle(FilterEvent::SelectNext);
        assert_eq!(presenter.view_model().category.chosen, Some(Some(FOOD)));
        presenter.handle(FilterEvent::SelectPrevious);
        presenter.handle(FilterEvent::SelectPrevious);
        assert_eq!(presenter.view_model().category.chosen, Some(Some(HOUSING)));

        presenter.handle(FilterEvent::Focus(FilterField::Payee));
        presenter.handle(FilterEvent::SelectNext);
        assert_eq!(presenter.view_model().category.chosen, Some(Some(HOUSING)));
        assert_eq!(presenter.view_model().payee.text, "", "not a select");
    }

    #[test]
    fn a_chosen_option_sets_its_select() {
        let mut presenter = au(|| open(&TransactionFilters::defaults(today())));
        presenter.handle(FilterEvent::Account(Some(WALLET)));
        presenter.handle(FilterEvent::Category(Some(GROCERIES)));
        let Some(FilterOutcome::Applied(filters)) = au(|| presenter.handle(FilterEvent::Apply))
        else {
            panic!("the draft applies");
        };
        assert_eq!(
            (filters.account, filters.category),
            (Some(WALLET), Some(GROCERIES))
        );
    }

    #[test]
    fn the_status_control_steps_wraps_and_takes_a_click() {
        let mut presenter = au(|| open(&TransactionFilters::defaults(today())));
        presenter.handle(FilterEvent::StatusNext);
        assert_eq!(
            presenter.view_model().status.chosen,
            Some(StatusFilter::Open)
        );
        presenter.handle(FilterEvent::StatusPrevious);
        presenter.handle(FilterEvent::StatusPrevious);
        assert_eq!(
            presenter.view_model().status.chosen,
            Some(StatusFilter::Reconciled)
        );

        presenter.handle(FilterEvent::Status(StatusFilter::Cleared));
        let view = presenter.view_model();
        assert_eq!(view.status.chosen, Some(StatusFilter::Cleared));
        assert_eq!(view.focused, FilterField::Status, "a click focuses it");
    }

    #[test]
    fn typing_and_backspace_reach_the_focused_text_field_only() {
        let mut presenter = au(|| open(&TransactionFilters::defaults(today())));
        type_text(&mut presenter, "x");
        presenter.handle(FilterEvent::Focus(FilterField::Payee));
        type_text(&mut presenter, "w");
        presenter.handle(FilterEvent::Type('\n'));
        presenter.handle(FilterEvent::Focus(FilterField::From));
        presenter.handle(FilterEvent::Backspace);
        let view = presenter.view_model();
        assert_eq!(view.payee.text, "w");
        assert_eq!(view.from.text, "1/1/202");
        assert_eq!(view.account.chosen, Some(None));
    }

    /// Every Date format Preference, the Locale's default (`None`) first.
    const DATE_STYLES: [Option<DateStyle>; 5] = [
        None,
        Some(DateStyle::Short),
        Some(DateStyle::Medium),
        Some(DateStyle::Long),
        Some(DateStyle::Iso),
    ];

    fn type_into(presenter: &mut FilterPresenter, field: FilterField, text: &str) {
        presenter.handle(FilterEvent::Focus(field));
        for _ in 0..40 {
            presenter.handle(FilterEvent::Backspace);
        }
        type_text(presenter, text);
    }

    fn applied(presenter: &mut FilterPresenter) -> TransactionFilters {
        match presenter.handle(FilterEvent::Apply) {
            Some(FilterOutcome::Applied(filters)) => filters,
            other => panic!("expected the draft to apply, got {other:?}"),
        }
    }

    #[test]
    fn a_typed_date_reads_in_each_date_style() {
        let typed = [
            (None, "4/3/2025"),
            (Some(DateStyle::Short), "4/3/2025"),
            (Some(DateStyle::Medium), "4/3/2025"),
            (Some(DateStyle::Long), "4/3/2025"),
            (Some(DateStyle::Iso), "2025-03-04"),
        ];
        au(|| {
            for (style, text) in typed {
                let mut presenter = FilterPresenter::open(
                    &reads(),
                    &TransactionFilters::defaults(today()),
                    None,
                    today(),
                    style,
                );
                type_into(&mut presenter, FilterField::From, text);
                assert_eq!(
                    applied(&mut presenter).from,
                    Some(date(2025, 3, 4)),
                    "{style:?}"
                );
            }
        });
    }

    #[test]
    fn an_iso_style_refuses_a_day_first_date() {
        au(|| {
            let mut presenter = FilterPresenter::open(
                &reads(),
                &TransactionFilters::defaults(today()),
                None,
                today(),
                Some(DateStyle::Iso),
            );
            type_into(&mut presenter, FilterField::To, "4/3/2025");
            assert!(presenter.view_model().to_error.is_some());
        });
    }

    #[test]
    fn a_draft_round_trips_through_every_date_style() {
        let filters = TransactionFilters {
            account: Some(WALLET),
            category: Some(GROCERIES),
            payee: "wool".to_string(),
            tag: "japan".to_string(),
            status: StatusFilter::Cleared,
            from: Some(date(2025, 3, 4)),
            to: Some(date(2026, 8, 30)),
            ..TransactionFilters::defaults(today())
        };
        au(|| {
            for style in DATE_STYLES {
                let mut presenter = FilterPresenter::open(&reads(), &filters, None, today(), style);
                assert_eq!(applied(&mut presenter), filters, "{style:?}");
            }
        });
    }

    #[test]
    fn an_unparseable_date_shows_a_hint_and_refuses_apply() {
        au(|| {
            let mut presenter = open(&TransactionFilters::defaults(today()));
            type_into(&mut presenter, FilterField::From, "nope");

            let view = presenter.view_model();
            assert_eq!(
                view.from_error.as_deref(),
                Some(
                    "Enter a date like 19/9/2026 or 2026-09-19, or a word such as today, yesterday, tomorrow."
                )
            );
            assert!(view.to_error.is_none());
            assert!(!view.frame.can_confirm);
            assert_eq!(
                presenter.handle(FilterEvent::Apply),
                None,
                "the popover stays open"
            );

            type_into(&mut presenter, FilterField::From, "");
            assert!(
                presenter.view_model().frame.can_confirm,
                "empty is unbounded"
            );
            assert_eq!(applied(&mut presenter).from, None);
        });
    }

    #[test]
    fn an_id_hand_off_shows_the_name_and_applies_the_id_until_edited() {
        let filters = TransactionFilters::for_tag(today(), WORK_TRIP);
        let mut presenter = au(|| open(&filters));
        assert_eq!(presenter.view_model().tag.text, "work-trip");
        assert_eq!(au(|| applied(&mut presenter)), filters);

        presenter.handle(FilterEvent::Focus(FilterField::Tag));
        presenter.handle(FilterEvent::Backspace);
        let edited = au(|| applied(&mut presenter));
        assert_eq!((edited.tag.as_str(), edited.tag_id), ("work-tri", None));

        let filters = TransactionFilters::for_payee(today(), WOOLWORTHS);
        let mut presenter = au(|| open(&filters));
        assert_eq!(presenter.view_model().payee.text, "Woolworths");
        assert_eq!(au(|| applied(&mut presenter)), filters);
    }

    #[test]
    fn a_chip_opens_the_popover_on_its_own_field() {
        let focus = |chip| {
            au(|| {
                FilterPresenter::open(
                    &reads(),
                    &TransactionFilters::defaults(today()),
                    chip,
                    today(),
                    None,
                )
            })
            .focused()
        };
        assert_eq!(focus(None), FilterField::Account);
        assert_eq!(focus(Some(ChipField::Category)), FilterField::Category);
        assert_eq!(focus(Some(ChipField::Date)), FilterField::From);
        assert_eq!(focus(Some(ChipField::Status)), FilterField::Status);
    }

    #[test]
    fn the_options_lead_with_all_and_list_categories_by_path() {
        locale();
        let view = au(|| open(&TransactionFilters::defaults(today()))).view_model();
        fn labels<T>(options: &[Choice<T>]) -> Vec<String> {
            options.iter().map(|choice| choice.label.clone()).collect()
        }
        assert_eq!(
            labels(&view.account.options),
            ["All accounts", "Everyday", "Wallet"]
        );
        assert_eq!(
            labels(&view.category.options),
            ["Any category", "Food", "Food \u{203a} Groceries", "Housing"]
        );
        assert_eq!(
            labels(&view.status.options),
            ["All", "Open", "Cleared", "Reconciled"]
        );
    }

    fn chips(filters: &TransactionFilters) -> Vec<ChipView> {
        au(|| FilterPresenter::chips(&reads(), filters, today(), None))
    }

    fn labels(chips: &[ChipView]) -> Vec<&str> {
        chips.iter().map(|chip| chip.label.as_str()).collect()
    }

    fn active(chips: &[ChipView]) -> Vec<bool> {
        chips.iter().map(|chip| chip.active).collect()
    }

    #[test]
    fn the_defaults_are_six_outline_chips_with_this_year_as_the_date_chip() {
        let chips = chips(&TransactionFilters::defaults(today()));
        assert_eq!(
            labels(&chips),
            [
                "account: all accounts",
                "category: any",
                "payee: any",
                "tag: any",
                "this year",
                "status: All"
            ]
        );
        assert_eq!(active(&chips), [false; 6]);
        assert_eq!(
            chips.iter().map(|chip| chip.field).collect::<Vec<_>>(),
            ChipField::ALL
        );
    }

    #[test]
    fn a_changed_filter_is_an_accent_chip_that_reads_its_value() {
        let chips = chips(&TransactionFilters {
            account: Some(WALLET),
            category: Some(GROCERIES),
            payee: "  wool ".to_string(),
            tag: "japan".to_string(),
            status: StatusFilter::Cleared,
            ..TransactionFilters::defaults(today())
        });
        assert_eq!(
            labels(&chips),
            [
                "account: Wallet",
                "category: Food \u{203a} Groceries",
                "payee: wool",
                "tag: japan",
                "this year",
                "status: Cleared"
            ]
        );
        assert_eq!(active(&chips), [true, true, true, true, false, true]);
    }

    #[test]
    fn a_payee_or_tag_hand_off_names_its_row_and_a_gone_one_reads_unknown() {
        let payee = chips(&TransactionFilters::for_payee(today(), WOOLWORTHS));
        assert_eq!(payee[2].label, "payee: Woolworths");
        assert!(payee[2].active);
        let tag = chips(&TransactionFilters::for_tag(today(), WORK_TRIP));
        assert_eq!(tag[3].label, "tag: work-trip");
        let gone = chips(&TransactionFilters::for_account(
            today(),
            AccountId::seed(99),
        ));
        assert_eq!(gone[0].label, "account: unknown");
    }

    #[test]
    fn the_date_chip_reads_this_year_a_range_or_all_dates() {
        let range = |from, to| {
            let chips = chips(&TransactionFilters {
                from,
                to,
                ..TransactionFilters::defaults(today())
            });
            (chips[4].label.clone(), chips[4].active)
        };
        assert_eq!(range(None, None), ("all dates".to_string(), true));
        assert_eq!(
            range(Some(date(2026, 8, 12)), Some(date(2026, 9, 12))),
            ("12 Aug 2026 \u{2013} 12 Sept 2026".to_string(), true)
        );
        assert_eq!(
            range(Some(date(2026, 8, 12)), Some(today())),
            ("12 Aug 2026 \u{2013} today".to_string(), true)
        );
        assert_eq!(range(Some(date(2026, 8, 12)), None).0, "from 12 Aug 2026");
        assert_eq!(range(None, Some(date(2026, 9, 12))).0, "until 12 Sept 2026");
        let iso = au(|| {
            FilterPresenter::chips(
                &reads(),
                &TransactionFilters {
                    from: Some(date(2025, 1, 28)),
                    to: Some(date(2026, 3, 2)),
                    ..TransactionFilters::defaults(today())
                },
                today(),
                Some(DateStyle::Iso),
            )
        });
        assert_eq!(iso[4].label, "2025-01-28 \u{2013} 2026-03-02");
    }

    #[test]
    fn each_chips_clear_resets_just_its_filter() {
        let mut filters = TransactionFilters {
            account: Some(WALLET),
            category: Some(FOOD),
            payee_id: Some(WOOLWORTHS),
            tag: "y".to_string(),
            status: StatusFilter::Open,
            from: None,
            to: None,
            ..TransactionFilters::defaults(today())
        };

        FilterPresenter::clear_chip(&mut filters, ChipField::Payee, today());
        assert_eq!(filters.payee_id, None);
        assert_eq!(filters.account, Some(WALLET), "the others are untouched");
        for chip in [
            ChipField::Account,
            ChipField::Category,
            ChipField::Tag,
            ChipField::Status,
        ] {
            FilterPresenter::clear_chip(&mut filters, chip, today());
        }
        assert_eq!(filters.from, None, "the date is still as it was");
        FilterPresenter::clear_chip(&mut filters, ChipField::Date, today());
        assert_eq!(
            (filters.from, filters.to),
            (Some(date(2026, 1, 1)), Some(today())),
            "this year, not all dates"
        );
        assert!(filters.is_default(today()));
    }

    #[test]
    fn clear_all_puts_every_filter_back_to_the_defaults() {
        let mut filters = TransactionFilters {
            account_ids: vec![WALLET],
            payee: "wool".to_string(),
            from: None,
            ..TransactionFilters::for_category(today(), FOOD)
        };
        FilterPresenter::clear_all(&mut filters, today());
        assert_eq!(filters, TransactionFilters::defaults(today()));
    }

    #[test]
    fn a_budget_s_accounts_survive_apply_until_the_account_select_changes() {
        let budget = TransactionFilters::for_budget_category(
            FOOD,
            lib_core::Period::of(today()),
            &[EVERYDAY, WALLET],
        );
        au(|| {
            let mut presenter = open(&budget);
            presenter.handle(FilterEvent::Status(StatusFilter::Open));
            assert_eq!(
                applied(&mut presenter).account_ids,
                [EVERYDAY, WALLET],
                "a draft that leaves the Account alone keeps the Budget's"
            );

            presenter.handle(FilterEvent::Account(Some(WALLET)));
            let filters = applied(&mut presenter);
            assert_eq!(filters.account, Some(WALLET));
            assert!(filters.account_ids.is_empty(), "the select replaces them");
        });
    }

    #[test]
    fn a_budget_s_accounts_are_an_accent_account_chip_its_clear_drops() {
        let mut filters = TransactionFilters::for_budget_category(
            FOOD,
            lib_core::Period::of(today()),
            &[EVERYDAY, WALLET],
        );
        au(|| {
            let chips = FilterPresenter::chips(&reads(), &filters, today(), None);
            assert_eq!(chips[0].label, "account: 2 accounts");
            assert!(chips[0].active);
        });

        FilterPresenter::clear_chip(&mut filters, ChipField::Account, today());

        assert!(filters.account_ids.is_empty());
    }

    #[test]
    fn a_new_date_style_shows_the_draft_s_dates_in_it() {
        au(|| {
            let mut presenter = FilterPresenter::open(
                &reads(),
                &TransactionFilters::defaults(today()),
                None,
                today(),
                None,
            );
            type_into(&mut presenter, FilterField::From, "4/3/2025");
            type_into(&mut presenter, FilterField::Payee, "wool");

            presenter.set_date_style(Some(DateStyle::Iso));

            let view = presenter.view_model();
            assert_eq!(view.from.text, "2025-03-04", "read day-first, shown ISO");
            assert_eq!(view.payee.text, "wool");
            assert_eq!(applied(&mut presenter).from, Some(date(2025, 3, 4)));
        });
    }
}
