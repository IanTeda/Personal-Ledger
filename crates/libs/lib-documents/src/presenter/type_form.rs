//! The Add and Edit Document type Dialog (#478's stacked-form variant A): one form, Edit
//! pre-filled from the type.
//!
//! - The fields, in `Tab` order: Name, Tracks date, Remind and Financial year. Remind is inert
//!   without a Tracks date, so it is disabled, `Tab` skips it and clearing the date clears it.
//! - Name is checked live against every other type's name, ignoring case, and its problem shows
//!   once it has been edited; a form with a problem can't confirm.
//! - Edit says how many Filed files the type has, and how many of them carry a date the kind
//!   relabels.
//!
//! The other types' names are read when the Dialog opens, as it is modal and nothing changes them
//! under it.

use lib_core::DocumentTypeId;
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::view::{Choice, DialogFrame, SelectView, TextFieldView, count};

use super::tracks_label;
use crate::interactor::{AddDocumentType, EditDocumentType};
use crate::msg;
use crate::protocols::{DocumentTypesOutcome, SharedRepository, TypeChange};
use crate::types::{DocumentTypeDraft, DocumentTypeRow, RemindLead, TracksDate, get};

/// Longest Document Type name, in characters (#477).
pub const NAME_MAX: usize = 40;

/// The Tracks date control's options, in order; `None` tracks no date.
pub const TRACKS_OPTIONS: [Option<TracksDate>; 5] = [
    None,
    Some(TracksDate::Renews),
    Some(TracksDate::Ends),
    Some(TracksDate::Expires),
    Some(TracksDate::Revalue),
];

/// The Remind control's options, in order; `None` raises no reminder.
pub const REMIND_OPTIONS: [Option<RemindLead>; 7] = [
    None,
    Some(RemindLead::Days(7)),
    Some(RemindLead::Days(14)),
    Some(RemindLead::Days(30)),
    Some(RemindLead::Days(60)),
    Some(RemindLead::Months(3)),
    Some(RemindLead::Months(6)),
];

/// The form's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentTypeField {
    #[default]
    Name,
    TracksDate,
    Remind,
    FinancialYear,
}

impl DocumentTypeField {
    const ORDER: [Self; 4] = [
        Self::Name,
        Self::TracksDate,
        Self::Remind,
        Self::FinancialYear,
    ];
}

/// Why a name can't be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong,
    /// Another type already has the name, ignoring case.
    Taken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeFormEvent {
    /// A Dialog command or typed key. What it means is the focused field's: `select` and `submit`
    /// confirm a valid form; `Left`/`Right` step the focused control, stopping at either end;
    /// `Space` toggles the Financial year chip; typing edits the Name and goes nowhere on a
    /// control; the field keys move focus, wrapping and skipping Remind while it is disabled.
    Input(DialogInput),
    /// The Name field's whole new text, as its View's input holds it.
    Name(String),
    /// A click on a Tracks date option.
    TracksDate(Option<TracksDate>),
    /// A click on a Remind option; inert while no date is tracked.
    Remind(Option<RemindLead>),
    /// A click on the Financial year chip.
    ToggleFinancialYear,
    /// A click on a field.
    Focus(DocumentTypeField),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeFormViewModel {
    pub frame: DialogFrame,
    /// What Edit's type is used by, `None` on Add.
    pub usage: Option<String>,
    pub name: TextFieldView,
    pub name_hint: String,
    /// The name's problem, shown once the name has been edited.
    pub name_error: Option<String>,
    pub tracks_date: SelectView<Option<TracksDate>>,
    pub tracks_date_hint: String,
    pub remind: SelectView<Option<RemindLead>>,
    pub remind_hint: String,
    /// Remind is inert, and so drawn dimmed, while no date is tracked.
    pub remind_enabled: bool,
    pub financial_year_label: String,
    pub financial_year: bool,
    pub financial_year_focused: bool,
    pub financial_year_hint: String,
    pub yes: String,
    pub no: String,
    /// Edit, rather than Add.
    pub editing: bool,
    pub focused: DocumentTypeField,
}

/// The type an Edit saves, and how many Filed files it has, for the usage note.
struct Editing {
    id: DocumentTypeId,
    files: u32,
}

pub struct TypeFormPresenter {
    ports: SharedRepository,
    editing: Option<Editing>,
    name: String,
    tracks_date: Option<TracksDate>,
    remind: Option<RemindLead>,
    financial_year: bool,
    focused: DocumentTypeField,
    /// Whether the name has been edited, which is when its error starts to show.
    touched: bool,
    /// Every other type's name, trimmed and lower-cased: the ones this name must not repeat.
    taken: Vec<String>,
    dated: u32,
}

impl TypeFormPresenter {
    /// A blank Add form among the types now held.
    pub fn add(ports: SharedRepository) -> Self {
        let types = ports.borrow().document_types();
        Self {
            ports,
            editing: None,
            name: String::new(),
            tracks_date: None,
            remind: None,
            financial_year: false,
            focused: DocumentTypeField::Name,
            touched: false,
            taken: taken_names(&types, None),
            dated: 0,
        }
    }

    /// The Edit form pre-filled from type `id`, or `None` when it is gone.
    pub fn edit(ports: SharedRepository, id: DocumentTypeId) -> Option<Self> {
        let types = ports.borrow().document_types();
        let row = get(&types, id)?;
        let mut form = Self::add(ports);
        form.name = row.name.clone();
        form.tracks_date = row.tracks_date;
        form.remind = row.remind;
        form.financial_year = row.financial_year;
        form.taken = taken_names(&types, Some(id));
        form.dated = if row.tracks_date.is_some() {
            row.files
        } else {
            0
        };
        form.editing = Some(Editing {
            id,
            files: row.files,
        });
        Some(form)
    }

    pub fn handle(
        &mut self,
        event: TypeFormEvent,
        router: &mut dyn DialogRouter<DocumentTypesOutcome>,
    ) {
        match event {
            TypeFormEvent::Input(input) => self.input(input, router),
            TypeFormEvent::Name(text) => {
                self.name = text;
                self.touched = true;
            }
            TypeFormEvent::TracksDate(option) => {
                self.focused = DocumentTypeField::TracksDate;
                self.set_tracks_date(option);
            }
            TypeFormEvent::Remind(option) => {
                if self.remind_enabled() {
                    self.focused = DocumentTypeField::Remind;
                }
                self.set_remind(option);
            }
            TypeFormEvent::ToggleFinancialYear => {
                self.focused = DocumentTypeField::FinancialYear;
                self.financial_year = !self.financial_year;
            }
            TypeFormEvent::Focus(field) => self.focused = field,
            TypeFormEvent::Confirm => self.confirm(router),
            TypeFormEvent::Cancel => router.close(None),
        }
    }

    /// Reads `input` for the focused field.
    fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<DocumentTypesOutcome>) {
        let on_name = self.focused == DocumentTypeField::Name;
        match input {
            DialogInput::NextField => self.cycle_focus(false),
            DialogInput::PreviousField => self.cycle_focus(true),
            DialogInput::Select | DialogInput::Submit => self.confirm(router),
            DialogInput::Left => self.step_focused(-1),
            DialogInput::Right => self.step_focused(1),
            DialogInput::Char(' ') if self.focused == DocumentTypeField::FinancialYear => {
                self.financial_year = !self.financial_year;
            }
            DialogInput::Char(ch) if on_name => {
                self.name.push(ch);
                self.touched = true;
            }
            DialogInput::Backspace if on_name => {
                self.name.pop();
                self.touched = true;
            }
            // A control swallows typing, and the arrows step nothing here.
            _ => {}
        }
    }

    pub fn view_model(&self) -> TypeFormViewModel {
        let (title, submit) = if self.editing.is_some() {
            (
                msg::documents_types_dialog_edit_title(),
                msg::documents_types_dialog_save(),
            )
        } else {
            (
                msg::documents_types_dialog_add_title(),
                msg::documents_types_dialog_add_submit(),
            )
        };
        let max = count(NAME_MAX);
        TypeFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.name_error().is_none(),
                destructive: false,
                hints: Vec::new(),
            },
            usage: self.editing.as_ref().map(|editing| {
                msg::documents_types_edit_usage(i64::from(editing.files), i64::from(self.dated))
            }),
            name: TextFieldView {
                label: msg::documents_types_field_name(),
                text: self.name.clone(),
                placeholder: msg::documents_types_field_name_placeholder(),
                focused: self.focused == DocumentTypeField::Name,
            },
            name_hint: msg::documents_types_field_name_hint(max),
            name_error: self
                .name_error()
                .filter(|_| self.touched)
                .map(|error| match error {
                    NameError::Empty => msg::documents_types_error_name_empty(),
                    NameError::TooLong => msg::documents_types_error_name_too_long(max),
                    NameError::Taken => msg::documents_types_error_name_taken(),
                }),
            tracks_date: SelectView {
                label: msg::documents_types_field_tracks_date(),
                options: TRACKS_OPTIONS
                    .iter()
                    .map(|option| Choice {
                        value: *option,
                        label: option.map_or_else(msg::documents_types_option_none, tracks_label),
                    })
                    .collect(),
                chosen: Some(self.tracks_date),
                focused: self.focused == DocumentTypeField::TracksDate,
            },
            tracks_date_hint: msg::documents_types_field_tracks_date_hint(),
            remind: SelectView {
                label: msg::documents_types_field_remind(),
                options: REMIND_OPTIONS
                    .iter()
                    .map(|option| Choice {
                        value: *option,
                        label: remind_option_label(*option),
                    })
                    .collect(),
                chosen: Some(self.remind),
                focused: self.focused == DocumentTypeField::Remind,
            },
            remind_hint: msg::documents_types_field_remind_hint(),
            remind_enabled: self.remind_enabled(),
            financial_year_label: msg::documents_types_field_financial_year(),
            financial_year: self.financial_year,
            financial_year_focused: self.focused == DocumentTypeField::FinancialYear,
            financial_year_hint: msg::documents_types_field_financial_year_hint(),
            yes: msg::documents_types_yes(),
            no: msg::documents_types_no(),
            editing: self.editing.is_some(),
            focused: self.focused,
        }
    }

    fn name_error(&self) -> Option<NameError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Some(NameError::Empty);
        }
        if name.chars().count() > NAME_MAX {
            return Some(NameError::TooLong);
        }
        self.taken
            .contains(&name.to_lowercase())
            .then_some(NameError::Taken)
    }

    /// Remind is inert, and so disabled, while no date is tracked.
    fn remind_enabled(&self) -> bool {
        self.tracks_date.is_some()
    }

    fn set_tracks_date(&mut self, tracks_date: Option<TracksDate>) {
        self.tracks_date = tracks_date;
        if tracks_date.is_none() {
            self.remind = None;
        }
    }

    fn set_remind(&mut self, remind: Option<RemindLead>) {
        if self.remind_enabled() {
            self.remind = remind;
        }
    }

    fn cycle_focus(&mut self, backward: bool) {
        let order = DocumentTypeField::ORDER;
        let count = order.len();
        let mut index = order
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        loop {
            index = if backward {
                (index + count - 1) % count
            } else {
                (index + 1) % count
            };
            if order[index] != DocumentTypeField::Remind || self.remind_enabled() {
                break;
            }
        }
        self.focused = order[index];
    }

    fn step_focused(&mut self, delta: isize) {
        match self.focused {
            DocumentTypeField::TracksDate => {
                self.set_tracks_date(step(&TRACKS_OPTIONS, self.tracks_date, delta));
            }
            DocumentTypeField::Remind => self.set_remind(step(&REMIND_OPTIONS, self.remind, delta)),
            DocumentTypeField::FinancialYear => self.financial_year = !self.financial_year,
            DocumentTypeField::Name => {}
        }
    }

    fn draft(&self) -> DocumentTypeDraft {
        DocumentTypeDraft {
            name: self.name.trim().to_string(),
            tracks_date: self.tracks_date,
            remind: self.remind,
            financial_year: self.financial_year,
        }
    }

    fn confirm(&mut self, router: &mut dyn DialogRouter<DocumentTypesOutcome>) {
        if self.name_error().is_some() {
            return;
        }
        let draft = self.draft();
        let mut repository = self.ports.borrow_mut();
        let saved = match &self.editing {
            Some(editing) => EditDocumentType {
                id: editing.id,
                draft,
            }
            .run(&mut *repository)
            .map(|()| editing.id),
            None => Ok(AddDocumentType(draft).run(&mut *repository)),
        };
        drop(repository);
        match saved {
            Ok(id) => router.close(Some(DocumentTypesOutcome {
                change: TypeChange::Saved,
                select: Some(id),
            })),
            // The type went while the Dialog was open: nothing was written and nothing is left to
            // edit, so it closes as a cancel does.
            Err(_unknown) => router.close(None),
        }
    }
}

fn remind_option_label(option: Option<RemindLead>) -> String {
    match option {
        None => msg::documents_types_option_none(),
        Some(RemindLead::Days(days)) => msg::documents_types_option_days(&days.to_string()),
        Some(RemindLead::Months(months)) => msg::documents_types_option_months(&months.to_string()),
    }
}

fn taken_names(types: &[DocumentTypeRow], own_id: Option<DocumentTypeId>) -> Vec<String> {
    types
        .iter()
        .filter(|row| Some(row.id) != own_id)
        .map(|row| row.name.trim().to_lowercase())
        .collect()
}

fn step<T: Copy + PartialEq>(options: &[T], current: T, delta: isize) -> T {
    let index = options
        .iter()
        .position(|option| *option == current)
        .unwrap_or(0);
    let next = index
        .saturating_add_signed(delta)
        .min(options.len().saturating_sub(1));
    options.get(next).copied().unwrap_or(current)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};

    struct Form {
        presenter: TypeFormPresenter,
        router: RecordingRouter,
    }

    impl Form {
        fn add() -> (
            Self,
            std::rc::Rc<std::cell::RefCell<crate::fake::InMemoryDocuments>>,
        ) {
            let (documents, shared) = stub();
            let form = Self {
                presenter: TypeFormPresenter::add(shared),
                router: RecordingRouter::default(),
            };
            (form, documents)
        }

        fn edit(
            id: DocumentTypeId,
        ) -> (
            Self,
            std::rc::Rc<std::cell::RefCell<crate::fake::InMemoryDocuments>>,
        ) {
            let (documents, shared) = stub();
            let form = Self {
                presenter: TypeFormPresenter::edit(shared, id).expect("a default type"),
                router: RecordingRouter::default(),
            };
            (form, documents)
        }

        fn send(&mut self, event: TypeFormEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn press(&mut self, input: DialogInput) -> Vec<RouterCall> {
            self.send(TypeFormEvent::Input(input))
        }

        fn type_name(&mut self, name: &str) {
            self.send(TypeFormEvent::Name(name.to_string()));
        }

        fn view(&self) -> TypeFormViewModel {
            self.presenter.view_model()
        }
    }

    #[test]
    fn names_are_required_trimmed_short_and_unique_ignoring_case() {
        let (mut form, _) = Form::add();
        assert!(!form.view().frame.can_confirm, "a blank name");

        form.type_name("  ");
        assert_eq!(form.presenter.name_error(), Some(NameError::Empty));
        form.type_name(&"a".repeat(41));
        assert_eq!(form.presenter.name_error(), Some(NameError::TooLong));
        form.type_name(&"a".repeat(40));
        assert_eq!(form.presenter.name_error(), None);
        form.type_name(" tAx ");
        assert_eq!(form.presenter.name_error(), Some(NameError::Taken));
        assert_eq!(
            form.view().name_error.as_deref(),
            Some("Another type already has that name.")
        );
        assert!(!form.view().frame.can_confirm);
    }

    #[test]
    fn a_name_error_waits_until_the_name_is_edited() {
        let (form, _) = Form::add();
        let view = form.view();

        assert_eq!(view.name_error, None);
        assert!(!view.frame.can_confirm);
        assert_eq!(view.frame.title, "Add Document type");
        assert_eq!(view.frame.submit, "Add type");
    }

    #[test]
    fn editing_a_type_keeps_its_own_name_free_even_in_another_case() {
        let (mut form, _) = Form::edit(DocumentTypeId::seed(3));
        assert_eq!(form.view().frame.title, "Edit Document type");

        form.type_name("TAX");
        assert_eq!(form.presenter.name_error(), None);
        form.type_name("receipts");
        assert_eq!(form.presenter.name_error(), Some(NameError::Taken));
    }

    #[test]
    fn edit_says_how_many_files_the_type_has_and_how_many_carry_a_date() {
        let (form, _) = Form::edit(DocumentTypeId::seed(4));

        assert_eq!(
            form.view().usage.as_deref(),
            Some(
                "Used by 18 Filed files. 18 files have dates. Changing the kind relabels them; \
                 clearing it makes the dates inert."
            )
        );
        // Tax tracks no date, so none of its files carries one.
        assert_eq!(
            Form::edit(DocumentTypeId::seed(3))
                .0
                .view()
                .usage
                .as_deref(),
            Some(
                "Used by 31 Filed files. 0 files have dates. Changing the kind relabels them; \
                 clearing it makes the dates inert."
            )
        );
        assert_eq!(Form::add().0.view().usage, None);
    }

    #[test]
    fn clearing_tracks_date_clears_and_disables_remind() {
        let (mut form, _) = Form::add();
        form.send(TypeFormEvent::Remind(Some(RemindLead::Days(7))));
        assert_eq!(form.view().remind.chosen, Some(None));
        assert!(!form.view().remind_enabled);

        form.send(TypeFormEvent::TracksDate(Some(TracksDate::Ends)));
        form.send(TypeFormEvent::Remind(Some(RemindLead::Days(7))));
        assert_eq!(form.view().remind.chosen, Some(Some(RemindLead::Days(7))));
        assert_eq!(form.view().focused, DocumentTypeField::Remind);

        form.send(TypeFormEvent::TracksDate(None));
        assert_eq!(form.view().remind.chosen, Some(None));
    }

    #[test]
    fn typing_edits_the_name_and_goes_nowhere_on_a_control() {
        let (mut form, _) = Form::add();

        for ch in "Wartyx".chars() {
            form.press(DialogInput::Char(ch));
        }
        form.press(DialogInput::Backspace);
        form.press(DialogInput::NextField);
        form.press(DialogInput::Char('z'));
        form.press(DialogInput::Backspace);
        form.press(DialogInput::SelectNext);

        let view = form.view();
        assert_eq!(view.name.text, "Warty");
        assert_eq!(view.tracks_date.chosen, Some(None));
    }

    #[test]
    fn select_and_submit_confirm_only_a_valid_form() {
        let (mut form, documents) = Form::add();
        assert_eq!(form.press(DialogInput::Select), [], "a blank name");

        for ch in "Warranty".chars() {
            form.press(DialogInput::Char(ch));
        }
        form.press(DialogInput::NextField);
        let calls = form.press(DialogInput::Submit);

        assert!(matches!(
            calls.as_slice(),
            [RouterCall::CloseDialog(Some(_))]
        ));
        assert!(
            documents
                .borrow()
                .types
                .iter()
                .any(|row| row.name == "Warranty")
        );
    }

    #[test]
    fn tab_skips_remind_while_it_is_disabled() {
        let (mut form, _) = Form::add();

        form.press(DialogInput::NextField);
        assert_eq!(form.view().focused, DocumentTypeField::TracksDate);
        form.press(DialogInput::NextField);
        assert_eq!(form.view().focused, DocumentTypeField::FinancialYear);
        form.send(TypeFormEvent::TracksDate(Some(TracksDate::Renews)));
        form.press(DialogInput::PreviousField);
        assert_eq!(form.view().focused, DocumentTypeField::Name);
        form.press(DialogInput::NextField);
        form.press(DialogInput::NextField);
        assert_eq!(form.view().focused, DocumentTypeField::Remind);
    }

    #[test]
    fn stepping_a_control_stops_at_either_end() {
        let (mut form, _) = Form::add();
        form.send(TypeFormEvent::Focus(DocumentTypeField::TracksDate));

        form.press(DialogInput::Left);
        assert_eq!(form.view().tracks_date.chosen, Some(None));
        for _ in 0..9 {
            form.press(DialogInput::Right);
        }
        assert_eq!(
            form.view().tracks_date.chosen,
            Some(Some(TracksDate::Revalue))
        );
    }

    #[test]
    fn stepping_the_financial_year_chip_toggles_it() {
        let (mut form, _) = Form::add();
        form.send(TypeFormEvent::Focus(DocumentTypeField::FinancialYear));

        form.press(DialogInput::Right);
        assert!(form.view().financial_year);
        form.press(DialogInput::Char(' '));
        assert!(!form.view().financial_year);
        form.send(TypeFormEvent::ToggleFinancialYear);
        assert!(form.view().financial_year);
        assert!(form.view().financial_year_focused);
    }

    #[test]
    fn the_options_read_as_labels() {
        let (form, _) = Form::add();
        let view = form.view();

        let tracks: Vec<&str> = view
            .tracks_date
            .options
            .iter()
            .map(|choice| choice.label.as_str())
            .collect();
        assert_eq!(tracks, ["None", "Renews", "Ends", "Expires", "Revalue"]);
        let remind: Vec<&str> = view
            .remind
            .options
            .iter()
            .map(|choice| choice.label.as_str())
            .collect();
        assert_eq!(
            remind,
            ["None", "7 d", "14 d", "30 d", "60 d", "3 mo", "6 mo"]
        );
    }

    #[test]
    fn confirming_an_add_writes_the_trimmed_type_at_the_end_and_selects_it() {
        let (mut form, documents) = Form::add();
        form.type_name("  Leases ");
        form.send(TypeFormEvent::TracksDate(Some(TracksDate::Renews)));
        form.send(TypeFormEvent::Remind(Some(RemindLead::Days(14))));
        form.send(TypeFormEvent::ToggleFinancialYear);

        let calls = form.send(TypeFormEvent::Confirm);

        let types = documents.borrow().types.clone();
        let added = types.last().expect("the added type");
        assert_eq!(added.name, "Leases");
        assert_eq!(added.tracks_date, Some(TracksDate::Renews));
        assert_eq!(added.remind, Some(RemindLead::Days(14)));
        assert!(added.financial_year);
        assert_eq!(types.len(), 10);
        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(DocumentTypesOutcome {
                change: TypeChange::Saved,
                select: Some(added.id),
            }))]
        );
    }

    #[test]
    fn confirming_an_edit_keeps_the_row_and_its_files() {
        let (mut form, documents) = Form::edit(DocumentTypeId::seed(3));
        form.type_name("Taxes");

        let calls = form.send(TypeFormEvent::Confirm);

        let types = documents.borrow().types.clone();
        assert_eq!(types[2].name, "Taxes");
        assert_eq!(types[2].files, 31);
        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(DocumentTypesOutcome {
                change: TypeChange::Saved,
                select: Some(DocumentTypeId::seed(3)),
            }))]
        );
    }

    #[test]
    fn an_invalid_form_confirms_nothing() {
        let (mut form, documents) = Form::add();
        form.type_name("tax");

        assert_eq!(form.send(TypeFormEvent::Confirm), []);
        assert_eq!(documents.borrow().types.len(), 9);
    }

    #[test]
    fn cancel_closes_without_an_outcome_and_writes_nothing() {
        let (mut form, documents) = Form::add();
        form.type_name("Leases");

        assert_eq!(
            form.send(TypeFormEvent::Cancel),
            [RouterCall::CloseDialog(None)]
        );
        assert_eq!(documents.borrow().types.len(), 9);
    }

    #[test]
    fn an_edit_of_a_type_removed_underneath_closes_as_a_cancel() {
        let (mut form, documents) = Form::edit(DocumentTypeId::seed(3));
        documents
            .borrow_mut()
            .types
            .retain(|row| row.id != DocumentTypeId::seed(3));
        form.type_name("Taxes");

        assert_eq!(
            form.send(TypeFormEvent::Confirm),
            [RouterCall::CloseDialog(None)]
        );
    }

    #[test]
    fn an_edit_of_a_gone_type_has_no_form() {
        let (_, shared) = stub();

        assert!(TypeFormPresenter::edit(shared, DocumentTypeId::seed(999)).is_none());
    }
}
