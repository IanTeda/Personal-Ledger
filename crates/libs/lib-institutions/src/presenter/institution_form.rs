//! The Add institution Dialog (2e): Name, Account types as a multi-select, and a Default unit.
//!
//! - Name is the only text field, so it is always the one typed into; there is nothing to `Tab`
//!   between.
//! - Confirming needs a name, at least one Account type and a Default unit. The Account types join
//!   in the order picked, with the same separator the table's cells use. The Default unit is not
//!   part of the Institution yet: it is asked for and kept out of the draft.
//! - A refusal from the Interactor shows under the name until the name is edited.
//! - A confirm raises its own Toast before it closes the Dialog.

use lib_toast::{Notice, ToastKind};
use lib_viper::view::{DialogFrame, TextFieldView};

use crate::interactor::AddInstitution;
use crate::protocols::{InstitutionsOutcome, InstitutionsRouter, SharedPorts};
use crate::{InstitutionDraft, InstitutionError, msg};

/// The Account types the Dialog offers (`docs/ux/desktop-mockups/16-settings/README.md`'s "2e":
/// savings / credit card / offset / loan / investment). Not `lib_core::AccountType`, which classes
/// an Account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstitutionProduct {
    Savings,
    CreditCard,
    Offset,
    Loan,
    Investment,
}

impl InstitutionProduct {
    pub const ALL: [Self; 5] = [
        Self::Savings,
        Self::CreditCard,
        Self::Offset,
        Self::Loan,
        Self::Investment,
    ];

    pub fn label(self) -> String {
        match self {
            Self::Savings => msg::institutions_product_savings(),
            Self::CreditCard => lib_locale::msg::account_kind_credit_card(),
            Self::Offset => msg::institutions_product_offset(),
            Self::Loan => lib_locale::msg::account_kind_loan(),
            Self::Investment => lib_locale::msg::account_kind_investment(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionFormEvent {
    /// The name field's whole new text, as its View's input holds it.
    Name(String),
    /// A chip: checked becomes unchecked and back.
    ToggleProduct(InstitutionProduct),
    /// A Default unit, by code.
    Unit(String),
    Confirm,
    Cancel,
}

/// One Account type chip as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductChip {
    pub product: InstitutionProduct,
    pub label: String,
    pub checked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionFormViewModel {
    pub frame: DialogFrame,
    pub name: TextFieldView,
    /// Why the last confirm was refused, until the name changes.
    pub name_error: Option<String>,
    pub products_label: String,
    pub products: Vec<ProductChip>,
    pub unit_label: String,
    /// The Units' codes, in order.
    pub units: Vec<String>,
    pub chosen_unit: Option<String>,
    /// What shows in place of the Units while there are none.
    pub no_units: String,
}

pub struct InstitutionFormPresenter {
    ports: SharedPorts,
    name: String,
    products: Vec<InstitutionProduct>,
    unit: Option<String>,
    /// The last refusal, with the name it refused.
    refusal: Option<(String, InstitutionError)>,
}

impl InstitutionFormPresenter {
    /// A fresh Add form: no name, Savings checked and the first Unit chosen.
    pub fn add(ports: SharedPorts) -> Self {
        let unit = ports.borrow().units().unit_codes().into_iter().next();
        Self {
            ports,
            name: String::new(),
            products: vec![InstitutionProduct::Savings],
            unit,
            refusal: None,
        }
    }

    pub fn handle(&mut self, event: InstitutionFormEvent, router: &mut dyn InstitutionsRouter) {
        match event {
            InstitutionFormEvent::Name(text) => self.name = text,
            InstitutionFormEvent::ToggleProduct(product) => {
                match self.products.iter().position(|chosen| *chosen == product) {
                    Some(at) => {
                        self.products.remove(at);
                    }
                    None => self.products.push(product),
                }
            }
            InstitutionFormEvent::Unit(code) => {
                if self.ports.borrow().units().unit_codes().contains(&code) {
                    self.unit = Some(code);
                }
            }
            InstitutionFormEvent::Confirm => self.confirm(router),
            InstitutionFormEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> InstitutionFormViewModel {
        InstitutionFormViewModel {
            frame: DialogFrame {
                title: msg::institutions_add_title(),
                submit: msg::institutions_add_submit(),
                can_confirm: self.is_valid(),
                destructive: false,
            },
            name: TextFieldView {
                label: msg::institutions_name_label(),
                text: self.name.clone(),
                placeholder: msg::institutions_name_placeholder(),
                focused: true,
            },
            name_error: self
                .refusal
                .as_ref()
                .filter(|(refused, _)| *refused == self.name)
                .map(|(_, refusal)| refusal_text(refusal)),
            products_label: msg::institutions_account_types(),
            products: InstitutionProduct::ALL
                .into_iter()
                .map(|product| ProductChip {
                    product,
                    label: product.label(),
                    checked: self.products.contains(&product),
                })
                .collect(),
            unit_label: msg::institutions_default_unit(),
            units: self.ports.borrow().units().unit_codes(),
            chosen_unit: self.unit.clone(),
            no_units: msg::institutions_no_units(),
        }
    }

    /// A name, at least one Account type and a Default unit: all required.
    fn is_valid(&self) -> bool {
        !self.name.trim().is_empty() && !self.products.is_empty() && self.unit.is_some()
    }

    fn confirm(&mut self, router: &mut dyn InstitutionsRouter) {
        if !self.is_valid() {
            return;
        }
        let draft = InstitutionDraft {
            name: self.name.clone(),
            account_type: self
                .products
                .iter()
                .map(|product| product.label())
                .collect::<Vec<_>>()
                .join(" \u{b7} "),
        };
        let added = {
            let mut ports = self.ports.borrow_mut();
            AddInstitution(draft).run(ports.institutions_mut())
        };
        match added {
            Ok(institution) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::institutions_add_toast(&institution.name),
                ));
                router.close_dialog(Some(InstitutionsOutcome {
                    id: institution.id,
                    name: institution.name,
                }));
            }
            Err(refusal) => self.refusal = Some((self.name.clone(), refusal)),
        }
    }
}

/// A name refusal as the Dialog words it.
fn refusal_text(error: &InstitutionError) -> String {
    match error {
        InstitutionError::DuplicateName(name) => msg::institutions_error_name_taken(name),
        InstitutionError::NameRequired => msg::institutions_error_name_required(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::presenter::test_support::stub;

    struct Form {
        presenter: InstitutionFormPresenter,
        ports: std::rc::Rc<std::cell::RefCell<crate::fake::InMemoryPorts>>,
        router: RecordingRouter,
    }

    impl Form {
        fn new() -> Self {
            let (ports, shared) = stub();
            Self {
                presenter: InstitutionFormPresenter::add(shared),
                ports,
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: InstitutionFormEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> InstitutionFormViewModel {
            self.presenter.view_model()
        }

        fn type_name(&mut self, name: &str) {
            self.send(InstitutionFormEvent::Name(name.to_string()));
        }
    }

    #[test]
    fn a_fresh_form_has_savings_and_the_first_unit_and_cannot_confirm() {
        let form = Form::new();
        let view = form.view();

        let checked: Vec<_> = view
            .products
            .iter()
            .filter(|chip| chip.checked)
            .map(|chip| chip.product)
            .collect();
        assert_eq!(checked, [InstitutionProduct::Savings]);
        assert_eq!(view.chosen_unit.as_deref(), Some("aud"));
        assert!(!view.frame.can_confirm);
        assert_eq!(view.frame.title, "Add institution");
    }

    #[test]
    fn confirming_needs_a_name_a_type_and_a_unit() {
        let mut form = Form::new();
        assert!(form.send(InstitutionFormEvent::Confirm).is_empty());

        form.type_name("ANZ");
        assert!(form.view().frame.can_confirm);
        form.send(InstitutionFormEvent::ToggleProduct(
            InstitutionProduct::Savings,
        ));
        assert!(!form.view().frame.can_confirm);
        form.send(InstitutionFormEvent::ToggleProduct(
            InstitutionProduct::Savings,
        ));
        assert!(form.view().frame.can_confirm);

        form.ports.borrow_mut().units.codes.clear();
        let none = InstitutionFormPresenter::add(form.ports.clone());
        assert!(!none.view_model().frame.can_confirm);
        assert_eq!(none.view_model().chosen_unit, None);
    }

    #[test]
    fn a_unit_not_offered_is_ignored() {
        let mut form = Form::new();

        form.send(InstitutionFormEvent::Unit("btc".into()));
        assert_eq!(form.view().chosen_unit.as_deref(), Some("btc"));
        form.send(InstitutionFormEvent::Unit("xyz".into()));
        assert_eq!(form.view().chosen_unit.as_deref(), Some("btc"));
    }

    #[test]
    fn confirming_adds_the_institution_raises_its_toast_then_closes() {
        let mut form = Form::new();
        form.type_name(" Bank X ");
        form.send(InstitutionFormEvent::ToggleProduct(
            InstitutionProduct::Offset,
        ));

        let calls = form.send(InstitutionFormEvent::Confirm);

        let added = form
            .ports
            .borrow()
            .institutions
            .institutions
            .last()
            .cloned()
            .expect("the added row");
        assert_eq!(added.name, "Bank X");
        assert_eq!(added.account_type, "savings \u{b7} offset");
        assert_eq!(
            calls,
            vec![
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Added Bank X".to_string()
                )),
                RouterCall::CloseDialog(Some(InstitutionsOutcome {
                    id: added.id,
                    name: "Bank X".to_string(),
                })),
            ]
        );
    }

    #[test]
    fn a_taken_name_stays_open_with_its_refusal_until_the_name_changes() {
        let mut form = Form::new();
        let before = form.ports.borrow().institutions.institutions.len();
        form.type_name("anz banking group");

        assert!(form.send(InstitutionFormEvent::Confirm).is_empty());

        assert_eq!(
            form.view().name_error.as_deref(),
            Some("ANZ Banking Group already exists")
        );
        assert_eq!(form.ports.borrow().institutions.institutions.len(), before);
        form.type_name("anz banking group 2");
        assert_eq!(form.view().name_error, None);
    }

    #[test]
    fn cancel_closes_with_no_outcome() {
        let mut form = Form::new();
        form.type_name("Bank X");

        assert_eq!(
            form.send(InstitutionFormEvent::Cancel),
            vec![RouterCall::CloseDialog(None)]
        );
        assert_eq!(form.ports.borrow().institutions.institutions.len(), 6);
    }
}
