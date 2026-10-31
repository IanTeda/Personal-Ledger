//! The Switcher popover (11b): the Budgets, active ones in creation order then archived ones,
//! each with its health line, narrowed by a search field, with `+ New budget` and
//! `Manage budgets…` in the footer. The list takes the movement keys until `/` hands the keys to
//! the search field; the first `Esc` gives them back.
//!
//! The Switcher closes itself through its Router, handing the page the Budget chosen or the
//! Dialog to open next ([`BudgetsOutcome`]).

use lib_core::{BudgetId, Clock, DateStyle};
use lib_locale::format::format_date;
use lib_viper::keymap::{BACK, Keymap, SELECT};
use lib_viper::view::Hint;

use super::{method_label, money};
use crate::protocols::{BudgetsOutcome, BudgetsRouter, SharedPorts};
use crate::{Budget, health, msg};

/// What the Switcher is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitcherEvent {
    /// The highlight, a row down or up.
    Down,
    Up,
    /// `/` and a click on the search field: the keys go to the search.
    Search,
    /// The first `Esc` while searching: the keys go back to the list.
    LeaveSearch,
    Type(char),
    Backspace,
    /// `Enter`: opens the highlighted Budget.
    Choose,
    /// A click on a row.
    ChooseBudget(BudgetId),
    /// `n` and `+ New budget`.
    New,
    /// `Manage budgets…`.
    Manage,
    /// `Esc` with the list focused, or a click outside the card.
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitcherViewModel {
    /// The search field's text, or its placeholder while empty.
    pub search: String,
    pub search_empty: bool,
    pub searching: bool,
    /// The rows matching the search, active first.
    pub rows: Vec<SwitcherRow>,
    pub selected: Option<usize>,
    /// The first archived row, which the `Archived` heading sits over.
    pub archived_from: Option<usize>,
    pub archived_label: String,
    /// What shows when no Budget matches.
    pub none: Option<String>,
    pub default_label: String,
    pub new_label: String,
    pub manage_label: String,
}

/// One Budget, already worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitcherRow {
    pub id: BudgetId,
    pub name: String,
    pub is_default: bool,
    /// The Budget the page shows: its row carries the check.
    pub is_current: bool,
    pub archived: bool,
    /// The health summary, or `archived <date>`.
    pub summary: String,
    pub method: String,
}

pub struct SwitcherPresenter {
    ports: SharedPorts,
    clock: Clock,
    date_style: Option<DateStyle>,
    /// The Budget the page shows.
    current: BudgetId,
    query: String,
    searching: bool,
    /// The highlight, a position in the rows matching the query.
    selected: usize,
}

impl SwitcherPresenter {
    /// Opens with Budget `current` highlighted. `date_style` writes an archived Budget's date.
    pub fn new(
        ports: SharedPorts,
        current: BudgetId,
        clock: Clock,
        date_style: Option<DateStyle>,
    ) -> Self {
        let mut switcher = Self {
            ports,
            clock,
            date_style,
            current,
            query: String::new(),
            searching: false,
            selected: 0,
        };
        switcher.selected = switcher
            .matching()
            .iter()
            .position(|budget| budget.id == current)
            .unwrap_or(0);
        switcher
    }

    /// Whether the search field has the keys.
    pub fn searching(&self) -> bool {
        self.searching
    }

    /// Every Budget whose name contains the query whatever its case, active first.
    fn matching(&self) -> Vec<Budget> {
        let query = self.query.trim().to_lowercase();
        let budgets = self.ports.borrow().budgets().budgets();
        let (archived, active): (Vec<Budget>, Vec<Budget>) =
            budgets.into_iter().partition(Budget::is_archived);
        active
            .into_iter()
            .chain(archived)
            .filter(|budget| query.is_empty() || budget.name.to_lowercase().contains(&query))
            .collect()
    }

    pub fn handle(&mut self, event: SwitcherEvent, router: &mut dyn BudgetsRouter) {
        match event {
            SwitcherEvent::Down => {
                let last = self.matching().len().saturating_sub(1);
                self.selected = (self.selected + 1).min(last);
            }
            SwitcherEvent::Up => {
                let last = self.matching().len().saturating_sub(1);
                self.selected = self.selected.min(last).saturating_sub(1);
            }
            SwitcherEvent::Search => self.searching = true,
            SwitcherEvent::LeaveSearch => self.searching = false,
            SwitcherEvent::Type(c) => {
                if self.searching && !c.is_control() {
                    self.query.push(c);
                    self.selected = 0;
                }
            }
            SwitcherEvent::Backspace => {
                if self.searching {
                    self.query.pop();
                    self.selected = 0;
                }
            }
            SwitcherEvent::Choose => {
                let matching = self.matching();
                let chosen = matching.get(self.selected.min(matching.len().saturating_sub(1)));
                if let Some(budget) = chosen {
                    router.close_dialog(Some(BudgetsOutcome::Show(budget.id)));
                }
            }
            SwitcherEvent::ChooseBudget(id) => {
                router.close_dialog(Some(BudgetsOutcome::Show(id)));
            }
            SwitcherEvent::New => router.close_dialog(Some(BudgetsOutcome::NewBudget)),
            SwitcherEvent::Manage => router.close_dialog(Some(BudgetsOutcome::Manage)),
            SwitcherEvent::Cancel => router.close_dialog(None),
        }
    }

    /// A Budget's one-line summary: its health this month, or when it was archived.
    fn summary(&self, budget: &Budget) -> String {
        if let Some(date) = budget.archived_at {
            return msg::budgets_switcher_archived_on(&format_date(date, self.date_style));
        }
        let health = {
            let ports = self.ports.borrow();
            health(budget, ports.reads(), self.clock.today())
        };
        let left = money(&health.left);
        if health.at_risk > 0 {
            msg::budgets_switcher_health_at_risk(
                &health.over.to_string(),
                &left,
                &health.at_risk.to_string(),
            )
        } else {
            msg::budgets_switcher_health(&health.over.to_string(), &left)
        }
    }

    pub fn view_model(&self) -> SwitcherViewModel {
        let rows: Vec<SwitcherRow> = self
            .matching()
            .iter()
            .map(|budget| SwitcherRow {
                id: budget.id,
                name: budget.name.clone(),
                is_default: budget.is_default,
                is_current: budget.id == self.current,
                archived: budget.is_archived(),
                summary: self.summary(budget),
                method: method_label(budget.method),
            })
            .collect();
        SwitcherViewModel {
            search: if self.query.is_empty() {
                msg::budgets_switcher_search()
            } else {
                self.query.clone()
            },
            search_empty: self.query.is_empty(),
            searching: self.searching,
            selected: (!rows.is_empty()).then(|| self.selected.min(rows.len() - 1)),
            archived_from: rows.iter().position(|row| row.archived),
            none: rows.is_empty().then(msg::budgets_switcher_none),
            rows,
            archived_label: msg::budgets_switcher_archived(),
            default_label: msg::budgets_switcher_default(),
            new_label: msg::budgets_switcher_new(),
            manage_label: msg::budgets_switcher_manage(),
        }
    }

    /// The status line's legend, `select` and `back` read from the Dialog `keymap` the Client
    /// looks keys up in. The list's own letters are the Switcher's, not a keymap's.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        let chord = |command: &str| keymap.chord(command).unwrap_or_default().to_string();
        vec![
            Hint::new("j/k", msg::budgets_hint_row()),
            Hint::new(chord(SELECT), msg::budgets_hint_open()),
            Hint::new("/", lib_locale::msg::hint_search()),
            Hint::new("n", msg::budgets_hint_new_budget()),
            Hint::new(chord(BACK), msg::budgets_hint_close()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PERSONAL_SPENDING_ID;
    use crate::fake::{RecordingRouter, Routed};
    use crate::presenter::test_support::{stub, today};

    fn switcher() -> (SwitcherPresenter, RecordingRouter) {
        let (_, shared) = stub();
        (
            SwitcherPresenter::new(shared, PERSONAL_SPENDING_ID, Clock::fixed(today()), None),
            RecordingRouter::default(),
        )
    }

    #[test]
    fn it_opens_on_the_budget_shown_and_enter_shows_the_one_highlighted() {
        let (mut switcher, mut router) = switcher();
        let view = switcher.view_model();
        assert_eq!(view.selected, Some(0));
        assert!(view.rows[0].is_current);

        switcher.handle(SwitcherEvent::Down, &mut router);
        switcher.handle(SwitcherEvent::Choose, &mut router);

        assert_eq!(
            router.routed,
            [Routed::Closed(Some(BudgetsOutcome::Show(view.rows[1].id)))]
        );
    }

    #[test]
    fn typing_narrows_the_list_only_while_searching() {
        let (mut switcher, mut router) = switcher();
        let all = switcher.view_model().rows.len();

        switcher.handle(SwitcherEvent::Type('z'), &mut router);
        assert_eq!(
            switcher.view_model().rows.len(),
            all,
            "the list has the keys"
        );

        switcher.handle(SwitcherEvent::Search, &mut router);
        for c in "pers".chars() {
            switcher.handle(SwitcherEvent::Type(c), &mut router);
        }
        let view = switcher.view_model();
        assert!(view.searching);
        assert_eq!(view.search, "pers");
        assert!(
            view.rows
                .iter()
                .all(|row| row.name.to_lowercase().contains("pers"))
        );

        for c in "qqq".chars() {
            switcher.handle(SwitcherEvent::Type(c), &mut router);
        }
        assert_eq!(
            switcher.view_model().none.as_deref(),
            Some("No budget matches.")
        );
        switcher.handle(SwitcherEvent::LeaveSearch, &mut router);
        assert!(!switcher.searching());
    }

    #[test]
    fn new_and_manage_close_it_naming_what_opens_next() {
        let (mut switcher, mut router) = switcher();

        switcher.handle(SwitcherEvent::New, &mut router);
        switcher.handle(SwitcherEvent::Manage, &mut router);
        switcher.handle(SwitcherEvent::Cancel, &mut router);

        assert_eq!(
            router.routed,
            [
                Routed::Closed(Some(BudgetsOutcome::NewBudget)),
                Routed::Closed(Some(BudgetsOutcome::Manage)),
                Routed::Closed(None),
            ]
        );
    }
}
