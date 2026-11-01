//! Stop budgeting (9g): a Category's Budget Amount ends from the current month or the next. A Stop
//! deletes nothing (a later Onward amount resumes the Category), so there is no typed confirmation.
//!
//! The Category must be a leaf with a Budget Amount this month or in the month asked for, in a
//! Budget that takes edits; otherwise there is nothing to stop and the Dialog doesn't open. A
//! confirm raises its own Toast and closes; a refusal stays on the form.

use lib_core::{BudgetId, CategoryId, Clock, Period};
use lib_toast::{Notice, ToastKind};
use lib_viper::keymap::{BACK, Keymap, SELECT};
use lib_viper::view::{Choice, DialogFrame, Hint, SelectView};

use super::{dialog_error_text, month_label};
use crate::interactor::StopBudgeting;
use crate::protocols::{BudgetsRouter, SharedPorts};
use crate::{applied, msg};

/// How many months From offers, the current one first.
const MONTHS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopEvent {
    /// The month the Stop starts in.
    From(Period),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopViewModel {
    pub frame: DialogFrame,
    pub from: SelectView<Period>,
    /// What the Stop does and keeps.
    pub body: String,
    /// The Category's Bill Plans, which keep counting as Known Costs.
    pub bills: Option<String>,
    pub error: Option<String>,
}

pub struct StopPresenter {
    ports: SharedPorts,
    clock: Clock,
    budget: BudgetId,
    category: CategoryId,
    category_name: String,
    months: Vec<Period>,
    from: Period,
    error: Option<String>,
}

impl StopPresenter {
    /// Stop budgeting on Category `category` of Budget `budget`, starting in `month` when that is
    /// offered, else in the current month. `None` when there is nothing to stop.
    pub fn new(
        ports: &SharedPorts,
        clock: Clock,
        budget: BudgetId,
        category: CategoryId,
        month: Period,
    ) -> Option<Self> {
        let found = ports.borrow().budgets().budget(budget)?;
        if found.is_archived() {
            return None;
        }
        let current = Period::of(clock.today());
        let chain = found.chain(category);
        if applied(chain, current).is_none() && applied(chain, month).is_none() {
            return None;
        }
        let category_name = ports
            .borrow()
            .reads()
            .budget_categories()
            .into_iter()
            .find(|each| each.id == category)?
            .name;
        let months: Vec<Period> = std::iter::successors(Some(current), |month| Some(month.next()))
            .take(MONTHS)
            .collect();
        let from = if months.contains(&month) {
            month
        } else {
            current
        };
        Some(Self {
            ports: ports.clone(),
            clock,
            budget,
            category,
            category_name,
            months,
            from,
            error: None,
        })
    }

    /// The Category the Stop is for.
    pub fn category(&self) -> CategoryId {
        self.category
    }

    pub fn handle(&mut self, event: StopEvent, router: &mut dyn BudgetsRouter) {
        match event {
            StopEvent::From(month) => {
                if self.months.contains(&month) {
                    self.from = month;
                    self.error = None;
                }
            }
            StopEvent::Cancel => router.close_dialog(None),
        }
    }

    /// Stops the Category through the Interactor, raises the Toast and closes the Dialog; a
    /// refusal stays on the form.
    pub fn confirm(&mut self, router: &mut dyn BudgetsRouter) {
        let today = self.clock.today();
        let done = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            StopBudgeting {
                id: self.budget,
                category_id: self.category,
                month: self.from,
                today,
            }
            .run(ports.reads, ports.budgets)
        };
        match done {
            Ok(()) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::budgets_toast_stopped(&self.category_name),
                ));
                router.close_dialog(None);
            }
            Err(refusal) => self.error = Some(dialog_error_text(&refusal)),
        }
    }

    pub fn view_model(&self) -> StopViewModel {
        let plans: Vec<String> = self
            .ports
            .borrow()
            .reads()
            .active_bill_plans()
            .into_iter()
            .filter(|plan| plan.category_id == self.category)
            .map(|plan| plan.name)
            .collect();
        StopViewModel {
            frame: DialogFrame {
                title: msg::budgets_stop_title(&self.category_name),
                submit: msg::budgets_stop_submit(),
                can_confirm: true,
                destructive: false,
                hints: Vec::new(),
            },
            from: SelectView {
                label: msg::budgets_stop_field_from(),
                options: self
                    .months
                    .iter()
                    .map(|month| Choice {
                        value: *month,
                        label: month_label(*month, Period::of(self.clock.today())),
                    })
                    .collect(),
                chosen: Some(self.from),
                focused: true,
            },
            body: msg::budgets_stop_body(&self.category_name),
            bills: (!plans.is_empty()).then(|| msg::budgets_stop_body_bills(&plans.join(", "))),
            error: self.error.clone(),
        }
    }

    /// The status line's legend, `select` and `back` read from the Dialog `keymap`.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        vec![
            Hint::new("\u{2191}/\u{2193}", msg::budgets_hint_choose()),
            Hint::new(
                keymap.chord(SELECT).unwrap_or_default(),
                msg::budgets_hint_confirm(),
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
    use crate::presenter::test_support::{stub, today};
    use crate::protocols::BudgetRepository as _;
    use crate::{PERSONAL_SPENDING_ID, applied};

    const GROCERIES: CategoryId = CategoryId::seed(7);
    const FOOD: CategoryId = CategoryId::seed(6);

    fn current() -> Period {
        Period::of(today())
    }

    fn stop(category: CategoryId, month: Period) -> Option<StopPresenter> {
        let (_, shared) = stub();
        StopPresenter::new(
            &shared,
            Clock::fixed(today()),
            PERSONAL_SPENDING_ID,
            category,
            month,
        )
    }

    #[test]
    fn it_opens_on_a_budgeted_leaf_in_the_month_asked_for_and_offers_this_month_and_next() {
        let presenter = stop(GROCERIES, current().next()).expect("groceries is budgeted");
        let view = presenter.view_model();

        assert_eq!(view.frame.title, "Stop budgeting Groceries");
        assert_eq!(view.from.chosen, Some(current().next()));
        assert_eq!(
            view.from
                .options
                .iter()
                .map(|each| each.value)
                .collect::<Vec<_>>(),
            [current(), current().next()]
        );
        assert!(view.from.options[0].label.ends_with("(current)"));
    }

    #[test]
    fn a_month_that_is_not_offered_falls_back_to_the_current_month() {
        let presenter = stop(GROCERIES, current().next().next()).expect("budgeted");

        assert_eq!(presenter.view_model().from.chosen, Some(current()));
    }

    #[test]
    fn there_is_nothing_to_stop_on_a_parent_an_unknown_category_or_an_archived_budget() {
        assert!(stop(FOOD, current()).is_none(), "a parent only rolls up");
        assert!(stop(CategoryId::seed(9999), current()).is_none());

        let (shared_ports, shared) = stub();
        shared_ports
            .borrow_mut()
            .budgets
            .budgets
            .iter_mut()
            .for_each(|budget| {
                if budget.id == PERSONAL_SPENDING_ID {
                    budget.archived_at = Some(today());
                }
            });
        assert!(
            StopPresenter::new(
                &shared,
                Clock::fixed(today()),
                PERSONAL_SPENDING_ID,
                GROCERIES,
                current()
            )
            .is_none()
        );
    }

    #[test]
    fn confirming_stops_the_category_from_the_chosen_month_raises_the_toast_then_closes() {
        let (ports, shared) = stub();
        let mut presenter = StopPresenter::new(
            &shared,
            Clock::fixed(today()),
            PERSONAL_SPENDING_ID,
            GROCERIES,
            current(),
        )
        .expect("budgeted");
        let mut router = RecordingRouter::default();
        presenter.handle(StopEvent::From(current().next()), &mut router);

        presenter.confirm(&mut router);

        assert_eq!(
            router.routed,
            [
                Routed::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Stopped budgeting Groceries".into()
                )),
                Routed::Closed(None),
            ]
        );
        let budget = ports
            .borrow()
            .budgets
            .budget(PERSONAL_SPENDING_ID)
            .expect("the stub's default");
        assert!(applied(budget.chain(GROCERIES), current()).is_some());
        assert!(applied(budget.chain(GROCERIES), current().next()).is_none());
    }

    #[test]
    fn cancelling_closes_having_written_nothing() {
        let (ports, shared) = stub();
        let before = ports.borrow().budgets.clone();
        let mut presenter = StopPresenter::new(
            &shared,
            Clock::fixed(today()),
            PERSONAL_SPENDING_ID,
            GROCERIES,
            current(),
        )
        .expect("budgeted");
        let mut router = RecordingRouter::default();

        presenter.handle(StopEvent::Cancel, &mut router);

        assert_eq!(router.routed, [Routed::Closed(None)]);
        assert_eq!(ports.borrow().budgets, before);
    }
}
