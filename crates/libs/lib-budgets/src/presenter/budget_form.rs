//! New budget (11c) and its edit mode. The rules are the Desktop Budgets Surface map's settled
//! decision (#400):
//!
//! - All four methods are drawn but only Category limits can be chosen, so the Method grid is
//!   never a focus stop.
//! - **Accounts on budget** offers the Accounts that take Transactions in the chosen Unit, all
//!   preselected; changing the Unit re-selects them.
//! - **Start from** is Empty, Copy categories from the Budget on show (offered while that Budget
//!   is in the chosen Unit) or Last 3 months' spending. Each writes Onward from the current month.
//! - Edit mode locks Unit and Method and has no Start from: it is the one place to rename a
//!   Budget or change its Accounts.
//!
//! A confirm raises its own Toast and closes; a create hands the page the new Budget to show.

use lib_core::{AccountId, BudgetId, Clock, UnitId};
use lib_toast::{Notice, ToastKind};
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};
use lib_viper::view::{Choice, DialogFrame, Hint, SelectView, TextFieldView};

use super::dialog_error_text;
use crate::interactor::{CreateBudget, EditBudget};
use crate::protocols::{BudgetAccounts, BudgetsOutcome, BudgetsRouter, SharedPorts};
use crate::{Budget, NewBudget, StartFrom, msg};

/// The form's focus stops, in `Tab` order. Edit mode skips Unit and Start from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetField {
    Name,
    Unit,
    Accounts,
    StartFrom,
}

/// Start from's three cells, in the control's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StartChoice {
    #[default]
    Empty,
    Copy,
    LastThreeMonths,
}

const START_CHOICES: [StartChoice; 3] = [
    StartChoice::Empty,
    StartChoice::Copy,
    StartChoice::LastThreeMonths,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetFormEvent {
    /// The name's whole new text, as its View's input holds it.
    Name(String),
    Unit(UnitId),
    /// A click on an Account chip.
    ToggleAccount(AccountId),
    /// `space` on the Accounts: toggles the chip under the cursor.
    ToggleCursorAccount,
    /// A click on a Start from cell; Copy is ignored while it doesn't apply.
    StartFrom(StartChoice),
    /// A click on a field.
    Focus(BudgetField),
    /// `Tab`/`Shift-Tab`, wrapping.
    NextField,
    PrevField,
    /// `left`/`right` on the Account chips or Start from, stopping at either end.
    StepBack,
    StepForward,
    Cancel,
}

/// A method card: its title, tag and description, and whether it can be chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodCard {
    pub title: String,
    pub tag: String,
    pub detail: String,
    pub enabled: bool,
}

/// An Account chip: solid with a `✓` while selected, the accent border on the keyboard cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountChip {
    pub id: AccountId,
    pub name: String,
    pub selected: bool,
    pub cursor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetFormViewModel {
    pub frame: DialogFrame,
    pub name: TextFieldView,
    /// The Unit's codes, to choose from on create.
    pub unit: SelectView<UnitId>,
    /// Edit's read-only Unit code, in place of the select.
    pub unit_fixed: Option<String>,
    pub method_label: String,
    /// The four methods, in two rows of two.
    pub methods: Vec<MethodCard>,
    pub method_later: String,
    pub accounts_label: String,
    pub accounts: Vec<AccountChip>,
    pub accounts_focused: bool,
    pub accounts_help: String,
    /// Start from's cells, on create only.
    pub start: Option<SelectView<StartChoice>>,
    pub start_help: String,
    /// A refused save, shown until the next change.
    pub error: Option<String>,
    pub editing: bool,
    pub focused: BudgetField,
}

/// An Account that takes Transactions, as the form needs it.
#[derive(Debug, Clone)]
struct Candidate {
    id: AccountId,
    name: String,
    unit: UnitId,
}

pub struct BudgetFormPresenter {
    ports: SharedPorts,
    clock: Clock,
    /// The Budget being edited; `None` while creating one.
    editing: Option<BudgetId>,
    /// The Budget the title (edit) or Copy (create) names.
    source_name: String,
    name: String,
    unit: Option<UnitId>,
    /// The on-budget Accounts, in no particular order.
    account_ids: Vec<AccountId>,
    /// The keyboard cursor among the Account chips.
    account_cursor: usize,
    start_from: StartChoice,
    /// The Budget "Copy categories from" names, with its Unit.
    copy_source: Option<(BudgetId, UnitId)>,
    /// The Accounts that take Transactions, copied in as the Dialog opens.
    candidates: Vec<Candidate>,
    /// Every Unit some such Account is kept in, by code.
    units: Vec<(UnitId, String)>,
    focused: BudgetField,
    error: Option<String>,
}

impl BudgetFormPresenter {
    /// A new Budget in the Unit of Budget `current` (or the first Unit on offer), every Account
    /// preselected; `current` is the Budget Copy categories names.
    pub fn new_budget(ports: &SharedPorts, clock: Clock, current: BudgetId) -> Self {
        let source = ports.borrow().budgets().budget(current);
        let (candidates, units) = Self::offers(ports);
        let unit = source
            .as_ref()
            .map(|budget| budget.unit)
            .filter(|unit| units.iter().any(|(id, _)| id == unit))
            .or_else(|| units.first().map(|(id, _)| *id));
        let mut form = Self {
            ports: ports.clone(),
            clock,
            editing: None,
            source_name: source
                .as_ref()
                .map(|budget| budget.name.clone())
                .unwrap_or_default(),
            name: String::new(),
            unit,
            account_ids: Vec::new(),
            account_cursor: 0,
            start_from: StartChoice::Empty,
            copy_source: source.map(|budget| (budget.id, budget.unit)),
            candidates,
            units,
            focused: BudgetField::Name,
            error: None,
        };
        form.account_ids = form.offered_ids();
        form
    }

    /// Edit mode on Budget `id`, prefilled from it, or `None` when it is gone or archived.
    pub fn edit(ports: &SharedPorts, clock: Clock, id: BudgetId) -> Option<Self> {
        let budget = ports
            .borrow()
            .budgets()
            .budget(id)
            .filter(|budget| !budget.is_archived())?;
        let (candidates, units) = Self::offers(ports);
        Some(Self::editing_from(ports, clock, &budget, candidates, units))
    }

    fn editing_from(
        ports: &SharedPorts,
        clock: Clock,
        budget: &Budget,
        candidates: Vec<Candidate>,
        units: Vec<(UnitId, String)>,
    ) -> Self {
        Self {
            ports: ports.clone(),
            clock,
            editing: Some(budget.id),
            source_name: budget.name.clone(),
            name: budget.name.clone(),
            unit: Some(budget.unit),
            account_ids: budget.account_ids.clone(),
            account_cursor: 0,
            start_from: StartChoice::Empty,
            copy_source: None,
            candidates,
            units,
            focused: BudgetField::Name,
            error: None,
        }
    }

    /// The Accounts that take Transactions and the Units they are kept in, by code.
    fn offers(ports: &SharedPorts) -> (Vec<Candidate>, Vec<(UnitId, String)>) {
        let ports = ports.borrow();
        let reads = ports.reads();
        let candidates: Vec<Candidate> = reads
            .budget_accounts()
            .into_iter()
            .filter(|account| account.takes_transactions)
            .map(|account| Candidate {
                id: account.id,
                name: account.name,
                unit: account.unit,
            })
            .collect();
        let mut units: Vec<(UnitId, String)> = Vec::new();
        for candidate in &candidates {
            if !units.iter().any(|(id, _)| *id == candidate.unit) {
                units.push((
                    candidate.unit,
                    reads.unit_code(candidate.unit).unwrap_or_default(),
                ));
            }
        }
        units.sort_by(|a, b| a.1.cmp(&b.1));
        (candidates, units)
    }

    fn is_edit(&self) -> bool {
        self.editing.is_some()
    }

    /// The Accounts taking Transactions in the form's Unit.
    fn offered(&self) -> Vec<&Candidate> {
        self.candidates
            .iter()
            .filter(|candidate| Some(candidate.unit) == self.unit)
            .collect()
    }

    fn offered_ids(&self) -> Vec<AccountId> {
        self.offered()
            .iter()
            .map(|candidate| candidate.id)
            .collect()
    }

    /// Whether Copy can be chosen: there is a source Budget and it is in the form's Unit.
    fn can_copy(&self) -> bool {
        self.copy_source
            .is_some_and(|(_, unit)| Some(unit) == self.unit)
    }

    fn stops(&self) -> Vec<BudgetField> {
        if self.is_edit() {
            vec![BudgetField::Name, BudgetField::Accounts]
        } else {
            vec![
                BudgetField::Name,
                BudgetField::Unit,
                BudgetField::Accounts,
                BudgetField::StartFrom,
            ]
        }
    }

    pub fn focused(&self) -> BudgetField {
        self.focused
    }

    pub fn handle(&mut self, event: BudgetFormEvent, router: &mut dyn BudgetsRouter) {
        match event {
            BudgetFormEvent::Name(text) => {
                self.name = text;
                self.error = None;
            }
            BudgetFormEvent::Unit(unit) => {
                if !self.is_edit() && self.units.iter().any(|(id, _)| *id == unit) {
                    self.unit = Some(unit);
                    self.error = None;
                    self.sync_unit();
                }
            }
            BudgetFormEvent::ToggleAccount(id) => self.toggle_account(id),
            BudgetFormEvent::ToggleCursorAccount => {
                if let Some(candidate) = self.offered().get(self.account_cursor) {
                    let id = candidate.id;
                    self.toggle_account(id);
                }
            }
            BudgetFormEvent::StartFrom(choice) => {
                if self.is_edit() || (choice == StartChoice::Copy && !self.can_copy()) {
                    return;
                }
                self.focused = BudgetField::StartFrom;
                self.start_from = choice;
                self.error = None;
            }
            BudgetFormEvent::Focus(field) => {
                if self.stops().contains(&field) {
                    self.focused = field;
                }
            }
            BudgetFormEvent::NextField => self.cycle(false),
            BudgetFormEvent::PrevField => self.cycle(true),
            BudgetFormEvent::StepBack => self.step(false),
            BudgetFormEvent::StepForward => self.step(true),
            BudgetFormEvent::Cancel => router.close_dialog(None),
        }
    }

    /// `Tab` / `Shift+Tab`, wrapping.
    fn cycle(&mut self, back: bool) {
        let stops = self.stops();
        let at = stops.iter().position(|f| *f == self.focused).unwrap_or(0);
        let next = if back {
            (at + stops.len() - 1) % stops.len()
        } else {
            (at + 1) % stops.len()
        };
        self.focused = stops[next];
    }

    /// After a Unit change: when the selection holds an Account outside the Unit's list, every
    /// Account in the Unit is selected again, and Copy falls back to Empty once it no longer
    /// applies.
    fn sync_unit(&mut self) {
        let offered = self.offered_ids();
        if self.account_ids.iter().any(|id| !offered.contains(id)) {
            self.account_ids = offered;
            self.account_cursor = 0;
        }
        if self.start_from == StartChoice::Copy && !self.can_copy() {
            self.start_from = StartChoice::Empty;
        }
    }

    fn step(&mut self, forward: bool) {
        match self.focused {
            BudgetField::Accounts => {
                let last = self.offered().len().saturating_sub(1);
                self.account_cursor = if forward {
                    (self.account_cursor + 1).min(last)
                } else {
                    self.account_cursor.min(last).saturating_sub(1)
                };
            }
            BudgetField::StartFrom => {
                if self.is_edit() {
                    return;
                }
                let choices = self.start_choices();
                let at = choices
                    .iter()
                    .position(|choice| *choice == self.start_from)
                    .unwrap_or(0);
                let next = if forward {
                    (at + 1).min(choices.len() - 1)
                } else {
                    at.saturating_sub(1)
                };
                self.start_from = choices[next];
                self.error = None;
            }
            _ => {}
        }
    }

    /// The Start from cells on offer: Copy only while it applies.
    fn start_choices(&self) -> Vec<StartChoice> {
        START_CHOICES
            .into_iter()
            .filter(|choice| *choice != StartChoice::Copy || self.can_copy())
            .collect()
    }

    fn toggle_account(&mut self, id: AccountId) {
        self.focused = BudgetField::Accounts;
        if let Some(at) = self.offered().iter().position(|each| each.id == id) {
            self.account_cursor = at;
        }
        if let Some(at) = self.account_ids.iter().position(|each| *each == id) {
            self.account_ids.remove(at);
        } else {
            self.account_ids.push(id);
        }
        self.error = None;
    }

    /// A Budget needs a name, a Unit and at least one on-budget Account.
    fn can_confirm(&self) -> bool {
        !self.name.trim().is_empty() && self.unit.is_some() && !self.account_ids.is_empty()
    }

    /// Saves the form through the Interactors, raises the Toast and closes the Dialog; a refusal
    /// stays on the form. A form that can't confirm does nothing.
    pub fn confirm(&mut self, router: &mut dyn BudgetsRouter) {
        if !self.can_confirm() {
            return;
        }
        let name = self.name.trim().to_string();
        let today = self.clock.today();
        let done = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            match (self.editing, self.unit) {
                (Some(id), _) => {
                    let accounts: &dyn BudgetAccounts = ports.reads;
                    EditBudget {
                        id,
                        name: name.clone(),
                        account_ids: self.account_ids.clone(),
                    }
                    .run(accounts, ports.budgets)
                    .map(|()| (msg::budgets_toast_saved(&name), None))
                }
                (None, Some(unit)) => CreateBudget {
                    new: NewBudget {
                        name: name.clone(),
                        unit,
                        account_ids: self.account_ids.clone(),
                        start_from: match (self.start_from, self.copy_source) {
                            (StartChoice::Copy, Some((source, _))) if self.can_copy() => {
                                StartFrom::Copy(source)
                            }
                            (StartChoice::LastThreeMonths, _) => StartFrom::LastThreeMonths,
                            _ => StartFrom::Empty,
                        },
                    },
                    today,
                }
                .run(ports.reads, ports.budgets)
                .map(|id| (msg::budgets_toast_created(&name), Some(id))),
                (None, None) => return,
            }
        };
        match done {
            Ok((toast, created)) => {
                router.notify(Notice::Toast(ToastKind::Success, toast));
                router.close_dialog(created.map(BudgetsOutcome::Show));
            }
            Err(refusal) => self.error = Some(dialog_error_text(&refusal)),
        }
    }

    pub fn view_model(&self) -> BudgetFormViewModel {
        let edit = self.is_edit();
        let (title, submit) = if edit {
            (
                msg::budgets_new_title_edit(&self.source_name),
                msg::budgets_new_submit_edit(),
            )
        } else {
            (msg::budgets_new_title(), msg::budgets_new_submit())
        };
        let offered = self.offered();
        let focused = self.focused;
        let unit_code = self
            .units
            .iter()
            .find(|(id, _)| Some(*id) == self.unit)
            .map(|(_, code)| code.clone())
            .or_else(|| {
                self.unit
                    .and_then(|unit| self.ports.borrow().reads().unit_code(unit))
            })
            .unwrap_or_default();
        let card = |title: String, tag: String, detail: String, enabled: bool| MethodCard {
            title,
            tag,
            detail,
            enabled,
        };
        BudgetFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.can_confirm(),
                destructive: false,
                hints: Vec::new(),
            },
            name: TextFieldView {
                label: msg::budgets_new_field_name(),
                text: self.name.clone(),
                placeholder: msg::budgets_new_name_placeholder(),
                focused: focused == BudgetField::Name,
            },
            unit: SelectView {
                label: lib_locale::msg::column_unit(),
                options: self
                    .units
                    .iter()
                    .map(|(id, code)| Choice {
                        value: *id,
                        label: code.clone(),
                    })
                    .collect(),
                chosen: self.unit,
                focused: focused == BudgetField::Unit,
            },
            // Locked after creation, like a Bill Plan's.
            unit_fixed: edit.then_some(unit_code),
            method_label: msg::budgets_new_field_method(),
            methods: vec![
                card(
                    msg::budgets_new_method_envelope(),
                    msg::budgets_new_method_envelope_tag(),
                    msg::budgets_new_method_envelope_detail(),
                    false,
                ),
                card(
                    msg::budgets_new_method_limits(),
                    msg::budgets_new_method_limits_tag(),
                    msg::budgets_new_method_limits_detail(),
                    true,
                ),
                card(
                    msg::budgets_new_method_split(),
                    msg::budgets_new_method_split_tag(),
                    msg::budgets_new_method_split_detail(),
                    false,
                ),
                card(
                    msg::budgets_new_method_project(),
                    msg::budgets_new_method_project_tag(),
                    msg::budgets_new_method_project_detail(),
                    false,
                ),
            ],
            method_later: msg::budgets_new_method_later(),
            accounts_label: msg::budgets_new_field_accounts(),
            accounts: offered
                .iter()
                .enumerate()
                .map(|(index, candidate)| AccountChip {
                    id: candidate.id,
                    name: candidate.name.clone(),
                    selected: self.account_ids.contains(&candidate.id),
                    cursor: focused == BudgetField::Accounts && index == self.account_cursor,
                })
                .collect(),
            accounts_focused: focused == BudgetField::Accounts,
            accounts_help: if offered.is_empty() {
                msg::budgets_new_accounts_none()
            } else {
                msg::budgets_new_accounts_help()
            },
            start: (!edit).then(|| SelectView {
                label: msg::budgets_new_field_start(),
                options: self
                    .start_choices()
                    .into_iter()
                    .map(|choice| Choice {
                        value: choice,
                        label: match choice {
                            StartChoice::Empty => msg::budgets_new_start_empty(),
                            StartChoice::Copy => msg::budgets_new_start_copy(&self.source_name),
                            StartChoice::LastThreeMonths => msg::budgets_new_start_average(),
                        },
                    })
                    .collect(),
                chosen: Some(self.start_from),
                focused: focused == BudgetField::StartFrom,
            }),
            start_help: msg::budgets_new_start_help(),
            error: self.error.clone(),
            editing: edit,
            focused,
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
    use crate::PERSONAL_SPENDING_ID;
    use crate::fake::{RecordingRouter, Routed};
    use crate::presenter::test_support::{stub, today};

    struct Form {
        ports: std::rc::Rc<std::cell::RefCell<crate::fake::InMemoryPorts>>,
        presenter: BudgetFormPresenter,
        router: RecordingRouter,
    }

    impl Form {
        fn new() -> Self {
            let (ports, shared) = stub();
            Self {
                presenter: BudgetFormPresenter::new_budget(
                    &shared,
                    Clock::fixed(today()),
                    PERSONAL_SPENDING_ID,
                ),
                ports,
                router: RecordingRouter::default(),
            }
        }

        fn edit() -> Self {
            let (ports, shared) = stub();
            Self {
                presenter: BudgetFormPresenter::edit(
                    &shared,
                    Clock::fixed(today()),
                    PERSONAL_SPENDING_ID,
                )
                .expect("the default takes edits"),
                ports,
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: BudgetFormEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        fn view(&self) -> BudgetFormViewModel {
            self.presenter.view_model()
        }

        fn budgets(&self) -> Vec<Budget> {
            self.ports.borrow().budgets.budgets.clone()
        }
    }

    #[test]
    fn a_new_form_takes_the_source_unit_and_preselects_its_accounts() {
        let form = Form::new();
        let view = form.view();

        assert_eq!(view.frame.title, "New budget");
        assert!(!view.accounts.is_empty());
        assert!(view.accounts.iter().all(|chip| chip.selected));
        assert!(!view.frame.can_confirm, "a Budget needs a name");
        assert!(view.unit.chosen.is_some());
        assert_eq!(view.methods.iter().filter(|card| card.enabled).count(), 1);
        let start = view.start.expect("a create has Start from");
        assert_eq!(
            start
                .options
                .iter()
                .map(|each| each.value)
                .collect::<Vec<_>>(),
            [
                StartChoice::Empty,
                StartChoice::Copy,
                StartChoice::LastThreeMonths
            ]
        );
    }

    #[test]
    fn the_form_confirms_with_a_name_a_unit_and_an_account() {
        let mut form = Form::new();
        form.send(BudgetFormEvent::Name("  ".into()));
        assert!(!form.view().frame.can_confirm);

        form.send(BudgetFormEvent::Name("Trip".into()));
        assert!(form.view().frame.can_confirm);

        let accounts: Vec<_> = form.view().accounts.iter().map(|chip| chip.id).collect();
        for id in &accounts {
            form.send(BudgetFormEvent::ToggleAccount(*id));
        }
        assert!(!form.view().frame.can_confirm, "no Account is on budget");
    }

    #[test]
    fn tab_and_the_arrows_walk_the_stops_and_the_chips() {
        let mut form = Form::new();
        let mut seen = vec![form.view().focused];
        for _ in 0..4 {
            form.send(BudgetFormEvent::NextField);
            seen.push(form.view().focused);
        }
        assert_eq!(
            seen,
            [
                BudgetField::Name,
                BudgetField::Unit,
                BudgetField::Accounts,
                BudgetField::StartFrom,
                BudgetField::Name,
            ]
        );

        form.send(BudgetFormEvent::Focus(BudgetField::StartFrom));
        form.send(BudgetFormEvent::StepForward);
        assert_eq!(
            form.view().start.expect("create").chosen,
            Some(StartChoice::Copy)
        );
        form.send(BudgetFormEvent::StepBack);
        form.send(BudgetFormEvent::StepBack);
        assert_eq!(
            form.view().start.expect("create").chosen,
            Some(StartChoice::Empty)
        );
    }

    #[test]
    fn space_toggles_the_chip_under_the_cursor() {
        let mut form = Form::new();
        form.send(BudgetFormEvent::Focus(BudgetField::Accounts));
        let before = form.view().accounts[0].selected;

        form.send(BudgetFormEvent::ToggleCursorAccount);

        assert_eq!(form.view().accounts[0].selected, !before);
        assert!(form.view().accounts[0].cursor);
    }

    #[test]
    fn creating_writes_the_budget_raises_the_toast_then_closes_on_it() {
        let mut form = Form::new();
        let stored = form.budgets().len();
        form.send(BudgetFormEvent::Name("Trip".into()));

        form.presenter.confirm(&mut form.router);

        let budgets = form.budgets();
        assert_eq!(budgets.len(), stored + 1);
        let created = budgets.last().expect("the new Budget");
        assert_eq!(created.name, "Trip");
        assert_eq!(
            form.router.routed,
            [
                Routed::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Created the budget Trip".into()
                )),
                Routed::Closed(Some(BudgetsOutcome::Show(created.id))),
            ]
        );
    }

    #[test]
    fn a_taken_name_is_refused_on_the_form_and_writes_nothing() {
        let mut form = Form::new();
        let taken = form.budgets()[0].name.clone();
        let stored = form.budgets();
        form.send(BudgetFormEvent::Name(taken));

        form.presenter.confirm(&mut form.router);

        assert_eq!(form.router.routed, []);
        assert_eq!(
            form.view().error.as_deref(),
            Some("Another budget already has that name.")
        );
        assert_eq!(form.budgets(), stored);
        form.send(BudgetFormEvent::Name("Fresh".into()));
        assert_eq!(form.view().error, None, "a change clears a stale error");
    }

    #[test]
    fn edit_mode_locks_the_unit_has_no_start_from_and_saves_the_rename_without_a_show() {
        let mut form = Form::edit();
        let view = form.view();
        assert!(view.editing);
        assert!(view.unit_fixed.is_some());
        assert!(view.start.is_none());
        assert_eq!(view.frame.submit, "Save");

        form.send(BudgetFormEvent::Name("Household".into()));
        form.presenter.confirm(&mut form.router);

        assert_eq!(
            form.router.routed,
            [
                Routed::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Saved the budget Household".into()
                )),
                Routed::Closed(None),
            ]
        );
        assert!(
            form.budgets()
                .iter()
                .any(|budget| budget.name == "Household")
        );
    }

    #[test]
    fn cancelling_closes_having_written_nothing() {
        let mut form = Form::new();
        let stored = form.budgets();

        form.send(BudgetFormEvent::Cancel);

        assert_eq!(form.router.routed, [Routed::Closed(None)]);
        assert_eq!(form.budgets(), stored);
    }
}
