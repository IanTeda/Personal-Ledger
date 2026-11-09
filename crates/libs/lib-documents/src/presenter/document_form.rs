//! The Add document and Edit document Dialog (`docs/ux/desktop-mockups/04-documents/`): one form,
//! Edit pre-filled from a Filed Document and without the Path.
//!
//! - **Add** takes a typed Path, a Title that follows the file name until it is typed over, a
//!   Document Type, the document date (`today` when left alone) and an optional Key Date. Its Links
//!   come afterwards through the link picker.
//! - **Edit** changes a Filed Document's Title, Document Type, date and Key Date in place.
//! - Problems show inline and refuse Save: a missing file, an unsupported one, a path already in
//!   the Library and a date the parser rejects. A blank Title only refuses.
//!
//! The Document Types are read when the Dialog opens, as it is modal and nothing changes them under
//! it; the Library is read for each check, since that is what Add's path must not repeat.

use std::path::Path;

use chrono::NaiveDate;
use lib_core::DocumentId;
use lib_locale::format::{format_date_input, parse_optional_date};
use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, NEXT_FIELD, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{
    Choice, DialogFrame, Hint, SelectList, SelectMove, SelectView, TextFieldView,
};

use super::DialogPorts;
use super::library::key_kind_label;
use crate::interactor::{AddDocument, EditDocument};
use crate::protocols::DocumentsOutcome;
use crate::types::{DocumentTypeRow, fallback_id};
use crate::{
    Document, DocumentType, KeyDate, KeyDateKind, LibraryScope, NewDocument, PathRefusal,
    check_path, date_for_scope, default_title, get, msg,
};

/// The Key date select's options, in order; `None` is no Key Date.
pub const KEY_KINDS: [Option<KeyDateKind>; 5] = [
    None,
    Some(KeyDateKind::Renews),
    Some(KeyDateKind::Ends),
    Some(KeyDateKind::Expires),
    Some(KeyDateKind::Revalue),
];

/// The form's fields, in `Tab` order. Edit has no Path, and the Key Date's date and reminder show
/// only once a kind is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentField {
    #[default]
    Path,
    Title,
    Type,
    Date,
    KeyKind,
    KeyDate,
    Reminder,
}

impl DocumentField {
    const ORDER: [Self; 7] = [
        Self::Path,
        Self::Title,
        Self::Type,
        Self::Date,
        Self::KeyKind,
        Self::KeyDate,
        Self::Reminder,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentFormEvent {
    /// A Dialog keymap command or a typed key, meaning what the focused field makes it: on the
    /// Document type and Key date selects `select`, `Space` and the arrows open the list, move it
    /// and commit it; `Space` flips the reminder; the text fields take typing, and a select or the
    /// reminder swallows it. `select` elsewhere confirms a valid form, and `submit` confirms from
    /// any field.
    Input(DialogInput),
    /// A text field's whole new text, as its View's input holds it.
    Path(String),
    Title(String),
    Date(String),
    KeyDate(String),
    /// A click on a row of the Document type's open list: picks it and closes the list.
    Type(DocumentType),
    /// A click on a row of the Key date's open list: picks it and closes the list.
    KeyKind(Option<KeyDateKind>),
    /// A click on a select's field: focuses it and opens or closes its list, closing the other's.
    ToggleList(DocumentField),
    /// Closes the open list without picking: the host's `back`.
    CloseList,
    /// The reminder checkbox, clicked or pressed.
    ToggleReminder,
    /// `Tab`/`Shift-Tab`, wrapping over the fields shown.
    NextField,
    PrevField,
    /// A click on a field.
    Focus(DocumentField),
    Confirm,
    Cancel,
}

/// The reminder checkbox, shown with the Key Date's date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxView {
    pub label: String,
    pub checked: bool,
    pub focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentFormViewModel {
    pub frame: DialogFrame,
    /// Edit, rather than Add.
    pub editing: bool,
    /// Add's Path; Edit has none.
    pub path: Option<TextFieldView>,
    pub path_error: Option<String>,
    pub title: TextFieldView,
    pub doc_type: SelectView<DocumentType>,
    pub date: TextFieldView,
    pub date_error: Option<String>,
    pub key_kind: SelectView<Option<KeyDateKind>>,
    /// The focused select's list: open, and its highlight.
    pub list: SelectList,
    /// The Key Date's date and reminder, once a kind is chosen.
    pub key_date: Option<TextFieldView>,
    pub key_date_error: Option<String>,
    pub reminder: Option<CheckboxView>,
    /// Add's note under the fields.
    pub note: Option<String>,
    pub focused: DocumentField,
    /// The status line's hints while the Dialog is open.
    pub hints: Vec<Hint>,
}

pub struct DocumentFormPresenter {
    /// The Dialog keymap's chords, which its hints show.
    keys: DialogKeys,
    ports: DialogPorts,
    /// The Document an Edit saves, `None` on Add.
    editing: Option<DocumentId>,
    types: Vec<DocumentTypeRow>,
    path: String,
    title: String,
    /// Whether the Title has been typed in, which stops it following the file name.
    title_typed: bool,
    doc_type: DocumentType,
    date: String,
    key_kind: Option<KeyDateKind>,
    key_date: String,
    reminder: bool,
    focused: DocumentField,
    /// The focused select's list; it closes whenever focus leaves.
    list: SelectList,
}

impl DocumentFormPresenter {
    /// A fresh Add form. A Type scope pre-fills the Type; a Financial Year scope pre-fills the date
    /// (today if inside that year, otherwise the year's last day); All pre-fills nothing.
    pub fn add(ports: DialogPorts, scope: LibraryScope, keys: DialogKeys) -> Self {
        let types = ports.documents.borrow().document_types();
        let today = ports.today;
        let doc_type = match scope {
            LibraryScope::Type(kind) => kind,
            _ => DocumentType(fallback_id(&types)),
        };
        let date = date_for_scope(scope, today);
        let date = if date == today {
            lib_locale::msg::date_word_today()
        } else {
            format_date_input(date, ports.date_style)
        };
        Self {
            keys,
            ports,
            editing: None,
            types,
            path: String::new(),
            title: String::new(),
            title_typed: false,
            doc_type,
            date,
            key_kind: None,
            key_date: String::new(),
            reminder: false,
            focused: DocumentField::Path,
            list: SelectList::default(),
        }
    }

    /// The Edit form pre-filled from Filed Document `id`, or `None` when it has gone.
    pub fn edit(ports: DialogPorts, id: DocumentId, keys: DialogKeys) -> Option<Self> {
        let document = get(&ports.documents.borrow().documents(), id)?.clone();
        let mut form = Self::add(ports, LibraryScope::All, keys);
        let style = form.ports.date_style;
        let key = document.key_date.as_ref();
        form.editing = Some(id);
        form.path = document.path.to_string_lossy().into_owned();
        form.title = document.title.clone();
        form.title_typed = true;
        form.doc_type = document.doc_type;
        form.date = format_date_input(document.date, style);
        form.key_kind = key.map(|key| key.kind);
        form.key_date = key
            .map(|key| format_date_input(key.date, style))
            .unwrap_or_default();
        form.reminder = key.is_some_and(|key| key.reminder);
        form.focused = DocumentField::Title;
        Some(form)
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
                DocumentField::Type,
            ) => {
                let options = self.type_choices();
                if let SelectMove::Chose(kind) =
                    self.list.input(input, &options, Some(&self.doc_type))
                {
                    self.doc_type = kind;
                }
            }
            (
                DialogInput::Select
                | DialogInput::Char(' ')
                | DialogInput::SelectNext
                | DialogInput::SelectPrevious,
                DocumentField::KeyKind,
            ) => {
                let options = self.key_kind_choices();
                if let SelectMove::Chose(kind) =
                    self.list.input(input, &options, Some(&self.key_kind))
                {
                    self.key_kind = kind;
                }
            }
            (DialogInput::Select, _) => self.confirm(router),
            (DialogInput::Char(' '), DocumentField::Reminder) => {
                self.handle(DocumentFormEvent::ToggleReminder, router);
            }
            (DialogInput::Char(ch), field) => self.edit_text(field, |text| text.push(ch)),
            (DialogInput::Backspace, field) => {
                self.edit_text(field, |text| {
                    text.pop();
                });
            }
            _ => {}
        }
    }

    /// Edits the text field `field` types into, with the Title following the file name until it
    /// is typed over. A select or the reminder has no text to edit.
    fn edit_text(&mut self, field: DocumentField, edit: impl FnOnce(&mut String)) {
        match field {
            DocumentField::Path if self.editing.is_none() => {
                edit(&mut self.path);
                if !self.title_typed {
                    self.title = default_title(Path::new(self.path.trim()));
                }
            }
            DocumentField::Title => {
                edit(&mut self.title);
                self.title_typed = true;
            }
            DocumentField::Date => edit(&mut self.date),
            DocumentField::KeyDate if self.key_kind.is_some() => edit(&mut self.key_date),
            _ => {}
        }
    }

    /// A click on a select's field: focuses it and opens or closes its list, closing the other's.
    fn toggle_list(&mut self, field: DocumentField) {
        match field {
            DocumentField::Type => {
                if self.focused != field {
                    self.list.close();
                }
                self.focused = field;
                let options = self.type_choices();
                self.list.toggle(&options, Some(&self.doc_type));
            }
            DocumentField::KeyKind => {
                if self.focused != field {
                    self.list.close();
                }
                self.focused = field;
                let options = self.key_kind_choices();
                self.list.toggle(&options, Some(&self.key_kind));
            }
            _ => {}
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

    fn key_kind_choices(&self) -> Vec<Choice<Option<KeyDateKind>>> {
        KEY_KINDS
            .iter()
            .map(|kind| Choice {
                value: *kind,
                label: kind.map_or_else(msg::documents_dialog_key_none, key_kind_label),
            })
            .collect()
    }

    /// The status line's hints, their chords the Dialog keys': cancel, confirm and the key that moves field.
    fn hints(&self) -> Vec<Hint> {
        form_legend(&self.keys)
    }

    /// Whether `field` is on the form.
    fn shows(&self, field: DocumentField) -> bool {
        match field {
            DocumentField::Path => self.editing.is_none(),
            DocumentField::KeyDate | DocumentField::Reminder => self.key_kind.is_some(),
            _ => true,
        }
    }

    fn cycle_focus(&mut self, backward: bool) {
        // Leaving a select closes its list, as a click elsewhere would.
        self.list.close();
        let order = DocumentField::ORDER;
        let mut index = order
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        for _ in 0..order.len() {
            index = if backward {
                (index + order.len() - 1) % order.len()
            } else {
                (index + 1) % order.len()
            };
            if self.shows(order[index]) {
                break;
            }
        }
        self.focused = order[index];
    }

    fn today(&self) -> NaiveDate {
        self.ports.today
    }

    fn parse(&self, text: &str) -> Result<Option<NaiveDate>, String> {
        parse_optional_date(text, self.today(), self.ports.date_style)
    }

    /// Every field's problem: Add's typed path checked against the Library and the disk, and the
    /// dates the parser rejects. A Key Date's date left blank is a problem with no words.
    fn problems(&self) -> Problems {
        let typed = self.path.trim();
        let path = if self.editing.is_some() || typed.is_empty() {
            None
        } else {
            let documents = self.ports.documents.borrow().documents();
            check_path(&*self.ports.files, &documents, typed).err()
        };
        let key_date = match (self.key_kind, self.parse(&self.key_date)) {
            (None, _) | (Some(_), Ok(Some(_))) => None,
            (Some(_), Ok(None)) => Some(String::new()),
            (Some(_), Err(hint)) => Some(hint),
        };
        Problems {
            path,
            date: self.parse(&self.date).err(),
            key_date,
        }
    }

    fn can_save(&self, problems: &Problems) -> bool {
        problems.is_empty()
            && !self.title.trim().is_empty()
            && (self.editing.is_some() || !self.path.trim().is_empty())
    }

    /// The Key Date, `None` when there is none or the chosen one is unreadable.
    fn key_date_value(&self) -> Option<KeyDate> {
        let kind = self.key_kind?;
        Some(KeyDate {
            kind,
            date: self.parse(&self.key_date).ok()??,
            reminder: self.reminder,
        })
    }

    fn confirm(&mut self, router: &mut dyn DialogRouter<DocumentsOutcome>) {
        if !self.can_save(&self.problems()) {
            return;
        }
        let title = self.title.trim().to_string();
        let date = self
            .parse(&self.date)
            .ok()
            .flatten()
            .unwrap_or(self.today());
        let key_date = self.key_date_value();
        let mut repository = self.ports.documents.borrow_mut();
        let saved = match self.editing {
            Some(id) => EditDocument {
                id,
                title: title.clone(),
                doc_type: self.doc_type,
                date,
                key_date,
            }
            .run(&mut *repository)
            .map(|()| (id, msg::documents_dialog_toast_saved(&title))),
            None => {
                let files = &*self.ports.files;
                let path = files.resolve(&self.path);
                AddDocument(NewDocument {
                    bytes: files.size(&path).unwrap_or(0),
                    path,
                    title: title.clone(),
                    doc_type: self.doc_type,
                    date,
                    key_date,
                })
                .run(&mut *repository)
                .map(|id| (id, msg::documents_dialog_toast_added(&title)))
            }
        };
        drop(repository);
        match saved {
            Ok((id, toast)) => {
                router.notify(Notice::Toast(ToastKind::Success, toast));
                router.close(Some(DocumentsOutcome::Shown(id)));
            }
            // An Edit whose Document went while the Dialog was open has nothing left to save. An
            // Add the checks above passed can only be refused by a file gone from under it, which
            // the form then shows.
            Err(_) if self.editing.is_some() => router.close(None),
            Err(_) => {}
        }
    }
}

/// What a form's fields refuse.
#[derive(Debug, Default)]
struct Problems {
    path: Option<PathRefusal>,
    /// The date parser's own hint.
    date: Option<String>,
    key_date: Option<String>,
}

impl Problems {
    fn is_empty(&self) -> bool {
        self.path.is_none() && self.date.is_none() && self.key_date.is_none()
    }
}

/// A form Dialog's legend: cancel, confirm and the key that moves field.
pub(super) fn form_legend(keys: &DialogKeys) -> Vec<Hint> {
    let key = |command| keys.chord(command).to_string();
    vec![
        Hint::new(key(BACK), msg::documents_dialog_hint_cancel()),
        Hint::new(key(NEXT_FIELD), msg::documents_dialog_hint_next_field()),
        Hint::new(key(SELECT), msg::documents_dialog_hint_confirm()),
    ]
}

/// The Document a Dialog acts on, as the repository holds it now.
pub(super) fn document(ports: &DialogPorts, id: DocumentId) -> Option<Document> {
    get(&ports.documents.borrow().documents(), id).cloned()
}

impl Presenter for DocumentFormPresenter {
    type Event = DocumentFormEvent;
    type ViewModel = DocumentFormViewModel;
    type Router<'r> = dyn DialogRouter<DocumentsOutcome> + 'r;

    fn handle(&mut self, event: DocumentFormEvent, router: &mut Self::Router<'_>) {
        match event {
            DocumentFormEvent::Input(input) => self.input(input, router),
            DocumentFormEvent::Path(text) => {
                self.path = text;
                if !self.title_typed {
                    self.title = default_title(Path::new(self.path.trim()));
                }
            }
            DocumentFormEvent::Title(text) => {
                self.title = text;
                self.title_typed = true;
            }
            DocumentFormEvent::Date(text) => self.date = text,
            DocumentFormEvent::KeyDate(text) => self.key_date = text,
            DocumentFormEvent::Type(kind) => {
                self.list.close();
                self.focused = DocumentField::Type;
                self.doc_type = kind;
            }
            DocumentFormEvent::KeyKind(kind) => {
                self.list.close();
                self.focused = DocumentField::KeyKind;
                self.key_kind = kind;
            }
            DocumentFormEvent::ToggleList(field) => self.toggle_list(field),
            DocumentFormEvent::CloseList => {
                self.list.close();
            }
            DocumentFormEvent::ToggleReminder => {
                if self.key_kind.is_some() {
                    self.list.close();
                    self.focused = DocumentField::Reminder;
                    self.reminder = !self.reminder;
                }
            }
            DocumentFormEvent::NextField => self.cycle_focus(false),
            DocumentFormEvent::PrevField => self.cycle_focus(true),
            DocumentFormEvent::Focus(field) => {
                self.list.close();
                if self.shows(field) {
                    self.focused = field;
                }
            }
            DocumentFormEvent::Confirm => self.confirm(router),
            DocumentFormEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> DocumentFormViewModel {
        let problems = self.problems();
        let (title, submit) = if self.editing.is_some() {
            (
                msg::documents_dialog_edit_title(),
                msg::documents_dialog_edit_submit(),
            )
        } else {
            (
                msg::documents_dialog_add_title(),
                msg::documents_dialog_add_submit(),
            )
        };
        let text = |label: String, text: &str, field: DocumentField| TextFieldView {
            label,
            text: text.to_string(),
            placeholder: String::new(),
            focused: self.focused == field,
        };
        let key_shown = self.key_kind.is_some();
        DocumentFormViewModel {
            hints: self.hints(),
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.can_save(&problems),
                destructive: false,
                hints: Vec::new(),
            },
            editing: self.editing.is_some(),
            path: self.editing.is_none().then(|| TextFieldView {
                placeholder: msg::documents_dialog_field_path_placeholder(),
                ..text(
                    msg::documents_dialog_field_path(),
                    &self.path,
                    DocumentField::Path,
                )
            }),
            path_error: problems.path.map(|problem| match problem {
                PathRefusal::Unsupported => msg::documents_dialog_error_unsupported(),
                PathRefusal::Already(title) => msg::documents_dialog_error_already(&title),
                PathRefusal::NotFound => msg::documents_dialog_error_path_missing(self.path.trim()),
            }),
            title: text(
                msg::documents_dialog_field_title(),
                &self.title,
                DocumentField::Title,
            ),
            doc_type: SelectView {
                label: msg::documents_dialog_field_type(),
                options: self.type_choices(),
                chosen: Some(self.doc_type),
                focused: self.focused == DocumentField::Type,
            },
            date: text(
                msg::documents_dialog_field_date(),
                &self.date,
                DocumentField::Date,
            ),
            date_error: problems.date,
            key_kind: SelectView {
                label: msg::documents_dialog_field_key_kind(),
                options: self.key_kind_choices(),
                chosen: Some(self.key_kind),
                focused: self.focused == DocumentField::KeyKind,
            },
            list: self.list,
            key_date: key_shown.then(|| {
                text(
                    msg::documents_dialog_field_key_date(),
                    &self.key_date,
                    DocumentField::KeyDate,
                )
            }),
            // A Key Date still to be typed refuses Save but says nothing.
            key_date_error: problems.key_date.filter(|hint| !hint.is_empty()),
            reminder: key_shown.then(|| CheckboxView {
                label: msg::documents_dialog_field_reminder(),
                checked: self.reminder,
                focused: self.focused == DocumentField::Reminder,
            }),
            note: self.editing.is_none().then(msg::documents_dialog_add_note),
            focused: self.focused,
        }
    }
}

impl DialogPresenter for DocumentFormPresenter {
    type Outcome = DocumentsOutcome;

    fn input_event(input: DialogInput) -> DocumentFormEvent {
        DocumentFormEvent::Input(input)
    }

    fn confirm_event() -> DocumentFormEvent {
        DocumentFormEvent::Confirm
    }

    fn cancel_event() -> DocumentFormEvent {
        DocumentFormEvent::Cancel
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
    use super::super::test_support::{dialog_ports, world};
    use super::*;
    use crate::fake::{DocumentsCall, InMemoryFiles, RecordingDocumentsRouter};

    /// A Filed Document with a Key Date.
    const POLICY: &str = "aami-car-policy.pdf";

    /// The Dialog over the sample Documents, with a disk holding `/tmp/policy.pdf` and a text file.
    struct Form {
        presenter: DocumentFormPresenter,
        ports: DialogPorts,
        router: RecordingDocumentsRouter,
    }

    fn disk() -> InMemoryFiles {
        InMemoryFiles::default()
            .with("/tmp/policy.pdf", 512)
            .with("/tmp/notes.txt", 10)
    }

    fn add() -> Form {
        let ports = dialog_ports(disk());
        Form {
            presenter: DocumentFormPresenter::add(
                ports.clone(),
                LibraryScope::All,
                DialogKeys::default(),
            ),
            ports,
            router: RecordingDocumentsRouter::default(),
        }
    }

    impl Form {
        fn send(&mut self, event: DocumentFormEvent) {
            self.presenter.handle(event, &mut self.router);
        }
    }

    #[test]
    fn the_title_follows_the_path_until_it_is_typed_over() {
        let mut form = add();
        form.send(DocumentFormEvent::Path("/tmp/rates-notice.pdf".to_string()));
        assert_eq!(form.presenter.view_model().title.text, "rates-notice");

        form.send(DocumentFormEvent::Title("Rates notice".to_string()));
        form.send(DocumentFormEvent::Path("/tmp/policy.pdf".to_string()));
        assert_eq!(form.presenter.view_model().title.text, "Rates notice");
    }

    #[test]
    fn a_type_scope_prefills_the_type_and_a_year_scope_the_date() {
        let ports = dialog_ports(disk());
        let by_type = DocumentFormPresenter::add(
            ports.clone(),
            LibraryScope::Type(DocumentType::INSURANCE),
            DialogKeys::default(),
        );
        assert_eq!(
            by_type.view_model().doc_type.chosen,
            Some(DocumentType::INSURANCE)
        );

        let by_year = DocumentFormPresenter::add(
            ports.clone(),
            LibraryScope::Year(crate::YearFacet::Year(2024)),
            DialogKeys::default(),
        );
        let view = by_year.view_model();
        assert_eq!(view.doc_type.chosen, Some(DocumentType::OTHER));
        assert_eq!(
            parse_optional_date(&view.date.text, ports.today, None),
            Ok(NaiveDate::from_ymd_opt(2025, 6, 30))
        );
        assert_eq!(
            DocumentFormPresenter::add(ports, LibraryScope::All, DialogKeys::default())
                .view_model()
                .date
                .text,
            "today"
        );
    }

    #[test]
    fn a_missing_unsupported_or_duplicate_path_says_why_and_refuses() {
        let mut first = add();
        first.send(DocumentFormEvent::Path("/tmp/policy.pdf".to_string()));
        first.send(DocumentFormEvent::Confirm);
        let mut form = Form {
            presenter: DocumentFormPresenter::add(
                first.ports.clone(),
                LibraryScope::All,
                DialogKeys::default(),
            ),
            ports: first.ports,
            router: RecordingDocumentsRouter::default(),
        };
        for (typed, said) in [
            ("/tmp/missing.pdf", "There is no file at /tmp/missing.pdf."),
            (
                "/tmp/notes.txt",
                "Only PDF, JPG and PNG files can be Documents.",
            ),
            ("/tmp/policy.pdf", "Already in the Library as policy."),
        ] {
            form.send(DocumentFormEvent::Path(typed.to_string()));
            let view = form.presenter.view_model();
            assert_eq!(view.path_error.as_deref(), Some(said), "{typed}");
            assert!(!view.frame.can_confirm, "{typed}");
            form.send(DocumentFormEvent::Confirm);
            assert!(form.router.take().is_empty(), "{typed} refuses Confirm");
        }
    }

    #[test]
    fn an_add_files_the_document_says_so_and_shows_it() {
        let mut form = add();
        form.send(DocumentFormEvent::Path("/tmp/policy.pdf".to_string()));
        form.send(DocumentFormEvent::KeyKind(Some(KeyDateKind::Renews)));
        assert!(
            !form.presenter.view_model().frame.can_confirm,
            "a Key date kind needs its date"
        );
        form.send(DocumentFormEvent::KeyDate("2027-01-14".to_string()));
        form.send(DocumentFormEvent::ToggleReminder);

        form.send(DocumentFormEvent::Confirm);

        let added = form
            .ports
            .documents
            .borrow()
            .documents()
            .into_iter()
            .find(|document| document.title == "policy")
            .expect("the Document is added");
        assert!(added.is_filed());
        assert_eq!(added.bytes, 512);
        assert_eq!(
            added.key_date,
            Some(KeyDate {
                kind: KeyDateKind::Renews,
                date: NaiveDate::from_ymd_opt(2027, 1, 14).expect("a real date"),
                reminder: true,
            })
        );
        assert_eq!(
            form.router.take(),
            [
                DocumentsCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Added \"policy\".".to_string()
                )),
                DocumentsCall::CloseDialog(Some(DocumentsOutcome::Shown(added.id))),
            ]
        );
    }

    #[test]
    fn an_unreadable_date_says_why_and_a_blank_title_refuses() {
        let mut form = add();
        form.send(DocumentFormEvent::Path("/tmp/policy.pdf".to_string()));
        form.send(DocumentFormEvent::Date("not a date".to_string()));
        let view = form.presenter.view_model();
        assert!(view.date_error.is_some());
        assert!(!view.frame.can_confirm);

        form.send(DocumentFormEvent::Date("today".to_string()));
        form.send(DocumentFormEvent::Title("  ".to_string()));
        assert!(!form.presenter.view_model().frame.can_confirm);
    }

    #[test]
    fn tab_skips_the_key_date_fields_until_a_kind_is_chosen() {
        let mut form = add();
        form.send(DocumentFormEvent::Focus(DocumentField::Date));
        form.send(DocumentFormEvent::NextField);
        assert_eq!(form.presenter.view_model().focused, DocumentField::KeyKind);
        form.send(DocumentFormEvent::NextField);
        assert_eq!(form.presenter.view_model().focused, DocumentField::Path);

        form.send(DocumentFormEvent::KeyKind(Some(KeyDateKind::Ends)));
        form.send(DocumentFormEvent::NextField);
        assert_eq!(form.presenter.view_model().focused, DocumentField::KeyDate);
        form.send(DocumentFormEvent::NextField);
        assert_eq!(form.presenter.view_model().focused, DocumentField::Reminder);
    }

    #[test]
    fn an_edit_has_no_path_and_saves_in_place() {
        let ports = dialog_ports(disk());
        let id = world().file(POLICY).id;
        let mut form = Form {
            presenter: DocumentFormPresenter::edit(ports.clone(), id, DialogKeys::default())
                .expect("the policy is there"),
            ports,
            router: RecordingDocumentsRouter::default(),
        };
        let view = form.presenter.view_model();
        assert!(view.editing);
        assert!(view.path.is_none());
        assert_eq!(view.focused, DocumentField::Title);
        assert_eq!(view.frame.title, "Edit document");
        form.send(DocumentFormEvent::PrevField);
        assert_eq!(
            form.presenter.view_model().focused,
            DocumentField::Reminder,
            "back from Title wraps past the hidden Path"
        );

        form.send(DocumentFormEvent::Title("Car policy".to_string()));
        form.send(DocumentFormEvent::Confirm);

        let saved = document(&form.ports, id).expect("still there");
        assert_eq!(saved.title, "Car policy");
        assert_eq!(
            form.router.take(),
            [
                DocumentsCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Saved \"Car policy\".".to_string()
                )),
                DocumentsCall::CloseDialog(Some(DocumentsOutcome::Shown(id))),
            ]
        );
    }

    #[test]
    fn cancel_closes_having_written_nothing() {
        let mut form = add();
        form.send(DocumentFormEvent::Path("/tmp/policy.pdf".to_string()));
        let before = form.ports.documents.borrow().documents();

        form.send(DocumentFormEvent::Cancel);

        assert_eq!(form.router.take(), [DocumentsCall::CloseDialog(None)]);
        assert_eq!(form.ports.documents.borrow().documents(), before);
    }

    fn type_into(form: &mut Form, text: &str) {
        for ch in text.chars() {
            form.send(DocumentFormEvent::Input(DialogInput::Char(ch)));
        }
    }

    #[test]
    fn typing_fills_the_focused_text_field_and_the_title_follows_a_typed_path() {
        let mut form = add();
        type_into(&mut form, "/tmp/policy.pdf");
        form.send(DocumentFormEvent::Input(DialogInput::Backspace));
        type_into(&mut form, "f");
        let view = form.presenter.view_model();
        assert_eq!(view.path.expect("add has a path").text, "/tmp/policy.pdf");
        assert_eq!(view.title.text, "policy");

        form.send(DocumentFormEvent::Input(DialogInput::NextField));
        assert!(form.presenter.view_model().title.focused);
        type_into(&mut form, "!");
        form.send(DocumentFormEvent::Input(DialogInput::PreviousField));
        type_into(&mut form, "x");
        assert_eq!(
            form.presenter.view_model().title.text,
            "policy!",
            "typed over, the title no longer follows the path"
        );
    }

    #[test]
    fn the_type_select_opens_on_space_the_arrows_move_and_select_commits() {
        let mut form = add();
        form.send(DocumentFormEvent::Focus(DocumentField::Type));
        let chosen = form.presenter.view_model().doc_type.chosen;

        form.send(DocumentFormEvent::Input(DialogInput::Char(' ')));
        assert!(form.presenter.view_model().list.is_open());
        form.send(DocumentFormEvent::Input(DialogInput::SelectNext));
        assert_eq!(form.presenter.view_model().doc_type.chosen, chosen);
        form.send(DocumentFormEvent::Input(DialogInput::Select));

        let view = form.presenter.view_model();
        assert!(!view.list.is_open());
        assert_ne!(view.doc_type.chosen, chosen);
        assert!(
            form.router.calls.is_empty(),
            "committing a list is not confirming"
        );
        type_into(&mut form, "zz");
        assert_eq!(
            form.presenter.view_model().doc_type.chosen,
            view.doc_type.chosen,
            "a select takes no text"
        );
    }

    #[test]
    fn the_key_date_select_steps_and_shows_its_date_and_space_flips_the_reminder() {
        let mut form = add();
        form.send(DocumentFormEvent::Focus(DocumentField::KeyKind));
        form.send(DocumentFormEvent::Input(DialogInput::SelectNext));
        let view = form.presenter.view_model();
        assert_eq!(view.key_kind.chosen, Some(Some(KeyDateKind::Renews)));
        assert!(view.key_date.is_some());

        form.send(DocumentFormEvent::Input(DialogInput::NextField));
        form.send(DocumentFormEvent::Input(DialogInput::NextField));
        assert!(form.presenter.view_model().reminder.expect("shown").focused);
        form.send(DocumentFormEvent::Input(DialogInput::Char(' ')));
        assert!(form.presenter.view_model().reminder.expect("shown").checked);
    }

    #[test]
    fn select_confirms_a_valid_form_from_a_text_field_and_submit_from_a_select() {
        let mut form = add();
        form.send(DocumentFormEvent::Input(DialogInput::Select));
        assert!(form.router.calls.is_empty(), "no path yet");

        form.send(DocumentFormEvent::Path("/tmp/policy.pdf".to_string()));
        form.send(DocumentFormEvent::Focus(DocumentField::Type));
        form.send(DocumentFormEvent::Input(DialogInput::Submit));

        assert!(
            form.router.calls.iter().any(|call| matches!(
                call,
                DocumentsCall::CloseDialog(Some(DocumentsOutcome::Shown(_)))
            )),
            "{:?}",
            form.router.calls
        );
    }

    #[test]
    fn tab_and_a_click_close_an_open_list_and_close_list_leaves_the_choice() {
        let mut form = add();
        form.send(DocumentFormEvent::ToggleList(DocumentField::Type));
        assert!(form.presenter.view_model().list.is_open());
        form.send(DocumentFormEvent::ToggleList(DocumentField::KeyKind));
        let view = form.presenter.view_model();
        assert!(
            view.list.is_open() && view.key_kind.focused,
            "the other's list opens in its place"
        );

        form.send(DocumentFormEvent::CloseList);
        assert!(!form.presenter.view_model().list.is_open());
        form.send(DocumentFormEvent::ToggleList(DocumentField::KeyKind));
        form.send(DocumentFormEvent::Input(DialogInput::NextField));
        assert!(!form.presenter.view_model().list.is_open());
    }
}
