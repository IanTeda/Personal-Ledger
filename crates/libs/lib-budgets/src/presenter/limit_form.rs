//! Edit budget (9e): one Budget Amount record starting in the chosen month, **Onward** or
//! **Month-only**, with the Rollover on it.
//!
//! - Opened on a budgeted Category the Category is fixed and the fields are prefilled from the
//!   amount the starting month resolves to. Opened from **+ Budget a category** (or `set` on an
//!   unbudgeted row) a picker offers the leaf Expense Categories with no Budget Amount in the
//!   current month, defaulting to the current month, Onward and no Rollover.
//! - Starting offers the current month and the two after it; closed months are never offered.
//! - The summary under the fields is the save run on a copy, so it shows exactly what a save
//!   would write; none while the form is incomplete or the save would be refused.
//! - A confirm raises its own Toast and closes. **Stop budgeting…** hands the page Stop on the same
//!   Category and month.

use bigdecimal::{BigDecimal, RoundingMode, Signed};
use lib_core::{BudgetId, CategoryId, Clock, Money, Period};
use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};
use lib_viper::view::{
    Choice, DialogFrame, Hint, SelectList, SelectMove, SelectView, TextFieldView,
};

use super::{EMPTY_CELL, dialog_error_text, money, month_label, period_label};
use crate::interactor::SetAmount;
use crate::protocols::{BudgetCategories, BudgetsOutcome, SharedPorts};
use crate::{
    Budget, PERSONAL_SPENDING_ID, Rollover, Span, applied, msg, total_budgeted, unbudgeted_leaves,
};

/// How many months Starting offers, the current one first.
const START_MONTHS: usize = 3;

const SPANS: [Span; 2] = [Span::MonthOnly, Span::Onward];
const ROLLOVERS: [Rollover; 3] = [Rollover::None, Rollover::CarryUnspent, Rollover::CarryBoth];

/// The fields, in `Tab` order. Category is skipped while it is fixed.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitFormEvent {
    /// A Dialog keymap command or a typed key, meaning what the focused field makes it: `select`
    /// commits an open list and otherwise confirms a valid form; `submit` confirms from any field;
    /// the arrows and `Space` drive the Category and Starting selects, `←`/`→` step Applies to and
    /// Rollover, and the Amount takes digits and one decimal point. The rest swallow text.
    Input(DialogInput),
    /// The Amount's whole new text, as its View's input holds it.
    Amount(String),
    /// A click on a row of the Category's or Starting's open list: picks it and closes the list.
    Category(CategoryId),
    Starting(Period),
    /// A click on the Category's or Starting's field: focuses it and opens or closes its list.
    ToggleList(LimitField),
    /// Closes the open list without picking: the host's `back`.
    CloseList,
    Span(Span),
    Rollover(Rollover),
    /// A click on a field.
    Focus(LimitField),
    /// `Tab`/`Shift-Tab`, wrapping.
    NextField,
    PrevField,
    /// `left`/`right` on a focused segmented control, stopping at either end.
    StepBack,
    StepForward,
    /// `Stop budgeting…`.
    StopBudgeting,
    Cancel,
}

/// One line of the before/after summary: what it is, the figures and whether they are strong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitPreviewView {
    pub what: String,
    pub figures: String,
    pub strong: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitFormViewModel {
    pub frame: DialogFrame,
    /// The Category picker; `None` while the Category is fixed.
    pub category: Option<SelectView<CategoryId>>,
    /// Shown in place of the picker when every Category is already budgeted.
    pub no_categories: Option<String>,
    pub amount: TextFieldView,
    /// `Enter an amount of 0.00 or more.`, once something unreadable is typed.
    pub amount_error: Option<String>,
    pub starting: SelectView<Period>,
    /// The focused Category or Starting select's list: open, and its highlight.
    pub list: SelectList,
    pub span: SelectView<Span>,
    pub rollover: SelectView<Rollover>,
    pub preview: Vec<LimitPreviewView>,
    pub note: String,
    /// A refused save, shown until the next change.
    pub error: Option<String>,
    /// `Stop budgeting…`, offered while editing a budgeted Category.
    pub stop: Option<String>,
    pub focused: LimitField,
}

pub struct LimitFormPresenter {
    ports: SharedPorts,
    clock: Clock,
    budget: BudgetId,
    /// The Category being edited; `None` while the picker chooses it.
    fixed: Option<CategoryId>,
    category: Option<CategoryId>,
    /// The unbudgeted leaf Expense Categories by path in tree order, as the Dialog opened.
    categories: Vec<(CategoryId, String)>,
    months: Vec<Period>,
    amount: String,
    starting: Period,
    span: Span,
    rollover: Rollover,
    focused: LimitField,
    /// The focused select's list; it closes whenever focus leaves.
    list: SelectList,
    error: Option<String>,
}

/// What a save writes.
struct Draft {
    category: CategoryId,
    month: Period,
    amount: Money,
}

impl LimitFormPresenter {
    /// Edit budget on Category `category` of Budget `budget`, starting in `month` when that is
    /// offered. A Category with no amount this month gets the picker with it preselected (9a's
    /// `set`). `None` when the Budget is archived, or the Category is a parent or can't be
    /// budgeted.
    pub fn edit(
        ports: &SharedPorts,
        clock: Clock,
        budget: BudgetId,
        category: CategoryId,
        month: Period,
    ) -> Option<Self> {
        let found = Self::editable(ports, budget)?;
        let current = Period::of(clock.today());
        let mut form = Self::open(ports, clock, &found);
        if applied(found.chain(category), current).is_some() {
            let is_leaf = ports
                .borrow()
                .reads()
                .budget_categories()
                .iter()
                .any(|each| each.id == category && each.is_leaf);
            if !is_leaf {
                return None;
            }
            form.fixed = Some(category);
            form.starting = if form.months.contains(&month) {
                month
            } else {
                current
            };
            let found = applied(found.chain(category), form.starting);
            form.amount = found
                .as_ref()
                .map(|found| found.amount.0.to_string())
                .unwrap_or_default();
            form.rollover = found.map(|found| found.rollover).unwrap_or_default();
            form.focused = LimitField::Amount;
        } else if form.categories.iter().any(|(id, _)| *id == category) {
            form.category = Some(category);
            form.focused = LimitField::Amount;
        } else {
            return None;
        }
        Some(form)
    }

    /// **+ Budget a category**: Edit budget with its Category picker.
    pub fn picker(ports: &SharedPorts, clock: Clock, budget: BudgetId) -> Option<Self> {
        let found = Self::editable(ports, budget)?;
        Some(Self::open(ports, clock, &found))
    }

    fn editable(ports: &SharedPorts, budget: BudgetId) -> Option<Budget> {
        ports
            .borrow()
            .budgets()
            .budget(budget)
            .filter(|budget| !budget.is_archived())
    }

    /// The picker's defaults: the current month, Onward and no Rollover.
    fn open(ports: &SharedPorts, clock: Clock, budget: &Budget) -> Self {
        let current = Period::of(clock.today());
        let unbudgeted = unbudgeted_leaves(budget, ports.borrow().reads(), current);
        let categories = ports
            .borrow()
            .reads()
            .budget_categories()
            .into_iter()
            .filter(|category| unbudgeted.contains(&category.id))
            .map(|category| (category.id, category.path))
            .collect();
        Self {
            ports: ports.clone(),
            clock,
            budget: budget.id,
            fixed: None,
            category: None,
            categories,
            months: std::iter::successors(Some(current), |month| Some(month.next()))
                .take(START_MONTHS)
                .collect(),
            amount: String::new(),
            starting: current,
            span: Span::Onward,
            rollover: Rollover::None,
            focused: LimitField::Category,
            list: SelectList::default(),
            error: None,
        }
    }

    fn is_pick(&self) -> bool {
        self.fixed.is_none()
    }

    fn category_id(&self) -> Option<CategoryId> {
        self.fixed.or(self.category)
    }

    pub fn focused(&self) -> LimitField {
        self.focused
    }

    pub fn handle(&mut self, event: LimitFormEvent, router: &mut dyn DialogRouter<BudgetsOutcome>) {
        match event {
            LimitFormEvent::Input(input) => self.input(input, router),
            LimitFormEvent::Amount(text) => {
                self.amount = text;
                self.error = None;
            }
            LimitFormEvent::Category(id) => {
                self.list.close();
                self.pick_category(id);
            }
            LimitFormEvent::Starting(month) => {
                self.list.close();
                self.pick_starting(month);
            }
            LimitFormEvent::ToggleList(field) => self.toggle_list(field),
            LimitFormEvent::CloseList => {
                self.list.close();
            }
            LimitFormEvent::Span(span) => {
                self.list.close();
                self.focused = LimitField::Span;
                self.span = span;
                self.error = None;
            }
            LimitFormEvent::Rollover(rollover) => {
                self.list.close();
                self.focused = LimitField::Rollover;
                self.rollover = rollover;
                self.error = None;
            }
            LimitFormEvent::Focus(field) => {
                self.list.close();
                if field != LimitField::Category || self.is_pick() {
                    self.focused = field;
                }
            }
            LimitFormEvent::NextField => self.cycle(false),
            LimitFormEvent::PrevField => self.cycle(true),
            LimitFormEvent::StepBack => self.step(false),
            LimitFormEvent::StepForward => self.step(true),
            LimitFormEvent::StopBudgeting => {
                if let Some(category) = self.fixed {
                    router.close(Some(BudgetsOutcome::Stop(category, self.starting)));
                }
            }
            LimitFormEvent::Cancel => router.close(None),
        }
    }

    /// What `input` means on the focused field.
    fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<BudgetsOutcome>) {
        match (input, self.focused) {
            (DialogInput::NextField, _) => self.cycle(false),
            (DialogInput::PreviousField, _) => self.cycle(true),
            (DialogInput::Submit, _) => self.confirm(router),
            (DialogInput::Left, _) => self.step(false),
            (DialogInput::Right, _) => self.step(true),
            // `select` confirms except where it commits an open list.
            (DialogInput::Select, _) if !self.list.is_open() => self.confirm(router),
            (_, LimitField::Category) => {
                if self.is_pick() {
                    let options = self.category_choices();
                    let moved = self.list.input(input, &options, self.category.as_ref());
                    if let SelectMove::Chose(id) = moved {
                        self.pick_category(id);
                    }
                }
            }
            (_, LimitField::Starting) => {
                let options = self.starting_choices();
                if let SelectMove::Chose(month) =
                    self.list.input(input, &options, Some(&self.starting))
                {
                    self.pick_starting(month);
                }
            }
            (DialogInput::Char(ch), LimitField::Amount)
                if ch.is_ascii_digit() || (ch == '.' && !self.amount.contains('.')) =>
            {
                self.amount.push(ch);
                self.error = None;
            }
            (DialogInput::Backspace, LimitField::Amount) => {
                self.amount.pop();
                self.error = None;
            }
            _ => {}
        }
    }

    fn pick_category(&mut self, id: CategoryId) {
        if self.categories.iter().any(|(each, _)| *each == id) && self.is_pick() {
            self.category = Some(id);
            self.error = None;
        }
    }

    fn pick_starting(&mut self, month: Period) {
        if self.months.contains(&month) {
            self.starting = month;
            self.error = None;
        }
    }

    /// A click on a select's field: focuses it, closing the other's list, and opens or closes its
    /// own.
    fn toggle_list(&mut self, field: LimitField) {
        let has_list = match field {
            LimitField::Category => self.is_pick() && !self.categories.is_empty(),
            LimitField::Starting => true,
            _ => false,
        };
        if !has_list {
            return;
        }
        if self.focused != field {
            self.list.close();
            self.focused = field;
        }
        match field {
            LimitField::Category => {
                let options = self.category_choices();
                self.list.toggle(&options, self.category.as_ref());
            }
            _ => {
                let options = self.starting_choices();
                self.list.toggle(&options, Some(&self.starting));
            }
        }
    }

    fn category_choices(&self) -> Vec<Choice<CategoryId>> {
        self.categories
            .iter()
            .map(|(id, path)| Choice {
                value: *id,
                label: path.clone(),
            })
            .collect()
    }

    fn starting_choices(&self) -> Vec<Choice<Period>> {
        let current = Period::of(self.clock.today());
        self.months
            .iter()
            .map(|month| Choice {
                value: *month,
                label: month_label(*month, current),
            })
            .collect()
    }

    /// `Tab` / `Shift+Tab`, wrapping, skipping the fixed Category. Leaving a select closes its
    /// list, as a click elsewhere would.
    fn cycle(&mut self, back: bool) {
        self.list.close();
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
        self.focused = order[next];
    }

    fn step(&mut self, forward: bool) {
        fn next<T: Copy + PartialEq>(all: &[T], current: T, forward: bool) -> T {
            let at = all.iter().position(|each| *each == current).unwrap_or(0);
            let to = if forward {
                (at + 1).min(all.len() - 1)
            } else {
                at.saturating_sub(1)
            };
            all[to]
        }
        match self.focused {
            LimitField::Span => self.span = next(&SPANS, self.span, forward),
            LimitField::Rollover => self.rollover = next(&ROLLOVERS, self.rollover, forward),
            _ => return,
        }
        self.error = None;
    }

    /// The typed amount to the cent; 0.00 is a real Budget Amount.
    fn amount_money(&self) -> Option<Money> {
        self.amount
            .trim()
            .parse::<BigDecimal>()
            .ok()
            .filter(|amount| !amount.is_negative())
            .map(|amount| Money(amount.with_scale_round(2, RoundingMode::HalfUp)))
    }

    fn draft(&self) -> Option<Draft> {
        Some(Draft {
            category: self.category_id()?,
            month: self.starting,
            amount: self.amount_money()?,
        })
    }

    fn can_confirm(&self) -> bool {
        self.draft().is_some()
    }

    /// Saves the amount through the Interactor, raises the Toast and closes the Dialog; a refusal
    /// stays on the form. A form that can't confirm does nothing.
    pub fn confirm(&mut self, router: &mut dyn DialogRouter<BudgetsOutcome>) {
        let Some(draft) = self.draft() else {
            return;
        };
        let today = self.clock.today();
        let done = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            SetAmount {
                id: self.budget,
                category_id: draft.category,
                month: draft.month,
                span: self.span,
                amount: draft.amount.clone(),
                rollover: Some(self.rollover),
                today,
            }
            .run(ports.reads, ports.budgets)
        };
        match done {
            Ok(()) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::budgets_toast_limit_saved(&self.category_name(draft.category)),
                ));
                router.close(None);
            }
            Err(refusal) => self.error = Some(dialog_error_text(&refusal)),
        }
    }

    fn category_name(&self, id: CategoryId) -> String {
        self.ports
            .borrow()
            .reads()
            .budget_categories()
            .into_iter()
            .find(|category| category.id == id)
            .map(|category| category.name)
            .unwrap_or_default()
    }

    /// The summary of `draft`: the save run on a copy, so it is `None` when the save would be
    /// refused.
    fn preview(&self, draft: &Draft) -> Option<Vec<LimitPreviewView>> {
        let ports = self.ports.borrow();
        let reads = ports.reads();
        let categories: &dyn BudgetCategories = reads;
        let budget = ports.budgets().budget(self.budget)?;
        let current = Period::of(self.clock.today());
        let amount_in = |budget: &Budget, month| {
            applied(budget.chain(draft.category), month).map(|found| found.amount)
        };
        let or_dash = |value: Option<&Money>| value.map_or_else(|| EMPTY_CELL.to_string(), money);
        let mut after = budget.clone();
        after
            .set_amount(
                categories,
                draft.category,
                draft.month,
                self.span,
                draft.amount.clone(),
                Some(self.rollover),
                self.clock.today(),
            )
            .ok()?;
        let month = period_label(draft.month);
        let mut lines = Vec::new();
        if draft.month > current {
            lines.push(LimitPreviewView {
                what: period_label(current),
                figures: msg::budgets_limit_preview_unchanged(&or_dash(
                    amount_in(&budget, current).as_ref(),
                )),
                strong: false,
            });
        }
        lines.push(LimitPreviewView {
            what: match self.span {
                Span::Onward => msg::budgets_limit_preview_onward(&month),
                Span::MonthOnly => msg::budgets_limit_preview_month_only(&month),
            },
            figures: msg::budgets_limit_preview_change(
                &or_dash(amount_in(&budget, draft.month).as_ref()),
                &money(&draft.amount),
            ),
            strong: true,
        });
        lines.push(LimitPreviewView {
            what: msg::budgets_limit_preview_total(&month),
            figures: msg::budgets_limit_preview_change(
                &money(&total_budgeted(&budget, categories, draft.month)),
                &money(&total_budgeted(&after, categories, draft.month)),
            ),
            strong: false,
        });
        Some(lines)
    }

    pub fn view_model(&self) -> LimitFormViewModel {
        let draft = self.draft();
        let title = match self.fixed {
            Some(category) => msg::budgets_limit_title(&self.category_name(category)),
            None => msg::budgets_limit_title_new(),
        };
        let pick = self.is_pick();
        let option = |field| self.focused == field;
        LimitFormViewModel {
            frame: DialogFrame {
                title,
                submit: msg::budgets_limit_submit(),
                can_confirm: draft.is_some(),
                destructive: false,
                hints: Vec::new(),
            },
            category: (pick && !self.categories.is_empty()).then(|| SelectView {
                label: msg::budgets_limit_field_category(),
                options: self.category_choices(),
                chosen: self.category,
                focused: option(LimitField::Category),
            }),
            no_categories: (pick && self.categories.is_empty())
                .then(msg::budgets_limit_no_categories),
            amount: TextFieldView {
                label: msg::budgets_limit_field_amount(),
                text: self.amount.clone(),
                placeholder: "0.00".to_string(),
                focused: option(LimitField::Amount),
            },
            amount_error: (!self.amount.trim().is_empty() && self.amount_money().is_none())
                .then(msg::budgets_limit_error_amount),
            starting: SelectView {
                label: msg::budgets_limit_field_starting(),
                options: self.starting_choices(),
                chosen: Some(self.starting),
                focused: option(LimitField::Starting),
            },
            list: self.list,
            span: SelectView {
                label: msg::budgets_limit_field_span(),
                options: SPANS
                    .into_iter()
                    .map(|span| Choice {
                        value: span,
                        label: match span {
                            Span::MonthOnly => msg::budgets_limit_span_month_only(),
                            Span::Onward => msg::budgets_limit_span_onward(),
                        },
                    })
                    .collect(),
                chosen: Some(self.span),
                focused: option(LimitField::Span),
            },
            rollover: SelectView {
                label: msg::budgets_limit_field_rollover(),
                options: ROLLOVERS
                    .into_iter()
                    .map(|rollover| Choice {
                        value: rollover,
                        label: match rollover {
                            Rollover::None => msg::budgets_limit_rollover_none(),
                            Rollover::CarryUnspent => msg::budgets_limit_rollover_unspent(),
                            Rollover::CarryBoth => msg::budgets_limit_rollover_both(),
                        },
                    })
                    .collect(),
                chosen: Some(self.rollover),
                focused: option(LimitField::Rollover),
            },
            preview: draft
                .as_ref()
                .and_then(|draft| self.preview(draft))
                .unwrap_or_default(),
            note: if self.budget == PERSONAL_SPENDING_ID {
                msg::budgets_limit_note()
            } else {
                msg::budgets_limit_note_other()
            },
            error: self.error.clone(),
            stop: (!pick).then(msg::budgets_limit_stop),
            focused: self.focused,
        }
    }

    /// The status line's legend, `next_field`, `select` and `back` read from the Dialog `keymap`.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        vec![
            Hint::new(
                keymap.chord(NEXT_FIELD).unwrap_or_default(),
                msg::budgets_hint_next_field(),
            ),
            Hint::new("\u{2190}/\u{2192}", msg::budgets_hint_choose()),
            Hint::new(
                keymap.chord(SELECT).unwrap_or_default(),
                msg::budgets_hint_save(),
            ),
            Hint::new(
                keymap.chord(BACK).unwrap_or_default(),
                msg::budgets_hint_cancel(),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{RecordingRouter, Routed};
    use crate::presenter::test_support::{sample, today};
    use crate::protocols::BudgetRepository as _;
    use crate::{PERSONAL_SPENDING_ID, cents_money};

    const GROCERIES: CategoryId = CategoryId::seed(7);

    fn current() -> Period {
        Period::of(today())
    }

    struct Form {
        ports: std::rc::Rc<std::cell::RefCell<crate::fake::InMemoryPorts>>,
        presenter: LimitFormPresenter,
        router: RecordingRouter,
    }

    impl Form {
        fn edit() -> Self {
            let (ports, shared) = sample();
            Self {
                presenter: LimitFormPresenter::edit(
                    &shared,
                    Clock::fixed(today()),
                    PERSONAL_SPENDING_ID,
                    GROCERIES,
                    current(),
                )
                .expect("groceries is budgeted"),
                ports,
                router: RecordingRouter::default(),
            }
        }

        fn picker() -> Self {
            let (ports, shared) = sample();
            // Groceries stops this month, so the picker has a leaf to offer.
            {
                let mut held = ports.borrow_mut();
                crate::interactor::StopBudgeting {
                    id: PERSONAL_SPENDING_ID,
                    category_id: GROCERIES,
                    month: current(),
                    today: today(),
                }
                .run(&held.reads.clone(), &mut held.budgets)
                .expect("groceries is budgeted");
            }
            Self {
                presenter: LimitFormPresenter::picker(
                    &shared,
                    Clock::fixed(today()),
                    PERSONAL_SPENDING_ID,
                )
                .expect("the default takes edits"),
                ports,
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: LimitFormEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        fn view(&self) -> LimitFormViewModel {
            self.presenter.view_model()
        }
    }

    #[test]
    fn editing_a_budgeted_category_fixes_it_and_prefills_the_amount_it_resolves_to() {
        let form = Form::edit();
        let view = form.view();

        assert_eq!(view.frame.title, "Edit budget — Groceries");
        assert!(view.category.is_none(), "the Category is fixed");
        assert!(!view.amount.text.is_empty());
        assert!(view.frame.can_confirm);
        assert!(view.stop.is_some());
        assert_eq!(view.focused, LimitField::Amount);
        assert_eq!(view.starting.options.len(), 3);
    }

    #[test]
    fn the_picker_offers_the_unbudgeted_leaves_and_cannot_confirm_until_one_and_an_amount_are_set()
    {
        let mut form = Form::picker();
        let view = form.view();

        assert_eq!(view.frame.title, "Budget a category");
        assert!(!view.frame.can_confirm);
        assert_eq!(view.focused, LimitField::Category);
        let category = view.category.expect("a picker");
        assert!(!category.options.is_empty());

        form.send(LimitFormEvent::Category(category.options[0].value));
        form.send(LimitFormEvent::Amount("120".into()));
        assert!(form.view().frame.can_confirm);
        assert!(form.view().stop.is_none(), "nothing to stop yet");
    }

    #[test]
    fn an_unreadable_amount_shows_its_error_and_blocks_confirm() {
        let mut form = Form::edit();

        form.send(LimitFormEvent::Amount("12x".into()));
        assert_eq!(
            form.view().amount_error.as_deref(),
            Some("Enter an amount of 0.00 or more.")
        );
        assert!(!form.view().frame.can_confirm);

        form.send(LimitFormEvent::Amount("-5".into()));
        assert!(
            !form.view().frame.can_confirm,
            "a negative amount is refused"
        );

        form.send(LimitFormEvent::Amount(String::new()));
        assert_eq!(
            form.view().amount_error,
            None,
            "nothing typed is not an error"
        );
    }

    #[test]
    fn tab_cycles_the_fields_skipping_a_fixed_category() {
        let mut form = Form::edit();
        let mut seen = vec![form.view().focused];
        for _ in 0..4 {
            form.send(LimitFormEvent::NextField);
            seen.push(form.view().focused);
        }

        assert_eq!(
            seen,
            [
                LimitField::Amount,
                LimitField::Starting,
                LimitField::Span,
                LimitField::Rollover,
                LimitField::Amount,
            ]
        );
        form.send(LimitFormEvent::PrevField);
        assert_eq!(form.view().focused, LimitField::Rollover);
    }

    #[test]
    fn left_and_right_step_a_focused_segmented_control_and_stop_at_either_end() {
        let mut form = Form::edit();
        form.send(LimitFormEvent::Focus(LimitField::Rollover));

        form.send(LimitFormEvent::StepForward);
        form.send(LimitFormEvent::StepForward);
        form.send(LimitFormEvent::StepForward);
        assert_eq!(form.view().rollover.chosen, Some(Rollover::CarryBoth));
        form.send(LimitFormEvent::StepBack);
        assert_eq!(form.view().rollover.chosen, Some(Rollover::CarryUnspent));

        form.send(LimitFormEvent::Focus(LimitField::Amount));
        form.send(LimitFormEvent::StepForward);
        assert_eq!(form.view().rollover.chosen, Some(Rollover::CarryUnspent));
    }

    #[test]
    fn the_summary_is_the_save_run_on_a_copy_and_shows_the_current_month_left_alone_for_a_later_start()
     {
        let mut form = Form::edit();
        form.send(LimitFormEvent::Amount("700".into()));
        let now = form.view().preview;
        assert_eq!(now.len(), 2, "this month's change and the total");
        assert!(now[0].strong);

        form.send(LimitFormEvent::Starting(current().next()));
        let later = form.view().preview;
        assert_eq!(later.len(), 3, "the current month is left alone");
        assert!(later[0].figures.ends_with("unchanged"));
        assert_eq!(
            form.ports
                .borrow()
                .budgets
                .budget(PERSONAL_SPENDING_ID)
                .map(|budget| applied(budget.chain(GROCERIES), current().next()).is_some()),
            Some(true),
            "the summary wrote nothing"
        );
    }

    #[test]
    fn confirming_writes_the_amount_raises_the_toast_then_closes() {
        let mut form = Form::edit();
        form.send(LimitFormEvent::Amount("700.456".into()));

        form.presenter.confirm(&mut form.router);

        assert_eq!(
            form.router.routed,
            [
                Routed::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Saved the budget for Groceries".into()
                )),
                Routed::Closed(None),
            ]
        );
        let budget = form
            .ports
            .borrow()
            .budgets
            .budget(PERSONAL_SPENDING_ID)
            .expect("the sample's default");
        assert_eq!(
            applied(budget.chain(GROCERIES), current()).map(|found| found.amount),
            Some(cents_money(70_046)),
            "rounded to the cent"
        );
    }

    #[test]
    fn stop_budgeting_hands_the_page_stop_on_the_category_and_month() {
        let mut form = Form::edit();
        form.send(LimitFormEvent::Starting(current().next()));

        form.send(LimitFormEvent::StopBudgeting);

        assert_eq!(
            form.router.routed,
            [Routed::Closed(Some(BudgetsOutcome::Stop(
                GROCERIES,
                current().next()
            )))]
        );
        let mut picker = Form::picker();
        picker.send(LimitFormEvent::StopBudgeting);
        assert_eq!(picker.router.routed, [], "the picker has nothing to stop");
    }

    #[test]
    fn it_does_not_open_on_a_parent_an_archived_budget_or_a_category_that_cannot_be_budgeted() {
        let (ports, shared) = sample();
        let open = |category| {
            LimitFormPresenter::edit(
                &shared,
                Clock::fixed(today()),
                PERSONAL_SPENDING_ID,
                category,
                current(),
            )
        };
        assert!(open(CategoryId::seed(6)).is_none(), "Food is a parent");
        assert!(open(CategoryId::seed(9999)).is_none());

        ports
            .borrow_mut()
            .budgets
            .budgets
            .iter_mut()
            .for_each(|budget| {
                budget.archived_at = Some(today());
            });
        assert!(open(GROCERIES).is_none());
        assert!(
            LimitFormPresenter::picker(&shared, Clock::fixed(today()), PERSONAL_SPENDING_ID)
                .is_none()
        );
    }

    fn type_into(form: &mut Form, text: &str) {
        for ch in text.chars() {
            form.send(LimitFormEvent::Input(DialogInput::Char(ch)));
        }
    }

    #[test]
    fn the_amount_takes_digits_and_one_point_and_backspace_takes_one_back() {
        let mut form = Form::edit();
        form.send(LimitFormEvent::Amount(String::new()));

        type_into(&mut form, "1a2.3.4-");
        assert_eq!(form.view().amount.text, "12.34");
        form.send(LimitFormEvent::Input(DialogInput::Backspace));
        assert_eq!(form.view().amount.text, "12.3");
    }

    #[test]
    fn tab_moves_focus_and_the_arrows_step_the_segmented_controls() {
        let mut form = Form::edit();
        form.send(LimitFormEvent::Input(DialogInput::NextField));
        assert_eq!(form.view().focused, LimitField::Starting);
        form.send(LimitFormEvent::Input(DialogInput::NextField));
        assert_eq!(form.view().focused, LimitField::Span);
        form.send(LimitFormEvent::Input(DialogInput::Right));
        let span = form.view().span.chosen;
        form.send(LimitFormEvent::Input(DialogInput::Left));
        assert_ne!(form.view().span.chosen, span);
        type_into(&mut form, "zz");
        assert_eq!(form.view().span.chosen, Some(Span::MonthOnly));
        form.send(LimitFormEvent::Input(DialogInput::PreviousField));
        assert_eq!(form.view().focused, LimitField::Starting);
    }

    #[test]
    fn space_opens_the_starting_list_the_arrows_move_and_select_commits_it() {
        let mut form = Form::edit();
        form.send(LimitFormEvent::Focus(LimitField::Starting));
        let chosen = form.view().starting.chosen;

        form.send(LimitFormEvent::Input(DialogInput::Char(' ')));
        assert!(form.view().list.is_open());
        form.send(LimitFormEvent::Input(DialogInput::SelectNext));
        assert_eq!(
            form.view().starting.chosen,
            chosen,
            "moving only highlights"
        );
        form.send(LimitFormEvent::Input(DialogInput::Select));

        assert!(!form.view().list.is_open());
        assert_ne!(form.view().starting.chosen, chosen);
        assert!(
            form.router.routed.is_empty(),
            "committing a list is not confirming"
        );
    }

    #[test]
    fn select_on_a_closed_field_confirms_a_valid_form_and_submit_confirms_from_a_list() {
        let mut form = Form::edit();
        form.send(LimitFormEvent::Focus(LimitField::Starting));
        form.send(LimitFormEvent::Input(DialogInput::Char(' ')));
        form.send(LimitFormEvent::Input(DialogInput::Submit));
        assert!(
            form.router.routed.contains(&Routed::Closed(None)),
            "{:?}",
            form.router.routed
        );

        let mut form = Form::edit();
        form.send(LimitFormEvent::Input(DialogInput::Select));
        assert!(form.router.routed.contains(&Routed::Closed(None)));
    }

    #[test]
    fn the_picker_category_list_opens_by_click_and_a_click_elsewhere_closes_it() {
        let mut form = Form::picker();
        assert_eq!(form.view().focused, LimitField::Category);
        form.send(LimitFormEvent::ToggleList(LimitField::Category));
        assert!(form.view().list.is_open());
        form.send(LimitFormEvent::Input(DialogInput::SelectNext));
        form.send(LimitFormEvent::Input(DialogInput::Select));
        assert!(form.view().category.expect("picker").chosen.is_some());

        form.send(LimitFormEvent::ToggleList(LimitField::Starting));
        assert!(form.view().list.is_open());
        assert_eq!(form.view().focused, LimitField::Starting);
        form.send(LimitFormEvent::Focus(LimitField::Amount));
        assert!(!form.view().list.is_open());

        form.send(LimitFormEvent::ToggleList(LimitField::Starting));
        form.send(LimitFormEvent::CloseList);
        assert!(!form.view().list.is_open());
    }
}
