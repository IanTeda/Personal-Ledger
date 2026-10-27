//! The Add institution Dialog (2e): Name, and Products as a multi-select.
//!
//! - Name is the only text field, so it is always the one typed into; there is nothing to `Tab`
//!   between.
//! - Confirming needs a name and at least one Product. The Products join in the order picked, with
//!   the same separator the table's cells use.
//! - A refusal from the Interactor shows under the name until the name is edited.
//! - A confirm raises its own Toast before it closes the Dialog.

use lib_toast::{Notice, ToastKind};
use lib_viper::view::{DialogFrame, TextFieldView};

use crate::interactor::AddInstitution;
use crate::protocols::{InstitutionsOutcome, InstitutionsRouter, SharedPorts};
use crate::{InstitutionDraft, InstitutionError, msg};

/// The Products the Dialog offers (`GLOSSARY.md`'s Products), enough that every seeded
/// Institution's label is made of them. Not `lib_core::AccountType`, which classes an Account:
/// a Product never limits which Accounts may name the Institution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstitutionProduct {
    Savings,
    Offset,
    CreditCard,
    Loan,
    Investment,
    Crypto,
    Retirement,
    Cash,
}

impl InstitutionProduct {
    /// In the order the Dialog draws them.
    pub const ALL: [Self; 8] = [
        Self::Savings,
        Self::Offset,
        Self::CreditCard,
        Self::Loan,
        Self::Investment,
        Self::Crypto,
        Self::Retirement,
        Self::Cash,
    ];

    pub fn label(self) -> String {
        match self {
            Self::Savings => msg::institutions_product_savings(),
            Self::Offset => msg::institutions_product_offset(),
            Self::CreditCard => msg::institutions_product_credit_card(),
            Self::Loan => msg::institutions_product_loan(),
            Self::Investment => msg::institutions_product_investment(),
            Self::Crypto => msg::institutions_product_crypto(),
            Self::Retirement => msg::institutions_product_retirement(),
            Self::Cash => msg::institutions_product_cash(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionFormEvent {
    /// The name field's whole new text, as its View's input holds it.
    Name(String),
    /// A chip: checked becomes unchecked and back.
    ToggleProduct(InstitutionProduct),
    Confirm,
    Cancel,
}

/// One Product chip as drawn.
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
}

pub struct InstitutionFormPresenter {
    ports: SharedPorts,
    name: String,
    products: Vec<InstitutionProduct>,
    /// The last refusal, with the name it refused.
    refusal: Option<(String, InstitutionError)>,
}

impl InstitutionFormPresenter {
    /// A fresh Add form: no name and Savings checked.
    pub fn add(ports: SharedPorts) -> Self {
        Self {
            ports,
            name: String::new(),
            products: vec![InstitutionProduct::Savings],
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
            products_label: msg::institutions_products(),
            products: InstitutionProduct::ALL
                .into_iter()
                .map(|product| ProductChip {
                    product,
                    label: product.label(),
                    checked: self.products.contains(&product),
                })
                .collect(),
        }
    }

    /// A name and at least one Product: both required.
    fn is_valid(&self) -> bool {
        !self.name.trim().is_empty() && !self.products.is_empty()
    }

    fn confirm(&mut self, router: &mut dyn InstitutionsRouter) {
        if !self.is_valid() {
            return;
        }
        let draft = InstitutionDraft {
            name: self.name.clone(),
            products: self
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
    fn a_fresh_form_has_savings_and_cannot_confirm() {
        let form = Form::new();
        let view = form.view();

        let checked: Vec<_> = view
            .products
            .iter()
            .filter(|chip| chip.checked)
            .map(|chip| chip.product)
            .collect();
        assert_eq!(checked, [InstitutionProduct::Savings]);
        assert!(!view.frame.can_confirm);
        assert_eq!(view.frame.title, "Add institution");
    }

    #[test]
    fn the_form_offers_the_eight_products_in_order() {
        let labels: Vec<_> = Form::new()
            .view()
            .products
            .into_iter()
            .map(|chip| chip.label)
            .collect();

        assert_eq!(
            labels,
            [
                "savings",
                "offset",
                "credit card",
                "loan",
                "investment",
                "crypto",
                "retirement",
                "cash",
            ]
        );
        assert_eq!(Form::new().view().products_label, "Products");
    }

    #[test]
    fn every_seeded_institutions_products_are_chips() {
        let labels: Vec<_> = InstitutionProduct::ALL
            .into_iter()
            .map(InstitutionProduct::label)
            .collect();

        for institution in crate::default_institutions() {
            for product in institution.products.split(" \u{b7} ") {
                assert!(
                    labels.iter().any(|label| label == product),
                    "{product} of {} is not a chip",
                    institution.name
                );
            }
        }
    }

    #[test]
    fn confirming_needs_a_name_and_a_product() {
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
        assert_eq!(added.products, "savings \u{b7} offset");
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
