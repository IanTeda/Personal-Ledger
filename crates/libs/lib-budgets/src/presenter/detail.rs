//! The Category detail (9d): four stats, the Progress row's bar with its caption, the largest
//! Transactions behind Spent, the track record and a footer that hands off to Transactions
//! already filtered.
//!
//! The Dialog writes nothing. `t` and `enter` hand off to Transactions and `e` to Edit budget, both
//! by closing with an outcome the page acts on, so the Dialog's Router never opens another.

use bigdecimal::{BigDecimal, Signed, ToPrimitive, Zero};
use lib_core::{BudgetId, CategoryId, CategoryTypes, Clock, Money, Period};
use lib_list_cursor::Step;
use lib_locale::format::format_month_day;
use lib_viper::keymap::{BACK, Keymap};
use lib_viper::view::{DialogFrame, Hint};

use super::{BarView, EMPTY_CELL, bar_view, count, money, period_label};
use crate::protocols::{BudgetsOutcome, BudgetsRouter, SharedPorts};
use crate::{CategoryDetail, Rollover, category_detail, msg};

/// The Transactions listed before the `+ N more` row.
pub const LISTED: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailEvent {
    /// `j`/`k`, a Transaction down or up.
    Down,
    Up,
    /// `t`, `enter` and `open in Transactions →`.
    Transactions,
    /// `e` and `Edit budget`; inert where the Category takes no edit.
    Edit,
    Close,
}

/// A Transaction row, worded for the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRow {
    pub date: String,
    pub payee: String,
    pub account: String,
    pub amount: String,
}

/// A figure with its label, and whether it is drawn as a negative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailStat {
    pub label: String,
    pub value: String,
    pub negative: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetailViewModel {
    pub frame: DialogFrame,
    pub stats: Vec<DetailStat>,
    pub bar: Option<BarView>,
    /// How far through the month it is, 0 to 1.
    pub elapsed: f32,
    pub caption: String,
    /// `3 transactions · largest first`.
    pub list_title: String,
    pub lines: Vec<LineRow>,
    /// The `+ N more · amount` row.
    pub more: Option<String>,
    /// The `j`/`k` cursor, into `lines`.
    pub selected: usize,
    pub bills: String,
    pub track: String,
    pub rollover_note: Option<String>,
    /// Whether **Edit budget** is offered.
    pub can_edit: bool,
    pub open_transactions: String,
    pub close: String,
    pub edit: String,
}

pub struct DetailPresenter {
    ports: SharedPorts,
    clock: Clock,
    budget: BudgetId,
    category: CategoryId,
    month: Period,
    selected: usize,
}

impl DetailPresenter {
    /// 9d for Category `category` (a leaf or a parent's rollup) of Budget `budget` in `month`, or
    /// `None` once the Budget or Category is gone.
    pub fn new(
        ports: &SharedPorts,
        clock: Clock,
        budget: BudgetId,
        category: CategoryId,
        month: Period,
    ) -> Option<Self> {
        let presenter = Self {
            ports: ports.clone(),
            clock,
            budget,
            category,
            month,
            selected: 0,
        };
        presenter.detail()?;
        Some(presenter)
    }

    fn detail(&self) -> Option<CategoryDetail> {
        let ports = self.ports.borrow();
        let budget = ports.budgets().budget(self.budget)?;
        category_detail(
            &budget,
            ports.reads(),
            self.category,
            self.month,
            self.clock.today(),
        )
    }

    /// The Transactions listed, which the cursor moves over.
    fn listed(&self) -> usize {
        self.detail()
            .map_or(0, |detail| detail.lines.len().min(LISTED))
    }

    /// Whether **Edit budget** is offered: a leaf Expense Category in a Budget that takes edits.
    fn can_edit(&self) -> bool {
        let ports = self.ports.borrow();
        let editable = ports
            .budgets()
            .budget(self.budget)
            .is_some_and(|budget| !budget.is_archived());
        editable
            && ports.reads().budget_categories().iter().any(|category| {
                category.id == self.category
                    && category.is_leaf
                    && category.category_type == CategoryTypes::Expense
            })
    }

    pub fn handle(&mut self, event: DetailEvent, router: &mut dyn BudgetsRouter) {
        match event {
            DetailEvent::Down => {
                self.selected = lib_list_cursor::step(self.selected, self.listed(), Step::Next);
            }
            DetailEvent::Up => {
                self.selected = lib_list_cursor::step(self.selected, self.listed(), Step::Prev);
            }
            DetailEvent::Transactions => router.close_dialog(Some(BudgetsOutcome::Transactions(
                self.category,
                self.month,
            ))),
            DetailEvent::Edit => {
                if self.can_edit() {
                    router.close_dialog(Some(BudgetsOutcome::Limit(self.category, self.month)));
                }
            }
            DetailEvent::Close => router.close_dialog(None),
        }
    }

    /// `enter` hands off to Transactions, as `t` does.
    pub fn confirm(&mut self, router: &mut dyn BudgetsRouter) {
        self.handle(DetailEvent::Transactions, router);
    }

    /// The share of the budget spent, to a whole percent; `None` while Unbudgeted or at zero.
    fn spent_percent(detail: &CategoryDetail) -> Option<i64> {
        let budget = detail.figures.budget.as_ref().filter(|b| !b.0.is_zero())?;
        let ratio: BigDecimal = detail.figures.spent.0.clone() * 100 / budget.0.clone();
        ratio.round(0).to_i64()
    }

    pub fn view_model(&self) -> Option<DetailViewModel> {
        let detail = self.detail()?;
        let ports = self.ports.borrow();
        let reads = ports.reads();
        let categories = reads.budget_categories();
        let accounts = reads.budget_accounts();
        let name = categories
            .iter()
            .find(|category| category.id == self.category)
            .map(|category| category.name.clone())
            .unwrap_or_default();
        let figures = &detail.figures;
        let cell = |amount: Option<&Money>| amount.map_or_else(|| EMPTY_CELL.to_string(), money);

        // OVER when Spent has passed the budget, otherwise what is LEFT.
        let last = if figures.over {
            let over_by = figures.budget.as_ref().map_or_else(
                || figures.spent.0.clone(),
                |budget| figures.spent.0.clone() - budget.0.clone(),
            );
            DetailStat {
                label: msg::budgets_detail_stat_over(),
                value: money(&Money(over_by)),
                negative: true,
            }
        } else {
            DetailStat {
                label: msg::budgets_detail_stat_left(),
                value: cell(figures.left.as_ref()),
                negative: figures
                    .left
                    .as_ref()
                    .is_some_and(|left| left.0.is_negative()),
            }
        };
        let stats = vec![
            DetailStat {
                label: msg::budgets_stat_budgeted(),
                value: cell(figures.budget.as_ref()),
                negative: false,
            },
            DetailStat {
                label: msg::budgets_stat_spent(),
                value: money(&figures.spent),
                negative: false,
            },
            DetailStat {
                label: msg::budgets_stat_known(),
                value: money(&figures.known),
                negative: false,
            },
            last,
        ];
        let elapsed = detail.elapsed.percent.to_string();
        let caption = match Self::spent_percent(&detail) {
            Some(spent) => msg::budgets_detail_caption(&spent.to_string(), &elapsed),
            None => msg::budgets_detail_caption_unbudgeted(&elapsed),
        };

        let lines = detail
            .lines
            .iter()
            .take(LISTED)
            .map(|line| LineRow {
                date: format_month_day(line.date),
                payee: line
                    .payee_id
                    .and_then(|id| reads.payee_name(id))
                    .unwrap_or_else(msg::budgets_detail_no_payee),
                account: accounts
                    .iter()
                    .find(|account| account.id == line.account_id)
                    .map(|account| account.name.clone())
                    .unwrap_or_default(),
                amount: money(&line.amount),
            })
            .collect();
        let hidden = detail
            .lines
            .iter()
            .skip(LISTED)
            .fold(BigDecimal::from(0), |sum, line| sum + line.amount.0.clone());
        let more = (detail.lines.len() > LISTED).then(|| {
            msg::budgets_detail_more(
                &(detail.lines.len() - LISTED).to_string(),
                &money(&Money(hidden)),
            )
        });
        let track = detail
            .track
            .as_ref()
            .map_or_else(msg::budgets_detail_track_none, |track| {
                msg::budgets_detail_track(
                    &track.over_months.to_string(),
                    i64::from(track.months),
                    &money(&track.average),
                )
            });
        let next = period_label(detail.month.next());
        let rollover_note = detail
            .figures
            .over
            .then_some(detail.rollover)
            .flatten()
            .map(|rollover| match rollover {
                Rollover::None => msg::budgets_detail_rollover_off(&next),
                Rollover::CarryUnspent => msg::budgets_detail_rollover_unspent(&next),
                Rollover::CarryBoth => msg::budgets_detail_rollover_both(&next),
            });
        let bills = match count(detail.bill_plans) {
            0 => msg::budgets_detail_no_bills(),
            plans => msg::budgets_detail_bills(plans),
        };
        let total = count(detail.lines.len());
        Some(DetailViewModel {
            frame: DialogFrame {
                title: msg::budgets_detail_title(&name, &period_label(detail.month)),
                submit: msg::budgets_detail_close(),
                can_confirm: true,
                destructive: false,
                hints: Vec::new(),
            },
            stats,
            bar: bar_view(figures),
            elapsed: detail.elapsed.percent as f32 / 100.0,
            caption,
            list_title: if total == 0 {
                msg::budgets_detail_transactions_none()
            } else {
                msg::budgets_detail_transactions(total)
            },
            lines,
            more,
            selected: self.selected,
            bills,
            track,
            rollover_note,
            can_edit: self.can_edit(),
            open_transactions: msg::budgets_detail_open_transactions(),
            close: msg::budgets_detail_close(),
            edit: msg::budgets_detail_edit(),
        })
    }

    /// The status line's legend, `back` read from the Dialog `keymap`.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        let mut hints = vec![
            Hint::new(
                keymap.chord(BACK).unwrap_or_default(),
                msg::budgets_hint_close(),
            ),
            Hint::new("j/k", msg::budgets_hint_row()),
        ];
        if self.can_edit() {
            hints.push(Hint::new("e", msg::budgets_hint_edit_budget()));
        }
        hints.push(Hint::new("t", msg::budgets_hint_transactions()));
        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PERSONAL_SPENDING_ID;
    use crate::fake::{RecordingRouter, Routed};
    use crate::presenter::test_support::{stub, today};

    const GROCERIES: CategoryId = CategoryId::seed(7);
    const FOOD: CategoryId = CategoryId::seed(6);

    fn current() -> Period {
        Period::of(today())
    }

    fn detail(category: CategoryId) -> Option<DetailPresenter> {
        let (_, shared) = stub();
        DetailPresenter::new(
            &shared,
            Clock::fixed(today()),
            PERSONAL_SPENDING_ID,
            category,
            current(),
        )
    }

    #[test]
    fn it_shows_four_stats_and_the_titled_transactions_of_a_leaf() {
        let view = detail(GROCERIES)
            .expect("budgeted")
            .view_model()
            .expect("detail");

        assert_eq!(view.frame.title, "Groceries — Jun 2026");
        assert_eq!(
            view.stats
                .iter()
                .map(|stat| stat.label.as_str())
                .collect::<Vec<_>>()[..3],
            ["Budgeted", "Spent", "Known costs"]
        );
        assert!(view.lines.len() <= LISTED);
        assert!(
            view.can_edit,
            "a leaf in an editable Budget offers Edit budget"
        );
    }

    #[test]
    fn a_parents_rollup_offers_no_edit_and_edit_is_inert() {
        let mut presenter = detail(FOOD).expect("a parent rolls up");
        let mut router = RecordingRouter::default();

        assert!(!presenter.view_model().expect("detail").can_edit);
        presenter.handle(DetailEvent::Edit, &mut router);

        assert_eq!(router.routed, []);
    }

    #[test]
    fn the_hand_offs_close_with_an_outcome_for_the_page() {
        let mut presenter = detail(GROCERIES).expect("budgeted");
        let mut router = RecordingRouter::default();

        presenter.handle(DetailEvent::Transactions, &mut router);
        presenter.handle(DetailEvent::Edit, &mut router);
        presenter.confirm(&mut router);
        presenter.handle(DetailEvent::Close, &mut router);

        assert_eq!(
            router.routed,
            [
                Routed::Closed(Some(BudgetsOutcome::Transactions(GROCERIES, current()))),
                Routed::Closed(Some(BudgetsOutcome::Limit(GROCERIES, current()))),
                Routed::Closed(Some(BudgetsOutcome::Transactions(GROCERIES, current()))),
                Routed::Closed(None),
            ]
        );
    }

    #[test]
    fn j_and_k_move_the_cursor_over_the_listed_transactions_and_stop_at_either_end() {
        let mut presenter = detail(GROCERIES).expect("budgeted");
        let listed = presenter.listed();
        let mut router = RecordingRouter::default();

        for _ in 0..listed + 2 {
            presenter.handle(DetailEvent::Down, &mut router);
        }
        assert_eq!(presenter.selected, listed.saturating_sub(1));
        for _ in 0..listed + 2 {
            presenter.handle(DetailEvent::Up, &mut router);
        }
        assert_eq!(presenter.selected, 0);
    }

    #[test]
    fn it_does_not_open_once_the_category_is_gone() {
        assert!(detail(CategoryId::seed(9999)).is_none());
    }
}
