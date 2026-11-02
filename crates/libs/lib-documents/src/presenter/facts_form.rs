//! The Edit extracted facts Dialog, opened from the Inbox: an Unfiled Document's merchant, date,
//! total and Document Type, as the Suggested Link reads them. Any of the first three may be left
//! blank, and a blank total makes the Document Unreadable; a date or total that can't be read says
//! why and refuses Save.

use chrono::NaiveDate;
use lib_core::{DocumentId, Money};
use lib_locale::format::{format_date_input, parse_optional_date};
use lib_toast::{Notice, ToastKind};
use lib_viper::keymap::Keymap;
use lib_viper::view::{Choice, DialogFrame, Hint, SelectView, TextFieldView};

use super::DialogPorts;
use super::document_form::{document, form_legend};
use crate::interactor::SetFacts;
use crate::protocols::{DocumentsOutcome, DocumentsRouter};
use crate::types::{DocumentTypeRow, fallback_id};
use crate::{DocumentType, ExtractedFacts, msg};

/// The form's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FactsField {
    #[default]
    Merchant,
    Date,
    Total,
    Type,
}

impl FactsField {
    const ORDER: [Self; 4] = [Self::Merchant, Self::Date, Self::Total, Self::Type];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactsFormEvent {
    /// A text field's whole new text, as its View's input holds it.
    Merchant(String),
    Date(String),
    Total(String),
    /// An option chosen on the Document type select.
    Type(DocumentType),
    /// `Tab`/`Shift-Tab`, wrapping.
    NextField,
    PrevField,
    /// A click on a field.
    Focus(FactsField),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactsFormViewModel {
    pub frame: DialogFrame,
    pub merchant: TextFieldView,
    pub date: TextFieldView,
    pub date_error: Option<String>,
    pub total: TextFieldView,
    pub total_error: Option<String>,
    pub doc_type: SelectView<DocumentType>,
    pub note: String,
    pub focused: FactsField,
}

pub struct FactsFormPresenter {
    ports: DialogPorts,
    id: DocumentId,
    title: String,
    types: Vec<DocumentTypeRow>,
    merchant: String,
    date: String,
    total: String,
    doc_type: DocumentType,
    focused: FactsField,
}

impl FactsFormPresenter {
    /// The form pre-filled from Document `id`'s Extracted Facts, or `None` when it has gone.
    pub fn new(ports: DialogPorts, id: DocumentId) -> Option<Self> {
        let document = document(&ports, id)?;
        let types = ports.documents.borrow().document_types();
        let facts = document
            .intake
            .map(|intake| intake.facts)
            .unwrap_or_default();
        Some(Self {
            id,
            title: document.title,
            doc_type: facts.doc_type.unwrap_or(DocumentType(fallback_id(&types))),
            types,
            merchant: facts.merchant.unwrap_or_default(),
            date: facts
                .date
                .map(|date| format_date_input(date, ports.date_style))
                .unwrap_or_default(),
            total: facts
                .total
                .map(|total| total.0.abs().with_scale(2).to_string())
                .unwrap_or_default(),
            focused: FactsField::Merchant,
            ports,
        })
    }

    pub fn handle(&mut self, event: FactsFormEvent, router: &mut dyn DocumentsRouter) {
        match event {
            FactsFormEvent::Merchant(text) => self.merchant = text,
            FactsFormEvent::Date(text) => self.date = text,
            FactsFormEvent::Total(text) => self.total = text,
            FactsFormEvent::Type(kind) => {
                self.focused = FactsField::Type;
                self.doc_type = kind;
            }
            FactsFormEvent::NextField => self.cycle_focus(false),
            FactsFormEvent::PrevField => self.cycle_focus(true),
            FactsFormEvent::Focus(field) => self.focused = field,
            FactsFormEvent::Confirm => self.confirm(router),
            FactsFormEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> FactsFormViewModel {
        let date_error = self.parse_date().err();
        let total_error = self
            .total_unreadable()
            .then(msg::documents_dialog_error_total);
        let text = |label: String, text: &str, field: FactsField| TextFieldView {
            label,
            text: text.to_string(),
            placeholder: String::new(),
            focused: self.focused == field,
        };
        FactsFormViewModel {
            frame: DialogFrame {
                title: msg::documents_dialog_facts_title(),
                submit: msg::documents_dialog_facts_submit(),
                can_confirm: date_error.is_none() && total_error.is_none(),
                destructive: false,
                hints: Vec::new(),
            },
            merchant: text(
                msg::documents_fact_merchant(),
                &self.merchant,
                FactsField::Merchant,
            ),
            date: text(
                msg::documents_dialog_field_date(),
                &self.date,
                FactsField::Date,
            ),
            // A date the parser can't place but gives no hint for is left unsaid.
            date_error: date_error.filter(|hint| !hint.is_empty()),
            total: text(msg::documents_fact_total(), &self.total, FactsField::Total),
            total_error,
            doc_type: SelectView {
                label: msg::documents_dialog_field_type(),
                options: self
                    .types
                    .iter()
                    .map(|row| Choice {
                        value: DocumentType(row.id),
                        label: row.name.clone(),
                    })
                    .collect(),
                chosen: Some(self.doc_type),
                focused: self.focused == FactsField::Type,
            },
            note: msg::documents_dialog_facts_note(),
            focused: self.focused,
        }
    }

    /// The status line's legend: cancel, confirm and the key that moves field.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        form_legend(keymap)
    }

    fn cycle_focus(&mut self, backward: bool) {
        let order = FactsField::ORDER;
        let index = order
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        let next = if backward {
            (index + order.len() - 1) % order.len()
        } else {
            (index + 1) % order.len()
        };
        self.focused = order[next];
    }

    fn parse_date(&self) -> Result<Option<NaiveDate>, String> {
        parse_optional_date(&self.date, self.ports.today, self.ports.date_style)
    }

    fn total_unreadable(&self) -> bool {
        !self.total.trim().is_empty() && parse_total(&self.total).is_none()
    }

    /// The form's values as Extracted Facts, or `None` while a field can't be read.
    fn facts(&self) -> Option<ExtractedFacts> {
        let merchant = self.merchant.trim();
        let total = self.total.trim();
        Some(ExtractedFacts {
            merchant: (!merchant.is_empty()).then(|| merchant.to_string()),
            date: self.parse_date().ok()?,
            total: if total.is_empty() {
                None
            } else {
                Some(parse_total(total)?)
            },
            doc_type: Some(self.doc_type),
        })
    }

    fn confirm(&mut self, router: &mut dyn DocumentsRouter) {
        let Some(facts) = self.facts() else {
            return;
        };
        let saved = SetFacts { id: self.id, facts }.run(&mut *self.ports.documents.borrow_mut());
        match saved {
            Ok(()) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::documents_dialog_toast_facts_saved(&self.title),
                ));
                router.close_dialog(Some(DocumentsOutcome::FactsSaved(self.id)));
            }
            // Filed or gone while the Dialog was open: there are no facts left to edit.
            Err(_) => router.close_dialog(None),
        }
    }
}

/// A typed total: digits with an optional decimal point, thousands commas and a leading currency
/// symbol or sign tolerated. Only the size matters to matching, so the sign is dropped.
pub fn parse_total(text: &str) -> Option<Money> {
    let cleaned: String = text
        .trim()
        .trim_start_matches(['-', '+', '$'])
        .chars()
        .filter(|ch| *ch != ',')
        .collect();
    if cleaned.is_empty() || !cleaned.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return None;
    }
    cleaned.parse::<Money>().ok()
}

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;

    use super::super::test_support::{dialog_ports, world};
    use super::*;
    use crate::fake::{DocumentsCall, InMemoryFiles, RecordingDocumentsRouter};

    /// The Woolworths receipt the Inbox reads a merchant, date and total off.
    const RECEIPT: &str = "IMG_4471.jpg";

    fn facts() -> (
        FactsFormPresenter,
        DialogPorts,
        RecordingDocumentsRouter,
        DocumentId,
    ) {
        let ports = dialog_ports(InMemoryFiles::default());
        let id = world().file(RECEIPT).id;
        (
            FactsFormPresenter::new(ports.clone(), id).expect("the receipt is Unfiled"),
            ports,
            RecordingDocumentsRouter::default(),
            id,
        )
    }

    #[test]
    fn a_typed_total_tolerates_symbols_commas_and_a_sign() {
        let cents = |text: &str| parse_total(text).map(|money| money.0.abs().with_scale(2));
        let expected = Some(BigDecimal::new(21_240.into(), 2));
        assert_eq!(cents("212.40"), expected);
        assert_eq!(cents("$212.40"), expected);
        assert_eq!(cents("-212.40"), expected);
        assert_eq!(cents("1,212.40"), Some(BigDecimal::new(121_240.into(), 2)));
        assert_eq!(parse_total("twelve"), None);
        assert_eq!(parse_total(""), None);
        assert_eq!(parse_total("1.2.3"), None);
    }

    #[test]
    fn the_form_opens_on_what_was_read() {
        let (presenter, ..) = facts();
        let view = presenter.view_model();
        assert_eq!(view.merchant.text, "Woolworths Metro");
        assert_eq!(view.total.text, "212.40");
        assert_eq!(view.doc_type.chosen, Some(DocumentType::RECEIPTS));
        assert!(view.frame.can_confirm);
        assert_eq!(view.focused, FactsField::Merchant);
    }

    #[test]
    fn an_unreadable_total_says_why_and_refuses() {
        let (mut presenter, _, mut router, _) = facts();
        presenter.handle(FactsFormEvent::Total("abc".to_string()), &mut router);
        let view = presenter.view_model();
        assert_eq!(
            view.total_error.as_deref(),
            Some("Enter an amount such as 212.40.")
        );
        assert!(!view.frame.can_confirm);
        presenter.handle(FactsFormEvent::Confirm, &mut router);
        assert!(router.take().is_empty());
    }

    #[test]
    fn tab_walks_the_four_fields_and_wraps() {
        let (mut presenter, _, mut router, _) = facts();
        let mut seen = Vec::new();
        for _ in 0..4 {
            presenter.handle(FactsFormEvent::NextField, &mut router);
            seen.push(presenter.view_model().focused);
        }
        assert_eq!(
            seen,
            [
                FactsField::Date,
                FactsField::Total,
                FactsField::Type,
                FactsField::Merchant
            ]
        );
        presenter.handle(FactsFormEvent::PrevField, &mut router);
        assert_eq!(presenter.view_model().focused, FactsField::Type);
    }

    #[test]
    fn saving_blank_facts_makes_the_document_unreadable_and_says_so() {
        let (mut presenter, ports, mut router, id) = facts();
        presenter.handle(FactsFormEvent::Total(String::new()), &mut router);
        presenter.handle(FactsFormEvent::Date(String::new()), &mut router);
        presenter.handle(FactsFormEvent::Merchant("  ".to_string()), &mut router);

        presenter.handle(FactsFormEvent::Confirm, &mut router);

        let saved = document(&ports, id)
            .and_then(|document| document.intake)
            .expect("still Unfiled")
            .facts;
        assert_eq!(saved.merchant, None);
        assert_eq!(saved.date, None);
        assert_eq!(saved.total, None);
        let calls = router.take();
        assert!(matches!(
            calls.as_slice(),
            [
                DocumentsCall::Notice(Notice::Toast(ToastKind::Success, _)),
                DocumentsCall::CloseDialog(Some(DocumentsOutcome::FactsSaved(saved_id))),
            ] if *saved_id == id
        ));
    }
}
