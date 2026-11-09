//! The Edit extracted facts Dialog, opened from the Inbox: an Unfiled Document's merchant, date,
//! total and Document Type, as the Suggested Link reads them. Any of the first three may be left
//! blank, and a blank total makes the Document Unreadable; a date or total that can't be read says
//! why and refuses Save.

use chrono::NaiveDate;
use lib_core::{DocumentId, Money};
use lib_locale::format::{format_date_input, parse_optional_date};
use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{
    Choice, DialogFrame, Hint, SelectList, SelectMove, SelectView, TextFieldView,
};

use super::DialogPorts;
use super::document_form::{document, form_legend};
use crate::interactor::SetFacts;
use crate::protocols::DocumentsOutcome;
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
    /// A Dialog keymap command or a typed key, meaning what the focused field makes it: on the
    /// Document type select `select`, `Space` and the arrows open its list, move it and commit
    /// it, and typing does nothing; elsewhere `select` confirms a valid form and typing edits the
    /// field. `submit` confirms from any field.
    Input(DialogInput),
    /// A text field's whole new text, as its View's input holds it.
    Merchant(String),
    Date(String),
    Total(String),
    /// A click on a row of the Document type's open list: picks it and closes the list.
    Type(DocumentType),
    /// A click on the Document type field: focuses it and opens or closes its list.
    ToggleTypeList,
    /// Closes the open list without picking: the host's `back`.
    CloseList,
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
    /// The Document type's list: open, and its highlight.
    pub doc_type_list: SelectList,
    pub note: String,
    pub focused: FactsField,
    /// The status line's hints while the Dialog is open.
    pub hints: Vec<Hint>,
}

pub struct FactsFormPresenter {
    /// The Dialog keymap's chords, which its hints show.
    keys: DialogKeys,
    ports: DialogPorts,
    id: DocumentId,
    title: String,
    types: Vec<DocumentTypeRow>,
    merchant: String,
    date: String,
    total: String,
    doc_type: DocumentType,
    focused: FactsField,
    /// The Document type's list.
    list: SelectList,
}

impl FactsFormPresenter {
    /// The form pre-filled from Document `id`'s Extracted Facts, or `None` when it has gone.
    pub fn new(ports: DialogPorts, id: DocumentId, keys: DialogKeys) -> Option<Self> {
        let document = document(&ports, id)?;
        let types = ports.documents.borrow().document_types();
        let facts = document
            .intake
            .map(|intake| intake.facts)
            .unwrap_or_default();
        Some(Self {
            keys,
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
            list: SelectList::default(),
            ports,
        })
    }

    /// What `input` means on the focused field.
    fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<DocumentsOutcome>) {
        match (input, self.focused) {
            (DialogInput::NextField, _) => self.cycle_focus(false),
            (DialogInput::PreviousField, _) => self.cycle_focus(true),
            (DialogInput::Submit, _) => self.confirm(router),
            (
                DialogInput::Select
                | DialogInput::Char(' ')
                | DialogInput::SelectNext
                | DialogInput::SelectPrevious,
                FactsField::Type,
            ) => {
                let options = self.type_choices();
                if let SelectMove::Chose(kind) =
                    self.list.input(input, &options, Some(&self.doc_type))
                {
                    self.doc_type = kind;
                }
            }
            (DialogInput::Select, _) => self.confirm(router),
            (DialogInput::Char(ch), field) => {
                if let Some(text) = self.text_mut(field) {
                    text.push(ch);
                }
            }
            (DialogInput::Backspace, field) => {
                if let Some(text) = self.text_mut(field) {
                    text.pop();
                }
            }
            _ => {}
        }
    }

    /// The text `field` types into; the Type select has none.
    fn text_mut(&mut self, field: FactsField) -> Option<&mut String> {
        match field {
            FactsField::Merchant => Some(&mut self.merchant),
            FactsField::Date => Some(&mut self.date),
            FactsField::Total => Some(&mut self.total),
            FactsField::Type => None,
        }
    }

    fn type_choices(&self) -> Vec<Choice<DocumentType>> {
        self.types
            .iter()
            .map(|row| Choice {
                value: DocumentType(row.id),
                label: row.name.clone(),
            })
            .collect()
    }

    /// The status line's hints, their chords the Dialog keys': cancel, confirm and the key that moves field.
    fn hints(&self) -> Vec<Hint> {
        form_legend(&self.keys)
    }

    fn cycle_focus(&mut self, backward: bool) {
        // Leaving a select closes its list, as a click elsewhere would.
        self.list.close();
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

    fn confirm(&mut self, router: &mut dyn DialogRouter<DocumentsOutcome>) {
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
                router.close(Some(DocumentsOutcome::FactsSaved(self.id)));
            }
            // Filed or gone while the Dialog was open: there are no facts left to edit.
            Err(_) => router.close(None),
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

impl Presenter for FactsFormPresenter {
    type Event = FactsFormEvent;
    type ViewModel = FactsFormViewModel;
    type Router<'r> = dyn DialogRouter<DocumentsOutcome> + 'r;

    fn handle(&mut self, event: FactsFormEvent, router: &mut Self::Router<'_>) {
        match event {
            FactsFormEvent::Input(input) => self.input(input, router),
            FactsFormEvent::Merchant(text) => self.merchant = text,
            FactsFormEvent::Date(text) => self.date = text,
            FactsFormEvent::Total(text) => self.total = text,
            FactsFormEvent::Type(kind) => {
                self.list.close();
                self.focused = FactsField::Type;
                self.doc_type = kind;
            }
            FactsFormEvent::ToggleTypeList => {
                self.focused = FactsField::Type;
                let options = self.type_choices();
                self.list.toggle(&options, Some(&self.doc_type));
            }
            FactsFormEvent::CloseList => {
                self.list.close();
            }
            FactsFormEvent::NextField => self.cycle_focus(false),
            FactsFormEvent::PrevField => self.cycle_focus(true),
            FactsFormEvent::Focus(field) => {
                self.list.close();
                self.focused = field;
            }
            FactsFormEvent::Confirm => self.confirm(router),
            FactsFormEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> FactsFormViewModel {
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
            hints: self.hints(),
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
                options: self.type_choices(),
                chosen: Some(self.doc_type),
                focused: self.focused == FactsField::Type,
            },
            doc_type_list: self.list,
            note: msg::documents_dialog_facts_note(),
            focused: self.focused,
        }
    }
}

impl DialogPresenter for FactsFormPresenter {
    type Outcome = DocumentsOutcome;

    fn input_event(input: DialogInput) -> FactsFormEvent {
        FactsFormEvent::Input(input)
    }

    fn confirm_event() -> FactsFormEvent {
        FactsFormEvent::Confirm
    }

    fn cancel_event() -> FactsFormEvent {
        FactsFormEvent::Cancel
    }

    fn can_confirm(&self) -> bool {
        self.view_model().frame.can_confirm
    }

    /// Closes the open select's list without picking: the host's `back`.
    fn close_open_select(&mut self) -> bool {
        let open = self.list.is_open();
        if open {
            self.list.close();
        }
        open
    }
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
            FactsFormPresenter::new(ports.clone(), id, DialogKeys::default())
                .expect("the receipt is Unfiled"),
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

    fn input(
        presenter: &mut FactsFormPresenter,
        router: &mut RecordingDocumentsRouter,
        input: DialogInput,
    ) {
        presenter.handle(FactsFormEvent::Input(input), router);
    }

    #[test]
    fn typing_edits_the_focused_text_field_and_the_type_select_swallows_it() {
        let (mut presenter, _, mut router, _) = facts();
        input(&mut presenter, &mut router, DialogInput::Backspace);
        input(&mut presenter, &mut router, DialogInput::Char('!'));
        assert_eq!(presenter.view_model().merchant.text, "Woolworths Metr!");

        input(&mut presenter, &mut router, DialogInput::NextField);
        input(&mut presenter, &mut router, DialogInput::NextField);
        assert_eq!(presenter.view_model().focused, FactsField::Total);
        input(&mut presenter, &mut router, DialogInput::Char('0'));
        assert_eq!(presenter.view_model().total.text, "212.400");

        input(&mut presenter, &mut router, DialogInput::NextField);
        assert_eq!(presenter.view_model().focused, FactsField::Type);
        input(&mut presenter, &mut router, DialogInput::Char('z'));
        input(&mut presenter, &mut router, DialogInput::Backspace);
        let view = presenter.view_model();
        assert_eq!(view.merchant.text, "Woolworths Metr!");
        assert_eq!(view.total.text, "212.400");
        input(&mut presenter, &mut router, DialogInput::PreviousField);
        assert_eq!(presenter.view_model().focused, FactsField::Total);
    }

    #[test]
    fn the_type_select_opens_on_select_the_arrows_move_and_select_commits() {
        let (mut presenter, _, mut router, _) = facts();
        presenter.handle(FactsFormEvent::Focus(FactsField::Type), &mut router);
        let chosen = presenter.view_model().doc_type.chosen;

        input(&mut presenter, &mut router, DialogInput::Select);
        assert!(presenter.view_model().doc_type_list.is_open());
        input(&mut presenter, &mut router, DialogInput::SelectNext);
        assert_eq!(
            presenter.view_model().doc_type.chosen,
            chosen,
            "only highlighted"
        );
        input(&mut presenter, &mut router, DialogInput::Select);

        assert!(!presenter.view_model().doc_type_list.is_open());
        assert_ne!(presenter.view_model().doc_type.chosen, chosen);
        assert!(
            router.take().is_empty(),
            "committing a list is not confirming"
        );
    }

    #[test]
    fn select_confirms_a_valid_form_from_a_text_field_and_submit_from_anywhere() {
        let (mut presenter, _, mut router, id) = facts();
        input(&mut presenter, &mut router, DialogInput::Select);
        assert!(matches!(
            router.take().as_slice(),
            [
                DocumentsCall::Notice(_),
                DocumentsCall::CloseDialog(Some(DocumentsOutcome::FactsSaved(saved)))
            ] if *saved == id
        ));

        let (mut presenter, _, mut router, _) = facts();
        presenter.handle(FactsFormEvent::Focus(FactsField::Type), &mut router);
        input(&mut presenter, &mut router, DialogInput::Submit);
        assert!(!router.take().is_empty());

        let (mut presenter, _, mut router, _) = facts();
        presenter.handle(FactsFormEvent::Total("abc".to_string()), &mut router);
        input(&mut presenter, &mut router, DialogInput::Select);
        input(&mut presenter, &mut router, DialogInput::Submit);
        assert!(router.take().is_empty(), "an invalid form never confirms");
    }

    #[test]
    fn a_click_toggles_the_list_and_close_list_and_tab_close_it() {
        let (mut presenter, _, mut router, _) = facts();
        presenter.handle(FactsFormEvent::ToggleTypeList, &mut router);
        assert!(presenter.view_model().doc_type_list.is_open());
        presenter.handle(FactsFormEvent::CloseList, &mut router);
        assert!(!presenter.view_model().doc_type_list.is_open());

        presenter.handle(FactsFormEvent::ToggleTypeList, &mut router);
        input(&mut presenter, &mut router, DialogInput::NextField);
        assert!(!presenter.view_model().doc_type_list.is_open());
    }
}
