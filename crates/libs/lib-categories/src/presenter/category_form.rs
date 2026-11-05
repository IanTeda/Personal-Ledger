//! The Add and Edit category Dialogs (`docs/ux/desktop-mockups/18-categories/README.md`'s 5b, 5c):
//! one form, Edit pre-filled from the Category.
//!
//! - The fields, in `Tab` order: Name, Parent category (none, then each Category it may go
//!   under), Type (Expense or Income) and Monthly budget.
//! - A child takes its parent's type, so the type is chosen only at top level, and on Edit only
//!   for a Category already at top level; a type chosen there cascades to its subtree on save.
//! - Add offers every Category above the depth cap as a parent; Edit leaves out the Category's own
//!   subtree and any parent that would nest it past three levels.
//! - Uncategorised is fixed: Add never offers it as a parent, and its Edit opens with Name, Parent
//!   and Type read-only and only the Monthly budget settable.
//! - Name is required and unique among its siblings, ignoring case, checked live.
//! - Monthly budget is the Category's amount this month in the one Budget the Dialogs save into.
//!   It is read-only on a parent, which shows its leaves' sum, and while that Budget is archived.
//!   Blank tracks only: an Add leaves the Budget alone, and an Edit stops the amount.
//!
//! The parents, the usage and the budget are read when the Dialog opens, as it is modal. A confirm
//! is lent the budget port, and a refusal stays in the Dialog.

use std::collections::BTreeMap;

use lib_core::{CategoryId, CategoryTypes, Money};
use lib_locale::format::format_month;
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::view::{
    Choice, DialogFrame, SelectList, SelectMove, SelectView, TextFieldView, count,
};

use super::{save_refusal_text, type_in_use_text};
use crate::interactor::{AddCategory, BudgetChange, EditCategory};
use crate::protocols::{
    CategoriesOutcome, CategoryChange, CategorySavePorts, CategoryUsage, Holdings, ParentStop,
    SharedCategories, TypeChangeBlockers,
};
use crate::{
    Category, depth, descendants_inclusive, get, is_leaf, is_uncategorised, move_exceeds_depth,
    msg, path,
};

/// The form's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CategoryField {
    #[default]
    Name,
    Parent,
    Type,
    Budget,
}

impl CategoryField {
    const ORDER: [Self; 4] = [Self::Name, Self::Parent, Self::Type, Self::Budget];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryFormEvent {
    /// A text field's whole new text, as its View's input holds it.
    Name(String),
    /// The Monthly budget's new text: only its digits and `.` are kept.
    Budget(String),
    /// The parent picked, closing its list. One that is not offered is ignored.
    Parent(Option<CategoryId>),
    /// A click on the type's segmented control, closing the parent's list.
    Type(CategoryTypes),
    /// A click on the Parent category's field: focuses it and opens or closes its list.
    ToggleParents,
    /// Closes the Parent category's open list without picking: the host's `back`.
    CloseParents,
    /// A click on a field, which closes an open list.
    Focus(CategoryField),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryFormViewModel {
    pub frame: DialogFrame,
    pub name: TextFieldView,
    /// The name's problem, checked live.
    pub name_error: Option<String>,
    pub parent: SelectView<Option<CategoryId>>,
    /// The Parent category's list: open, and its highlight.
    pub parent_list: SelectList,
    /// The type's segmented control.
    pub category_type: SelectView<CategoryTypes>,
    /// Whether the type follows the parent, drawn dimmed and not clickable.
    pub type_locked: bool,
    /// Name is read-only: Uncategorised keeps its name.
    pub name_read_only: bool,
    /// Parent category is read-only: Uncategorised stays at top level.
    pub parent_read_only: bool,
    pub budget: BudgetField,
    /// The suffix on Parent category's and Monthly budget's labels.
    pub optional: String,
    /// The note under the fields.
    pub notice: String,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
    /// Edit, rather than Add.
    pub editing: bool,
    pub focused: CategoryField,
}

/// The Monthly budget field as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetField {
    pub field: TextFieldView,
    /// What shows in place of the input while it is read-only.
    pub read_only: Option<String>,
}

/// Why the Monthly budget can't be typed into.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BudgetLock {
    /// A parent only rolls up its children.
    Parent,
    /// The Budget it saves into, named, is archived.
    Archived(String),
}

/// The Category an Edit saves, as it opened.
struct Editing {
    id: CategoryId,
    /// The Splits in it and its subtree.
    splits: usize,
    is_parent: bool,
    top_level: bool,
    /// Uncategorised: Name, Parent and Type can't change.
    fixed: bool,
    /// The type it opened with, and what in its subtree Budgets and Bill Plans use: a type change
    /// is refused while that is anything.
    original_type: CategoryTypes,
    name_on_open: String,
    blockers: TypeChangeBlockers,
}

pub struct CategoryFormPresenter {
    categories: SharedCategories,
    editing: Option<Editing>,
    parents: Vec<Choice<Option<CategoryId>>>,
    /// Where each offered parent is budgeted, read when the Dialog opened.
    stops: BTreeMap<CategoryId, ParentStop>,
    /// What each offered parent holds that a first child would take, read when the Dialog opened.
    holdings: BTreeMap<CategoryId, Holdings>,
    name: String,
    parent: Option<CategoryId>,
    category_type: CategoryTypes,
    budget: String,
    lock: Option<BudgetLock>,
    focused: CategoryField,
    parent_list: SelectList,
    refusal: Option<String>,
}

impl CategoryFormPresenter {
    /// A fresh Add form, at top level as an Expense or under `parent` as its type.
    pub fn add(
        categories: SharedCategories,
        budget: &dyn CategorySavePorts,
        parent: Option<CategoryId>,
    ) -> Self {
        let rows = categories.borrow().categories();
        let parents = choices(
            &rows,
            rows.iter()
                .filter(|c| depth(&rows, c.id) < 2 && !is_uncategorised(c))
                .collect(),
        );
        let parent = parent.filter(|id| parents.iter().any(|choice| choice.value == Some(*id)));
        let category_type = parent
            .and_then(|id| get(&rows, id))
            .map_or(CategoryTypes::Expense, |p| p.category_type.clone());
        let lock = budget
            .monthly_budget(&rows, None)
            .archived
            .map(BudgetLock::Archived);
        Self {
            stops: parent_stops(budget, &parents),
            holdings: parent_holdings(budget, &parents),
            categories,
            editing: None,
            parents,
            name: String::new(),
            parent,
            category_type,
            budget: String::new(),
            lock,
            focused: CategoryField::Name,
            parent_list: SelectList::default(),
            refusal: None,
        }
    }

    /// The Edit form pre-filled from Category `id`, or `None` when it is gone. Its Monthly budget
    /// shows this month's amount, a parent's being its leaves' sum.
    pub fn edit(
        categories: SharedCategories,
        budget: &dyn CategorySavePorts,
        usage: &dyn CategoryUsage,
        id: CategoryId,
    ) -> Option<Self> {
        let rows = categories.borrow().categories();
        let category = get(&rows, id)?.clone();
        let subtree = descendants_inclusive(&rows, id);
        let offered = rows
            .iter()
            .filter(|c| {
                !subtree.contains(&c.id)
                    && !move_exceeds_depth(&rows, id, c.id)
                    && !is_uncategorised(c)
            })
            .collect();
        let monthly = budget.monthly_budget(&rows, Some(id));
        let is_parent = !is_leaf(&rows, id);
        let lock = if is_parent {
            Some(BudgetLock::Parent)
        } else {
            monthly.archived.map(BudgetLock::Archived)
        };
        let parents = choices(&rows, offered);
        Some(Self {
            stops: parent_stops(budget, &parents),
            holdings: parent_holdings(budget, &parents),
            parents,
            editing: Some(Editing {
                id,
                splits: subtree
                    .iter()
                    .map(|below| usage.category_usage(*below).splits)
                    .sum(),
                is_parent,
                top_level: category.parent.is_none(),
                fixed: is_uncategorised(&category),
                original_type: category.category_type.clone(),
                name_on_open: category.name.clone(),
                blockers: usage.type_change_blockers(&subtree),
            }),
            categories,
            name: category.name,
            parent: category.parent,
            category_type: category.category_type,
            budget: monthly
                .amount
                .map(|amount| amount.0.to_string())
                .unwrap_or_default(),
            lock,
            focused: CategoryField::Name,
            parent_list: SelectList::default(),
            refusal: None,
        })
    }

    pub fn handle(
        &mut self,
        event: CategoryFormEvent,
        router: &mut dyn DialogRouter<CategoriesOutcome>,
    ) {
        match event {
            CategoryFormEvent::Name(text) => self.set_name(text),
            CategoryFormEvent::Budget(text) => self.set_budget(&text),
            CategoryFormEvent::Parent(id) => {
                self.parent_list.close();
                self.pick_parent(id);
            }
            CategoryFormEvent::Type(category_type) => {
                self.parent_list.close();
                self.pick_type(category_type);
            }
            CategoryFormEvent::ToggleParents => {
                self.focused = CategoryField::Parent;
                if !self.fixed() {
                    self.parent_list.toggle(&self.parents, Some(&self.parent));
                }
            }
            CategoryFormEvent::CloseParents => {
                self.parent_list.close();
            }
            CategoryFormEvent::Focus(field) => {
                self.parent_list.close();
                self.focused = field;
            }
            CategoryFormEvent::Cancel => router.close(None),
        }
    }

    /// Reads a Dialog command or typed key for the focused field: `select` and `submit` confirm a
    /// valid form through `budget`, except that `select` on the Parent category opens its list and
    /// picks from it; typing edits Name and Monthly budget and goes nowhere on the select or Type;
    /// the arrows step the select and switch Type; the field keys move focus, wrapping and
    /// committing an open list's highlight. `budget` is lent for the call, since the Desktop
    /// writes its Ledger only with the gpui context the call runs in.
    pub fn input(
        &mut self,
        input: DialogInput,
        budget: &mut dyn CategorySavePorts,
        router: &mut dyn DialogRouter<CategoriesOutcome>,
    ) {
        match (input, self.focused) {
            (DialogInput::NextField | DialogInput::PreviousField, _) => {
                // Leaving the select commits an open list's highlight, as a click on it would.
                if let Some(id) = self.parent_list.commit(&self.parents) {
                    self.pick_parent(id);
                }
                let order = CategoryField::ORDER;
                let at = order.iter().position(|f| *f == self.focused).unwrap_or(0);
                let next = if input == DialogInput::PreviousField {
                    at + order.len() - 1
                } else {
                    at + 1
                };
                self.focused = order[next % order.len()];
            }
            (DialogInput::Submit, _) => self.confirm(budget, router),
            (_, CategoryField::Parent) if self.fixed() => {}
            (_, CategoryField::Parent) => {
                // A select swallows typing, so it never reaches the Shell.
                if let SelectMove::Chose(id) =
                    self.parent_list
                        .input(input, &self.parents, Some(&self.parent))
                {
                    self.pick_parent(id);
                }
            }
            (DialogInput::Select, _) => self.confirm(budget, router),
            (DialogInput::SelectNext | DialogInput::SelectPrevious, CategoryField::Type) => {
                let other = match self.category_type {
                    CategoryTypes::Expense => CategoryTypes::Income,
                    CategoryTypes::Income => CategoryTypes::Expense,
                };
                self.pick_type(other);
            }
            (DialogInput::Char(ch), CategoryField::Name) => {
                let mut name = self.name.clone();
                name.push(ch);
                self.set_name(name);
            }
            (DialogInput::Backspace, CategoryField::Name) => {
                let mut name = self.name.clone();
                name.pop();
                self.set_name(name);
            }
            (DialogInput::Char(ch), CategoryField::Budget) => {
                let mut text = self.budget.clone();
                text.push(ch);
                self.set_budget(&text);
            }
            (DialogInput::Backspace, CategoryField::Budget) => {
                let mut text = self.budget.clone();
                text.pop();
                self.set_budget(&text);
            }
            _ => {}
        }
    }

    fn set_name(&mut self, text: String) {
        if self.fixed() {
            return;
        }
        self.name = text;
        self.refusal = None;
    }

    /// Keeps only an amount's characters, while the Monthly budget can be typed into.
    fn set_budget(&mut self, text: &str) {
        if self.lock.is_none() {
            self.budget = text
                .chars()
                .filter(|c| c.is_numeric() || *c == '.')
                .collect();
        }
    }

    /// Sets the parent, and with it the type, ignoring a parent that is not offered.
    fn pick_parent(&mut self, id: Option<CategoryId>) {
        if !self.fixed() && self.parents.iter().any(|choice| choice.value == id) {
            self.parent = id;
            if let Some(parent) = id.and_then(|id| self.category(id)) {
                self.category_type = parent.category_type;
            }
        }
    }

    fn pick_type(&mut self, category_type: CategoryTypes) {
        if !self.type_locked() {
            self.category_type = category_type;
        }
    }

    /// Saves the form through the Interactor, the budget written through `budget`, and closes the
    /// Dialog; a refusal stays on the form. A form that can't confirm does nothing.
    pub fn confirm(
        &mut self,
        budget: &mut dyn CategorySavePorts,
        router: &mut dyn DialogRouter<CategoriesOutcome>,
    ) {
        if !self.can_confirm() {
            return;
        }
        let name = self.name.trim().to_string();
        let monthly = self.budget_change();
        let saved = {
            let mut categories = self.categories.borrow_mut();
            match &self.editing {
                Some(editing) => EditCategory {
                    id: editing.id,
                    name,
                    parent_id: self.parent,
                    category_type: Some(self.category_type.clone()),
                    monthly,
                }
                .run(&mut *categories, budget),
                None => AddCategory {
                    name,
                    parent_id: self.parent,
                    category_type: self.category_type.clone(),
                    monthly,
                }
                .run(&mut *categories, budget),
            }
        };
        match saved {
            Ok(category) => router.close(Some(CategoriesOutcome {
                change: CategoryChange::Saved,
                id: category.id,
                name: category.name,
            })),
            Err(refusal) => self.refusal = Some(save_refusal_text(&refusal)),
        }
    }

    pub fn view_model(&self) -> CategoryFormViewModel {
        let (title, submit) = if self.editing.is_some() {
            (msg::categories_edit_title(), msg::categories_edit_submit())
        } else {
            (msg::categories_add_title(), msg::categories_add_submit())
        };
        CategoryFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.can_confirm(),
                destructive: false,
                hints: Vec::new(),
            },
            name: TextFieldView {
                label: lib_locale::msg::column_name(),
                text: self.name.clone(),
                placeholder: msg::categories_name_placeholder(),
                focused: self.focused == CategoryField::Name,
            },
            name_error: self.name_taken().then(msg::categories_name_taken),
            parent: SelectView {
                label: msg::categories_field_parent(),
                options: self.parents.clone(),
                chosen: Some(self.parent),
                focused: self.focused == CategoryField::Parent,
            },
            parent_list: self.parent_list,
            category_type: SelectView {
                label: lib_locale::msg::column_type(),
                options: [CategoryTypes::Expense, CategoryTypes::Income]
                    .into_iter()
                    .map(|value| Choice {
                        label: type_label(&value),
                        value,
                    })
                    .collect(),
                chosen: Some(self.category_type.clone()),
                focused: self.focused == CategoryField::Type,
            },
            type_locked: self.type_locked(),
            name_read_only: self.fixed(),
            parent_read_only: self.fixed(),
            budget: BudgetField {
                field: TextFieldView {
                    label: msg::categories_field_monthly_budget(),
                    text: self.budget.clone(),
                    placeholder: msg::categories_budget_placeholder(),
                    focused: self.focused == CategoryField::Budget,
                },
                read_only: self.read_only_budget(),
            },
            optional: msg::categories_field_optional(),
            notice: self.notice(),
            refusal: self.refusal.clone(),
            editing: self.editing.is_some(),
            focused: self.focused,
        }
    }

    fn category(&self, id: CategoryId) -> Option<Category> {
        self.categories.borrow().category(id)
    }

    /// A child's type is its parent's, and on Edit only a Category already at top level changes
    /// type.
    fn type_locked(&self) -> bool {
        self.parent.is_some()
            || self.fixed()
            || self
                .editing
                .as_ref()
                .is_some_and(|editing| !editing.top_level)
    }

    /// Editing Uncategorised, whose Name, Parent and Type are fixed.
    fn fixed(&self) -> bool {
        self.editing.as_ref().is_some_and(|editing| editing.fixed)
    }

    /// Another Category under the same parent has this name, ignoring case.
    fn name_taken(&self) -> bool {
        let name = self.name.trim();
        let own = self.editing.as_ref().map(|editing| editing.id);
        !name.is_empty()
            && self.categories.borrow().categories().iter().any(|c| {
                Some(c.id) != own && c.parent == self.parent && c.name.eq_ignore_ascii_case(name)
            })
    }

    fn can_confirm(&self) -> bool {
        !self.name.trim().is_empty() && !self.name_taken()
    }

    /// What the save does to the Monthly budget. Text that isn't an amount leaves it alone.
    fn budget_change(&self) -> BudgetChange {
        if self.lock.is_some() {
            return BudgetChange::Keep;
        }
        if self.budget.trim().is_empty() {
            return if self.editing.is_some() {
                BudgetChange::Stop
            } else {
                BudgetChange::Keep
            };
        }
        self.budget
            .trim()
            .parse::<Money>()
            .map_or(BudgetChange::Keep, BudgetChange::Set)
    }

    fn read_only_budget(&self) -> Option<String> {
        let amount = (!self.budget.is_empty()).then_some(self.budget.as_str());
        match self.lock.as_ref()? {
            BudgetLock::Parent => Some(match amount {
                Some(amount) => msg::categories_parent_budget_rollup_sum(amount),
                None => msg::categories_parent_budget_rollup(),
            }),
            BudgetLock::Archived(budget) => Some(msg::categories_budget_archived(
                &amount.map_or_else(msg::categories_budget_none, str::to_string),
                budget,
            )),
        }
    }

    /// Where the chosen parent is budgeted, when this save would give it its first child.
    fn parent_stop(&self) -> Option<(Category, ParentStop)> {
        let parent = self.category(self.parent?)?;
        let rows = self.categories.borrow().categories();
        let stop = *self.stops.get(&parent.id)?;
        is_leaf(&rows, parent.id).then_some((parent, stop))
    }

    /// What the chosen parent hands to this Category, when the save would give it its first child.
    fn handover_notice(&self) -> Option<String> {
        let parent = self.category(self.parent?)?;
        let rows = self.categories.borrow().categories();
        let held = *self.holdings.get(&parent.id)?;
        if !is_leaf(&rows, parent.id) {
            return None;
        }
        let mut parts = Vec::new();
        if held.splits > 0 {
            parts.push(msg::categories_handover_splits(count(held.splits)));
        }
        if held.bill_plans > 0 {
            parts.push(msg::categories_handover_bill_plans(count(held.bill_plans)));
        }
        if held.payee_defaults > 0 {
            parts.push(msg::categories_handover_payee_defaults(count(
                held.payee_defaults,
            )));
        }
        let rows = match parts.as_slice() {
            [one] => one.clone(),
            [first, second] => msg::categories_handover_two(first, second),
            [first, second, third] => msg::categories_handover_three(first, second, third),
            _ => return None,
        };
        let name = self.name.trim();
        let child = if name.is_empty() {
            msg::categories_handover_this_category()
        } else {
            name.to_string()
        };
        Some(msg::categories_notice_handover(&parent.name, &rows, &child))
    }

    /// Why the picked type is refused, as soon as it is picked.
    fn type_in_use_notice(&self) -> Option<String> {
        let editing = self.editing.as_ref()?;
        (editing.top_level
            && self.category_type != editing.original_type
            && !editing.blockers.is_empty())
        .then(|| type_in_use_text(&editing.name_on_open, &self.category_type, editing.blockers))
    }

    fn notice(&self) -> String {
        if let Some(notice) = self.type_in_use_notice() {
            return notice;
        }
        let stop = self.parent_stop().map(|(parent, stop)| {
            msg::categories_notice_parent_stop(
                &parent.name,
                &format_month(stop.month),
                count(stop.budgets),
            )
        });
        match (stop, self.handover_notice()) {
            (Some(stop), Some(handover)) => return format!("{stop} {handover}"),
            (Some(notice), None) | (None, Some(notice)) => return notice,
            (None, None) => {}
        }
        let nested = self.parent.and_then(|id| self.category(id)).map(|parent| {
            msg::categories_notice_nested(&parent.name, &type_label(&parent.category_type))
        });
        match &self.editing {
            Some(editing) if editing.fixed => msg::categories_edit_uncategorised_notice(),
            Some(editing) if editing.splits > 0 => {
                msg::categories_edit_usage_notice(count(editing.splits))
            }
            Some(editing) => nested.unwrap_or_else(|| {
                if editing.is_parent {
                    msg::categories_edit_parent_notice()
                } else {
                    msg::categories_edit_notice()
                }
            }),
            None => nested.unwrap_or_else(msg::categories_notice_default),
        }
    }
}

/// Where each parent on offer is budgeted this month.
fn parent_stops(
    budget: &dyn CategorySavePorts,
    parents: &[Choice<Option<CategoryId>>],
) -> BTreeMap<CategoryId, ParentStop> {
    parents
        .iter()
        .filter_map(|choice| choice.value)
        .filter_map(|id| Some((id, budget.parent_stop(id)?)))
        .collect()
}

/// What each parent on offer holds, left out when it holds nothing.
fn parent_holdings(
    ports: &dyn CategorySavePorts,
    parents: &[Choice<Option<CategoryId>>],
) -> BTreeMap<CategoryId, Holdings> {
    parents
        .iter()
        .filter_map(|choice| choice.value)
        .map(|id| (id, ports.holdings(id)))
        .filter(|(_, held)| !held.is_empty())
        .collect()
}

/// None, then each of `offered` by its path, in tree order.
fn choices(rows: &[Category], offered: Vec<&Category>) -> Vec<Choice<Option<CategoryId>>> {
    std::iter::once(Choice {
        value: None,
        label: msg::categories_parent_none(),
    })
    .chain(offered.into_iter().map(|category| Choice {
        value: Some(category.id),
        label: path(rows, category.id).unwrap_or_else(|| category.name.clone()),
    }))
    .collect()
}

fn type_label(category_type: &CategoryTypes) -> String {
    match category_type {
        CategoryTypes::Expense => msg::categories_type_expense(),
        CategoryTypes::Income => msg::categories_type_income(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::CategoryError;
    use crate::CategoryRepository as _;
    use crate::fake::{InMemoryCategoryBudget, InMemoryCategoryUsage, RecordingRouter, RouterCall};
    use crate::protocols::{CategoryBudget as _, CategoryUse, MonthlyBudgetRefusal};

    const HOUSING: CategoryId = CategoryId::seed(1);
    const UTILITIES: CategoryId = CategoryId::seed(3);
    const FOOD: CategoryId = CategoryId::seed(6);
    const GROCERIES: CategoryId = CategoryId::seed(7);
    const TRANSPORT: CategoryId = CategoryId::seed(9);
    const SALARY: CategoryId = CategoryId::seed(11);

    fn money(text: &str) -> Money {
        text.parse().expect("an amount")
    }

    fn send(form: &mut CategoryFormPresenter, events: Vec<CategoryFormEvent>) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            form.handle(event, &mut router);
        }
        router.calls
    }

    fn confirm(
        form: &mut CategoryFormPresenter,
        budget: &mut InMemoryCategoryBudget,
    ) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        form.confirm(budget, &mut router);
        router.calls
    }

    fn press(
        form: &mut CategoryFormPresenter,
        budget: &mut InMemoryCategoryBudget,
        inputs: &[DialogInput],
    ) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for input in inputs {
            form.input(*input, budget, &mut router);
        }
        router.calls
    }

    fn keys(text: &str) -> Vec<DialogInput> {
        text.chars().map(DialogInput::Char).collect()
    }

    fn name(text: &str) -> CategoryFormEvent {
        CategoryFormEvent::Name(text.to_string())
    }

    fn labels(view: &SelectView<Option<CategoryId>>) -> Vec<String> {
        view.options
            .iter()
            .map(|choice| choice.label.clone())
            .collect()
    }

    #[test]
    fn add_needs_a_name_unique_among_its_siblings_then_saves_and_closes() {
        let (categories, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        let mut form = CategoryFormPresenter::add(shared, &budget, Some(FOOD));
        assert!(!form.view_model().frame.can_confirm);

        send(&mut form, vec![name("groceries")]);
        let view = form.view_model();
        assert!(!view.frame.can_confirm);
        assert_eq!(
            view.name_error.as_deref(),
            Some("Another category under that parent already has this name.")
        );
        assert_eq!(confirm(&mut form, &mut budget), []);

        send(&mut form, vec![name("Takeaway")]);
        let calls = confirm(&mut form, &mut budget);

        let added = categories
            .borrow()
            .categories()
            .into_iter()
            .find(|c| c.name == "Takeaway")
            .expect("added");
        assert_eq!(added.parent, Some(FOOD));
        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(CategoriesOutcome {
                change: CategoryChange::Saved,
                id: added.id,
                name: "Takeaway".into(),
            }))]
        );
    }

    #[test]
    fn a_parent_sets_and_locks_the_type_and_says_so() {
        let (_, shared) = stub();
        let budget = InMemoryCategoryBudget::default();
        let mut form = CategoryFormPresenter::add(shared, &budget, None);
        let view = form.view_model();
        assert!(!view.type_locked);
        assert_eq!(
            view.notice,
            "Picking a parent will lock the type control to the parent's type."
        );
        // Electricity sits at the depth cap, so it takes no children and isn't offered.
        assert!(
            !labels(&view.parent)
                .iter()
                .any(|label| label.ends_with("Electricity"))
        );
        assert!(labels(&view.parent).contains(&"Housing \u{203a} Utilities".to_string()));

        send(
            &mut form,
            vec![
                CategoryFormEvent::Parent(Some(SALARY)),
                CategoryFormEvent::Type(CategoryTypes::Expense),
            ],
        );
        let view = form.view_model();
        assert!(view.type_locked);
        assert_eq!(view.category_type.chosen, Some(CategoryTypes::Income));
        assert_eq!(
            view.notice,
            "Nesting under Salary inherits its type (income) \u{2014} the type control locks once a parent is picked."
        );
    }

    #[test]
    fn tab_walks_the_fields_and_the_budget_keeps_only_an_amonts_characters() {
        let (_, shared) = stub();
        let budget = InMemoryCategoryBudget::default();
        let mut form = CategoryFormPresenter::add(shared, &budget, None);

        let mut budget = budget;
        press(&mut form, &mut budget, &[DialogInput::NextField]);
        assert_eq!(form.view_model().focused, CategoryField::Parent);
        press(
            &mut form,
            &mut budget,
            &[DialogInput::PreviousField, DialogInput::PreviousField],
        );
        assert_eq!(form.view_model().focused, CategoryField::Budget);
        send(&mut form, vec![CategoryFormEvent::Budget("1a2.5".into())]);
        assert_eq!(form.view_model().budget.field.text, "12.5");
        assert_eq!(
            send(&mut form, vec![CategoryFormEvent::Cancel]),
            [RouterCall::CloseDialog(None)]
        );
    }

    #[test]
    fn typing_edits_name_and_budget_and_goes_nowhere_on_parent_or_type() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        let mut form = CategoryFormPresenter::add(shared, &budget, None);

        press(&mut form, &mut budget, &keys("Petx"));
        press(&mut form, &mut budget, &[DialogInput::Backspace]);
        press(&mut form, &mut budget, &keys("s"));
        assert_eq!(form.view_model().name.text, "Pets");
        press(&mut form, &mut budget, &[DialogInput::NextField]);
        press(&mut form, &mut budget, &keys("ab"));
        press(&mut form, &mut budget, &[DialogInput::NextField]);
        press(&mut form, &mut budget, &keys("cd"));
        press(&mut form, &mut budget, &[DialogInput::Backspace]);
        press(&mut form, &mut budget, &[DialogInput::NextField]);
        press(&mut form, &mut budget, &keys("4x0.59"));
        press(&mut form, &mut budget, &[DialogInput::Backspace]);

        let view = form.view_model();
        assert_eq!(view.name.text, "Pets");
        assert_eq!(view.parent.chosen, Some(None));
        assert_eq!(view.budget.field.text, "40.5");
    }

    #[test]
    fn the_arrows_switch_an_unlocked_type_and_leave_a_locked_one() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        let mut form = CategoryFormPresenter::add(shared.clone(), &budget, None);
        form.handle(
            CategoryFormEvent::Focus(CategoryField::Type),
            &mut RecordingRouter::default(),
        );

        press(&mut form, &mut budget, &[DialogInput::SelectNext]);
        assert_eq!(
            form.view_model().category_type.chosen,
            Some(CategoryTypes::Income)
        );
        press(&mut form, &mut budget, &[DialogInput::SelectPrevious]);
        assert_eq!(
            form.view_model().category_type.chosen,
            Some(CategoryTypes::Expense)
        );

        let mut child = CategoryFormPresenter::add(shared, &budget, Some(SALARY));
        child.handle(
            CategoryFormEvent::Focus(CategoryField::Type),
            &mut RecordingRouter::default(),
        );
        press(&mut child, &mut budget, &[DialogInput::SelectNext]);
        assert_eq!(
            child.view_model().category_type.chosen,
            Some(CategoryTypes::Income)
        );
    }

    #[test]
    fn the_parent_select_opens_moves_and_picks_and_leaving_it_commits_the_highlight() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        let mut form = CategoryFormPresenter::add(shared, &budget, None);
        press(&mut form, &mut budget, &[DialogInput::NextField]);

        assert_eq!(press(&mut form, &mut budget, &[DialogInput::Select]), []);
        assert_eq!(
            form.view_model().parent_list.highlight(),
            Some(0),
            "on None"
        );
        press(&mut form, &mut budget, &[DialogInput::SelectNext]);
        assert_eq!(
            form.view_model().parent.chosen,
            Some(None),
            "not yet picked"
        );
        press(&mut form, &mut budget, &[DialogInput::Select]);
        let view = form.view_model();
        assert!(!view.parent_list.is_open());
        let picked = view.parent.options[1].value;
        assert_eq!(view.parent.chosen, Some(picked));

        press(
            &mut form,
            &mut budget,
            &[
                DialogInput::Char(' '),
                DialogInput::SelectNext,
                DialogInput::NextField,
            ],
        );
        let view = form.view_model();
        assert!(!view.parent_list.is_open());
        assert_eq!(view.focused, CategoryField::Type);
        assert_eq!(view.parent.chosen, Some(view.parent.options[2].value));
    }

    #[test]
    fn a_click_toggles_the_parent_list_and_back_or_a_focus_click_closes_it() {
        let (_, shared) = stub();
        let budget = InMemoryCategoryBudget::default();
        let mut form = CategoryFormPresenter::add(shared, &budget, None);

        send(&mut form, vec![CategoryFormEvent::ToggleParents]);
        assert!(form.view_model().parent_list.is_open());
        assert_eq!(form.view_model().focused, CategoryField::Parent);
        send(&mut form, vec![CategoryFormEvent::CloseParents]);
        assert!(!form.view_model().parent_list.is_open());
        send(
            &mut form,
            vec![
                CategoryFormEvent::ToggleParents,
                CategoryFormEvent::Focus(CategoryField::Name),
            ],
        );
        assert!(!form.view_model().parent_list.is_open());
        send(
            &mut form,
            vec![
                CategoryFormEvent::ToggleParents,
                CategoryFormEvent::Parent(Some(FOOD)),
            ],
        );
        assert!(!form.view_model().parent_list.is_open());
        assert_eq!(form.view_model().parent.chosen, Some(Some(FOOD)));
    }

    #[test]
    fn select_confirms_off_the_select_and_submit_confirms_from_it() {
        let (categories, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        let mut form = CategoryFormPresenter::add(shared, &budget, None);
        assert_eq!(
            press(&mut form, &mut budget, &[DialogInput::Select]),
            [],
            "no name"
        );

        press(&mut form, &mut budget, &keys("Pets"));
        press(&mut form, &mut budget, &[DialogInput::NextField]);
        let calls = press(&mut form, &mut budget, &[DialogInput::Submit]);

        assert!(matches!(
            calls.as_slice(),
            [RouterCall::CloseDialog(Some(outcome))] if outcome.name == "Pets"
        ));
        assert!(
            categories
                .borrow()
                .categories()
                .iter()
                .any(|c| c.name == "Pets")
        );

        let (_, shared) = stub();
        let mut form = CategoryFormPresenter::add(shared, &budget, None);
        press(&mut form, &mut budget, &keys("Gifts"));
        let calls = press(&mut form, &mut budget, &[DialogInput::Select]);
        assert!(matches!(
            calls.as_slice(),
            [RouterCall::CloseDialog(Some(_))]
        ));
    }

    #[test]
    fn edit_offers_no_parent_in_its_subtree_or_past_the_depth_cap() {
        let (_, shared) = stub();
        let budget = InMemoryCategoryBudget::default();
        let usage = InMemoryCategoryUsage::default();

        let housing =
            CategoryFormPresenter::edit(shared.clone(), &budget, &usage, HOUSING).expect("seeded");
        let offered = labels(&housing.view_model().parent);
        assert!(
            !offered.contains(&"Transport".to_string()),
            "Housing is two levels tall"
        );
        assert!(!offered.iter().any(|label| label.starts_with("Housing")));

        let utilities = CategoryFormPresenter::edit(shared.clone(), &budget, &usage, UTILITIES)
            .expect("seeded");
        assert!(labels(&utilities.view_model().parent).contains(&"Transport".to_string()));
        assert!(CategoryFormPresenter::edit(shared, &budget, &usage, CategoryId::new()).is_none());
    }

    #[test]
    fn edit_quotes_the_subtrees_transactions_and_its_type_change_cascades_on_save() {
        let (categories, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        let mut usage = InMemoryCategoryUsage::default();
        usage.uses.insert(
            GROCERIES,
            CategoryUse {
                splits: 3,
                budgets: 0,
            },
        );
        let mut form = CategoryFormPresenter::edit(shared, &budget, &usage, FOOD).expect("seeded");
        assert_eq!(
            form.view_model().notice,
            "This category has 3 transactions. Changing its type will affect these transactions."
        );

        send(
            &mut form,
            vec![CategoryFormEvent::Type(CategoryTypes::Income)],
        );
        assert_eq!(
            categories
                .borrow()
                .category(GROCERIES)
                .map(|c| c.category_type),
            Some(CategoryTypes::Expense),
            "nothing is written until the save"
        );
        confirm(&mut form, &mut budget);
        assert_eq!(
            categories
                .borrow()
                .category(GROCERIES)
                .map(|c| c.category_type),
            Some(CategoryTypes::Income)
        );
    }

    #[test]
    fn a_blocked_type_shows_its_notice_on_pick_and_the_save_is_refused_with_the_same_text() {
        let (categories, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        let used = TypeChangeBlockers {
            budgets: 2,
            bill_plans: 1,
        };
        // The form reads the notice through `usage` and the save refuses through `budget`; a
        // Client lends one set of ports as both, so both hold the same blockers.
        budget.blockers.insert(GROCERIES, used);
        let mut usage = InMemoryCategoryUsage::default();
        usage.blockers.insert(GROCERIES, used);
        let mut form = CategoryFormPresenter::edit(shared, &budget, &usage, FOOD).expect("seeded");
        assert!(!form.view_model().notice.contains("can't become"));

        send(
            &mut form,
            vec![CategoryFormEvent::Type(CategoryTypes::Income)],
        );
        let said = "Food can't become Income: 2 Budgets and 1 Bill Plan use its Categories.";
        assert_eq!(form.view_model().notice, said);

        let calls = confirm(&mut form, &mut budget);
        assert!(calls.is_empty(), "the Dialog stays open");
        assert_eq!(form.view_model().refusal.as_deref(), Some(said));
        assert_eq!(
            categories.borrow().category(FOOD).map(|c| c.category_type),
            Some(CategoryTypes::Expense)
        );

        send(
            &mut form,
            vec![CategoryFormEvent::Type(CategoryTypes::Expense)],
        );
        assert!(!form.view_model().notice.contains("can't become"));
    }

    #[test]
    fn an_unused_subtree_shows_no_type_notice() {
        let (_, shared) = stub();
        let budget = InMemoryCategoryBudget::default();
        let usage = InMemoryCategoryUsage::default();
        let mut form = CategoryFormPresenter::edit(shared, &budget, &usage, FOOD).expect("seeded");

        send(
            &mut form,
            vec![CategoryFormEvent::Type(CategoryTypes::Income)],
        );

        assert!(!form.view_model().notice.contains("can't become"));
    }

    #[test]
    fn a_childs_type_is_locked_on_edit() {
        let (_, shared) = stub();
        let mut form = CategoryFormPresenter::edit(
            shared,
            &InMemoryCategoryBudget::default(),
            &InMemoryCategoryUsage::default(),
            GROCERIES,
        )
        .expect("seeded");

        send(&mut form, vec![CategoryFormEvent::Parent(None)]);
        assert!(form.view_model().type_locked, "it opened under Food");
    }

    #[test]
    fn the_monthly_budget_opens_with_this_months_amount_and_a_parent_shows_its_rollup() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.amounts.insert(GROCERIES, money("600"));
        budget.amounts.insert(CategoryId::seed(8), money("150"));
        let usage = InMemoryCategoryUsage::default();

        let groceries = CategoryFormPresenter::edit(shared.clone(), &budget, &usage, GROCERIES)
            .expect("seeded");
        let view = groceries.view_model();
        assert_eq!(view.budget.field.text, "600");
        assert_eq!(view.budget.read_only, None);

        let food =
            CategoryFormPresenter::edit(shared.clone(), &budget, &usage, FOOD).expect("seeded");
        assert_eq!(
            food.view_model().budget.read_only.as_deref(),
            Some("750 \u{b7} rollup of children's budgets")
        );
        let transport =
            CategoryFormPresenter::edit(shared, &budget, &usage, TRANSPORT).expect("seeded");
        assert_eq!(transport.view_model().budget.read_only, None);
    }

    #[test]
    fn an_archived_budget_is_read_only_and_a_save_leaves_it() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget {
            archived: Some("Personal spending".into()),
            ..Default::default()
        };
        let mut form = CategoryFormPresenter::add(shared, &budget, None);
        assert_eq!(
            form.view_model().budget.read_only.as_deref(),
            Some("\u{2014} \u{b7} Personal spending is archived, so its amounts are read-only")
        );

        send(
            &mut form,
            vec![name("Pets"), CategoryFormEvent::Budget("40".into())],
        );
        assert_eq!(form.view_model().budget.field.text, "");
        let calls = confirm(&mut form, &mut budget);
        assert!(matches!(
            calls.as_slice(),
            [RouterCall::CloseDialog(Some(_))]
        ));
    }

    #[test]
    fn a_budget_on_an_income_category_is_refused_and_writes_nothing() {
        let (categories, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        let before = categories.borrow().clone();
        let mut form = CategoryFormPresenter::add(shared, &budget, None);

        send(
            &mut form,
            vec![
                name("Bonus"),
                CategoryFormEvent::Type(CategoryTypes::Income),
                CategoryFormEvent::Budget("40".into()),
            ],
        );
        assert_eq!(confirm(&mut form, &mut budget), []);

        assert_eq!(
            form.view_model().refusal.as_deref(),
            Some("Only an expense category with no subcategories can have a monthly budget.")
        );
        assert_eq!(*categories.borrow(), before);
        assert_eq!(
            budget.check_monthly_budget(&[], SALARY, Some(&money("1"))),
            Err(MonthlyBudgetRefusal::NotAnExpenseLeaf)
        );
    }

    #[test]
    fn a_blank_budget_on_edit_stops_the_amount() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.amounts.insert(GROCERIES, money("600"));
        let mut form = CategoryFormPresenter::edit(
            shared,
            &budget,
            &InMemoryCategoryUsage::default(),
            GROCERIES,
        )
        .expect("seeded");

        send(&mut form, vec![CategoryFormEvent::Budget(String::new())]);
        confirm(&mut form, &mut budget);

        assert!(budget.amounts.is_empty());
    }

    #[test]
    fn a_budgeted_parent_chosen_warns_its_budget_stops() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget {
            other_budgets: 1,
            ..Default::default()
        };
        budget.amounts.insert(TRANSPORT, money("600"));
        budget.amounts.insert(FOOD, money("1"));
        let mut form = CategoryFormPresenter::add(shared.clone(), &budget, None);
        assert_eq!(
            form.view_model().notice,
            "Picking a parent will lock the type control to the parent's type."
        );

        send(&mut form, vec![CategoryFormEvent::Parent(Some(TRANSPORT))]);
        let notice = form.view_model().notice;
        assert!(
            notice.starts_with("Transport' budget stops from Nov in 2 budgets"),
            "{notice}"
        );

        // Food already has children, so it is a parent that only rolls up.
        send(&mut form, vec![CategoryFormEvent::Parent(Some(FOOD))]);
        assert!(!form.view_model().notice.contains("stops from"));

        send(&mut form, vec![CategoryFormEvent::Parent(Some(UTILITIES))]);
        assert!(
            !form.view_model().notice.contains("stops from"),
            "unbudgeted"
        );
    }

    #[test]
    fn the_stop_notice_shows_on_edit_when_moving_under_a_budgeted_leaf() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.amounts.insert(TRANSPORT, money("600"));
        let usage = InMemoryCategoryUsage::default();
        let mut form =
            CategoryFormPresenter::edit(shared, &budget, &usage, GROCERIES).expect("seeded");

        send(&mut form, vec![CategoryFormEvent::Parent(Some(TRANSPORT))]);

        assert!(
            form.view_model()
                .notice
                .contains("stops from Nov in 1 budget")
        );
    }

    fn held(splits: usize, bill_plans: usize, payee_defaults: usize) -> Holdings {
        Holdings {
            splits,
            bill_plans,
            payee_defaults,
        }
    }

    #[test]
    fn the_handover_notice_names_what_a_leaf_parent_holds_and_the_child() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.held.insert(TRANSPORT, held(42, 1, 0));
        let mut form = CategoryFormPresenter::add(shared, &budget, Some(TRANSPORT));
        send(&mut form, vec![CategoryFormEvent::Name("Fuel".into())]);

        assert_eq!(
            form.view_model().notice,
            "Transport' 42 Splits and 1 Bill Plan move to Fuel."
        );
    }

    #[test]
    fn the_handover_notice_lists_all_three_and_shows_beside_the_stop_notice() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.held.insert(TRANSPORT, held(2, 3, 1));
        budget.amounts.insert(TRANSPORT, money("600"));
        let form = CategoryFormPresenter::add(shared, &budget, Some(TRANSPORT));

        let notice = form.view_model().notice;

        assert!(notice.contains("stops from Nov in 1 budget"), "{notice}");
        assert!(
            notice.contains("2 Splits, 3 Bill Plans and 1 Payee default move to this category."),
            "{notice}"
        );
    }

    #[test]
    fn no_handover_notice_for_an_empty_leaf_a_parent_with_children_or_a_top_level_add() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.held.insert(FOOD, held(9, 0, 0));
        let empty = CategoryFormPresenter::add(shared.clone(), &budget, Some(TRANSPORT));
        let parent = CategoryFormPresenter::add(shared.clone(), &budget, Some(FOOD));
        let top = CategoryFormPresenter::add(shared, &budget, None);

        for form in [empty, parent, top] {
            assert!(!form.view_model().notice.contains("move to"));
        }
    }

    #[test]
    fn the_handover_notice_shows_on_edit_when_moving_under_a_leaf_holding_rows() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.held.insert(TRANSPORT, held(0, 0, 2));
        let usage = InMemoryCategoryUsage::default();
        let mut form =
            CategoryFormPresenter::edit(shared, &budget, &usage, GROCERIES).expect("seeded");

        send(&mut form, vec![CategoryFormEvent::Parent(Some(TRANSPORT))]);

        assert_eq!(
            form.view_model().notice,
            "Transport' 2 Payee defaults move to Groceries."
        );
    }

    /// The stub tree with an Expense Uncategorised added, and its id.
    fn stub_with_uncategorised() -> (CategoryId, SharedCategories) {
        let (categories, shared) = stub();
        let id = crate::get_or_create_uncategorised(
            &mut categories.borrow_mut().categories,
            CategoryTypes::Expense,
        );
        (id, shared)
    }

    #[test]
    fn add_never_offers_uncategorised_as_a_parent() {
        let (uncategorised, shared) = stub_with_uncategorised();
        let budget = InMemoryCategoryBudget::default();

        let form = CategoryFormPresenter::add(shared.clone(), &budget, None);
        assert!(
            !labels(&form.view_model().parent).contains(&"Uncategorised".to_string()),
            "not in Parent"
        );

        let asked_under_it = CategoryFormPresenter::add(shared, &budget, Some(uncategorised));
        assert_eq!(asked_under_it.view_model().parent.chosen, Some(None));
    }

    #[test]
    fn edit_never_offers_uncategorised_as_a_parent() {
        let (_, shared) = stub_with_uncategorised();
        let budget = InMemoryCategoryBudget::default();
        let usage = InMemoryCategoryUsage::default();

        let form = CategoryFormPresenter::edit(shared, &budget, &usage, TRANSPORT).expect("seeded");

        assert!(!labels(&form.view_model().parent).contains(&"Uncategorised".to_string()));
    }

    #[test]
    fn uncategorised_opens_with_name_parent_and_type_read_only_and_the_budget_settable() {
        let (uncategorised, shared) = stub_with_uncategorised();
        let mut budget = InMemoryCategoryBudget::default();
        let usage = InMemoryCategoryUsage::default();
        let mut form = CategoryFormPresenter::edit(shared.clone(), &budget, &usage, uncategorised)
            .expect("added");

        let view = form.view_model();
        assert!(view.name_read_only && view.parent_read_only && view.type_locked);
        assert_eq!(
            view.notice,
            "Uncategorised is fixed \u{2014} only its monthly budget can change."
        );
        assert!(view.budget.read_only.is_none(), "the budget is settable");

        // Nothing the user does changes the fixed fields.
        send(
            &mut form,
            vec![
                name("Misc"),
                CategoryFormEvent::Parent(Some(FOOD)),
                CategoryFormEvent::Type(CategoryTypes::Income),
                CategoryFormEvent::ToggleParents,
            ],
        );
        press(&mut form, &mut budget, &[DialogInput::SelectNext]);
        let view = form.view_model();
        assert_eq!(view.name.text, "Uncategorised");
        assert_eq!(view.parent.chosen, Some(None));
        assert_eq!(view.category_type.chosen, Some(CategoryTypes::Expense));
        assert!(!view.parent_list.is_open());

        // The Monthly budget saves, and the Category stays as it was.
        send(&mut form, vec![CategoryFormEvent::Budget("25".into())]);
        let calls = confirm(&mut form, &mut budget);
        assert_eq!(calls.len(), 1, "closed: {calls:?}");
        assert_eq!(budget.amounts.get(&uncategorised), Some(&money("25")));
        assert_eq!(
            shared.borrow().category(uncategorised).map(|c| c.name),
            Some("Uncategorised".into())
        );
    }

    #[test]
    fn a_save_that_moves_a_parent_under_a_leaf_with_rows_says_so_and_stays_open() {
        let (_, shared) = stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.held.insert(TRANSPORT, held(2, 1, 0));
        let usage = InMemoryCategoryUsage::default();
        let mut form = CategoryFormPresenter::edit(shared, &budget, &usage, FOOD).expect("seeded");
        send(&mut form, vec![CategoryFormEvent::Parent(Some(TRANSPORT))]);

        let calls = confirm(&mut form, &mut budget);

        assert_eq!(calls, []);
        assert_eq!(
            form.view_model().refusal.as_deref(),
            Some("Transport has Splits and Bill Plans; add a Category under it first.")
        );
    }

    #[test]
    fn uncategorised_fixed_has_its_own_refusal_text() {
        stub();
        assert_eq!(
            save_refusal_text(&CategoryError::UncategorisedFixed.into()),
            "Uncategorised is fixed: it can't be renamed, moved, retyped or given subcategories."
        );
    }
}
