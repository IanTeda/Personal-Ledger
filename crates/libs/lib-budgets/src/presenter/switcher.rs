//! The Switcher popover (11b): the Budgets, active ones in creation order then archived ones,
//! each with its health line, narrowed by a search field, with `+ New budget` and
//! `Manage budgets…` in the footer. The list takes the movement keys until `/` hands the keys to
//! the search field; the first `Esc` gives them back.
//!
//! The Switcher closes itself through its Router, handing the page the Budget chosen or the
//! Dialog to open next ([`BudgetsOutcome`]).

use lib_core::{BudgetId, Clock, DateStyle};
use lib_locale::format::format_date;
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::Hint;

use super::{method_label, money};
use crate::protocols::{BudgetsOutcome, SharedPorts};
use crate::{Budget, health, msg};

/// What the Switcher is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitcherEvent {
    /// A Dialog keymap command or a typed key: the list's own letters (`j`/`k`, `/`, `n`), or the
    /// search's text while it has the keys; `select` and `submit` open the highlighted Budget.
    Input(DialogInput),
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
    /// The status line's hints, in the Dialog keymap's chords.
    pub hints: Vec<Hint>,
}

/// One Budget, already worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitcherRow {
    pub id: BudgetId,
    pub name: String,
    pub is_default: bool,
    /// The Budget the page shows: its row carries the check.
    pub is_current: bool,
    /// The date an archived Budget was archived, in the Switcher's `DateStyle`; `None` while active.
    pub archived_on: Option<String>,
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
    /// The Dialog keymap's chords, which its hints show.
    keys: DialogKeys,
}

impl SwitcherPresenter {
    /// Opens with Budget `current` highlighted. `date_style` writes an archived Budget's date.
    pub fn new(
        ports: SharedPorts,
        current: BudgetId,
        clock: Clock,
        date_style: Option<DateStyle>,
        keys: DialogKeys,
    ) -> Self {
        let mut switcher = Self {
            ports,
            clock,
            date_style,
            current,
            query: String::new(),
            searching: false,
            selected: 0,
            keys,
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

    /// What `input` means: the movement keys and the list's letters, or the search's text while it
    /// has the keys.
    fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<BudgetsOutcome>) {
        let event = match input {
            DialogInput::SelectNext => SwitcherEvent::Down,
            DialogInput::SelectPrevious => SwitcherEvent::Up,
            DialogInput::Select | DialogInput::Submit => SwitcherEvent::Choose,
            DialogInput::Backspace if self.searching => SwitcherEvent::Backspace,
            DialogInput::Char(c) if self.searching => SwitcherEvent::Type(c),
            DialogInput::Char('/') => SwitcherEvent::Search,
            DialogInput::Char('n') => SwitcherEvent::New,
            _ => return,
        };
        self.handle(event, router);
    }

    /// A Budget's one-line summary: its health this month, or when it was archived.
    fn summary(&self, budget: &Budget, archived_on: Option<&str>) -> String {
        if let Some(date) = archived_on {
            return msg::budgets_switcher_archived_on(date);
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

    /// The status line's hints, in the Dialog keymap's chords it was handed at open.
    fn hints(&self) -> Vec<Hint> {
        let chord = |command: &str| self.keys.chord(command).to_string();
        vec![
            Hint::new(self.keys.list_step(), msg::budgets_hint_row()),
            Hint::new(chord(SELECT), msg::budgets_hint_open()),
            Hint::new("/", lib_locale::msg::hint_search()),
            Hint::new("n", msg::budgets_hint_new_budget()),
            Hint::new(chord(BACK), msg::budgets_hint_close()),
        ]
    }
}

impl Presenter for SwitcherPresenter {
    type Event = SwitcherEvent;
    type ViewModel = SwitcherViewModel;
    type Router<'r> = dyn DialogRouter<BudgetsOutcome> + 'r;

    fn handle(&mut self, event: SwitcherEvent, router: &mut Self::Router<'_>) {
        match event {
            SwitcherEvent::Input(input) => self.input(input, router),
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
                    router.close(Some(BudgetsOutcome::Show(budget.id)));
                }
            }
            SwitcherEvent::ChooseBudget(id) => {
                router.close(Some(BudgetsOutcome::Show(id)));
            }
            SwitcherEvent::New => router.close(Some(BudgetsOutcome::NewBudget)),
            SwitcherEvent::Manage => router.close(Some(BudgetsOutcome::Manage)),
            SwitcherEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> SwitcherViewModel {
        let rows: Vec<SwitcherRow> = self
            .matching()
            .iter()
            .map(|budget| {
                let archived_on = budget
                    .archived_at
                    .map(|date| format_date(date, self.date_style));
                SwitcherRow {
                    id: budget.id,
                    name: budget.name.clone(),
                    is_default: budget.is_default,
                    is_current: budget.id == self.current,
                    summary: self.summary(budget, archived_on.as_deref()),
                    archived_on,
                    method: method_label(budget.method),
                }
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
            archived_from: rows.iter().position(|row| row.archived_on.is_some()),
            none: rows.is_empty().then(msg::budgets_switcher_none),
            rows,
            archived_label: msg::budgets_switcher_archived(),
            default_label: msg::budgets_switcher_default(),
            new_label: msg::budgets_switcher_new(),
            manage_label: msg::budgets_switcher_manage(),
            hints: self.hints(),
        }
    }
}

impl DialogPresenter for SwitcherPresenter {
    type Outcome = BudgetsOutcome;

    fn input_event(input: DialogInput) -> Self::Event {
        SwitcherEvent::Input(input)
    }

    fn confirm_event() -> Self::Event {
        SwitcherEvent::Choose
    }

    fn cancel_event() -> Self::Event {
        SwitcherEvent::Cancel
    }

    fn can_confirm(&self) -> bool {
        true
    }

    /// The first `back` while searching gives the keys back to the list.
    fn close_open_select(&mut self) -> bool {
        std::mem::replace(&mut self.searching, false)
    }

    /// Whether the Dialog steps a list right now (not while the search has the keys), so the Dialog keymap's list step (`j`/`k` by
    /// default) reaches it as a select step rather than typing.
    fn steps_list(&self) -> bool {
        !self.searching
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PERSONAL_SPENDING_ID;
    use crate::fake::{RecordingRouter, Routed};
    use crate::presenter::test_support::{sample, today};

    fn switcher() -> (SwitcherPresenter, RecordingRouter) {
        let (_, shared) = sample();
        (
            SwitcherPresenter::new(
                shared,
                PERSONAL_SPENDING_ID,
                Clock::fixed(today()),
                None,
                DialogKeys::default(),
            ),
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
    fn an_archived_row_carries_its_date_in_the_switchers_style_and_an_active_one_has_none() {
        let (ports, shared) = sample();
        ports.borrow_mut().budgets.budgets[0].archived_at = Some(today());
        let archived_id = ports.borrow().budgets.budgets[0].id;
        let switcher = SwitcherPresenter::new(
            shared,
            archived_id,
            Clock::fixed(today()),
            Some(DateStyle::Medium),
            DialogKeys::default(),
        );

        let rows = switcher.view_model().rows;

        let expected = format_date(today(), Some(DateStyle::Medium));
        let archived = rows
            .iter()
            .find(|row| row.archived_on.is_some())
            .expect("an archived row");
        assert_eq!(archived.archived_on.as_deref(), Some(expected.as_str()));
        assert!(archived.summary.contains(&expected));
        assert!(
            rows.iter().any(|row| row.archived_on.is_none()),
            "an active row carries no date"
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

    fn input(switcher: &mut SwitcherPresenter, router: &mut RecordingRouter, input: DialogInput) {
        switcher.handle(SwitcherEvent::Input(input), router);
    }

    #[test]
    fn the_arrows_and_j_k_move_the_highlight_and_select_and_submit_open_it() {
        let (mut switcher, mut router) = switcher();
        let rows = switcher.view_model().rows;

        input(&mut switcher, &mut router, DialogInput::SelectNext);
        input(&mut switcher, &mut router, DialogInput::SelectNext);
        input(&mut switcher, &mut router, DialogInput::SelectPrevious);
        input(&mut switcher, &mut router, DialogInput::SelectPrevious);
        assert_eq!(switcher.view_model().selected, Some(0));

        input(&mut switcher, &mut router, DialogInput::SelectNext);
        input(&mut switcher, &mut router, DialogInput::Select);
        input(&mut switcher, &mut router, DialogInput::Submit);

        assert_eq!(
            router.routed,
            [
                Routed::Closed(Some(BudgetsOutcome::Show(rows[1].id))),
                Routed::Closed(Some(BudgetsOutcome::Show(rows[1].id))),
            ]
        );
    }

    #[test]
    fn slash_hands_the_keys_to_the_search_and_n_opens_a_new_budget_otherwise() {
        let (mut switcher, mut router) = switcher();
        input(&mut switcher, &mut router, DialogInput::Char('/'));
        assert!(switcher.searching());
        assert!(
            !switcher.steps_list(),
            "the list step types into the search"
        );
        input(&mut switcher, &mut router, DialogInput::Char('n'));
        input(&mut switcher, &mut router, DialogInput::Char('j'));
        assert_eq!(
            switcher.view_model().search,
            "nj",
            "the search takes the letters"
        );
        input(&mut switcher, &mut router, DialogInput::Backspace);
        assert_eq!(switcher.view_model().search, "n");
        assert!(router.routed.is_empty());

        switcher.handle(SwitcherEvent::LeaveSearch, &mut router);
        assert!(switcher.steps_list());
        input(&mut switcher, &mut router, DialogInput::Char('n'));
        assert_eq!(
            router.routed,
            [Routed::Closed(Some(BudgetsOutcome::NewBudget))]
        );
    }

    #[test]
    fn typing_and_backspace_do_nothing_while_the_list_has_the_keys() {
        let (mut switcher, mut router) = switcher();
        let before = switcher.view_model();

        input(&mut switcher, &mut router, DialogInput::Char('x'));
        input(&mut switcher, &mut router, DialogInput::Backspace);
        input(&mut switcher, &mut router, DialogInput::NextField);

        assert_eq!(switcher.view_model(), before);
        assert!(router.routed.is_empty());
    }
}
