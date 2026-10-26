//! The Add and Edit category Dialogs (`docs/ux/desktop-mockups/18-categories/README.md`'s 5b, 5c):
//! one form, Edit pre-filled from the Category.
//!
//! - The fields, in `Tab` order: Name, Parent category (none, then each Category it may go
//!   under), Type (Expense or Income) and Monthly budget.
//! - A child takes its parent's type, so the type is chosen only at top level, and on Edit only
//!   for a Category already at top level; a type chosen there cascades to its subtree on save.
//! - Add offers every Category above the depth cap as a parent; Edit leaves out the Category's own
//!   subtree and any parent that would nest it past three levels.
//! - Name is required and unique among its siblings, ignoring case, checked live.
//! - Monthly budget is the Category's amount this month in the one Budget the Dialogs save into.
//!   It is read-only on a parent, which shows its leaves' sum, and while that Budget is archived.
//!   Blank tracks only: an Add leaves the Budget alone, and an Edit stops the amount.
//!
//! The parents, the usage and the budget are read when the Dialog opens, as it is modal. A confirm
//! is lent the budget port, and a refusal stays in the Dialog.

use lib_core::{CategoryId, CategoryTypes, Money};
use lib_viper::view::{Choice, DialogFrame, SelectView, TextFieldView};

use super::{count, save_refusal_text};
use crate::interactor::{AddCategory, BudgetChange, EditCategory};
use crate::protocols::{
    CategoriesOutcome, CategoriesRouter, CategoryBudget, CategoryChange, CategoryUsage,
    SharedCategories,
};
use crate::{Category, depth, descendants_inclusive, get, is_leaf, move_exceeds_depth, msg, path};

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
    Parent(Option<CategoryId>),
    Type(CategoryTypes),
    /// `Tab`/`Shift-Tab`, wrapping.
    NextField,
    PrevField,
    /// A click on a field.
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
    /// The type's segmented control.
    pub category_type: SelectView<CategoryTypes>,
    /// Whether the type follows the parent, drawn dimmed and not clickable.
    pub type_locked: bool,
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
}

pub struct CategoryFormPresenter {
    categories: SharedCategories,
    editing: Option<Editing>,
    parents: Vec<Choice<Option<CategoryId>>>,
    name: String,
    parent: Option<CategoryId>,
    category_type: CategoryTypes,
    budget: String,
    lock: Option<BudgetLock>,
    focused: CategoryField,
    refusal: Option<String>,
}

impl CategoryFormPresenter {
    /// A fresh Add form, at top level as an Expense or under `parent` as its type.
    pub fn add(
        categories: SharedCategories,
        budget: &dyn CategoryBudget,
        parent: Option<CategoryId>,
    ) -> Self {
        let rows = categories.borrow().categories();
        let parents = choices(
            &rows,
            rows.iter().filter(|c| depth(&rows, c.id) < 2).collect(),
        );
        let parent = parent.filter(|id| get(&rows, *id).is_some());
        let category_type = parent
            .and_then(|id| get(&rows, id))
            .map_or(CategoryTypes::Expense, |p| p.category_type.clone());
        let lock = budget
            .monthly_budget(&rows, None)
            .archived
            .map(BudgetLock::Archived);
        Self {
            categories,
            editing: None,
            parents,
            name: String::new(),
            parent,
            category_type,
            budget: String::new(),
            lock,
            focused: CategoryField::Name,
            refusal: None,
        }
    }

    /// The Edit form pre-filled from Category `id`, or `None` when it is gone. Its Monthly budget
    /// shows this month's amount, a parent's being its leaves' sum.
    pub fn edit(
        categories: SharedCategories,
        budget: &dyn CategoryBudget,
        usage: &dyn CategoryUsage,
        id: CategoryId,
    ) -> Option<Self> {
        let rows = categories.borrow().categories();
        let category = get(&rows, id)?.clone();
        let subtree = descendants_inclusive(&rows, id);
        let offered = rows
            .iter()
            .filter(|c| !subtree.contains(&c.id) && !move_exceeds_depth(&rows, id, c.id))
            .collect();
        let monthly = budget.monthly_budget(&rows, Some(id));
        let is_parent = !is_leaf(&rows, id);
        let lock = if is_parent {
            Some(BudgetLock::Parent)
        } else {
            monthly.archived.map(BudgetLock::Archived)
        };
        Some(Self {
            parents: choices(&rows, offered),
            editing: Some(Editing {
                id,
                splits: subtree
                    .iter()
                    .map(|below| usage.category_usage(*below).splits)
                    .sum(),
                is_parent,
                top_level: category.parent.is_none(),
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
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: CategoryFormEvent, router: &mut dyn CategoriesRouter) {
        match event {
            CategoryFormEvent::Name(text) => {
                self.name = text;
                self.refusal = None;
            }
            CategoryFormEvent::Budget(text) => {
                if self.lock.is_none() {
                    self.budget = text
                        .chars()
                        .filter(|c| c.is_numeric() || *c == '.')
                        .collect();
                }
            }
            CategoryFormEvent::Parent(id) => {
                if self.parents.iter().any(|choice| choice.value == id) {
                    self.parent = id;
                    if let Some(parent) = id.and_then(|id| self.category(id)) {
                        self.category_type = parent.category_type;
                    }
                }
            }
            CategoryFormEvent::Type(category_type) => {
                if !self.type_locked() {
                    self.category_type = category_type;
                }
            }
            CategoryFormEvent::NextField | CategoryFormEvent::PrevField => {
                let order = CategoryField::ORDER;
                let at = order.iter().position(|f| *f == self.focused).unwrap_or(0);
                let next = if event == CategoryFormEvent::PrevField {
                    at + order.len() - 1
                } else {
                    at + 1
                };
                self.focused = order[next % order.len()];
            }
            CategoryFormEvent::Focus(field) => self.focused = field,
            CategoryFormEvent::Cancel => router.close_dialog(None),
        }
    }

    /// Saves the form through the Interactor, the budget written through `budget`, and closes the
    /// Dialog; a refusal stays on the form. A form that can't confirm does nothing.
    pub fn confirm(&mut self, budget: &mut dyn CategoryBudget, router: &mut dyn CategoriesRouter) {
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
            Ok(category) => router.close_dialog(Some(CategoriesOutcome {
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
            || self
                .editing
                .as_ref()
                .is_some_and(|editing| !editing.top_level)
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

    fn notice(&self) -> String {
        let nested = self.parent.and_then(|id| self.category(id)).map(|parent| {
            msg::categories_notice_nested(&parent.name, &type_label(&parent.category_type))
        });
        match &self.editing {
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
    use crate::CategoryRepository as _;
    use crate::fake::{InMemoryCategoryBudget, InMemoryCategoryUsage, RecordingRouter, RouterCall};
    use crate::protocols::{CategoryUse, MonthlyBudgetRefusal};

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

        send(&mut form, vec![CategoryFormEvent::NextField]);
        assert_eq!(form.view_model().focused, CategoryField::Parent);
        send(
            &mut form,
            vec![CategoryFormEvent::PrevField, CategoryFormEvent::PrevField],
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
}
