//! The Documents dialogs' live forms (`docs/ux/desktop/04-documents/`; the Add and Import decisions in
//! the Documents map's `+ Add and Import…` ticket): **Add document**, **Import files** and **Edit
//! document**. `gpui`-free and unit-tested, the same split `bills/form.rs` uses.
//!
//! - **Add** takes a typed Path, a Title that follows the file name until it is typed over, a
//!   Document Type, the document date (`today` when left alone) and an optional Key Date. Its Links
//!   come afterwards through the picker.
//! - **Import** takes one path per line. Paths that exist are imported and the rest reported.
//! - **Edit** is Add without the Path: a Filed Document's metadata is changed in place.
//! - Problems show inline and refuse Save: a missing file, an unsupported one, a path already in
//!   the Library, a blank Title and a date the parser rejects.

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use lib_core::DateStyle;
use lib_locale::format::format_date_input;

use crate::{
    accounts::SelectKey,
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    documents::types::DocumentTypeRow,
    documents::{
        self, Document, DocumentType, FileKind, KeyDate, KeyDateKind, LibraryScope, NewDocument,
    },
    form::field::TextField,
    form::select::SelectState,
    transactions::filter_form::parse_date,
};

/// The Add and Edit forms' fields, in `Tab` order. Edit has no Path.
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
    const ORDER: [DocumentField; 7] = [
        Self::Path,
        Self::Title,
        Self::Type,
        Self::Date,
        Self::KeyKind,
        Self::KeyDate,
        Self::Reminder,
    ];
}

/// The selects' option labels: the Ledger's Document Types (in Settings order) and the Key Date
/// kinds, "None" first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentOptions {
    pub type_labels: Vec<String>,
    /// The type behind each of `type_labels`.
    pub type_ids: Vec<DocumentType>,
    /// The Default type, which a form falls back to when it has no type of its own.
    pub fallback: DocumentType,
    pub key_labels: Vec<String>,
}

impl DocumentOptions {
    pub fn new(types: &[DocumentTypeRow], key_label: fn(Option<KeyDateKind>) -> String) -> Self {
        Self {
            type_labels: types.iter().map(|row| row.name.clone()).collect(),
            type_ids: types.iter().map(|row| DocumentType(row.id)).collect(),
            fallback: DocumentType(documents::types::fallback_id(types)),
            key_labels: std::iter::once(key_label(None))
                .chain(KEY_KINDS.into_iter().map(|kind| key_label(Some(kind))))
                .collect(),
        }
    }

    fn type_position(&self, kind: DocumentType) -> usize {
        self.type_ids
            .iter()
            .position(|candidate| *candidate == kind)
            .unwrap_or(0)
    }

    fn type_index(&self, label: Option<&str>) -> usize {
        self.type_labels
            .iter()
            .position(|candidate| Some(candidate.as_str()) == label)
            .unwrap_or(0)
    }

    fn key_index(&self, label: Option<&str>) -> usize {
        self.key_labels
            .iter()
            .position(|candidate| Some(candidate.as_str()) == label)
            .unwrap_or(0)
    }
}

const KEY_KINDS: [KeyDateKind; 4] = [
    KeyDateKind::Renews,
    KeyDateKind::Ends,
    KeyDateKind::Expires,
    KeyDateKind::Revalue,
];

/// A field's inline problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldProblem {
    NoPath,
    /// There is no file at the path, shown as typed.
    PathMissing(String),
    Unsupported,
    /// The Library holds this path already, under this Title.
    Already(String),
    NoTitle,
    /// A date the parser's own hint explains.
    Date(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormProblems {
    pub path: Option<FieldProblem>,
    pub title: Option<FieldProblem>,
    pub date: Option<FieldProblem>,
    pub key_date: Option<FieldProblem>,
}

impl FormProblems {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The Add and Edit forms' live state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentForm {
    pub path: TextField,
    pub title: TextField,
    /// Whether the Title has been typed in, which stops it following the file name.
    pub title_typed: bool,
    pub doc_type: SelectState,
    pub date: TextField,
    pub key_kind: SelectState,
    pub key_date: TextField,
    pub reminder: bool,
    pub is_edit: bool,
    pub focused: DocumentField,
    /// What the selects choose from, and what a date is read against, copied in as the Dialog
    /// opens.
    pub options: DocumentOptions,
    today: NaiveDate,
    date_style: Option<DateStyle>,
}

impl DocumentForm {
    /// A fresh Add form. A Type scope pre-fills the Type; a Financial Year scope pre-fills the date
    /// (today if inside that year, otherwise the year's last day); All pre-fills nothing.
    pub fn new(
        options: &DocumentOptions,
        scope: LibraryScope,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Self {
        let doc_type = match scope {
            LibraryScope::Type(kind) => kind,
            _ => options.fallback,
        };
        let date = documents::date_for_scope(scope, today);
        Self {
            path: TextField::default(),
            title: TextField::default(),
            title_typed: false,
            doc_type: SelectState::new(
                options
                    .type_labels
                    .get(options.type_position(doc_type))
                    .cloned(),
            ),
            date: TextField::new(if date == today {
                lib_locale::msg::date_word_today()
            } else {
                format_date_input(date, date_style)
            }),
            key_kind: SelectState::new(options.key_labels.first().cloned()),
            key_date: TextField::default(),
            reminder: false,
            is_edit: false,
            focused: DocumentField::Path,
            options: options.clone(),
            today,
            date_style,
        }
    }

    /// An Edit form pre-filled from `document`.
    pub fn for_edit(
        document: &Document,
        options: &DocumentOptions,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Self {
        let key = document.key_date.as_ref();
        Self {
            path: TextField::new(document.path.to_string_lossy().into_owned()),
            title: TextField::new(document.title.clone()),
            title_typed: true,
            doc_type: SelectState::new(
                options
                    .type_labels
                    .get(options.type_position(document.doc_type))
                    .cloned(),
            ),
            date: TextField::new(format_date_input(document.date, date_style)),
            key_kind: SelectState::new(
                options
                    .key_labels
                    .get(key.map_or(0, |key| 1 + key_position(key.kind)))
                    .cloned(),
            ),
            key_date: TextField::new(
                key.map(|key| format_date_input(key.date, date_style))
                    .unwrap_or_default(),
            ),
            reminder: key.is_some_and(|key| key.reminder),
            is_edit: true,
            focused: DocumentField::Title,
            options: options.clone(),
            today,
            date_style,
        }
    }

    pub fn doc_type(&self) -> DocumentType {
        self.options
            .type_ids
            .get(self.options.type_index(self.doc_type.value()))
            .copied()
            .unwrap_or(self.options.fallback)
    }

    pub fn key_kind(&self) -> Option<KeyDateKind> {
        self.options
            .key_index(self.key_kind.value())
            .checked_sub(1)
            .and_then(|index| KEY_KINDS.get(index).copied())
    }

    /// Whether `field` is on the form: Edit has no Path, and a Key Date's date and reminder only
    /// show once a kind is chosen.
    pub fn shows(&self, field: DocumentField) -> bool {
        match field {
            DocumentField::Path => !self.is_edit,
            DocumentField::KeyDate | DocumentField::Reminder => self.key_kind().is_some(),
            _ => true,
        }
    }

    /// The select behind `field` with the labels it lists.
    fn select_mut(&mut self, field: DocumentField) -> Option<(&mut SelectState, &[String])> {
        match field {
            DocumentField::Type => Some((&mut self.doc_type, &self.options.type_labels)),
            DocumentField::KeyKind => Some((&mut self.key_kind, &self.options.key_labels)),
            _ => None,
        }
    }

    pub fn is_select(field: DocumentField) -> bool {
        matches!(field, DocumentField::Type | DocumentField::KeyKind)
    }

    fn close_selects(&mut self) {
        self.doc_type.cancel();
        self.key_kind.cancel();
    }

    pub fn focus(&mut self, field: DocumentField) {
        if field != self.focused {
            self.close_selects();
        }
        self.focused = field;
    }

    /// `Tab` / `Shift-Tab`: commits an open list's highlight, then moves to the next shown field.
    pub fn cycle_focus(&mut self, backward: bool) {
        let field = self.focused;
        if let Some((state, list)) = self.select_mut(field) {
            state.commit(list);
        }
        let order = DocumentField::ORDER;
        let mut index = order.iter().position(|f| *f == field).unwrap_or(0);
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

    /// A key on the focused select. Returns whether a select is focused (and so took the key).
    pub fn handle_select_key(&mut self, key: SelectKey) -> bool {
        let field = self.focused;
        let Some((state, list)) = self.select_mut(field) else {
            return false;
        };
        match (key, state.is_open()) {
            (SelectKey::Up, true) => state.move_highlight(list, -1),
            (SelectKey::Down, true) => state.move_highlight(list, 1),
            (SelectKey::Up, false) => state.step(list, -1),
            (SelectKey::Down, false) => state.step(list, 1),
            (SelectKey::Activate, true) => state.commit(list),
            (SelectKey::Activate, false) => state.open(list),
        }
        true
    }

    /// A click on a select's closed field: focuses it and toggles its list.
    pub fn click_select(&mut self, field: DocumentField) {
        self.focus(field);
        if let Some((state, list)) = self.select_mut(field) {
            if state.is_open() {
                state.cancel();
            } else {
                state.open(list);
            }
        }
    }

    /// A click on row `index` of `field`'s open list.
    pub fn choose(&mut self, field: DocumentField, index: usize) {
        if let Some((state, list)) = self.select_mut(field) {
            state.choose(list, index);
        }
    }

    pub fn toggle_reminder(&mut self) {
        self.reminder = !self.reminder;
    }

    /// Until the Title is typed over, it is the typed path's file name without its extension.
    fn follow_path(&mut self) {
        if !self.title_typed {
            self.title =
                TextField::new(documents::default_title(Path::new(self.path.text().trim())));
        }
    }

    /// Every field's problem, or none. `library` is checked for a duplicate path (Add only) and
    /// `exists` is the file-system check, passed in so tests need no files.
    pub fn problems(&self, library: &[Document], exists: impl Fn(&Path) -> bool) -> FormProblems {
        let (today, date_style) = (self.today, self.date_style);
        let mut problems = FormProblems::default();
        if !self.is_edit {
            let typed = self.path.text().trim();
            problems.path = if typed.is_empty() {
                None
            } else {
                let path = documents::resolve_path(typed);
                if FileKind::from_path(&path).is_none() {
                    Some(FieldProblem::Unsupported)
                } else if let Some(existing) = documents::find_by_path(library, &path) {
                    Some(FieldProblem::Already(existing.title.clone()))
                } else if !exists(&path) {
                    Some(FieldProblem::PathMissing(typed.to_string()))
                } else {
                    None
                }
            };
        }
        problems.date = match parse_date(self.date.text(), today, date_style) {
            Err(hint) => Some(FieldProblem::Date(hint)),
            Ok(_) => None,
        };
        if self.key_kind().is_some() {
            problems.key_date = match parse_date(self.key_date.text(), today, date_style) {
                Err(hint) => Some(FieldProblem::Date(hint)),
                Ok(None) => Some(FieldProblem::Date(String::new())),
                Ok(Some(_)) => None,
            };
        }
        problems
    }

    /// Whether the form can be saved: nothing wrong, and the required fields filled.
    pub fn can_save(&self, problems: &FormProblems) -> bool {
        problems.is_empty() && !self.title.is_blank() && (self.is_edit || !self.path.is_blank())
    }

    /// Whether the fields that need nothing outside the form are filled and readable: the Title,
    /// the Path (Add), the dates. What a Path points at (a missing, unsupported or duplicate file)
    /// is `Shell`'s to check, with the Library and the disk, when the form is applied.
    pub fn is_complete(&self) -> bool {
        let readable = |field: &TextField| parse_date(field.text(), self.today, self.date_style);
        !self.title.is_blank()
            && (self.is_edit || !self.path.is_blank())
            && readable(&self.date).is_ok()
            && (self.key_kind().is_none() || matches!(readable(&self.key_date), Ok(Some(_))))
    }

    /// The form's values as a [`NewDocument`], or `None` while it cannot be saved.
    pub fn build(&self) -> Option<NewDocument> {
        let date = parse_date(self.date.text(), self.today, self.date_style)
            .ok()?
            .unwrap_or(self.today);
        Some(NewDocument {
            path: documents::resolve_path(self.path.text()),
            title: self.title.text().trim().to_string(),
            doc_type: self.doc_type(),
            date,
            key_date: self.key_date_value()?,
        })
    }

    /// The Key Date, `Some(None)` when there is none, and `None` when the chosen one is unreadable.
    pub fn key_date_value(&self) -> Option<Option<KeyDate>> {
        let Some(kind) = self.key_kind() else {
            return Some(None);
        };
        let date = parse_date(self.key_date.text(), self.today, self.date_style).ok()??;
        Some(Some(KeyDate {
            kind,
            date,
            reminder: self.reminder,
        }))
    }
}

fn key_position(kind: KeyDateKind) -> usize {
    KEY_KINDS
        .iter()
        .position(|candidate| *candidate == kind)
        .unwrap_or(0)
}

/// The open Documents dialog, if any. `NavState::mode` is `InputMode::Dialog` for exactly as long
/// as one is open, following the other pages' `*_dialog` fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentsDialog {
    Add(Box<DocumentForm>),
    Edit(u32, Box<DocumentForm>),
    /// The typed paths, and why the last attempt imported none of them.
    Import(ImportForm, Vec<String>),
    /// An Unfiled Document's Extracted Facts, edited from the Inbox.
    Facts(u32, Box<FactsForm>),
    /// The count-first confirm for Accept all strong matches: how many it will file.
    AcceptAll(usize),
    /// The link picker: link toggling, filing from the Inbox, or following one of several Links.
    Picker(Box<documents::picker::PickerState>),
}

/// The Import dialog's live state: one path per line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportForm {
    pub paths: TextField,
}

/// What an import made of one typed path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportOutcome {
    Imported(u32),
    NotFound(String),
    Unsupported(String),
    Already(String),
}

impl ImportForm {
    pub fn newline(&mut self) {
        let text = self.paths.text();
        if !text.is_empty() && !text.ends_with('\n') {
            self.paths.push('\n');
        }
    }

    /// The non-blank typed lines.
    pub fn lines(&self) -> Vec<&str> {
        self.paths
            .text()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect()
    }

    pub fn can_save(&self) -> bool {
        !self.lines().is_empty()
    }
}

/// Imports every typed path that exists into the Inbox, reporting what became of each. A path
/// repeated in the same batch is refused as already present, since the first copy was added.
pub fn import_all(
    types: &[DocumentTypeRow],
    form: &ImportForm,
    library: &mut Vec<Document>,
    today: NaiveDate,
    exists: impl Fn(&Path) -> bool,
) -> Vec<ImportOutcome> {
    import_typed(
        types,
        form.lines(),
        documents::Source::Import,
        library,
        today,
        exists,
    )
}

/// The same import for paths a drop delivered rather than typed, so a drop and `Import…` agree on
/// what is refused and why.
pub fn import_dropped(
    types: &[DocumentTypeRow],
    paths: &[PathBuf],
    library: &mut Vec<Document>,
    today: NaiveDate,
    exists: impl Fn(&Path) -> bool,
) -> Vec<ImportOutcome> {
    let typed: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
    import_typed(
        types,
        typed.iter().map(String::as_str),
        documents::Source::Dropped,
        library,
        today,
        exists,
    )
}

fn import_typed<'a>(
    types: &[DocumentTypeRow],
    lines: impl IntoIterator<Item = &'a str>,
    source: documents::Source,
    library: &mut Vec<Document>,
    today: NaiveDate,
    exists: impl Fn(&Path) -> bool,
) -> Vec<ImportOutcome> {
    lines
        .into_iter()
        .map(|typed| {
            let path: PathBuf = documents::resolve_path(typed);
            if FileKind::from_path(&path).is_none() {
                return ImportOutcome::Unsupported(typed.to_string());
            }
            if let Some(existing) = documents::find_by_path(library, &path) {
                return ImportOutcome::Already(existing.title.clone());
            }
            if !exists(&path) {
                return ImportOutcome::NotFound(typed.to_string());
            }
            match documents::import_path(types, library, path, today, source) {
                Ok(id) => ImportOutcome::Imported(id),
                Err(documents::AddError::AlreadyInLibrary(title)) => ImportOutcome::Already(title),
                Err(_) => ImportOutcome::Unsupported(typed.to_string()),
            }
        })
        .collect()
}

/// The Extracted Facts form's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FactsField {
    #[default]
    Merchant,
    Date,
    Total,
    Type,
}

impl FactsField {
    const ORDER: [FactsField; 4] = [Self::Merchant, Self::Date, Self::Total, Self::Type];
}

/// A fact's inline problem: only the date and the total are parsed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FactsProblems {
    /// The date parser's own hint.
    pub date: Option<String>,
    pub total: bool,
}

impl FactsProblems {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The Inbox's edit-facts form: merchant, date, total and Document Type. Any of the first three may
/// be left blank, and a blank total makes the Document Unreadable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactsForm {
    pub merchant: TextField,
    pub date: TextField,
    pub total: TextField,
    pub doc_type: SelectState,
    pub focused: FactsField,
    /// What the Type select chooses from, and what the date is read against, copied in as the
    /// Dialog opens.
    pub options: DocumentOptions,
    today: NaiveDate,
    date_style: Option<DateStyle>,
}

impl FactsForm {
    pub fn new(
        facts: &documents::ExtractedFacts,
        options: &DocumentOptions,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Self {
        let kind = facts.doc_type.unwrap_or(options.fallback);
        Self {
            merchant: TextField::new(facts.merchant.clone().unwrap_or_default()),
            date: TextField::new(
                facts
                    .date
                    .map(|date| format_date_input(date, date_style))
                    .unwrap_or_default(),
            ),
            total: TextField::new(
                facts
                    .total
                    .as_ref()
                    .map(|total| total.0.abs().with_scale(2).to_string())
                    .unwrap_or_default(),
            ),
            doc_type: SelectState::new(
                options
                    .type_labels
                    .get(options.type_position(kind))
                    .cloned(),
            ),
            focused: FactsField::Merchant,
            options: options.clone(),
            today,
            date_style,
        }
    }

    pub fn is_select(field: FactsField) -> bool {
        field == FactsField::Type
    }

    pub fn focus(&mut self, field: FactsField) {
        if field != self.focused {
            self.doc_type.cancel();
        }
        self.focused = field;
    }

    pub fn cycle_focus(&mut self, backward: bool) {
        if self.focused == FactsField::Type {
            self.doc_type.commit(&self.options.type_labels);
        }
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

    /// A key on the Type select. Returns whether the select is focused (and so took the key).
    pub fn handle_select_key(&mut self, key: SelectKey) -> bool {
        if self.focused != FactsField::Type {
            return false;
        }
        let list = &self.options.type_labels;
        let state = &mut self.doc_type;
        match (key, state.is_open()) {
            (SelectKey::Up, true) => state.move_highlight(list, -1),
            (SelectKey::Down, true) => state.move_highlight(list, 1),
            (SelectKey::Up, false) => state.step(list, -1),
            (SelectKey::Down, false) => state.step(list, 1),
            (SelectKey::Activate, true) => state.commit(list),
            (SelectKey::Activate, false) => state.open(list),
        }
        true
    }

    pub fn click_select(&mut self) {
        self.focus(FactsField::Type);
        if self.doc_type.is_open() {
            self.doc_type.cancel();
        } else {
            self.doc_type.open(&self.options.type_labels);
        }
    }

    pub fn choose(&mut self, index: usize) {
        self.doc_type.choose(&self.options.type_labels, index);
    }

    pub fn problems(&self) -> FactsProblems {
        FactsProblems {
            date: parse_date(self.date.text(), self.today, self.date_style).err(),
            total: !self.total.is_blank() && parse_total(self.total.text()).is_none(),
        }
    }

    pub fn can_save(&self, problems: &FactsProblems) -> bool {
        problems.is_empty()
    }

    /// The form's values as Extracted Facts, or `None` while a field cannot be read.
    pub fn build(&self) -> Option<documents::ExtractedFacts> {
        let options = &self.options;
        let merchant = self.merchant.text().trim();
        let total = self.total.text().trim();
        Some(documents::ExtractedFacts {
            merchant: (!merchant.is_empty()).then(|| merchant.to_string()),
            date: parse_date(self.date.text(), self.today, self.date_style).ok()?,
            total: if total.is_empty() {
                None
            } else {
                Some(parse_total(total)?)
            },
            doc_type: Some(
                options
                    .type_ids
                    .get(options.type_index(self.doc_type.value()))
                    .copied()
                    .unwrap_or(options.fallback),
            ),
        })
    }
}

/// The text fields are typed into by `dialog_host`; the Title following the Path is the one thing
/// they need doing around it.
impl Dialog for DocumentForm {
    /// `Shift-Tab` goes back a field. On a select `↑`/`↓` step or move the highlight, and `Space`
    /// and `Enter` open or commit its list; `Space` flips the reminder. Typing the Path keeps an
    /// untouched Title following the file name, and typing the Title stops that.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let focused = self.focused;
        let on_select = Self::is_select(focused);
        match key {
            DialogKey::BackTab => self.cycle_focus(true),
            DialogKey::Up if on_select => {
                self.handle_select_key(SelectKey::Up);
            }
            DialogKey::Down if on_select => {
                self.handle_select_key(SelectKey::Down);
            }
            DialogKey::Char(' ') | DialogKey::Enter if on_select => {
                self.handle_select_key(SelectKey::Activate);
            }
            DialogKey::Char(' ') if focused == DocumentField::Reminder => {
                self.reminder = !self.reminder
            }
            DialogKey::Char(ch) => match focused {
                DocumentField::Path => {
                    self.path.push(ch);
                    self.follow_path();
                }
                DocumentField::Title => {
                    self.title.push(ch);
                    self.title_typed = true;
                }
                DocumentField::Date | DocumentField::KeyDate => return None,
                _ => {}
            },
            DialogKey::Backspace => match focused {
                DocumentField::Path => {
                    self.path.backspace();
                    self.follow_path();
                }
                DocumentField::Title => {
                    self.title.backspace();
                    self.title_typed = true;
                }
                DocumentField::Date | DocumentField::KeyDate => return None,
                _ => {}
            },
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self.focused {
            DocumentField::Date => Some(&mut self.date),
            DocumentField::KeyDate => Some(&mut self.key_date),
            _ => None,
        }
    }

    fn cycle_field(&mut self) {
        self.cycle_focus(false);
    }

    fn is_valid(&self) -> bool {
        self.is_complete()
    }

    fn close_open_select(&mut self) -> bool {
        let open = self.doc_type.is_open() || self.key_kind.is_open();
        self.close_selects();
        open
    }
}

impl Dialog for FactsForm {
    /// `Shift-Tab` goes back a field. On the Type select `↑`/`↓` step or move the highlight, and
    /// `Space` and `Enter` open or commit its list. The other three fields take text.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let on_select = self.focused == FactsField::Type;
        match key {
            DialogKey::BackTab => self.cycle_focus(true),
            DialogKey::Up if on_select => {
                self.handle_select_key(SelectKey::Up);
            }
            DialogKey::Down if on_select => {
                self.handle_select_key(SelectKey::Down);
            }
            DialogKey::Char(' ') | DialogKey::Enter if on_select => {
                self.handle_select_key(SelectKey::Activate);
            }
            DialogKey::Char(_) | DialogKey::Backspace if on_select => {}
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self.focused {
            FactsField::Merchant => Some(&mut self.merchant),
            FactsField::Date => Some(&mut self.date),
            FactsField::Total => Some(&mut self.total),
            FactsField::Type => None,
        }
    }

    fn cycle_field(&mut self) {
        self.cycle_focus(false);
    }

    fn is_valid(&self) -> bool {
        self.can_save(&self.problems())
    }

    fn close_open_select(&mut self) -> bool {
        let open = self.doc_type.is_open();
        self.doc_type.cancel();
        open
    }
}

impl Dialog for DocumentsDialog {
    /// Import is a multi-line box: `Enter` types a newline and `Ctrl-Enter` imports, and any
    /// change clears the last attempt's notes. The count-first confirm takes `Enter` and swallows
    /// everything else.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match self {
            Self::Import(form, notes) => match key {
                DialogKey::CtrlEnter => Some(if form.can_save() {
                    DialogOutcome::Confirm
                } else {
                    DialogOutcome::Handled
                }),
                DialogKey::Enter => {
                    form.newline();
                    Some(DialogOutcome::Handled)
                }
                DialogKey::Backspace => {
                    form.paths.backspace();
                    notes.clear();
                    Some(DialogOutcome::Handled)
                }
                DialogKey::Char(ch) => {
                    if !ch.is_control() {
                        form.paths.push(ch);
                        notes.clear();
                    }
                    Some(DialogOutcome::Handled)
                }
                DialogKey::Tab | DialogKey::BackTab => Some(DialogOutcome::Handled),
                _ => None,
            },
            Self::AcceptAll(_) => match key {
                DialogKey::Enter => None,
                _ => Some(DialogOutcome::Handled),
            },
            Self::Picker(state) => state.handle_own_key(key),
            Self::Add(form) | Self::Edit(_, form) => form.handle_own_key(key),
            Self::Facts(_, form) => form.handle_own_key(key),
        }
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.focused_text(),
            Self::Facts(_, form) => form.focused_text(),
            Self::Import(..) | Self::AcceptAll(_) | Self::Picker(_) => None,
        }
    }

    fn cycle_field(&mut self) {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.cycle_field(),
            Self::Facts(_, form) => form.cycle_field(),
            Self::Import(..) | Self::AcceptAll(_) | Self::Picker(_) => {}
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.is_valid(),
            Self::Facts(_, form) => form.is_valid(),
            Self::Import(form, _) => form.can_save(),
            Self::AcceptAll(_) => true,
            Self::Picker(state) => state.is_valid(),
        }
    }

    fn close_open_select(&mut self) -> bool {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.close_open_select(),
            Self::Facts(_, form) => form.close_open_select(),
            Self::Import(..) | Self::AcceptAll(_) | Self::Picker(_) => false,
        }
    }
}

/// A typed total: digits with an optional decimal point, thousands commas and a leading currency
/// symbol or sign tolerated. Only the magnitude matters to matching, so the sign is dropped.
pub fn parse_total(text: &str) -> Option<lib_core::Money> {
    let cleaned: String = text
        .trim()
        .trim_start_matches(['-', '+', '$'])
        .chars()
        .filter(|ch| *ch != ',')
        .collect();
    if cleaned.is_empty() || !cleaned.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return None;
    }
    cleaned.parse::<lib_core::Money>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a valid test date")
    }

    fn options() -> DocumentOptions {
        DocumentOptions::new(&documents::types::default_types(), |kind| {
            kind.map_or("None".to_string(), |kind| format!("{kind:?}"))
        })
    }

    fn form() -> DocumentForm {
        DocumentForm::new(&options(), LibraryScope::All, day(2026, 9, 12), None)
    }

    /// Types `text` as a keyboard would: the form's own keys first, then the shared typing.
    fn type_text(dialog: &mut impl Dialog, text: &str) {
        for ch in text.chars() {
            crate::chrome::dialog_host::handle_key(dialog, DialogKey::Char(ch));
        }
    }

    #[test]
    fn title_follows_the_path_until_it_is_typed_over() {
        let mut form = form();
        type_text(&mut form, "/tmp/rates-notice.pdf");
        assert_eq!(form.title.text(), "rates-notice");

        form.focus(DocumentField::Title);
        type_text(&mut form, "!");
        form.focus(DocumentField::Path);
        type_text(&mut form, "x");
        assert_eq!(form.title.text(), "rates-notice!");
    }

    #[test]
    fn a_type_scope_prefills_the_type_and_a_year_scope_the_date() {
        let options = options();
        let by_type = DocumentForm::new(
            &options,
            LibraryScope::Type(DocumentType::INSURANCE),
            day(2026, 9, 12),
            None,
        );
        assert_eq!(by_type.doc_type(), DocumentType::INSURANCE);

        let by_year = DocumentForm::new(
            &options,
            LibraryScope::Year(documents::YearFacet::Year(2024)),
            day(2026, 9, 12),
            None,
        );
        assert_eq!(by_year.doc_type(), DocumentType::OTHER);
        assert_eq!(
            parse_date(by_year.date.text(), day(2026, 9, 12), None),
            Ok(Some(day(2025, 6, 30)))
        );
    }

    #[test]
    fn a_missing_unsupported_or_duplicate_path_is_a_problem() {
        let mut form = form();
        form.path = TextField::new("/tmp/missing.pdf");
        let problems = form.problems(&[], |_| false);
        assert_eq!(
            problems.path,
            Some(FieldProblem::PathMissing("/tmp/missing.pdf".to_string()))
        );

        form.path = TextField::new("/tmp/notes.txt");
        let problems = form.problems(&[], |_| true);
        assert_eq!(problems.path, Some(FieldProblem::Unsupported));

        form.path = TextField::new("/tmp/there.pdf");
        let problems = form.problems(&[], |_| true);
        assert!(problems.is_empty());
    }

    #[test]
    fn a_key_kind_demands_a_readable_key_date() {
        let mut form = form();
        form.title = TextField::new("Policy");
        form.path = TextField::new("/tmp/policy.pdf");
        form.key_kind = SelectState::new(Some("Renews".to_string()));

        let problems = form.problems(&[], |_| true);
        assert!(problems.key_date.is_some());
        assert!(!form.can_save(&problems));

        form.key_date = TextField::new("2027-01-14");
        let problems = form.problems(&[], |_| true);
        assert!(form.can_save(&problems));
        let new = form.build().expect("the form is complete");
        assert_eq!(
            new.key_date,
            Some(KeyDate {
                kind: KeyDateKind::Renews,
                date: day(2027, 1, 14),
                reminder: false,
            })
        );
    }

    #[test]
    fn tab_skips_the_key_date_fields_until_a_kind_is_chosen() {
        let mut form = form();
        form.focused = DocumentField::Date;
        form.cycle_focus(false);
        assert_eq!(form.focused, DocumentField::KeyKind);
        form.cycle_focus(false);
        assert_eq!(form.focused, DocumentField::Path);
    }

    #[test]
    fn edit_has_no_path_field() {
        let options = options();
        let document = Document {
            id: 1,
            path: PathBuf::from("/tmp/a.pdf"),
            kind: FileKind::Pdf,
            pages: 1,
            bytes: 0,
            title: "A".to_string(),
            doc_type: DocumentType::TAX,
            date: day(2026, 1, 2),
            key_date: None,
            links: Vec::new(),
            extracted_text: None,
            filing: documents::Filing::Filed,
            intake: None,
        };
        let mut form = DocumentForm::for_edit(&document, &options, day(2026, 9, 12), None);
        assert_eq!(form.focused, DocumentField::Title);
        assert!(!form.shows(DocumentField::Path));
        assert_eq!(form.doc_type(), DocumentType::TAX);
        form.cycle_focus(true);
        // Back from Title wraps past the hidden Path and Key Date fields to the Key Date kind.
        assert_eq!(form.focused, DocumentField::KeyKind);
    }

    #[test]
    fn import_reports_each_line() {
        let mut library = Vec::new();
        let form = ImportForm {
            paths: TextField::new(
                "/tmp/coles-receipt.jpg\n/tmp/none.pdf\n/tmp/readme.txt\n/tmp/coles-receipt.jpg\n",
            ),
        };
        let outcomes = import_all(
            &documents::types::default_types(),
            &form,
            &mut library,
            day(2026, 9, 12),
            |path| !path.ends_with("none.pdf"),
        );
        assert_eq!(
            outcomes,
            vec![
                ImportOutcome::Imported(1),
                ImportOutcome::NotFound("/tmp/none.pdf".to_string()),
                ImportOutcome::Unsupported("/tmp/readme.txt".to_string()),
                ImportOutcome::Already("coles-receipt".to_string()),
            ]
        );
        assert_eq!(library.len(), 1);
        assert!(library[0].is_unfiled());
        assert!(
            library[0]
                .intake
                .as_ref()
                .is_some_and(|intake| intake.facts.merchant.as_deref() == Some("Coles"))
        );
    }

    #[test]
    fn a_typed_total_tolerates_symbols_commas_and_a_sign() {
        let cents = |text: &str| parse_total(text).map(|money| money.0.abs().with_scale(2));
        let expected = Some(bigdecimal::BigDecimal::new(21_240.into(), 2));
        assert_eq!(cents("212.40"), expected);
        assert_eq!(cents("$212.40"), expected);
        assert_eq!(cents("-212.40"), expected);
        assert_eq!(
            cents("1,212.40"),
            Some(bigdecimal::BigDecimal::new(121_240.into(), 2))
        );
        assert_eq!(parse_total("twelve"), None);
        assert_eq!(parse_total(""), None);
        assert_eq!(parse_total("1.2.3"), None);
    }

    #[test]
    fn dropped_paths_land_in_the_inbox_as_dropped() {
        let mut library = Vec::new();
        let paths = [
            PathBuf::from("/tmp/coles-receipt.jpg"),
            PathBuf::from("/tmp/x.txt"),
        ];
        let outcomes = import_dropped(
            &documents::types::default_types(),
            &paths,
            &mut library,
            day(2026, 9, 12),
            |_| true,
        );
        assert_eq!(
            outcomes,
            vec![
                ImportOutcome::Imported(1),
                ImportOutcome::Unsupported("/tmp/x.txt".to_string()),
            ]
        );
        assert_eq!(
            library[0].intake.as_ref().map(|intake| intake.source),
            Some(documents::Source::Dropped)
        );
    }

    fn facts() -> documents::ExtractedFacts {
        documents::ExtractedFacts {
            merchant: Some("Woolworths Metro".to_string()),
            date: Some(day(2026, 9, 29)),
            total: parse_total("212.40"),
            doc_type: Some(DocumentType::RECEIPTS),
        }
    }

    #[test]
    fn the_facts_form_round_trips_what_was_read() {
        let options = options();
        let form = FactsForm::new(&facts(), &options, day(2026, 10, 2), Some(DateStyle::Iso));
        assert_eq!(form.total.text(), "212.40");
        let built = form.build().expect("a form of read facts builds");
        assert_eq!(built.merchant, facts().merchant);
        assert_eq!(built.date, facts().date);
        assert_eq!(built.doc_type, Some(DocumentType::RECEIPTS));
        assert_eq!(
            built.total.map(|total| total.0.with_scale(2)),
            facts().total.map(|total| total.0.with_scale(2))
        );
    }

    #[test]
    fn clearing_the_total_and_date_builds_blank_facts_and_a_bad_total_is_a_problem() {
        let options = options();
        let style = Some(DateStyle::Iso);
        let mut form = FactsForm::new(&facts(), &options, day(2026, 10, 2), style);
        form.total = TextField::default();
        form.date = TextField::default();
        form.merchant = TextField::new("   ");
        let built = form.build().expect("blank optional facts build");
        assert_eq!(built.total, None);
        assert_eq!(built.date, None);
        assert_eq!(built.merchant, None);
        assert!(form.problems().is_empty());

        form.total = TextField::new("abc");
        let problems = form.problems();
        assert!(problems.total);
        assert!(!form.can_save(&problems));
        assert!(form.build().is_none());
    }

    #[test]
    fn the_facts_form_tabs_through_its_four_fields_and_types_into_the_focused_one() {
        let options = options();
        let mut form = FactsForm::new(
            &documents::ExtractedFacts::default(),
            &options,
            day(2026, 10, 2),
            None,
        );
        assert_eq!(form.focused, FactsField::Merchant);
        type_text(&mut form, "A");
        form.cycle_focus(false);
        assert_eq!(form.focused, FactsField::Date);
        form.cycle_focus(false);
        type_text(&mut form, "9");
        assert_eq!(form.total.text(), "9");
        form.cycle_focus(false);
        assert_eq!(form.focused, FactsField::Type);
        form.cycle_focus(false);
        assert_eq!(form.focused, FactsField::Merchant);
        form.cycle_focus(true);
        assert_eq!(form.focused, FactsField::Type);
        assert_eq!(form.merchant.text(), "A");
    }
}
