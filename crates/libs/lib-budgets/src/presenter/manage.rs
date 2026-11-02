//! Manage budgets (11f): every Budget with its method, Accounts and scope, and the actions each
//! offers, over the header's counts and the three notes. An archived row offers only `open`,
//! `restore` and `duplicate`; the default offers neither `set default` nor `archive` (#400).
//!
//! Set default, Archive and Restore run here, through their Interactors, and Manage stays open on
//! the refreshed rows with any refusal shown. Open, Edit, Duplicate and New close it, handing the
//! page what to show or open next ([`BudgetsOutcome`]).

use lib_core::{BudgetId, Clock, Period};
use lib_locale::Segment;
use lib_viper::keymap::{Keymap, SELECT};
use lib_viper::view::{Choice, Hint, count};

use super::{error_text, method_label};
use crate::interactor::{ArchiveBudget, DuplicateBudget, RestoreBudget, SetDefaultBudget};
use crate::protocols::{BudgetsOutcome, BudgetsRouter, ManageAction, SharedPorts};
use crate::{Budget, Error, applied, msg};

/// What Manage budgets is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageEvent {
    /// The row cursor, a row down or up.
    Down,
    Up,
    /// A click on row `index`.
    Select(usize),
    /// A key's action on the cursor's row: `enter` opens, `e` edits, `d` duplicates, `*` sets the
    /// default and `x` archives, or restores an archived row.
    Act(ManageAction),
    /// A click on one of a row's actions.
    ActOn(BudgetId, ManageAction),
    /// `n` and `+ New budget`.
    New,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManageViewModel {
    pub title: String,
    /// `4 active · 1 archived · opens to Household`, the default's name strong.
    pub counts: Vec<Segment>,
    pub columns: ManageColumns,
    pub rows: Vec<ManageRow>,
    pub selected: usize,
    /// The last refused action, shown until the next one.
    pub error: Option<String>,
    /// Each note's title and text.
    pub notes: Vec<(String, String)>,
    pub new_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManageColumns {
    pub budget: String,
    pub method: String,
    pub period: String,
    pub accounts: String,
    pub scope: String,
    pub actions: String,
}

/// One Budget, already worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManageRow {
    pub id: BudgetId,
    pub name: String,
    pub method: String,
    pub period: String,
    pub accounts: String,
    pub scope: String,
    /// `default` on the default Budget.
    pub default: Option<String>,
    pub archived: bool,
    /// The actions the row offers, in the order drawn.
    pub actions: Vec<Choice<ManageAction>>,
}

/// The actions a row offers, in the order drawn.
pub fn actions(is_default: bool, archived: bool) -> Vec<ManageAction> {
    if archived {
        return vec![
            ManageAction::Open,
            ManageAction::Restore,
            ManageAction::Duplicate,
        ];
    }
    let mut all = vec![
        ManageAction::Open,
        ManageAction::Edit,
        ManageAction::Duplicate,
    ];
    if !is_default {
        all.push(ManageAction::SetDefault);
        all.push(ManageAction::Archive);
    }
    all
}

fn action_label(action: ManageAction) -> String {
    match action {
        ManageAction::Open => msg::budgets_manage_action_open(),
        ManageAction::Edit => msg::budgets_manage_action_edit(),
        ManageAction::Duplicate => msg::budgets_manage_action_duplicate(),
        ManageAction::SetDefault => msg::budgets_manage_action_default(),
        ManageAction::Archive => msg::budgets_manage_action_archive(),
        ManageAction::Restore => msg::budgets_manage_action_restore(),
    }
}

pub struct ManagePresenter {
    ports: SharedPorts,
    clock: Clock,
    selected: usize,
    error: Option<String>,
}

impl ManagePresenter {
    /// Opens with Budget `current` under the cursor.
    pub fn new(ports: SharedPorts, current: BudgetId, clock: Clock) -> Self {
        let mut manage = Self {
            ports,
            clock,
            selected: 0,
            error: None,
        };
        manage.follow(current);
        manage
    }

    /// Every Budget, active ones in creation order then archived ones.
    fn budgets(&self) -> Vec<Budget> {
        let budgets = self.ports.borrow().budgets().budgets();
        let (archived, active): (Vec<Budget>, Vec<Budget>) =
            budgets.into_iter().partition(Budget::is_archived);
        active.into_iter().chain(archived).collect()
    }

    /// Puts the cursor on Budget `id` wherever it now sits: archiving or restoring moves a row
    /// between the two sections.
    fn follow(&mut self, id: BudgetId) {
        self.selected = self
            .budgets()
            .iter()
            .position(|budget| budget.id == id)
            .unwrap_or(0);
    }

    pub fn handle(&mut self, event: ManageEvent, router: &mut dyn BudgetsRouter) {
        let budgets = self.budgets();
        let last = budgets.len().saturating_sub(1);
        match event {
            ManageEvent::Down => self.selected = (self.selected + 1).min(last),
            ManageEvent::Up => self.selected = self.selected.min(last).saturating_sub(1),
            ManageEvent::Select(index) => self.selected = index.min(last),
            ManageEvent::Act(action) => {
                if let Some(budget) = budgets.get(self.selected.min(last)) {
                    let action = match action {
                        ManageAction::Archive if budget.is_archived() => ManageAction::Restore,
                        other => other,
                    };
                    self.act(budget, action, router);
                }
            }
            ManageEvent::ActOn(id, action) => {
                if let Some(budget) = budgets.iter().find(|budget| budget.id == id) {
                    self.act(budget, action, router);
                }
            }
            ManageEvent::New => router.close_dialog(Some(BudgetsOutcome::NewBudget)),
            ManageEvent::Cancel => router.close_dialog(None),
        }
    }

    fn act(&mut self, budget: &Budget, action: ManageAction, router: &mut dyn BudgetsRouter) {
        let id = budget.id;
        let done = match action {
            ManageAction::Open => {
                router.close_dialog(Some(BudgetsOutcome::Show(id)));
                return;
            }
            ManageAction::Edit if budget.is_archived() => Err(Error::Archived),
            ManageAction::Edit => {
                router.close_dialog(Some(BudgetsOutcome::EditBudget(id)));
                return;
            }
            ManageAction::Duplicate => {
                let copy = self.with_repo(|repo| DuplicateBudget(id).run(repo));
                match copy {
                    Ok(copy) => {
                        router.close_dialog(Some(BudgetsOutcome::ShowAndEdit(copy)));
                        return;
                    }
                    Err(error) => Err(error),
                }
            }
            ManageAction::SetDefault => self.with_repo(|repo| SetDefaultBudget(id).run(repo)),
            ManageAction::Archive => self.with_repo(|repo| {
                ArchiveBudget {
                    id,
                    today: self.clock.today(),
                }
                .run(repo)
            }),
            ManageAction::Restore => self.with_repo(|repo| RestoreBudget(id).run(repo)),
        };
        self.follow(id);
        self.error = done.err().as_ref().map(error_text);
    }

    /// Runs `write` on the repository, borrowed for one Interactor.
    fn with_repo<T>(
        &self,
        write: impl FnOnce(&mut dyn crate::protocols::BudgetRepository) -> T,
    ) -> T {
        write(self.ports.borrow_mut().split_mut().budgets)
    }

    pub fn view_model(&self) -> ManageViewModel {
        let budgets = self.budgets();
        let current = Period::of(self.clock.today());
        let active = budgets
            .iter()
            .filter(|budget| !budget.is_archived())
            .count();
        let default_name = budgets
            .iter()
            .find(|budget| budget.is_default)
            .map(|budget| budget.name.clone())
            .unwrap_or_default();
        let rows = budgets
            .iter()
            .map(|budget| ManageRow {
                id: budget.id,
                name: budget.name.clone(),
                method: method_label(budget.method),
                period: msg::budgets_manage_period_monthly(),
                accounts: msg::budgets_manage_accounts(count(budget.account_ids.len())),
                scope: msg::budgets_manage_scope(count(
                    budget
                        .category_ids()
                        .filter(|id| applied(budget.chain(*id), current).is_some())
                        .count(),
                )),
                default: budget.is_default.then(msg::budgets_switcher_default),
                archived: budget.is_archived(),
                actions: actions(budget.is_default, budget.is_archived())
                    .into_iter()
                    .map(|action| Choice {
                        value: action,
                        label: action_label(action),
                    })
                    .collect(),
            })
            .collect();
        ManageViewModel {
            title: msg::budgets_manage_title(),
            counts: msg::budgets_manage_counts(
                &active.to_string(),
                &(budgets.len() - active).to_string(),
                &default_name,
            ),
            columns: ManageColumns {
                budget: msg::budgets_manage_column_budget(),
                method: msg::budgets_manage_column_method(),
                period: msg::budgets_manage_column_period(),
                accounts: msg::budgets_manage_column_accounts(),
                scope: msg::budgets_manage_column_scope(),
                actions: msg::budgets_manage_column_actions(),
            },
            rows,
            selected: self.selected.min(budgets.len().saturating_sub(1)),
            error: self.error.clone(),
            notes: vec![
                (
                    msg::budgets_manage_note_views_title(),
                    msg::budgets_manage_note_views(),
                ),
                (
                    msg::budgets_manage_note_switching_title(),
                    msg::budgets_manage_note_switching(),
                ),
                (
                    msg::budgets_manage_note_archive_title(),
                    msg::budgets_manage_note_archive(),
                ),
            ],
            new_label: msg::budgets_switcher_new(),
        }
    }

    /// The status line's legend, `select` read from the Dialog `keymap` the Client looks keys up
    /// in. The row actions' letters are Manage's own, not a keymap's.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        vec![
            Hint::new("j/k", msg::budgets_hint_row()),
            Hint::new(
                keymap.chord(SELECT).unwrap_or_default(),
                msg::budgets_hint_open(),
            ),
            Hint::new("n", msg::budgets_hint_new_budget()),
            Hint::new("e", msg::budgets_hint_edit()),
            Hint::new("d", msg::budgets_hint_duplicate()),
            Hint::new("*", msg::budgets_hint_set_default()),
            Hint::new("x", msg::budgets_hint_archive()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PERSONAL_SPENDING_ID;
    use crate::fake::{RecordingRouter, Routed};
    use crate::presenter::test_support::{stub, today};

    fn manage() -> (
        std::rc::Rc<std::cell::RefCell<crate::fake::InMemoryPorts>>,
        ManagePresenter,
        RecordingRouter,
    ) {
        let (ports, shared) = stub();
        (
            ports,
            ManagePresenter::new(shared, PERSONAL_SPENDING_ID, Clock::fixed(today())),
            RecordingRouter::default(),
        )
    }

    #[test]
    fn the_default_offers_no_set_default_or_archive_and_an_archived_row_only_restores() {
        assert_eq!(
            actions(true, false),
            [
                ManageAction::Open,
                ManageAction::Edit,
                ManageAction::Duplicate
            ]
        );
        assert_eq!(
            actions(false, true),
            [
                ManageAction::Open,
                ManageAction::Restore,
                ManageAction::Duplicate
            ]
        );
    }

    #[test]
    fn archiving_the_default_stays_open_with_the_refusal_shown() {
        let (_, mut manage, mut router) = manage();

        manage.handle(ManageEvent::Act(ManageAction::Archive), &mut router);

        assert!(router.routed.is_empty(), "Manage stays open");
        assert_eq!(
            manage.view_model().error.as_deref(),
            Some("The default budget can't be archived. Set another default first.")
        );
    }

    #[test]
    fn archive_follows_the_row_into_the_archived_section_and_x_restores_it() {
        let (ports, mut manage, mut router) = manage();
        manage.handle(ManageEvent::Down, &mut router);
        let id = manage.view_model().rows[1].id;

        manage.handle(ManageEvent::Act(ManageAction::Archive), &mut router);
        let view = manage.view_model();
        assert_eq!(view.rows[view.selected].id, id);
        assert!(view.rows[view.selected].archived);
        assert_eq!(view.error, None);

        manage.handle(ManageEvent::Act(ManageAction::Archive), &mut router);
        let restored = ports
            .borrow()
            .budgets
            .budgets
            .iter()
            .find(|b| b.id == id)
            .cloned();
        assert!(restored.is_some_and(|budget| !budget.is_archived()));
    }

    #[test]
    fn open_edit_duplicate_and_new_close_it_naming_what_comes_next() {
        let (_, mut manage, mut router) = manage();

        manage.handle(ManageEvent::Act(ManageAction::Open), &mut router);
        manage.handle(ManageEvent::Act(ManageAction::Edit), &mut router);
        manage.handle(ManageEvent::New, &mut router);
        manage.handle(
            ManageEvent::ActOn(PERSONAL_SPENDING_ID, ManageAction::Duplicate),
            &mut router,
        );

        assert_eq!(
            router.routed[..3],
            [
                Routed::Closed(Some(BudgetsOutcome::Show(PERSONAL_SPENDING_ID))),
                Routed::Closed(Some(BudgetsOutcome::EditBudget(PERSONAL_SPENDING_ID))),
                Routed::Closed(Some(BudgetsOutcome::NewBudget)),
            ]
        );
        assert!(matches!(
            router.routed[3],
            Routed::Closed(Some(BudgetsOutcome::ShowAndEdit(copy))) if copy != PERSONAL_SPENDING_ID
        ));
    }

    #[test]
    fn the_counts_name_the_default() {
        let (_, manage, _) = manage();

        let counts: String = manage
            .view_model()
            .counts
            .into_iter()
            .map(|segment| segment.text)
            .collect();

        assert!(counts.contains("opens to Personal spending"), "{counts}");
    }
}
