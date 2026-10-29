//! The Add and Edit payee Dialog (6b, 6c): one form, Edit pre-filled from the Payee.
//!
//! - The fields, in `Tab` order: Name, Default category (none, then every leaf Category), and the
//!   match-rule input beside its chips.
//! - Name is checked live against every other Payee's name and rules (ADR-0012), and its problem
//!   shows under it; a blank name only keeps Save from confirming.
//! - A rule is added from the input normalised, and ignored when blank or already a chip. One
//!   another Payee owns is refused and stays in the input, with the reason under the rules until
//!   either text changes. A refusal from the Interactor shows there too.
//!
//! The Category choices and the Payee's use are read when the Dialog opens, as it is modal and
//! nothing changes them under it.

use lib_core::{CategoryId, PayeeId};
use lib_viper::view::{Choice, DialogFrame, SelectView, TextFieldView};

use super::{count, refusal_text};
use crate::interactor::{AddPayee, EditPayee};
use crate::protocols::{PayeeChange, PayeesOutcome, PayeesRouter, SharedPorts};
use crate::{PayeeDraft, PayeeError, alias_owner, clean_name, msg, normalise_alias};

/// The form's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PayeeField {
    #[default]
    Name,
    DefaultCategory,
    /// The match-rule input beside **+ add**.
    Rule,
}

impl PayeeField {
    const ORDER: [Self; 3] = [Self::Name, Self::DefaultCategory, Self::Rule];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeeFormEvent {
    /// A text field's whole new text, as its View's input holds it.
    Name(String),
    RuleInput(String),
    Category(Option<CategoryId>),
    /// **+ add**, or `Enter` in a rule input holding text: the input becomes a chip.
    AddRule,
    /// A chip's `✕`.
    RemoveRule(usize),
    /// `Backspace` in the empty rule input.
    RemoveLastRule,
    /// `Tab`/`Shift-Tab`, wrapping.
    NextField,
    PrevField,
    /// A click on a field.
    Focus(PayeeField),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeFormViewModel {
    pub frame: DialogFrame,
    pub name: TextFieldView,
    /// The name's problem, checked live.
    pub name_error: Option<String>,
    pub default_category: SelectView<Option<CategoryId>>,
    /// The chips, as stored.
    pub rules: Vec<String>,
    /// The neutral chip shown while there are none.
    pub no_rules: String,
    /// The rule input, labelled as the rules field.
    pub rule_input: TextFieldView,
    pub rule_add: String,
    /// Why the last rule or confirm was refused.
    pub rule_error: Option<String>,
    pub callout: Callout,
    /// Edit, rather than Add.
    pub editing: bool,
    pub focused: PayeeField,
}

/// The note under the fields: how rules match, or on Edit, while Splits carry the Payee, the
/// handoff's usage warning in its place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Callout {
    pub text: String,
    pub warning: bool,
}

/// The Payee an Edit saves: its id, the stored name the title keeps while Name is retyped, and
/// how many Splits carry it.
struct Editing {
    id: PayeeId,
    name: String,
    splits: usize,
}

pub struct PayeeFormPresenter {
    ports: SharedPorts,
    editing: Option<Editing>,
    categories: Vec<Choice<Option<CategoryId>>>,
    name: String,
    category: Option<CategoryId>,
    rules: Vec<String>,
    rule_input: String,
    focused: PayeeField,
    error: Option<PayeeError>,
}

impl PayeeFormPresenter {
    /// A fresh Add form: no name, no Default category, no rules.
    pub fn add(ports: SharedPorts) -> Self {
        let categories = Self::choices(&ports);
        Self {
            ports,
            editing: None,
            categories,
            name: String::new(),
            category: None,
            rules: Vec::new(),
            rule_input: String::new(),
            focused: PayeeField::Name,
            error: None,
        }
    }

    /// The Edit form pre-filled from Payee `id`, or `None` when it is gone. A Default category
    /// that is no longer a leaf opens as none. A use that can't be counted shows no callout: it only
    /// says how many transactions a new default category leaves as they are, and saving is safe
    /// whatever names the Payee.
    pub fn edit(ports: SharedPorts, id: PayeeId) -> Option<Self> {
        let (payee, splits) = {
            let ports = ports.borrow();
            (
                ports.payees().payee(id)?,
                ports.usage().payee_usage(id).map_or(0, |uses| uses.splits),
            )
        };
        let mut form = Self::add(ports);
        form.category = payee
            .default_category
            .filter(|chosen| form.categories.iter().any(|c| c.value == Some(*chosen)));
        form.name = payee.name.clone();
        form.rules = payee.aliases;
        form.editing = Some(Editing {
            id,
            name: payee.name,
            splits,
        });
        Some(form)
    }

    fn choices(ports: &SharedPorts) -> Vec<Choice<Option<CategoryId>>> {
        let categories = ports.borrow().categories().payee_categories();
        std::iter::once(Choice {
            value: None,
            label: msg::payees_category_none(),
        })
        .chain(categories.into_iter().map(|category| Choice {
            value: Some(category.id),
            label: category.label,
        }))
        .collect()
    }

    pub fn handle(&mut self, event: PayeeFormEvent, router: &mut dyn PayeesRouter) {
        match event {
            PayeeFormEvent::Name(text) => {
                self.name = text;
                self.error = None;
            }
            PayeeFormEvent::RuleInput(text) => {
                self.rule_input = text;
                self.error = None;
            }
            PayeeFormEvent::Category(id) => {
                if self.categories.iter().any(|choice| choice.value == id) {
                    self.category = id;
                }
            }
            PayeeFormEvent::AddRule => {
                self.focused = PayeeField::Rule;
                self.add_rule();
            }
            PayeeFormEvent::RemoveRule(index) => {
                if index < self.rules.len() {
                    self.rules.remove(index);
                }
            }
            PayeeFormEvent::RemoveLastRule => {
                self.rules.pop();
                self.error = None;
            }
            PayeeFormEvent::NextField | PayeeFormEvent::PrevField => {
                let order = PayeeField::ORDER;
                let at = order.iter().position(|f| *f == self.focused).unwrap_or(0);
                let next = if event == PayeeFormEvent::PrevField {
                    at + order.len() - 1
                } else {
                    at + 1
                };
                self.focused = order[next % order.len()];
            }
            PayeeFormEvent::Focus(field) => self.focused = field,
            PayeeFormEvent::Confirm => self.confirm(router),
            PayeeFormEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> PayeeFormViewModel {
        let (title, submit, placeholder) = match &self.editing {
            Some(editing) => (
                msg::payees_edit_title(&editing.name),
                msg::payees_edit_submit(),
                msg::payees_edit_rule_placeholder(),
            ),
            None => (
                msg::payees_add_title(),
                msg::payees_add_submit(),
                msg::payees_rule_placeholder(),
            ),
        };
        let callout = match &self.editing {
            Some(editing) if editing.splits > 0 => Callout {
                text: msg::payees_edit_usage_callout(count(editing.splits)),
                warning: true,
            },
            _ => Callout {
                text: msg::payees_rules_callout(),
                warning: false,
            },
        };
        PayeeFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.clean_name().is_ok(),
                destructive: false,
                hints: Vec::new(),
            },
            name: TextFieldView {
                label: lib_locale::msg::column_name(),
                text: self.name.clone(),
                placeholder: msg::payees_name_placeholder(),
                focused: self.focused == PayeeField::Name,
            },
            name_error: match self.clean_name() {
                Err(PayeeError::NameRequired) | Ok(_) => None,
                Err(error) => Some(refusal_text(&error)),
            },
            default_category: SelectView {
                label: msg::payees_field_default_category(),
                options: self.categories.clone(),
                chosen: Some(self.category),
                focused: self.focused == PayeeField::DefaultCategory,
            },
            rules: self.rules.clone(),
            no_rules: msg::payees_no_rules(),
            rule_input: TextFieldView {
                label: msg::payees_field_match_rules(),
                text: self.rule_input.clone(),
                placeholder,
                focused: self.focused == PayeeField::Rule,
            },
            rule_add: msg::payees_rule_add("+"),
            rule_error: self.error.as_ref().map(refusal_text),
            callout,
            editing: self.editing.is_some(),
            focused: self.focused,
        }
    }

    fn own_id(&self) -> Option<PayeeId> {
        self.editing.as_ref().map(|editing| editing.id)
    }

    fn clean_name(&self) -> Result<String, PayeeError> {
        let payees = self.ports.borrow().payees().payees();
        clean_name(&payees, self.own_id(), &self.name)
    }

    /// Turns the rule input into a chip: normalised, ignored when blank or already a chip, and
    /// refused, kept in the input, when another Payee owns it.
    fn add_rule(&mut self) {
        let Some(rule) = normalise_alias(&self.rule_input) else {
            self.rule_input.clear();
            return;
        };
        let payees = self.ports.borrow().payees().payees();
        if let Some(owner) = alias_owner(&payees, &rule).filter(|p| Some(p.id) != self.own_id()) {
            self.error = Some(PayeeError::AliasTaken {
                alias: rule,
                owner: owner.name.clone(),
            });
            return;
        }
        if !self.rules.contains(&rule) {
            self.rules.push(rule);
        }
        self.rule_input.clear();
        self.error = None;
    }

    fn confirm(&mut self, router: &mut dyn PayeesRouter) {
        if self.clean_name().is_err() {
            return;
        }
        let draft = PayeeDraft {
            name: self.name.clone(),
            default_category: self.category,
            aliases: self.rules.clone(),
        };
        let saved = {
            let mut ports = self.ports.borrow_mut();
            let payees = ports.split_mut().payees;
            match self.own_id() {
                Some(id) => EditPayee { id, draft }.run(payees),
                None => AddPayee(draft).run(payees),
            }
        };
        match saved {
            Ok(payee) => router.close_dialog(Some(PayeesOutcome {
                change: PayeeChange::Saved,
                id: payee.id,
                name: payee.name,
            })),
            Err(refusal) => self.error = Some(refusal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::{PayeeRepository as _, PayeeUse};

    fn send(form: &mut PayeeFormPresenter, events: Vec<PayeeFormEvent>) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            form.handle(event, &mut router);
        }
        router.calls
    }

    fn name(text: &str) -> PayeeFormEvent {
        PayeeFormEvent::Name(text.to_string())
    }

    fn rule(text: &str) -> PayeeFormEvent {
        PayeeFormEvent::RuleInput(text.to_string())
    }

    #[test]
    fn a_taken_name_shows_under_the_field_and_a_blank_one_only_waits() {
        let (_, shared) = stub();
        let mut form = PayeeFormPresenter::add(shared);
        let view = form.view_model();
        assert_eq!(view.frame.title, "Add payee");
        assert!(!view.frame.can_confirm);
        assert_eq!(view.name_error, None);

        send(&mut form, vec![name("coles")]);
        let view = form.view_model();
        assert_eq!(view.name_error.as_deref(), Some("Coles already exists"));
        assert!(!view.frame.can_confirm);
        assert_eq!(send(&mut form, vec![PayeeFormEvent::Confirm]), []);

        send(&mut form, vec![name("woolies")]);
        assert_eq!(
            form.view_model().name_error.as_deref(),
            Some("WOOLIES is already a match rule on Woolworths")
        );
    }

    #[test]
    fn rules_are_normalised_deduped_and_another_payees_is_refused_in_the_input() {
        let (_, shared) = stub();
        let mut form = PayeeFormPresenter::add(shared);

        send(
            &mut form,
            vec![
                rule(" aussie candle "),
                PayeeFormEvent::AddRule,
                rule("AUSSIE CANDLE"),
                PayeeFormEvent::AddRule,
                rule("   "),
                PayeeFormEvent::AddRule,
            ],
        );
        let view = form.view_model();
        assert_eq!(view.rules, ["AUSSIE CANDLE"]);
        assert_eq!(view.rule_input.text, "");
        assert!(view.rule_input.focused);

        send(&mut form, vec![rule("woolies"), PayeeFormEvent::AddRule]);
        let view = form.view_model();
        assert_eq!(view.rules, ["AUSSIE CANDLE"]);
        assert_eq!(view.rule_input.text, "woolies");
        assert_eq!(
            view.rule_error.as_deref(),
            Some("WOOLIES is already a match rule on Woolworths")
        );
        send(&mut form, vec![rule("woolie")]);
        assert_eq!(form.view_model().rule_error, None, "typing clears it");

        send(&mut form, vec![PayeeFormEvent::RemoveLastRule]);
        assert!(form.view_model().rules.is_empty());
    }

    #[test]
    fn an_add_saves_the_draft_and_closes_with_the_new_payee() {
        let (ports, shared) = stub();
        let mut form = PayeeFormPresenter::add(shared);
        let groceries = Some(CategoryId::seed(7));

        let calls = send(
            &mut form,
            vec![
                name("Corner Shop"),
                PayeeFormEvent::Category(groceries),
                rule("corner"),
                PayeeFormEvent::AddRule,
                PayeeFormEvent::Confirm,
            ],
        );

        let [RouterCall::CloseDialog(Some(outcome))] = calls.as_slice() else {
            panic!("expected one close, got {calls:?}");
        };
        assert_eq!(outcome.change, PayeeChange::Saved);
        let payee = ports.borrow().payees.payee(outcome.id).expect("added");
        assert_eq!(payee.name, "Corner Shop");
        assert_eq!(payee.default_category, groceries);
        assert_eq!(payee.aliases, ["CORNER"]);
    }

    #[test]
    fn the_default_category_offers_none_then_every_leaf_and_ignores_others() {
        let (_, shared) = stub();
        let mut form = PayeeFormPresenter::add(shared);
        let view = form.view_model().default_category;

        assert_eq!(view.options[0].value, None);
        assert_eq!(view.options[0].label, "— none, ask every time —");
        assert_eq!(view.options.len(), 10);
        assert_eq!(view.chosen, Some(None));
        assert_eq!(view.label, "Default category (optional)");

        send(
            &mut form,
            vec![PayeeFormEvent::Category(Some(CategoryId::seed(6)))],
        );
        assert_eq!(
            form.view_model().default_category.chosen,
            Some(None),
            "Food is a parent, so it isn't offered"
        );
    }

    #[test]
    fn edit_prefills_the_payee_and_warns_while_splits_carry_it() {
        let (ports, shared) = stub();
        ports.borrow_mut().usage.uses.insert(
            PayeeId::seed(1),
            PayeeUse {
                splits: 4,
                bill_plans: 0,
            },
        );
        let mut form = PayeeFormPresenter::edit(shared.clone(), PayeeId::seed(1)).expect("stub");
        let view = form.view_model();
        assert_eq!(view.frame.title, "Edit payee — Woolworths");
        assert_eq!(view.name.text, "Woolworths");
        assert_eq!(
            view.default_category.chosen,
            Some(Some(CategoryId::seed(7)))
        );
        assert_eq!(view.rules, ["WOOLWORTHS", "WW SUPERMARKET", "WOOLIES"]);
        assert!(view.callout.warning);
        assert!(view.callout.text.starts_with("Used by 4 transactions."));
        assert!(view.frame.can_confirm, "its own name isn't taken");

        send(
            &mut form,
            vec![name("Woolies Metro"), PayeeFormEvent::Confirm],
        );
        let payee = ports.borrow().payees.payee(PayeeId::seed(1)).expect("kept");
        assert_eq!(payee.name, "Woolies Metro");
        assert_eq!(
            form.view_model().frame.title,
            "Edit payee — Woolworths",
            "the title keeps the stored name"
        );
        assert!(PayeeFormPresenter::edit(shared, PayeeId::new()).is_none());
    }

    #[test]
    fn a_payee_gone_under_the_edit_is_refused_onto_the_form() {
        let (ports, shared) = stub();
        let mut form = PayeeFormPresenter::edit(shared, PayeeId::seed(8)).expect("stub");
        ports.borrow_mut().payees.delete_payee(PayeeId::seed(8));

        assert_eq!(send(&mut form, vec![PayeeFormEvent::Confirm]), []);
        assert_eq!(
            form.view_model().rule_error.as_deref(),
            Some("This payee has since been deleted.")
        );
    }

    #[test]
    fn the_dialog_keymap_keys_move_focus_both_ways_and_cancel_closes() {
        let (ports, shared) = stub();
        let before = ports.borrow().clone();
        let mut form = PayeeFormPresenter::add(shared);
        let focused = |form: &PayeeFormPresenter| form.view_model().focused;

        send(&mut form, vec![PayeeFormEvent::NextField]);
        assert_eq!(focused(&form), PayeeField::DefaultCategory);
        assert!(form.view_model().default_category.focused);
        send(&mut form, vec![PayeeFormEvent::NextField]);
        assert_eq!(focused(&form), PayeeField::Rule);
        send(&mut form, vec![PayeeFormEvent::NextField]);
        assert_eq!(focused(&form), PayeeField::Name);
        send(&mut form, vec![PayeeFormEvent::PrevField]);
        assert_eq!(focused(&form), PayeeField::Rule);

        assert_eq!(
            send(&mut form, vec![name("Corner Shop"), PayeeFormEvent::Cancel]),
            [RouterCall::CloseDialog(None)]
        );
        assert_eq!(*ports.borrow(), before, "a cancel writes nothing");
    }
}
