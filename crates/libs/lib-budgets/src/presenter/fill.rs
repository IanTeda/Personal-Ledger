//! Fill (9f): a month's Month-only amounts from the month before as budgeted, or from the 3-month
//! average Spent. "The plan" is not a source but the baseline the preview diffs against (#386).
//!
//! Both sources show the Total budgeted the month would end with, and the chosen one the diff
//! against the plan. A Fill that would change nothing can't confirm, as its disabled button says.

use bigdecimal::BigDecimal;
use lib_core::{BudgetId, Clock, Money, Period};
use lib_locale::format::format_month;
use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::keymap::{BACK, Keymap, SELECT, list_step_chords};
use lib_viper::view::{DialogFrame, Hint, count};

use super::{EMPTY_CELL, dialog_error_text, money, period_label, signed_money};
use crate::interactor::Fill;
use crate::protocols::{BudgetsOutcome, SharedPorts};
use crate::{FillPreview, FillSource, fill_preview, msg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillEvent {
    /// A Dialog keymap command or a typed key: `j`/`k` and the arrows pick the source; `select`
    /// and `submit` fill, once there is something to fill.
    Input(DialogInput),
    /// A click on a source.
    Source(FillSource),
    /// `j`/`k`, a source down or up.
    Down,
    Up,
    Cancel,
}

/// A source's radio row, already worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRow {
    pub source: FillSource,
    pub title: String,
    pub detail: String,
    /// The target month's Total budgeted if this source fills it.
    pub total: String,
    pub chosen: bool,
}

/// One diff line: the Category, `before → after` and the signed difference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRow {
    pub name: String,
    pub figures: String,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillViewModel {
    pub frame: DialogFrame,
    pub sources: Vec<SourceRow>,
    /// `Changes from the plan`.
    pub changes_title: String,
    pub changes: Vec<ChangeRow>,
    /// The Categories with their own record in the month, never overwritten.
    pub kept: Vec<ChangeRow>,
    /// `Nothing would change.`, while there is no change.
    pub no_changes: Option<String>,
    /// `3 other categories unchanged`.
    pub unchanged: Option<String>,
    pub note: String,
    pub error: Option<String>,
}

pub struct FillPresenter {
    ports: SharedPorts,
    clock: Clock,
    budget: BudgetId,
    month: Period,
    source: FillSource,
    error: Option<String>,
}

impl FillPresenter {
    /// Fill `month` of Budget `budget`, or `None` when the Budget is gone or archived.
    pub fn new(ports: &SharedPorts, clock: Clock, budget: BudgetId, month: Period) -> Option<Self> {
        let found = ports.borrow().budgets().budget(budget)?;
        if found.is_archived() {
            return None;
        }
        Some(Self {
            ports: ports.clone(),
            clock,
            budget,
            month,
            source: FillSource::default(),
            error: None,
        })
    }

    pub fn source(&self) -> FillSource {
        self.source
    }

    pub fn handle(&mut self, event: FillEvent, router: &mut dyn DialogRouter<BudgetsOutcome>) {
        let at = FillSource::ALL
            .iter()
            .position(|each| *each == self.source)
            .unwrap_or(0);
        match event {
            FillEvent::Input(input) => {
                match input {
                    DialogInput::SelectNext => {
                        self.handle(FillEvent::Down, router);
                    }
                    DialogInput::SelectPrevious => {
                        self.handle(FillEvent::Up, router);
                    }
                    DialogInput::Select | DialogInput::Submit => self.confirm(router),
                    _ => {}
                }
                return;
            }
            FillEvent::Source(source) => self.source = source,
            FillEvent::Down => {
                self.source = FillSource::ALL[(at + 1).min(FillSource::ALL.len() - 1)]
            }
            FillEvent::Up => self.source = FillSource::ALL[at.saturating_sub(1)],
            FillEvent::Cancel => {
                router.close(None);
                return;
            }
        }
        self.error = None;
    }

    fn preview(&self, source: FillSource) -> Option<FillPreview> {
        let budget = self.ports.borrow().budgets().budget(self.budget)?;
        let ports = self.ports.borrow();
        Some(fill_preview(&budget, ports.reads(), self.month, source))
    }

    fn can_confirm(&self) -> bool {
        self.preview(self.source)
            .is_some_and(|preview| !preview.changes.is_empty())
    }

    /// Fills the month through the Interactor, raises the Toast and closes the Dialog. A Fill
    /// that would change nothing does nothing; a refusal stays on the form.
    pub fn confirm(&mut self, router: &mut dyn DialogRouter<BudgetsOutcome>) {
        if !self.can_confirm() {
            return;
        }
        let today = self.clock.today();
        let done = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            Fill {
                id: self.budget,
                target: self.month,
                source: self.source,
                today,
            }
            .run(ports.reads, ports.budgets)
        };
        match done {
            Ok(_) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::budgets_toast_filled(&period_label(self.month)),
                ));
                router.close(None);
            }
            Err(refusal) => self.error = Some(dialog_error_text(&refusal)),
        }
    }

    pub fn view_model(&self) -> FillViewModel {
        let month = period_label(self.month);
        let categories = self.ports.borrow().reads().budget_categories();
        let name = |id| {
            categories
                .iter()
                .find(|category| category.id == id)
                .map(|category| category.name.clone())
                .unwrap_or_default()
        };
        let chosen = self.preview(self.source);
        let sources = FillSource::ALL
            .into_iter()
            .map(|source| {
                let total = self
                    .preview(source)
                    .map_or_else(|| EMPTY_CELL.to_string(), |preview| money(&preview.total));
                let (title, detail) = match source {
                    FillSource::PreviousMonth => (
                        msg::budgets_fill_source_previous(&format_month(self.month.prev().month)),
                        msg::budgets_fill_source_previous_detail(),
                    ),
                    FillSource::Average => (
                        msg::budgets_fill_source_average(),
                        msg::budgets_fill_source_average_detail(
                            &format_month(self.month.prev().prev().prev().month),
                            &format_month(self.month.prev().month),
                        ),
                    ),
                };
                SourceRow {
                    source,
                    title,
                    detail,
                    total,
                    chosen: source == self.source,
                }
            })
            .collect();
        let (changes, kept, no_changes, unchanged, can_confirm) = match &chosen {
            Some(preview) => (
                preview
                    .changes
                    .iter()
                    .map(|line| {
                        let delta = Money(
                            line.after.0.clone()
                                - line
                                    .before
                                    .as_ref()
                                    .map_or_else(|| BigDecimal::from(0), |value| value.0.clone()),
                        );
                        ChangeRow {
                            name: name(line.category_id),
                            figures: msg::budgets_fill_change(
                                &line
                                    .before
                                    .as_ref()
                                    .map_or_else(|| EMPTY_CELL.to_string(), money),
                                &money(&line.after),
                            ),
                            delta: signed_money(&delta),
                        }
                    })
                    .collect(),
                preview
                    .kept
                    .iter()
                    .map(|id| ChangeRow {
                        name: name(*id),
                        figures: String::new(),
                        delta: msg::budgets_fill_kept(),
                    })
                    .collect(),
                preview
                    .changes
                    .is_empty()
                    .then(msg::budgets_fill_no_changes),
                (preview.unchanged > 0)
                    .then(|| msg::budgets_fill_unchanged(count(preview.unchanged))),
                !preview.changes.is_empty(),
            ),
            None => (Vec::new(), Vec::new(), None, None, false),
        };
        FillViewModel {
            frame: DialogFrame {
                title: msg::budgets_fill_title(&month),
                submit: msg::budgets_fill_submit(&month),
                can_confirm,
                destructive: false,
                hints: Vec::new(),
            },
            sources,
            changes_title: msg::budgets_fill_changes(),
            changes,
            kept,
            no_changes,
            unchanged,
            note: msg::budgets_fill_note(),
            error: self.error.clone(),
        }
    }

    /// The status line's legend, `select` and `back` read from the Dialog `keymap`.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        vec![
            Hint::new(list_step_chords(keymap), msg::budgets_hint_source()),
            Hint::new(
                keymap.chord(SELECT).unwrap_or_default(),
                msg::budgets_hint_fill_confirm(),
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
    use lib_core::CategoryId;

    use super::*;
    use crate::fake::{RecordingRouter, Routed};
    use crate::interactor::SetAmount;
    use crate::presenter::test_support::{sample, today};
    use crate::protocols::BudgetRepository as _;
    use crate::{PERSONAL_SPENDING_ID, Span, applied, cents_money};

    const GROCERIES: CategoryId = CategoryId::seed(7);

    fn next() -> Period {
        Period::of(today()).next()
    }

    fn fill() -> (
        std::rc::Rc<std::cell::RefCell<crate::fake::InMemoryPorts>>,
        FillPresenter,
    ) {
        let (ports, shared) = sample();
        // This month's own amount, so next month's plan differs from "as budgeted".
        {
            let mut held = ports.borrow_mut();
            SetAmount {
                id: PERSONAL_SPENDING_ID,
                category_id: GROCERIES,
                month: Period::of(today()),
                span: Span::MonthOnly,
                amount: cents_money(99_900),
                rollover: None,
                today: today(),
            }
            .run(&held.reads.clone(), &mut held.budgets)
            .expect("a leaf this month");
        }
        let presenter =
            FillPresenter::new(&shared, Clock::fixed(today()), PERSONAL_SPENDING_ID, next())
                .expect("the default takes edits");
        (ports, presenter)
    }

    #[test]
    fn it_opens_on_the_previous_month_with_each_sources_total_and_the_plan_diff() {
        let (_, presenter) = fill();
        let view = presenter.view_model();

        assert_eq!(view.frame.title, "Fill Jul 2026 from…");
        assert_eq!(view.frame.submit, "Fill Jul 2026");
        assert_eq!(view.sources.len(), 2);
        assert!(view.sources[0].chosen && !view.sources[1].chosen);
        assert!(view.sources.iter().all(|source| !source.total.is_empty()));
        assert_eq!(view.changes_title, "Changes from the plan");
    }

    #[test]
    fn j_and_k_pick_the_source_and_stop_at_either_end() {
        let (_, mut presenter) = fill();
        let mut router = RecordingRouter::default();

        presenter.handle(FillEvent::Down, &mut router);
        presenter.handle(FillEvent::Down, &mut router);
        assert_eq!(presenter.source(), FillSource::Average);
        presenter.handle(FillEvent::Up, &mut router);
        presenter.handle(FillEvent::Up, &mut router);
        assert_eq!(presenter.source(), FillSource::PreviousMonth);
        presenter.handle(FillEvent::Source(FillSource::Average), &mut router);
        assert_eq!(presenter.source(), FillSource::Average);
    }

    #[test]
    fn confirming_writes_the_month_only_amounts_raises_the_toast_then_closes() {
        let (ports, mut presenter) = fill();
        let mut router = RecordingRouter::default();
        assert!(
            presenter.view_model().frame.can_confirm,
            "the sample has a diff"
        );

        presenter.confirm(&mut router);

        assert_eq!(
            router.routed,
            [
                Routed::Notice(Notice::Toast(ToastKind::Success, "Filled Jul 2026".into())),
                Routed::Closed(None),
            ]
        );
        let budget = ports
            .borrow()
            .budgets
            .budget(PERSONAL_SPENDING_ID)
            .expect("the sample's default");
        assert!(applied(budget.chain(GROCERIES), next()).is_some());
    }

    #[test]
    fn a_fill_that_would_change_nothing_cannot_confirm() {
        let (_, mut presenter) = fill();
        let mut router = RecordingRouter::default();
        presenter.confirm(&mut router);
        router.routed.clear();

        assert!(!presenter.view_model().frame.can_confirm, "already filled");
        assert_eq!(
            presenter.view_model().no_changes.as_deref(),
            Some("Nothing would change.")
        );
        presenter.confirm(&mut router);
        assert_eq!(
            router.routed,
            [],
            "a Fill with nothing to write does nothing"
        );
    }

    #[test]
    fn it_does_not_open_on_an_archived_budget() {
        let (ports, shared) = sample();
        ports
            .borrow_mut()
            .budgets
            .budgets
            .iter_mut()
            .for_each(|budget| {
                budget.archived_at = Some(today());
            });

        assert!(
            FillPresenter::new(&shared, Clock::fixed(today()), PERSONAL_SPENDING_ID, next())
                .is_none()
        );
    }

    #[test]
    fn the_arrows_and_j_k_pick_the_source_and_select_and_submit_fill() {
        let (_, mut presenter) = fill();
        let mut router = RecordingRouter::default();
        let send = |presenter: &mut FillPresenter, input| {
            presenter.handle(FillEvent::Input(input), &mut RecordingRouter::default());
        };

        send(&mut presenter, DialogInput::SelectNext);
        assert_eq!(presenter.source(), FillSource::Average);
        send(&mut presenter, DialogInput::SelectPrevious);
        assert_eq!(presenter.source(), FillSource::PreviousMonth);
        send(&mut presenter, DialogInput::SelectNext);
        send(&mut presenter, DialogInput::SelectPrevious);
        assert_eq!(presenter.source(), FillSource::PreviousMonth);
        send(&mut presenter, DialogInput::Char('x'));
        assert_eq!(presenter.source(), FillSource::PreviousMonth);

        presenter.handle(FillEvent::Input(DialogInput::Select), &mut router);
        assert!(
            router.routed.contains(&Routed::Closed(None)),
            "{:?}",
            router.routed
        );
    }

    #[test]
    fn submit_fills_like_select_and_a_fill_that_changes_nothing_does_nothing() {
        let (_, mut presenter) = fill();
        let mut router = RecordingRouter::default();
        presenter.handle(FillEvent::Input(DialogInput::Submit), &mut router);
        assert!(router.routed.contains(&Routed::Closed(None)));

        let (_, shared) = sample();
        let mut empty = FillPresenter::new(
            &shared,
            Clock::fixed(today()),
            PERSONAL_SPENDING_ID,
            Period::of(today()).prev().prev().prev().prev(),
        )
        .expect("the default takes edits");
        let mut router = RecordingRouter::default();
        if !empty.view_model().frame.can_confirm {
            empty.handle(FillEvent::Input(DialogInput::Select), &mut router);
            assert!(router.routed.is_empty());
        }
    }
}
