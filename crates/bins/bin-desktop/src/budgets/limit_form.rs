//! The live forms behind **Edit budget** (9e) and **Stop budgeting** (9g) in
//! `docs/ux/desktop/14-budgets-v2/README.md`: `gpui`-free and unit-tested, the same split
//! `bills/pay_form.rs` uses.
//!
//! The behaviour is the Desktop Budgets Surface map's settled decisions (#383, #386):
//!
//! - 9e writes one record starting in the chosen month: **Onward** or **Month-only**, with the
//!   Rollover on it. Starting offers the current month and the two after it; closed months are
//!   never offered.
//! - Opened on a budgeted Category, the Category is fixed and the fields are prefilled from the
//!   amount the starting month resolves to. Opened from **+ Budget a category** (or `set` on an
//!   unbudgeted row), a picker offers the leaf Expense Categories with no Budget Amount in the
//!   current month, defaulting to the current month, Onward and no Rollover.
//! - 9g writes a **Stop** from the current month or the next.

use bigdecimal::{BigDecimal, RoundingMode, Signed};
use lib_core::Money;

use crate::{
    accounts::SelectKey,
    budgets::{self, Budget, BudgetError, Budgets, Rollover, Span},
    categories::{self, Category},
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    field::TextField,
    period::Period,
    select::SelectState,
};

/// How many months 9e's Starting offers, the current one first.
pub const START_MONTHS: usize = 3;

/// How many months 9g's From offers, the current one first.
pub const STOP_MONTHS: usize = 2;

/// 9e's fields, in `Tab` order. Category is skipped while it is fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitField {
    Category,
    Amount,
    Starting,
    Span,
    Rollover,
}

impl LimitField {
    const ORDER: [LimitField; 5] = [
        Self::Category,
        Self::Amount,
        Self::Starting,
        Self::Span,
        Self::Rollover,
    ];
}

pub const SPANS: [Span; 2] = [Span::MonthOnly, Span::Onward];
pub const ROLLOVERS: [Rollover; 3] = [Rollover::None, Rollover::CarryUnspent, Rollover::CarryBoth];

/// The selects' options. Each `*_labels` is what its select shows and stores; the matching ids
/// and months run parallel to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitOptions {
    /// The unbudgeted leaf Expense Categories, by path in tree order.
    pub category_labels: Vec<String>,
    category_ids: Vec<u32>,
    pub month_labels: Vec<String>,
    months: Vec<Period>,
}

impl LimitOptions {
    /// `count` months from `current`; `month_label` words one, told whether it is the current.
    pub fn new(
        budget: &Budget,
        categories: &[Category],
        current: Period,
        count: usize,
        month_label: impl Fn(Period, bool) -> String,
    ) -> Self {
        let unbudgeted = budgets::unbudgeted_leaves(budget, categories, current);
        let (category_ids, category_labels) = categories::paths_in_tree_order(categories)
            .into_iter()
            .filter(|(id, _)| unbudgeted.contains(id))
            .unzip();
        let months: Vec<Period> = std::iter::successors(Some(current), |month| Some(month.next()))
            .take(count)
            .collect();
        Self {
            category_labels,
            category_ids,
            month_labels: months
                .iter()
                .map(|month| month_label(*month, *month == current))
                .collect(),
            months,
        }
    }

    fn category_label(&self, id: u32) -> Option<String> {
        let index = self.category_ids.iter().position(|c| *c == id)?;
        self.category_labels.get(index).cloned()
    }

    fn category_id(&self, label: Option<&str>) -> Option<u32> {
        let index = self
            .category_labels
            .iter()
            .position(|l| Some(l.as_str()) == label)?;
        self.category_ids.get(index).copied()
    }

    fn month_label(&self, month: Period) -> Option<String> {
        let index = self.months.iter().position(|m| *m == month)?;
        self.month_labels.get(index).cloned()
    }

    fn month(&self, label: Option<&str>) -> Option<Period> {
        let index = self
            .month_labels
            .iter()
            .position(|l| Some(l.as_str()) == label)?;
        self.months.get(index).copied()
    }

    /// `preferred` when it is offered, else the first (current) month.
    fn starting(&self, preferred: Period) -> SelectState {
        SelectState::new(
            self.month_label(preferred)
                .or_else(|| self.month_labels.first().cloned()),
        )
    }
}

/// What saving 9e writes.
#[derive(Debug, Clone, PartialEq)]
pub struct LimitDraft {
    pub category_id: u32,
    pub month: Period,
    pub span: Span,
    pub amount: Money,
    pub rollover: Rollover,
}

/// 9e's before/after summary.
#[derive(Debug, Clone, PartialEq)]
pub struct LimitPreview {
    /// The current month and its amount, when the change starts later and so leaves it alone.
    pub unchanged: Option<(Period, Option<Money>)>,
    pub month: Period,
    pub span: Span,
    /// The starting month's amount now; `None` while Unbudgeted.
    pub before: Option<Money>,
    pub after: Money,
    /// The starting month's Total budgeted, before and after.
    pub total_before: Money,
    pub total_after: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitForm {
    /// The Category being edited; `None` while the picker chooses it.
    pub fixed_category: Option<u32>,
    pub category: SelectState,
    pub amount: TextField,
    pub starting: SelectState,
    pub span: Span,
    pub rollover: Rollover,
    /// What the selects choose from, copied in as the Dialog opens.
    pub options: LimitOptions,
    pub focused: LimitField,
    /// A refused save, shown until the next change.
    pub error: Option<BudgetError>,
}

impl LimitForm {
    /// 9e on a Category that is budgeted in `month` (or the first offered month): prefilled from
    /// the amount that month resolves to.
    pub fn edit(budget: &Budget, category_id: u32, month: Period, options: &LimitOptions) -> Self {
        let starting = options.starting(month);
        let applied = options
            .month(starting.value())
            .and_then(|month| budgets::applied(budget.chain(category_id), month));
        Self {
            fixed_category: Some(category_id),
            category: SelectState::default(),
            amount: TextField::new(
                applied
                    .as_ref()
                    .map(|found| found.amount.0.to_string())
                    .unwrap_or_default(),
            ),
            starting,
            span: Span::Onward,
            rollover: applied.map(|found| found.rollover).unwrap_or_default(),
            options: options.clone(),
            focused: LimitField::Amount,
            error: None,
        }
    }

    /// 9e from **+ Budget a category**: the picker, optionally preselected (`set` on a row).
    pub fn pick(preselected: Option<u32>, options: &LimitOptions) -> Self {
        let label = preselected.and_then(|id| options.category_label(id));
        Self {
            fixed_category: None,
            focused: if label.is_some() {
                LimitField::Amount
            } else {
                LimitField::Category
            },
            category: SelectState::new(label),
            amount: TextField::default(),
            starting: SelectState::new(options.month_labels.first().cloned()),
            span: Span::Onward,
            rollover: Rollover::None,
            options: options.clone(),
            error: None,
        }
    }

    pub fn is_pick(&self) -> bool {
        self.fixed_category.is_none()
    }

    pub fn category_id(&self) -> Option<u32> {
        self.fixed_category
            .or_else(|| self.options.category_id(self.category.value()))
    }

    pub fn month(&self) -> Option<Period> {
        self.options.month(self.starting.value())
    }

    pub fn focus(&mut self, field: LimitField) {
        if field == LimitField::Category && !self.is_pick() {
            return;
        }
        if self.focused != field {
            self.close_selects();
        }
        self.focused = field;
    }

    /// `Tab` / `Shift+Tab`, wrapping, skipping the fixed Category.
    pub fn cycle_focus(&mut self, back: bool) {
        let order: Vec<LimitField> = LimitField::ORDER
            .into_iter()
            .filter(|field| *field != LimitField::Category || self.is_pick())
            .collect();
        let at = order.iter().position(|f| *f == self.focused).unwrap_or(0);
        let next = if back {
            (at + order.len() - 1) % order.len()
        } else {
            (at + 1) % order.len()
        };
        self.close_selects();
        self.focused = order[next];
    }

    fn close_selects(&mut self) {
        self.category.cancel();
        self.starting.cancel();
    }

    fn select_mut(&mut self, field: LimitField) -> Option<(&mut SelectState, &[String])> {
        match field {
            LimitField::Category if self.fixed_category.is_none() => {
                Some((&mut self.category, &self.options.category_labels))
            }
            LimitField::Starting => Some((&mut self.starting, &self.options.month_labels)),
            _ => None,
        }
    }

    /// A key on the focused select. Returns whether a select is focused (and so took the key).
    pub fn handle_select_key(&mut self, key: SelectKey) -> bool {
        let Some((state, list)) = self.select_mut(self.focused) else {
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
        self.error = None;
        true
    }

    /// A click on a select's closed field: focuses it and toggles its list.
    pub fn click_select(&mut self, field: LimitField) {
        self.focus(field);
        if let Some((state, list)) = self.select_mut(field) {
            if state.is_open() {
                state.cancel();
            } else {
                state.open(list);
            }
        }
    }

    /// A click on row `index` of `field`'s open list.
    pub fn choose(&mut self, field: LimitField, index: usize) {
        if let Some((state, list)) = self.select_mut(field) {
            state.choose(list, index);
        }
        self.error = None;
    }

    pub fn set_span(&mut self, span: Span) {
        self.focus(LimitField::Span);
        self.span = span;
        self.error = None;
    }

    pub fn set_rollover(&mut self, rollover: Rollover) {
        self.focus(LimitField::Rollover);
        self.rollover = rollover;
        self.error = None;
    }

    /// `left`/`right` on a focused segmented control, stopping at either end. Returns whether one
    /// is focused.
    pub fn step_segment(&mut self, forward: bool) -> bool {
        fn step<T: Copy + PartialEq>(all: &[T], current: T, forward: bool) -> T {
            let at = all.iter().position(|v| *v == current).unwrap_or(0);
            let next = if forward {
                (at + 1).min(all.len() - 1)
            } else {
                at.saturating_sub(1)
            };
            all[next]
        }
        match self.focused {
            LimitField::Span => self.span = step(&SPANS, self.span, forward),
            LimitField::Rollover => self.rollover = step(&ROLLOVERS, self.rollover, forward),
            _ => return false,
        }
        self.error = None;
        true
    }

    /// The typed amount to the cent; 0.00 is a real Budget Amount.
    fn amount_money(&self) -> Option<Money> {
        self.amount
            .text()
            .trim()
            .parse::<BigDecimal>()
            .ok()
            .filter(|amount| !amount.is_negative())
            .map(|amount| Money(amount.with_scale_round(2, RoundingMode::HalfUp)))
    }

    /// The amount's problem, once something is typed.
    pub fn amount_invalid(&self) -> bool {
        !self.amount.is_blank() && self.amount_money().is_none()
    }

    /// What Save writes, or `None` while the form is incomplete.
    pub fn draft(&self) -> Option<LimitDraft> {
        Some(LimitDraft {
            category_id: self.category_id()?,
            month: self.month()?,
            span: self.span,
            amount: self.amount_money()?,
            rollover: self.rollover,
        })
    }
}

/// Writes 9e's draft into Budget `id`.
pub fn save(
    budgets: &mut Budgets,
    id: u32,
    categories: &[Category],
    draft: &LimitDraft,
    today: chrono::NaiveDate,
) -> Result<(), BudgetError> {
    budgets.set_amount(
        id,
        categories,
        draft.category_id,
        draft.month,
        draft.span,
        draft.amount.clone(),
        Some(draft.rollover),
        today,
    )
}

/// 9e's before/after summary for `draft`, or `None` when the save would be refused.
pub fn preview(
    budgets: &Budgets,
    id: u32,
    categories: &[Category],
    draft: &LimitDraft,
    today: chrono::NaiveDate,
) -> Option<LimitPreview> {
    let budget = budgets.get(id)?;
    let chain = budget.chain(draft.category_id);
    let amount_in = |month| budgets::applied(chain, month).map(|found| found.amount);
    let current = Period::of(today);
    // The stub is small, so the after side is simply the save run on a copy.
    let mut after = budgets.clone();
    save(&mut after, id, categories, draft, today).ok()?;
    Some(LimitPreview {
        unchanged: (draft.month > current).then(|| (current, amount_in(current))),
        month: draft.month,
        span: draft.span,
        before: amount_in(draft.month),
        after: draft.amount.clone(),
        total_before: budgets::total_budgeted(budget, categories, draft.month),
        total_after: budgets::total_budgeted(after.get(id)?, categories, draft.month),
    })
}

/// 9g's form: the Category and the month the Stop starts in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopForm {
    pub category_id: u32,
    pub from: SelectState,
    /// The months on offer, copied in as the Dialog opens.
    pub options: LimitOptions,
    /// A refused confirm, shown until the next change.
    pub error: Option<BudgetError>,
}

impl StopForm {
    /// Defaults to `month` when it is offered, else the current month.
    pub fn new(category_id: u32, month: Period, options: &LimitOptions) -> Self {
        Self {
            category_id,
            from: options.starting(month),
            options: options.clone(),
            error: None,
        }
    }

    pub fn month(&self) -> Option<Period> {
        self.options.month(self.from.value())
    }

    pub fn handle_select_key(&mut self, key: SelectKey) {
        let list = &self.options.month_labels;
        match (key, self.from.is_open()) {
            (SelectKey::Up, true) => self.from.move_highlight(list, -1),
            (SelectKey::Down, true) => self.from.move_highlight(list, 1),
            (SelectKey::Up, false) => self.from.step(list, -1),
            (SelectKey::Down, false) => self.from.step(list, 1),
            (SelectKey::Activate, true) => self.from.commit(list),
            (SelectKey::Activate, false) => self.from.open(list),
        }
        self.error = None;
    }

    pub fn click_select(&mut self) {
        if self.from.is_open() {
            self.from.cancel();
        } else {
            self.from.open(&self.options.month_labels);
        }
    }

    pub fn choose(&mut self, index: usize) {
        self.from.choose(&self.options.month_labels, index);
        self.error = None;
    }
}

impl Dialog for LimitForm {
    /// `Shift-Tab` goes back a field. On a select `↑`/`↓` step or move the highlight, and `Space`
    /// opens or commits its list, as `Enter` does while one is open (otherwise `Enter`
    /// confirms). `←`/`→` step the focused segmented control. The Amount takes digits and one
    /// decimal point; nothing else takes text. Any key clears a stale Save error.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.error = None;
        let focused = self.focused;
        let on_select = matches!(focused, LimitField::Category | LimitField::Starting);
        let list_open = self.category.is_open() || self.starting.is_open();
        match key {
            DialogKey::BackTab => self.cycle_focus(true),
            DialogKey::Up if on_select => {
                self.handle_select_key(SelectKey::Up);
            }
            DialogKey::Down if on_select => {
                self.handle_select_key(SelectKey::Down);
            }
            DialogKey::Char(' ') if on_select => {
                self.handle_select_key(SelectKey::Activate);
            }
            DialogKey::Enter if list_open => {
                self.handle_select_key(SelectKey::Activate);
            }
            DialogKey::Left | DialogKey::Right => {
                self.step_segment(key == DialogKey::Right);
            }
            DialogKey::Char(ch) if focused == LimitField::Amount => {
                if ch.is_ascii_digit() || (ch == '.' && !self.amount.text().contains('.')) {
                    return None;
                }
            }
            DialogKey::Char(_) | DialogKey::Backspace if focused != LimitField::Amount => {}
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        (self.focused == LimitField::Amount).then_some(&mut self.amount)
    }

    fn cycle_field(&mut self) {
        self.cycle_focus(false);
    }

    fn is_valid(&self) -> bool {
        self.draft().is_some()
    }

    fn close_open_select(&mut self) -> bool {
        let open = self.category.is_open() || self.starting.is_open();
        self.close_selects();
        open
    }
}

impl Dialog for StopForm {
    /// `↑`/`↓` (and `j`/`k`) pick the month and `Space` opens or commits its list, as `Enter`
    /// does while it is open (otherwise `Enter` confirms). Nothing takes text.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let select = match key {
            DialogKey::Up | DialogKey::Char('k') => SelectKey::Up,
            DialogKey::Down | DialogKey::Char('j') => SelectKey::Down,
            DialogKey::Char(' ') => SelectKey::Activate,
            DialogKey::Enter if self.from.is_open() => SelectKey::Activate,
            _ => return None,
        };
        self.handle_select_key(select);
        Some(DialogOutcome::Handled)
    }

    fn is_valid(&self) -> bool {
        self.month().is_some()
    }

    fn close_open_select(&mut self) -> bool {
        let open = self.from.is_open();
        self.from.cancel();
        open
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::{accounts::default_accounts, categories::default_categories};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).unwrap()
    }

    fn sep() -> Period {
        Period::of(today())
    }

    fn money(text: &str) -> Money {
        text.parse().unwrap()
    }

    struct World {
        categories: Vec<Category>,
        budgets: Budgets,
    }

    impl World {
        fn new() -> Self {
            let categories = default_categories();
            let mut budgets = budgets::default_budgets(&default_accounts(), &categories, today());
            // The seed budgets every Expense leaf, so two are Stopped for the picker to offer.
            for name in ["Household", "Water"] {
                let id = categories::find_by_name(&categories, name).unwrap();
                budgets
                    .stop(
                        budgets::PERSONAL_SPENDING_ID,
                        &categories,
                        id,
                        Period::of(today()),
                        today(),
                    )
                    .unwrap();
            }
            Self {
                categories,
                budgets,
            }
        }

        fn budget(&self) -> &Budget {
            self.budgets.get(budgets::PERSONAL_SPENDING_ID).unwrap()
        }

        fn options(&self, count: usize) -> LimitOptions {
            LimitOptions::new(
                self.budget(),
                &self.categories,
                sep(),
                count,
                |month, now| {
                    format!(
                        "{}-{}{}",
                        month.year,
                        month.month,
                        if now { " now" } else { "" }
                    )
                },
            )
        }

        fn category(&self, name: &str) -> u32 {
            categories::find_by_name(&self.categories, name).unwrap()
        }

        /// A leaf the seeded Budget has an amount for this month.
        fn budgeted(&self) -> u32 {
            self.budget()
                .category_ids()
                .find(|id| budgets::applied(self.budget().chain(*id), sep()).is_some())
                .unwrap()
        }
    }

    #[test]
    fn options_offer_the_current_month_first_and_only_unbudgeted_leaves() {
        let world = World::new();
        let options = world.options(START_MONTHS);
        assert_eq!(
            options.month_labels,
            ["2026-9 now", "2026-10", "2026-11"].map(String::from)
        );
        assert!(!options.category_labels.is_empty());
        for id in &options.category_ids {
            assert!(categories::is_leaf(&world.categories, *id));
            assert!(budgets::applied(world.budget().chain(*id), sep()).is_none());
        }
        assert!(!options.category_ids.contains(&world.budgeted()));
    }

    #[test]
    fn edit_prefills_from_the_starting_month_and_fixes_the_category() {
        let world = World::new();
        let options = world.options(START_MONTHS);
        let id = world.budgeted();
        let applied = budgets::applied(world.budget().chain(id), sep()).unwrap();
        let mut form = LimitForm::edit(world.budget(), id, sep(), &options);
        assert_eq!(form.amount.text(), applied.amount.0.to_string());
        assert_eq!(form.rollover, applied.rollover);
        assert_eq!(form.span, Span::Onward);
        assert_eq!(form.month(), Some(sep()));
        assert_eq!(form.category_id(), Some(id));

        // Tab never lands on the fixed Category.
        for _ in 0..8 {
            form.cycle_focus(false);
            assert_ne!(form.focused, LimitField::Category);
        }
        // A closed month isn't offered, so the form falls back to the current one.
        let past = LimitForm::edit(world.budget(), id, sep().prev(), &options);
        assert_eq!(past.month(), Some(sep()));
    }

    #[test]
    fn pick_defaults_and_needs_a_category_and_an_amount() {
        let world = World::new();
        let options = world.options(START_MONTHS);
        let mut form = LimitForm::pick(None, &options);
        assert_eq!(form.focused, LimitField::Category);
        assert_eq!(form.rollover, Rollover::None);
        assert_eq!(form.span, Span::Onward);
        assert_eq!(form.month(), Some(sep()));
        assert_eq!(form.draft(), None);

        assert!(form.handle_select_key(SelectKey::Down));
        assert_eq!(form.category_id(), Some(options.category_ids[0]));
        assert_eq!(form.draft(), None, "no amount yet");

        form.cycle_focus(false);
        assert_eq!(form.focused, LimitField::Amount);
        for ch in "12.5.0x".chars() {
            if form.handle_own_key(DialogKey::Char(ch)).is_none() {
                form.focused_text().unwrap().push(ch);
            }
        }
        assert_eq!(form.amount.text(), "12.50");
        let draft = form.draft().unwrap();
        assert_eq!(draft.amount, money("12.50"));
        assert_eq!(draft.span, Span::Onward);

        let preselected = LimitForm::pick(Some(options.category_ids[1]), &options);
        assert_eq!(preselected.focused, LimitField::Amount);
        assert_eq!(preselected.category_id(), Some(options.category_ids[1]));
    }

    #[test]
    fn zero_is_an_amount_and_segments_step_without_wrapping() {
        let world = World::new();
        let options = world.options(START_MONTHS);
        let mut form = LimitForm::pick(Some(options.category_ids[0]), &options);
        form.amount.push('0');
        assert_eq!(form.draft().unwrap().amount, money("0.00"));
        assert!(!form.step_segment(true), "the Amount is no segment");

        form.focus(LimitField::Span);
        assert!(form.step_segment(false));
        assert_eq!(form.span, Span::MonthOnly);
        form.step_segment(false);
        assert_eq!(form.span, Span::MonthOnly);
        form.focus(LimitField::Rollover);
        form.step_segment(true);
        form.step_segment(true);
        form.step_segment(true);
        assert_eq!(form.rollover, Rollover::CarryBoth);
    }

    #[test]
    fn save_writes_the_record_and_preview_shows_before_and_after() {
        let mut world = World::new();
        let options = world.options(START_MONTHS);
        let id = world.budgeted();
        let before = budgets::applied(world.budget().chain(id), sep())
            .unwrap()
            .amount;
        let mut form = LimitForm::edit(world.budget(), id, sep(), &options);
        form.focus(LimitField::Starting);
        form.handle_select_key(SelectKey::Down);
        let october = sep().next();
        assert_eq!(form.month(), Some(october));
        form.focus(LimitField::Amount);
        form.amount = TextField::new((before.0.clone() + BigDecimal::from(50)).to_string());
        form.set_rollover(Rollover::CarryUnspent);
        let draft = form.draft().unwrap();

        let preview = preview(
            &world.budgets,
            budgets::PERSONAL_SPENDING_ID,
            &world.categories,
            &draft,
            today(),
        )
        .unwrap();
        assert_eq!(preview.unchanged, Some((sep(), Some(before.clone()))));
        assert_eq!(preview.before, Some(before.clone()));
        assert_eq!(preview.after, draft.amount);
        assert_eq!(
            preview.total_after.0,
            preview.total_before.0.clone() + BigDecimal::from(50),
            "only this Category moved"
        );

        save(
            &mut world.budgets,
            budgets::PERSONAL_SPENDING_ID,
            &world.categories,
            &draft,
            today(),
        )
        .unwrap();
        let chain = world.budget().chain(id);
        assert_eq!(budgets::applied(chain, sep()).unwrap().amount, before);
        let applied = budgets::applied(chain, october).unwrap();
        assert_eq!(applied.amount, draft.amount);
        assert_eq!(applied.rollover, Rollover::CarryUnspent);
        assert!(applied.own_record);
        assert_eq!(
            budgets::applied(chain, october.next()).unwrap().amount,
            draft.amount,
            "Onward carries forward"
        );
    }

    #[test]
    fn month_only_leaves_the_next_month_alone() {
        let mut world = World::new();
        let options = world.options(START_MONTHS);
        let id = world.budgeted();
        let before = budgets::applied(world.budget().chain(id), sep())
            .unwrap()
            .amount;
        let mut form = LimitForm::edit(world.budget(), id, sep(), &options);
        form.amount = TextField::new("1");
        form.set_span(Span::MonthOnly);
        let draft = form.draft().unwrap();
        let shown = preview(
            &world.budgets,
            budgets::PERSONAL_SPENDING_ID,
            &world.categories,
            &draft,
            today(),
        )
        .unwrap();
        assert_eq!(shown.unchanged, None, "the change starts this month");
        save(
            &mut world.budgets,
            budgets::PERSONAL_SPENDING_ID,
            &world.categories,
            &draft,
            today(),
        )
        .unwrap();
        let chain = world.budget().chain(id);
        assert_eq!(
            budgets::applied(chain, sep()).unwrap().amount,
            money("1.00")
        );
        assert_eq!(
            budgets::applied(chain, sep().next()).unwrap().amount,
            before
        );
    }

    #[test]
    fn picking_budgets_an_unbudgeted_category() {
        let mut world = World::new();
        let options = world.options(START_MONTHS);
        let id = options.category_ids[0];
        let mut form = LimitForm::pick(Some(id), &options);
        form.amount = TextField::new("80");
        let draft = form.draft().unwrap();
        save(
            &mut world.budgets,
            budgets::PERSONAL_SPENDING_ID,
            &world.categories,
            &draft,
            today(),
        )
        .unwrap();
        assert_eq!(
            budgets::applied(world.budget().chain(id), sep())
                .unwrap()
                .amount,
            money("80.00")
        );
        assert!(!world.options(START_MONTHS).category_ids.contains(&id));
    }

    #[test]
    fn stop_form_offers_two_months_and_defaults_to_an_offered_one() {
        let world = World::new();
        let options = world.options(STOP_MONTHS);
        let id = world.category("Dining");
        let mut form = StopForm::new(id, sep().next(), &options);
        assert_eq!(form.month(), Some(sep().next()));
        form.handle_select_key(SelectKey::Down);
        assert_eq!(form.month(), Some(sep()), "two months, wrapping");
        let far = StopForm::new(id, sep().next().next(), &options);
        assert_eq!(far.month(), Some(sep()));
    }
}
