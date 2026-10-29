//! The Add and Edit institution Dialog (2e, 16e): Name, Products as a multi-select and the Usual
//! unit as a select. Edit is the same form opened on one Institution, pre-filled, its Products
//! split back into chips.
//!
//! - Name is the only text field, so it holds the typing; `Tab` moves between it and the Usual
//!   unit select.
//! - The Usual unit is optional: a None choice, then the active Units. An Institution already
//!   naming a Unit since deactivated keeps it listed, so Edit never drops what it cannot show.
//! - Confirming needs a name and at least one Product. The Products join in the order picked, with
//!   the same separator the table's cells use.
//! - A refusal from the Interactor shows under the name until the name is edited.
//! - A confirm raises its own Toast before it closes the Dialog.

use lib_toast::{Notice, ToastKind};
use lib_viper::view::{Choice, DialogFrame, SelectView, TextFieldView};

use lib_core::{InstitutionId, UnitId};

use crate::interactor::{AddInstitution, EditInstitution};
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

    /// The part stored in an Institution's label: the same in every Locale, so a Ledger made in one
    /// Locale reads the same in another. The chip's own text is `label`.
    pub const fn canonical(self) -> &'static str {
        match self {
            Self::Savings => "savings",
            Self::Offset => "offset",
            Self::CreditCard => "credit card",
            Self::Loan => "loan",
            Self::Investment => "investment",
            Self::Crypto => "crypto",
            Self::Retirement => "retirement",
            Self::Cash => "cash",
        }
    }

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

/// The fields that take focus, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstitutionField {
    Name,
    UsualUnit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionFormEvent {
    /// The name field's whole new text, as its View's input holds it.
    Name(String),
    /// The Usual unit picked, `None` for none. A Unit that is not an option is ignored.
    UsualUnit(Option<UnitId>),
    Focus(InstitutionField),
    NextField,
    PrevField,
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
    pub usual_unit: SelectView<Option<UnitId>>,
    pub focused: InstitutionField,
    /// Why the last confirm was refused, until the name changes.
    pub name_error: Option<String>,
    pub products_label: String,
    pub products: Vec<ProductChip>,
}

pub struct InstitutionFormPresenter {
    ports: SharedPorts,
    /// The Institution being edited, `None` when adding.
    editing: Option<InstitutionId>,
    name: String,
    /// The label's parts in stored order. A part that is no chip is kept as it was, so Edit never
    /// drops or reorders what it cannot show.
    products: Vec<String>,
    /// The Usual unit picked, `None` for none.
    usual_unit: Option<UnitId>,
    /// The Usual unit's options, read when the Dialog opens: None, then the active Units.
    units: Vec<Choice<Option<UnitId>>>,
    focused: InstitutionField,
    /// The last refusal, with the name it refused.
    refusal: Option<(String, InstitutionError)>,
}

impl InstitutionFormPresenter {
    /// A fresh Add form: no name and Savings checked.
    pub fn add(ports: SharedPorts) -> Self {
        let units = unit_choices(&ports, None);
        Self {
            ports,
            editing: None,
            name: String::new(),
            products: vec![InstitutionProduct::Savings.canonical().to_string()],
            usual_unit: None,
            units,
            focused: InstitutionField::Name,
            refusal: None,
        }
    }

    /// An Edit form on Institution `id`, filled with its name and Products. `None` when it is no
    /// longer on the Ledger.
    pub fn edit(ports: SharedPorts, id: InstitutionId) -> Option<Self> {
        let institution = ports.borrow().institutions().institution(id)?;
        let products = institution
            .products
            .split(PRODUCTS_SEPARATOR)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
        let units = unit_choices(&ports, institution.usual_unit);
        Some(Self {
            ports,
            editing: Some(id),
            name: institution.name,
            products,
            usual_unit: institution.usual_unit,
            units,
            focused: InstitutionField::Name,
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: InstitutionFormEvent, router: &mut dyn InstitutionsRouter) {
        match event {
            InstitutionFormEvent::Name(text) => self.name = text,
            InstitutionFormEvent::UsualUnit(unit) => {
                if self.units.iter().any(|choice| choice.value == unit) {
                    self.usual_unit = unit;
                }
            }
            InstitutionFormEvent::Focus(field) => self.focused = field,
            InstitutionFormEvent::NextField | InstitutionFormEvent::PrevField => {
                self.focused = match self.focused {
                    InstitutionField::Name => InstitutionField::UsualUnit,
                    InstitutionField::UsualUnit => InstitutionField::Name,
                };
            }
            InstitutionFormEvent::ToggleProduct(product) => {
                match self
                    .products
                    .iter()
                    .position(|chosen| chosen == product.canonical())
                {
                    Some(at) => {
                        self.products.remove(at);
                    }
                    None => self.products.push(product.canonical().to_string()),
                }
            }
            InstitutionFormEvent::Confirm => self.confirm(router),
            InstitutionFormEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> InstitutionFormViewModel {
        InstitutionFormViewModel {
            frame: DialogFrame {
                title: if self.editing.is_some() {
                    msg::institutions_edit_title()
                } else {
                    msg::institutions_add_title()
                },
                submit: if self.editing.is_some() {
                    msg::institutions_edit_submit()
                } else {
                    msg::institutions_add_submit()
                },
                can_confirm: self.is_valid(),
                destructive: false,
                hints: Vec::new(),
            },
            name: TextFieldView {
                label: msg::institutions_name_label(),
                text: self.name.clone(),
                placeholder: msg::institutions_name_placeholder(),
                focused: self.focused == InstitutionField::Name,
            },
            usual_unit: SelectView {
                label: msg::institutions_usual_unit_label(),
                options: self.units.clone(),
                chosen: Some(self.usual_unit),
                focused: self.focused == InstitutionField::UsualUnit,
            },
            focused: self.focused,
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
                    checked: self.products.iter().any(|part| part == product.canonical()),
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
            products: self.products.join(PRODUCTS_SEPARATOR),
            usual_unit: self.usual_unit,
        };
        let saved = {
            let mut ports = self.ports.borrow_mut();
            match self.editing {
                Some(id) => EditInstitution { id, draft }.run(ports.institutions_mut()),
                None => AddInstitution(draft).run(ports.institutions_mut()),
            }
        };
        match saved {
            Ok(institution) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    if self.editing.is_some() {
                        msg::institutions_edit_toast(&institution.name)
                    } else {
                        msg::institutions_add_toast(&institution.name)
                    },
                ));
                router.close_dialog(Some(InstitutionsOutcome {
                    id: institution.id,
                    name: institution.name,
                    next: None,
                }));
            }
            Err(refusal) => self.refusal = Some((self.name.clone(), refusal)),
        }
    }
}

/// The Usual unit's options: None, then each active Unit, then `named` if it is a deactivated Unit
/// an Institution already names.
fn unit_choices(ports: &SharedPorts, named: Option<UnitId>) -> Vec<Choice<Option<UnitId>>> {
    let units = ports.borrow().units().institution_units();
    std::iter::once(Choice {
        value: None,
        label: msg::institutions_usual_unit_none(),
    })
    .chain(
        units
            .into_iter()
            .filter(|unit| unit.is_active || Some(unit.id) == named)
            .map(|unit| Choice {
                value: Some(unit.id),
                label: unit.code,
            }),
    )
    .collect()
}

/// What joins the Products into the one label the table's cell shows.
const PRODUCTS_SEPARATOR: &str = " \u{b7} ";

/// A name refusal as the Dialog words it.
fn refusal_text(error: &InstitutionError) -> String {
    match error {
        InstitutionError::DuplicateName(name) => msg::institutions_error_name_taken(name),
        InstitutionError::NameRequired => msg::institutions_error_name_required(),
        InstitutionError::NotFound => msg::institutions_error_not_found(),
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
            .map(InstitutionProduct::canonical)
            .collect();

        for institution in crate::default_institutions() {
            for product in institution.products.split(" \u{b7} ") {
                assert!(
                    labels.contains(&product),
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
                    next: None,
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

    fn edit_form(id: InstitutionId) -> Form {
        let (ports, shared) = stub();
        Form {
            presenter: InstitutionFormPresenter::edit(shared, id).expect("a stub Institution"),
            ports,
            router: RecordingRouter::default(),
        }
    }

    #[test]
    fn edit_opens_filled_with_the_name_and_its_products_as_ticked_chips() {
        let form = edit_form(InstitutionId::SEED_ANZ);
        let view = form.view();

        assert_eq!(view.frame.title, "Edit institution");
        assert_eq!(view.frame.submit, "Save institution");
        assert!(view.frame.can_confirm);
        assert_eq!(view.name.text, "ANZ Banking Group");
        let checked: Vec<_> = view
            .products
            .iter()
            .filter(|chip| chip.checked)
            .map(|chip| chip.product)
            .collect();
        assert_eq!(
            checked,
            [InstitutionProduct::Savings, InstitutionProduct::Offset]
        );
    }

    #[test]
    fn edit_keeps_a_part_that_is_no_chip_and_the_stored_order() {
        let (ports, shared) = stub();
        let id = InstitutionId::SEED_ANZ;
        ports
            .borrow_mut()
            .institutions
            .institutions
            .iter_mut()
            .find(|row| row.id == id)
            .expect("the ANZ row")
            .products = "wealth \u{b7} offset \u{b7} savings".to_string();
        let mut form = Form {
            presenter: InstitutionFormPresenter::edit(shared, id).expect("a stub Institution"),
            ports,
            router: RecordingRouter::default(),
        };

        form.send(InstitutionFormEvent::Confirm);
        let saved = form.ports.borrow().institutions.institutions.clone();
        let anz = saved.iter().find(|row| row.id == id).expect("the ANZ row");
        assert_eq!(anz.products, "wealth \u{b7} offset \u{b7} savings");

        form.send(InstitutionFormEvent::ToggleProduct(
            InstitutionProduct::Cash,
        ));
        form.send(InstitutionFormEvent::Confirm);
        let saved = form.ports.borrow().institutions.institutions.clone();
        let anz = saved.iter().find(|row| row.id == id).expect("the ANZ row");
        assert_eq!(
            anz.products,
            "wealth \u{b7} offset \u{b7} savings \u{b7} cash"
        );
    }

    #[test]
    fn the_stored_label_does_not_follow_the_locale_of_its_chips() {
        assert_eq!(InstitutionProduct::CreditCard.canonical(), "credit card");
    }

    #[test]
    fn edit_of_an_institution_that_is_gone_does_not_open() {
        let (_, shared) = stub();

        assert!(InstitutionFormPresenter::edit(shared, InstitutionId::new()).is_none());
    }

    #[test]
    fn confirming_an_edit_rewrites_the_row_toasts_and_closes_on_it() {
        let mut form = edit_form(InstitutionId::SEED_AMEX);
        form.type_name("Amex");
        form.send(InstitutionFormEvent::ToggleProduct(
            InstitutionProduct::Cash,
        ));

        let calls = form.send(InstitutionFormEvent::Confirm);

        let rows = form.ports.borrow().institutions.institutions.clone();
        assert_eq!(rows.len(), 7);
        assert_eq!(rows[2].name, "Amex");
        assert_eq!(rows[2].products, "credit card \u{b7} cash");
        assert_eq!(
            calls,
            vec![
                RouterCall::Notice(Notice::Toast(ToastKind::Success, "Saved Amex".to_string())),
                RouterCall::CloseDialog(Some(InstitutionsOutcome {
                    id: InstitutionId::SEED_AMEX,
                    name: "Amex".to_string(),
                    next: None,
                })),
            ]
        );
    }

    #[test]
    fn an_unchanged_confirm_still_writes_and_toasts() {
        let mut form = edit_form(InstitutionId::SEED_WESTPAC);

        let calls = form.send(InstitutionFormEvent::Confirm);

        assert_eq!(calls.len(), 2);
        assert_eq!(
            form.ports.borrow().institutions.institutions[4].name,
            "Westpac Banking"
        );
    }

    #[test]
    fn a_rename_skips_its_own_id_but_not_another_holders() {
        let mut form = edit_form(InstitutionId::SEED_WESTPAC);

        form.type_name("westpac banking");
        assert_eq!(form.send(InstitutionFormEvent::Confirm).len(), 2);
        assert_eq!(
            form.ports.borrow().institutions.institutions[4].name,
            "westpac banking"
        );

        let mut form = edit_form(InstitutionId::SEED_WESTPAC);
        form.type_name("anz banking group");
        assert!(form.send(InstitutionFormEvent::Confirm).is_empty());
        assert_eq!(
            form.view().name_error.as_deref(),
            Some("ANZ Banking Group already exists")
        );
        assert_eq!(
            form.ports.borrow().institutions.institutions[4].name,
            "Westpac Banking"
        );
    }

    fn usual_labels(form: &Form) -> Vec<String> {
        form.view()
            .usual_unit
            .options
            .into_iter()
            .map(|choice| choice.label)
            .collect()
    }

    #[test]
    fn the_usual_unit_offers_none_then_the_active_units_and_starts_on_none() {
        let form = Form::new();
        form.ports.borrow_mut().units.units[1].is_active = false;
        let form = Form {
            presenter: InstitutionFormPresenter::add(form.ports.clone()),
            ..form
        };

        assert_eq!(usual_labels(&form), ["None", "aud", "vas"]);
        let view = form.view();
        assert_eq!(view.usual_unit.label, "Usual unit");
        assert_eq!(view.usual_unit.chosen, Some(None));
    }

    #[test]
    fn tab_moves_focus_between_the_name_and_the_usual_unit() {
        let mut form = Form::new();
        assert!(form.view().name.focused);

        form.send(InstitutionFormEvent::NextField);
        assert!(form.view().usual_unit.focused);
        assert!(!form.view().name.focused);

        form.send(InstitutionFormEvent::PrevField);
        assert_eq!(form.view().focused, InstitutionField::Name);
    }

    #[test]
    fn a_picked_usual_unit_is_saved_with_a_new_institution() {
        let mut form = Form::new();
        form.type_name("Broker");
        form.send(InstitutionFormEvent::UsualUnit(Some(UnitId::SEED_BTC)));
        assert_eq!(form.view().usual_unit.chosen, Some(Some(UnitId::SEED_BTC)));

        form.send(InstitutionFormEvent::Confirm);

        let added = form
            .ports
            .borrow()
            .institutions
            .institutions
            .last()
            .cloned();
        assert_eq!(
            added.map(|institution| institution.usual_unit),
            Some(Some(UnitId::SEED_BTC))
        );
    }

    #[test]
    fn a_unit_that_is_not_an_option_is_ignored() {
        let mut form = Form::new();

        form.send(InstitutionFormEvent::UsualUnit(Some(UnitId::new())));

        assert_eq!(form.view().usual_unit.chosen, Some(None));
    }

    #[test]
    fn edit_opens_on_the_usual_unit_and_clearing_it_to_none_is_saved() {
        let mut form = edit_form(InstitutionId::SEED_CRYPTO_EXCHANGE);
        assert_eq!(form.view().usual_unit.chosen, Some(Some(UnitId::SEED_BTC)));

        form.send(InstitutionFormEvent::UsualUnit(None));
        form.send(InstitutionFormEvent::Confirm);

        let saved = form.ports.borrow().institutions.institutions.clone();
        let exchange = saved
            .iter()
            .find(|row| row.id == InstitutionId::SEED_CRYPTO_EXCHANGE)
            .expect("the exchange row");
        assert_eq!(exchange.usual_unit, None);
    }

    #[test]
    fn edit_keeps_listing_a_usual_unit_since_deactivated_but_add_does_not() {
        let (ports, shared) = stub();
        ports.borrow_mut().units.units[1].is_active = false;
        let edit =
            InstitutionFormPresenter::edit(shared.clone(), InstitutionId::SEED_CRYPTO_EXCHANGE)
                .expect("a stub Institution");
        let add = InstitutionFormPresenter::add(shared);

        let codes = |presenter: &InstitutionFormPresenter| -> Vec<String> {
            presenter
                .view_model()
                .usual_unit
                .options
                .into_iter()
                .map(|choice| choice.label)
                .collect()
        };
        assert_eq!(codes(&edit), ["None", "aud", "btc", "vas"]);
        assert_eq!(codes(&add), ["None", "aud", "vas"]);
    }

    #[test]
    fn cancel_closes_with_no_outcome() {
        let mut form = Form::new();
        form.type_name("Bank X");

        assert_eq!(
            form.send(InstitutionFormEvent::Cancel),
            vec![RouterCall::CloseDialog(None)]
        );
        assert_eq!(form.ports.borrow().institutions.institutions.len(), 7);
    }
}
